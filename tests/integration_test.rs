use proven::arena::{CausalReflexionEngine, TriAgentArena};
use proven::ast_graph::{
    build_repo_graph_parallel, extract_scoped_context, extract_scoped_context_with_options,
    is_noisy_dir, is_safe_git_ref, parse_source_content, read_bounded_source_file,
    sanitize_repo_relative_path, scan_repository_changes, scan_repository_changes_with_options,
};
use proven::benchmark::BenchmarkEngine;
use proven::graph_rag::CodeGraphRAG;
use proven::memory_cls::GraphMemoryManager;
use proven::models::MutantEvalDetail;
use proven::mutation_gate::{
    sanitize_pytest_extra_args, FileRestoreGuard, MicroMutationGate,
};
use proven::oracle::{
    audit_changed_files_env_keys, infer_signature_invariants, is_secret_env_file,
};
use std::fs;
use std::panic;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_noisy_dir_hardware_filter() {
    assert!(is_noisy_dir(".git", ".git"));
    assert!(is_noisy_dir("node_modules", "frontend/node_modules"));
    assert!(is_noisy_dir("target", "rust/target"));
    assert!(is_noisy_dir(".venv", ".venv"));
    assert!(is_noisy_dir("venv_test", "venv_test"));
    assert!(is_noisy_dir("temp_dirty_data", "temp_dirty_data"));
    assert!(is_noisy_dir("mypkg.egg-info", "mypkg.egg-info"));
    assert!(!is_noisy_dir("src", "src"));
    assert!(!is_noisy_dir("payment", "src/payment"));
}

#[test]
fn test_polyglot_ast_parser_and_scoped_context() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    let src_dir = repo_root.join("src");
    fs::create_dir_all(&src_dir).unwrap();

    let py_file = src_dir.join("billing.py");
    let source = r#"def compute_tax(amount: int) -> int:
    if amount <= 0:
        return 0
    return amount // 10

def settle_invoice(subtotal: int, fee: int) -> int:
    if subtotal < 0 or fee < 0:
        raise ValueError("Invalid negative input")
    tax = compute_tax(subtotal)
    return subtotal + tax + fee
"#;
    fs::write(&py_file, source).unwrap();

    let parsed = parse_source_content(source, "src/billing.py", "python");
    assert_eq!(parsed.functions.len(), 2);
    assert!(parsed
        .functions
        .iter()
        .any(|f| f.name == "settle_invoice" && f.calls.contains(&"compute_tax".to_string())));

    let scoped = extract_scoped_context(
        repo_root,
        Some("FUNC:settle_invoice"),
        Some("src/billing.py"),
        true,
    );
    assert_eq!(scoped.status, "success");
    assert_eq!(scoped.track, "rust_native");
    assert!(scoped.target_id.contains("settle_invoice"));
    assert!(scoped.callees.iter().any(|c| c.contains("compute_tax")));
    if let Some(mem) = &scoped.repo_memory {
        assert!(mem.split_whitespace().count() < 80);
    }
}

#[test]
fn test_multiline_python_and_polyglot_brace_languages() {
    // 1. Multiline Python class method + async def
    let py_multiline = r#"class OrderProcessor:
    async def process_order(
        self,
        order_id: str,
        amount: float = 0.0,
        is_vip: bool = True,
    ) -> bool:
        """Process a customer order."""
        if amount <= 0:
            raise ValueError("Amount must be positive")
        self.total_processed += amount
        return True
"#;
    let parsed_py = parse_source_content(py_multiline, "orders.py", "python");
    assert_eq!(parsed_py.classes, vec!["OrderProcessor".to_string()]);
    assert_eq!(parsed_py.functions.len(), 1);
    let fn_span = &parsed_py.functions[0];
    assert_eq!(fn_span.function_id, "FUNC:OrderProcessor.process_order");
    assert_eq!(fn_span.docstring, "Process a customer order.");
    assert!(fn_span.signature.contains("order_id: str"));
    assert!(fn_span.signature.contains("is_vip: bool = True"));

    // Ensure MicroMutationGate skips the multiline signature (`is_vip: bool = True`)
    // and only mutates the body (`amount <= 0`, `self.total_processed += amount`, `return True`)
    let mutants =
        MicroMutationGate::collect_mutants_for_target(py_multiline, fn_span.start, fn_span.end, 10);
    assert!(!mutants.is_empty());
    assert!(
        !mutants.iter().any(|m| m.line <= 7),
        "Mutator must never mutate lines inside a multiline function signature"
    );
    assert!(mutants.iter().any(|m| m.persona == "Red1_BoundaryHunter"));
    assert!(mutants
        .iter()
        .any(|m| m.persona == "Red2_SideEffectNullifier" && m.mutation.contains("AugAdd")));

    // 2. Rust source
    let rs_src = r#"use std::collections::HashMap;
pub async fn verify_signature(payload: &str, key: &str) -> bool {
    if payload.is_empty() {
        return false;
    }
    compute_digest(payload, key)
}
"#;
    let parsed_rs = parse_source_content(rs_src, "src/auth.rs", "rust");
    assert_eq!(parsed_rs.functions.len(), 1);
    assert_eq!(parsed_rs.functions[0].function_id, "FUNC:verify_signature");
    assert!(parsed_rs.functions[0]
        .calls
        .contains(&"compute_digest".to_string()));

    // 3. Go source with pointer receiver method
    let go_src = r#"package billing
func (s *LedgerService) ReconcileAccount(balance int, delta int) int {
    return applyDelta(balance, delta)
}
"#;
    let parsed_go = parse_source_content(go_src, "ledger.go", "go");
    assert_eq!(parsed_go.functions.len(), 1);
    assert_eq!(parsed_go.functions[0].function_id, "FUNC:ReconcileAccount");
    assert!(parsed_go.functions[0]
        .calls
        .contains(&"applyDelta".to_string()));

    // 4. TypeScript source
    let ts_src = r#"import { hash } from './crypto';
export async function issueToken<T>(userId: string): Promise<string> {
    return hash(userId);
}
"#;
    let parsed_ts = parse_source_content(ts_src, "auth.ts", "typescript");
    assert_eq!(parsed_ts.functions.len(), 1);
    assert_eq!(parsed_ts.functions[0].function_id, "FUNC:issueToken");
    assert!(parsed_ts.functions[0].calls.contains(&"hash".to_string()));

    // 5. Java source
    let java_src = r#"import java.util.List;
public class TaxCalculator {
    public static int calculateVat(int netAmount) {
        return adjustRate(netAmount);
    }
}
"#;
    let parsed_java = parse_source_content(java_src, "TaxCalculator.java", "java");
    assert_eq!(parsed_java.functions.len(), 1);
    assert_eq!(parsed_java.functions[0].function_id, "FUNC:calculateVat");
}

#[test]
fn test_dirty_repo_noise_immunity() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();

    let clean_dir = repo_root.join("core");
    fs::create_dir_all(&clean_dir).unwrap();
    fs::write(
        clean_dir.join("engine.py"),
        "def run_engine(x: int) -> int:\n    return x + 1\n",
    )
    .unwrap();

    // Inject noisy directories
    let noisy_venv = repo_root.join(".venv").join("lib");
    let noisy_nm = repo_root.join("node_modules").join("pkg");
    let noisy_tmp = repo_root.join("temp_dirty");
    fs::create_dir_all(&noisy_venv).unwrap();
    fs::create_dir_all(&noisy_nm).unwrap();
    fs::create_dir_all(&noisy_tmp).unwrap();

    fs::write(noisy_venv.join("junk.py"), "def junk(:\n    return 0\n").unwrap();
    fs::write(noisy_nm.join("index.js"), "function junkJs() { return 0; }\n").unwrap();
    fs::write(noisy_tmp.join("dirty.py"), "def dirty():\n    return 0\n").unwrap();

    let graph = build_repo_graph_parallel(repo_root);
    assert!(graph.nodes.contains_key("FUNC:run_engine"));
    assert!(!graph.nodes.keys().any(|k| k.contains("junk")));
    assert!(!graph.nodes.keys().any(|k| k.contains("dirty")));
}

#[test]
fn test_topological_graph_rag_cyclic_and_missing_targets() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    let src_dir = repo_root.join("pkg");
    fs::create_dir_all(&src_dir).unwrap();

    // Create a cyclic call graph: alpha -> beta -> gamma -> alpha
    fs::write(
        src_dir.join("service.py"),
        r#"def alpha(x: int) -> int:
    if x <= 0:
        return 0
    return beta(x - 1)

def beta(y: int) -> int:
    return gamma(y)

def gamma(z: int) -> int:
    if z > 10:
        return alpha(z - 2)
    return z
"#,
    )
    .unwrap();

    let attn = CodeGraphRAG::extract_subgraph_attention(
        repo_root,
        "FUNC:alpha",
        3,
        6,
        "topological",
    );
    assert_eq!(attn.engine, "topological_ppr");
    assert!(attn.prompt_block.split_whitespace().count() <= 130);
    assert!(attn
        .attended_nodes
        .iter()
        .any(|n| n.symbol.contains("beta") || n.symbol.contains("gamma")));

    // Non-existent target should not panic
    let missing = CodeGraphRAG::extract_subgraph_attention(
        repo_root,
        "FUNC:non_existent_symbol",
        2,
        5,
        "topological",
    );
    assert_eq!(missing.total_subgraph_nodes, 0);

    let impact = CodeGraphRAG::predict_impact(repo_root, "FUNC:beta", "deterministic");
    assert_eq!(impact.status, "success");
    assert!(!impact.direct_dependents.is_empty());

    let triage = CodeGraphRAG::triage_bug(repo_root, "Bug in alpha and beta recursion", 3, 5);
    assert_eq!(triage.status, "success");
    assert!(!triage.related_files.is_empty());
}

#[test]
fn test_mutation_gate_all_3_red_personas_and_panic_restore_guard() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    let file_path = repo_root.join("vault.py");
    let original_code = r#"def withdraw(balance: int, amount: int, token=None) -> int:
    if token is None:
        raise ValueError("missing token")
    if amount <= 0 or amount > balance:
        return balance
    approved = True
    balance -= amount
    return balance if approved else 0
"#;
    fs::write(&file_path, original_code).unwrap();

    let mutants = MicroMutationGate::collect_mutants_for_target(original_code, 1, 9, 10);
    assert!(mutants.len() >= 3);
    assert!(mutants.iter().any(|m| m.persona == "Red3_ChaosPoisoner"));
    assert!(mutants.iter().any(|m| m.persona == "Red1_BoundaryHunter"));
    assert!(mutants
        .iter()
        .any(|m| m.persona == "Red2_SideEffectNullifier"));

    // Verify FileRestoreGuard restores modified file even if a panic occurs mid-flight!
    let path_clone = file_path.clone();
    let orig_clone = original_code.to_string();
    let res = panic::catch_unwind(move || {
        let _guard = FileRestoreGuard::new(path_clone.clone(), orig_clone);
        fs::write(&path_clone, "PANIC CORRUPTED FILE").unwrap();
        panic!("Simulated mid-mutation crash");
    });
    assert!(res.is_err());
    assert_eq!(
        fs::read_to_string(&file_path).unwrap(),
        original_code,
        "FileRestoreGuard MUST restore original source even on thread panic"
    );
}

#[test]
fn test_verify_proof_grades_and_tri_agent_arena_stop_conditions() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    fs::create_dir_all(repo_root.join("src")).unwrap();
    fs::create_dir_all(repo_root.join("tests")).unwrap();

    fs::write(
        repo_root.join("src/Transfer.py"),
        r#"def execute_transfer(balance: int, amount: int) -> int:
    if balance is None or amount is None:
        raise ValueError("Missing args")
    if amount <= 0:
        raise ValueError("Amount must be positive")
    if amount > balance:
        return -1
    return balance - amount
"#,
    )
    .unwrap();

    let test_file = repo_root.join("tests/test_transfer.py");

    // Case 1: FAILED (test fails at runtime)
    fs::write(
        &test_file,
        "def test_broken():\n    assert 1 == 2\n",
    )
    .unwrap();
    let res_failed = MicroMutationGate::verify_proof(
        repo_root,
        "FUNC:execute_transfer",
        "tests/test_transfer.py",
        Some("src/Transfer.py"),
        true,
        6,
        None,
    );
    assert_eq!(res_failed.proof_grade, "FAILED");
    assert_eq!(res_failed.proof_status, "fail");

    // Case 2: NONE (test passes, but never calls execute_transfer -> mock/noop escape)
    fs::write(
        &test_file,
        "def test_unrelated():\n    assert 1 + 1 == 2\n",
    )
    .unwrap();
    let res_none = MicroMutationGate::verify_proof(
        repo_root,
        "FUNC:execute_transfer",
        "tests/test_transfer.py",
        Some("src/Transfer.py"),
        true,
        6,
        None,
    );
    assert_eq!(res_none.proof_grade, "NONE");
    assert_eq!(res_none.proof_status, "fail");

    // Case 3: SILVER (test calls execute_transfer, but weak assertion lets mutants survive)
    fs::write(
        &test_file,
        r#"from src.Transfer import execute_transfer
def test_weak_smoke():
    res = execute_transfer(100, 20)
    assert res is not None
"#,
    )
    .unwrap();
    let res_silver = MicroMutationGate::verify_proof(
        repo_root,
        "FUNC:execute_transfer",
        "tests/test_transfer.py",
        Some("src/Transfer.py"),
        true,
        6,
        None,
    );
    assert_eq!(res_silver.proof_grade, "SILVER");
    assert!(res_silver.mutation_proof.as_ref().unwrap().mutants_survived > 0);

    // Arena Round 1 with weak test -> CONTINUE_NEXT_ROUND
    let r1 = TriAgentArena::referee_round(
        repo_root,
        "FUNC:execute_transfer",
        "tests/test_transfer.py",
        1,
        None,
    );
    assert!(!r1.is_titanium);
    assert_eq!(r1.stop_condition, "CONTINUE_NEXT_ROUND");
    assert!(r1.reflexion_prompt.is_some());

    // Arena Round 3 (MAX_ROUNDS) with weak test -> MAX_ROUNDS_EXCEEDED_VULNERABILITY
    let r3 = TriAgentArena::referee_round(
        repo_root,
        "FUNC:execute_transfer",
        "tests/test_transfer.py",
        3,
        None,
    );
    assert!(!r3.is_titanium);
    assert_eq!(r3.stop_condition, "MAX_ROUNDS_EXCEEDED_VULNERABILITY");

    // Case 4: TITANIUM (strong assertions kill all mutants)
    fs::write(
        &test_file,
        r#"import pytest
from src.Transfer import execute_transfer

def test_titanium_transfer():
    assert execute_transfer(100, 30) == 70
    assert execute_transfer(50, 50) == 0
    assert execute_transfer(50, 51) == -1
    with pytest.raises(ValueError):
        execute_transfer(None, 10)
    with pytest.raises(ValueError):
        execute_transfer(100, None)
    with pytest.raises(ValueError):
        execute_transfer(100, 0)
    with pytest.raises(ValueError):
        execute_transfer(100, -5)
"#,
    )
    .unwrap();
    let r2 = TriAgentArena::referee_round(
        repo_root,
        "FUNC:execute_transfer",
        "tests/test_transfer.py",
        2,
        None,
    );
    assert!(r2.is_titanium);
    assert_eq!(r2.proof_grade, "TITANIUM");
    assert_eq!(r2.stop_condition, "TITANIUM_VICTORY");
}

#[test]
fn test_cls_memory_and_cross_repo_meta_axiom_transfer() {
    let dir = tempdir().unwrap();
    let repo_a = dir.path().join("repo_a");
    let repo_b = dir.path().join("repo_b");
    fs::create_dir_all(&repo_a).unwrap();
    fs::create_dir_all(&repo_b).unwrap();

    for i in 0..2 {
        GraphMemoryManager::pin_lesson(
            &repo_a,
            "FUNC:calc",
            "Assert exact boundary equality (x == limit and x == 0)",
            "WEAK_ASSERTION",
            &format!("LtE -> Gt on line {}", i + 2),
        );
    }

    let sleep_res = GraphMemoryManager::consolidate_axioms(&repo_a, 1, true, true);
    assert_eq!(sleep_res.status, "success");
    assert!(sleep_res.axioms_count >= 1);
    assert!(sleep_res.promoted_meta_axioms >= 1);

    let reserve_code = r#"def reserve_stock(qty: int, available: int) -> int:
    if qty <= 0 or available < 0:
        raise ValueError("Invalid quantity")
    if qty > available:
        return 0
    return available - qty
"#;
    fs::write(repo_b.join("inventory.py"), reserve_code).unwrap();

    let bound = GraphMemoryManager::bind_meta_axioms_to_target(
        &repo_b,
        "FUNC:reserve_stock",
        reserve_code,
    );
    assert!(!bound.is_empty());

    let prompt = GraphMemoryManager::get_scoped_memory_prompt(&repo_b, "FUNC:reserve_stock")
        .expect("Expected scoped memory prompt");
    assert!(!prompt.is_empty());
    assert!(prompt.split_whitespace().count() < 80);
}

#[test]
fn test_full_4_regime_arena_init_solve_and_grade_100_percent() {
    let dummy_mut = MutantEvalDetail {
        mutant_id: 1,
        persona: "Red1_BoundaryHunter".to_string(),
        line: 5,
        mutation: "Lt -> GtE".to_string(),
        original_code: "if amount < 0:".to_string(),
        mutated_code: "if amount >= 0:".to_string(),
        status: "SURVIVED".to_string(),
        verdict: "SURVIVED".to_string(),
    };
    let report = CausalReflexionEngine::diagnose("FUNC:compute_final_price", &dummy_mut);
    assert_eq!(report.failure_mode, "WEAK_ASSERTION");
    assert!(report.reflexion_prompt.contains("CAUSAL REFLEXION"));

    let dir = tempdir().unwrap();
    let arena_ws = dir.path().join("arena_ws");
    let init_res = BenchmarkEngine::init_arena(&arena_ws);
    assert_eq!(init_res["status"], "success");

    // Initial grade before agent writes Titanium tests: 0/4 passed
    let initial_grade = BenchmarkEngine::grade_arena(&arena_ws, None);
    assert_eq!(initial_grade["scenarios_passed"], 0);

    // 1. Solve 01_small_slop_repo
    fs::write(
        arena_ws.join("01_small_slop_repo/tests/test_pricing_engine.py"),
        r#"import pytest
from src.pricing_engine import compute_final_price

def test_compute_final_price_titanium():
    assert compute_final_price(100.0, None, False) == 100.0
    assert compute_final_price(100.0, None, True) == 90.0
    assert compute_final_price(100.0, "FLAT20", False) == 80.0
    assert compute_final_price(99.0, "FLAT20", False) == 99.0
    assert compute_final_price(100.0, "HALF", False) == 50.0
    assert compute_final_price(0, None, False) == 0.0
    with pytest.raises(ValueError):
        compute_final_price(None)
    with pytest.raises(ValueError):
        compute_final_price(-1.0)
"#,
    )
    .unwrap();

    // 2. Solve 02_large_monorepo
    fs::write(
        arena_ws.join("02_large_monorepo/tests/test_quota_guard.py"),
        r#"import pytest
from packages.core_billing.quota_guard import enforce_quota

def test_enforce_quota_titanium():
    assert enforce_quota(50, 100, False) == "WITHIN_QUOTA"
    assert enforce_quota(0, 100, False) == "WITHIN_QUOTA"
    assert enforce_quota(100, 100, False) == "QUOTA_EXCEEDED"
    assert enforce_quota(100, 100, True) == "OVERAGE_ALLOWED"
    with pytest.raises(ValueError):
        enforce_quota(None, 100)
    with pytest.raises(ValueError):
        enforce_quota(10, None)
    with pytest.raises(ValueError):
        enforce_quota(10, 0)
    with pytest.raises(ValueError):
        enforce_quota(-1, 100)
"#,
    )
    .unwrap();

    // 3. Solve 03_dirty_zero_test
    fs::create_dir_all(arena_ws.join("03_dirty_zero_test/tests")).unwrap();
    fs::write(
        arena_ws.join("03_dirty_zero_test/tests/test_legacy_inventory.py"),
        r#"import pytest
from src.legacy_inventory import reserve_inventory

def test_reserve_inventory_titanium():
    assert reserve_inventory(10, 5, 5) is True
    assert reserve_inventory(9, 5, 5) is False
    assert reserve_inventory(5, 1, 5) is False
    with pytest.raises(ValueError):
        reserve_inventory(None, 5)
    with pytest.raises(ValueError):
        reserve_inventory(10, None)
    with pytest.raises(ValueError):
        reserve_inventory(10, 0)
    with pytest.raises(ValueError):
        reserve_inventory(-1, 5)
"#,
    )
    .unwrap();

    // 4. Solve 04_cross_repo_iot
    fs::write(
        arena_ws.join("04_cross_repo_iot/tests/test_patient_monitor.py"),
        r#"import pytest
from src.patient_monitor import PatientVitalsMonitor

def test_evaluate_heart_rate_titanium():
    mon = PatientVitalsMonitor()
    assert mon.evaluate_heart_rate("P1", 80, 140, 45) == "NORMAL"
    assert len(mon.alarms) == 0
    assert mon.evaluate_heart_rate("P1", 140, 140, 45) == "CRITICAL_ALARM"
    assert mon.alarms[-1] == ("P1", 140)
    assert mon.evaluate_heart_rate("P1", 45, 140, 45) == "CRITICAL_ALARM"
    assert len(mon.alarms) == 2
    with pytest.raises(ValueError):
        mon.evaluate_heart_rate(None, 80)
    with pytest.raises(ValueError):
        mon.evaluate_heart_rate("   ", 80)
    with pytest.raises(ValueError):
        mon.evaluate_heart_rate("P1", None)
    with pytest.raises(ValueError):
        mon.evaluate_heart_rate("P1", 0)
"#,
    )
    .unwrap();

    let report_md = arena_ws.join("SCORECARD.md");
    let final_grade = BenchmarkEngine::grade_arena(&arena_ws, report_md.to_str());
    assert_eq!(final_grade["scenarios_passed"], 4);
    assert_eq!(final_grade["scenarios_total"], 4);
    assert_eq!(final_grade["overall_verdict"], "TITANIUM_CERTIFIED");
    assert!(report_md.exists());
    let md_text = fs::read_to_string(&report_md).unwrap();
    assert!(md_text.contains("TITANIUM_CERTIFIED"));
}

#[test]
fn test_cli_skill_wrappers_end_to_end() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    fs::create_dir_all(repo_root.join("src")).unwrap();
    fs::write(
        repo_root.join("src/calc.py"),
        "def add_positive(a: int, b: int) -> int:\n    if a < 0 or b < 0:\n        raise ValueError('neg')\n    return a + b\n",
    )
    .unwrap();

    let out = Command::new("python")
        .args([
            "skills/proven/scripts/get_target_context.py",
            "--target",
            "FUNC:add_positive",
            "--file",
            "src/calc.py",
            "--path",
            &repo_root.to_string_lossy(),
        ])
        .output()
        .expect("Failed to run get_target_context.py wrapper");

    assert!(out.status.success());
    let json_val: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("Wrapper must emit valid JSON");
    assert_eq!(json_val["status"], "success");
    assert_eq!(json_val["target_id"], "FUNC:add_positive");
}

#[test]
fn test_security_path_traversal_and_escape_blocked() {
    let dir = tempdir().unwrap();
    let outer_dir = dir.path();
    let repo_root = outer_dir.join("repo");
    fs::create_dir_all(repo_root.join("src")).unwrap();

    let secret_file = outer_dir.join("secret_outside.py");
    let secret_content = "def secret_fn(x: int) -> int:\n    if x >= 0:\n        return x\n    return 0\n";
    fs::write(&secret_file, secret_content).unwrap();

    // Rejects parent traversal, absolute paths, drive letters, and null bytes
    assert!(sanitize_repo_relative_path(&repo_root, "../secret_outside.py").is_none());
    assert!(sanitize_repo_relative_path(&repo_root, "..\\secret_outside.py").is_none());
    assert!(sanitize_repo_relative_path(&repo_root, "src/../../secret_outside.py").is_none());
    assert!(sanitize_repo_relative_path(&repo_root, "/etc/passwd").is_none());
    assert!(sanitize_repo_relative_path(&repo_root, "C:\\Windows\\System32\\cmd.exe").is_none());
    assert!(sanitize_repo_relative_path(&repo_root, "src/file.py\0.txt").is_none());

    // Scoped context must reject path traversal
    let ctx = extract_scoped_context(
        &repo_root,
        Some("FUNC:secret_fn"),
        Some("../secret_outside.py"),
        false,
    );
    assert_eq!(ctx.status, "error");
    assert!(ctx.source_code.is_empty());

    // Mutation gate must reject path traversal and never modify files outside repo_root
    let mut_res = MicroMutationGate::evaluate_mutations(
        &repo_root,
        "FUNC:secret_fn",
        "../secret_outside.py",
        "tests/test_secret.py",
        5,
        None,
    );
    assert_eq!(mut_res.status, "error");
    assert_eq!(mut_res.mutation_gate_status, "rejected");
    assert_eq!(fs::read_to_string(&secret_file).unwrap(), secret_content);
}

#[test]
fn test_security_git_and_pytest_argument_injection_blocked() {
    // Git ref flag injection checks
    assert!(is_safe_git_ref("main"));
    assert!(is_safe_git_ref("HEAD~1"));
    assert!(is_safe_git_ref("feature/auth-v2"));
    assert!(!is_safe_git_ref("--output=/tmp/pwned"));
    assert!(!is_safe_git_ref("--ext-diff=sh"));
    assert!(!is_safe_git_ref("-o"));
    assert!(!is_safe_git_ref("main..HEAD"));
    assert!(!is_safe_git_ref(""));

    // Pytest extra_args sanitization checks
    let sanitized = sanitize_pytest_extra_args(Some(
        "-v -k test_valid -o cache_dir=/tmp -c evil.ini -p evil_plugin --rootdir=/ --maxfail=1",
    ));
    assert_eq!(
        sanitized,
        vec![
            "-v".to_string(),
            "-k".to_string(),
            "test_valid".to_string(),
            "--maxfail=1".to_string()
        ]
    );

    // verify_proof rejects test_target starting with `-` or containing `..`
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    let proof_flag = MicroMutationGate::verify_proof(
        repo_root,
        "FUNC:foo",
        "--version",
        None,
        false,
        1,
        None,
    );
    assert_eq!(proof_flag.proof_grade, "FAILED");
    assert!(proof_flag.message.contains("Security boundary violation"));
}

#[test]
fn test_utf8_multibyte_memory_truncation_no_panic() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();

    // Pin multi-byte UTF-8 lessons (Vietnamese + mathematical symbols) exceeding 260 bytes/chars
    let long_vn_lesson = "Kiểm tra biên nghiêm ngặt cho hàm thanh toán: số tiền >= 0, không vượt quá hạn mức tín dụng tối đa (x ≤ limit → trả về kết quả chính xác tuyệt đối), đồng thời kiểm tra ngoại lệ ValueError khi tham số đầu vào âm hoặc rỗng.";
    GraphMemoryManager::pin_lesson(
        repo_root,
        "FUNC:thanh_toan",
        long_vn_lesson,
        "WEAK_ASSERTION",
        "LtE -> Gt",
    );
    GraphMemoryManager::pin_lesson(
        repo_root,
        "FUNC:thanh_toan",
        "Đảm bảo trạng thái tài khoản được cập nhật chính xác sau giao dịch (self.balance -= amount).",
        "UNCHECKED_SIDE_EFFECT",
        "AugSub -> AugAdd",
    );

    // Must not panic on UTF-8 char boundary!
    let prompt = GraphMemoryManager::get_scoped_memory_prompt(repo_root, "FUNC:thanh_toan")
        .expect("Expected UTF-8 scoped memory prompt");
    assert!(prompt.ends_with("..."));
    assert!(prompt.chars().count() <= 260);
}

#[test]
fn test_dos_oversized_file_and_comment_string_mutation_immunity() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    fs::create_dir_all(repo_root.join("src")).unwrap();
    fs::create_dir_all(repo_root.join("tests")).unwrap();

    // 1. Oversized file (> 1 MB) must be skipped by read_bounded_source_file
    let giant_file = repo_root.join("src/giant_bundle.py");
    let giant_content = format!("def giant_fn():\n    x = '{}'\n", "a".repeat(1_200_000));
    fs::write(&giant_file, giant_content).unwrap();
    assert!(read_bounded_source_file(&giant_file).is_none());

    // 2. Operators inside string literals or inline comments must NOT be mutated
    let tricky_code = r#"def check_status(code: int) -> str:
    msg = "status >= 0 and code == 1 is True"  # comment with x >= 0 and y <= 1
    if code == 200:
        return msg
    return "ERR"
"#;
    let mutants = MicroMutationGate::collect_mutants_for_target(tricky_code, 1, 6, 10);
    assert_eq!(
        mutants.len(),
        1,
        "Only the real `if code == 200:` line should be mutated, never string literals or comments"
    );
    assert_eq!(mutants[0].line, 3);

    // 3. Test file that only mentions target inside a comment or in `def test_<target>` must get grade NONE
    fs::write(repo_root.join("src/status.py"), tricky_code).unwrap();
    fs::write(
        repo_root.join("tests/test_status.py"),
        r#"# TODO: call check_status(200) later
def test_check_status():
    msg = "check_status fake string"
    assert True
"#,
    )
    .unwrap();

    let fake_proof = MicroMutationGate::verify_proof(
        repo_root,
        "FUNC:check_status",
        "tests/test_status.py",
        Some("src/status.py"),
        true,
        5,
        None,
    );
    assert_eq!(
        fake_proof.proof_grade, "NONE",
        "Comments, string literals, and `def test_<target>` headers must not count as invoking the target"
    );
}

#[test]
fn test_multiline_docstring_trailing_comment_sig_and_ident_boundary() {
    let py_code = r#"class BillingEngine:
    def evaluate_credit(self, score: int) -> bool:  # entry signature comment
        """
        Multi-line docstring mentioning score >= 700 and True or False.
        Unindented line inside docstring:
score <= 0
        """
        isTrueCandidate = assertTrueHelper(score)
        if score >= 700:
            return isTrueCandidate
        return False

    def secondary_method(self, x: int) -> int:
        return x + 1
"#;

    let parsed = parse_source_content(py_code, "src/billing.py", "python");
    assert_eq!(parsed.functions.len(), 2);
    assert_eq!(
        parsed.functions[0].function_id,
        "FUNC:BillingEngine.evaluate_credit"
    );
    assert!(parsed.functions[0]
        .docstring
        .contains("Multi-line docstring mentioning score >= 700"));
    // Unindented line inside multiline triple-quoted docstring must not break class context!
    assert_eq!(
        parsed.functions[1].function_id,
        "FUNC:BillingEngine.secondary_method"
    );

    let mutants = MicroMutationGate::collect_mutants_for_target(
        py_code,
        parsed.functions[0].start,
        parsed.functions[0].end,
        10,
    );
    assert!(
        !mutants.is_empty(),
        "Trailing comment after `def ...:` must not prevent body mutant collection"
    );
    assert!(
        !mutants.iter().any(|m| m.line <= 7),
        "Multi-line docstring lines must never be mutated"
    );
    assert!(
        !mutants.iter().any(|m| m.line == 8),
        "`True` inside identifier `isTrueCandidate` or `assertTrueHelper` must never be mutated"
    );
    assert!(mutants.iter().any(|m| m.line == 9 && m.mutation.contains("GtE -> Lt")));
    assert!(mutants.iter().any(|m| m.line == 11 && m.mutation.contains("False -> True")));
}

#[test]
fn test_symbol_suffix_collision_and_intra_file_callers() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    fs::create_dir_all(repo_root.join("src")).unwrap();

    let code = r#"def pre_calculate_tax(amount: int) -> int:
    return amount + 100

def calculate_tax(amount: int) -> int:
    if amount <= 0:
        return 0
    return amount // 10

def settle_order(total: int) -> int:
    return total + calculate_tax(total)
"#;
    fs::write(repo_root.join("src/tax.py"), code).unwrap();

    let ctx = extract_scoped_context(
        repo_root,
        Some("FUNC:calculate_tax"),
        Some("src/tax.py"),
        true,
    );
    assert_eq!(ctx.status, "success");
    assert_eq!(ctx.signature, "def calculate_tax(amount: int)");
    assert!(
        !ctx.source_code.contains("pre_calculate_tax"),
        "FUNC:calculate_tax must never suffix-match pre_calculate_tax"
    );
    assert!(
        ctx.callers.contains(&"FUNC:settle_order".to_string()),
        "Intra-file caller FUNC:settle_order must be populated in callers"
    );
}

#[test]
fn test_brace_parser_unmatched_braces_in_strings_and_comments() {
    let rs_code = r#"pub fn parse_payload(input: &str) -> bool {
    let closing_brace_in_str = "}"; // trailing comment with } and fake_comment_call()
    /* block comment with } and fake_block_call() */
    if input.is_empty() {
        return false;
    }
    real_validator(input)
}
"#;
    let parsed = parse_source_content(rs_code, "src/parser.rs", "rust");
    assert_eq!(parsed.functions.len(), 1);
    let fn_span = &parsed.functions[0];
    assert_eq!(fn_span.start, 1);
    assert_eq!(
        fn_span.end, 8,
        "Braces inside string literals or comments must not truncate function end line"
    );
    assert_eq!(
        fn_span.calls,
        vec!["is_empty".to_string(), "real_validator".to_string()]
    );
    assert!(!fn_span.calls.contains(&"fake_comment_call".to_string()));
    assert!(!fn_span.calls.contains(&"fake_block_call".to_string()));
}

#[test]
fn test_multiline_import_and_docstring_spoofing_blocked() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    fs::create_dir_all(repo_root.join("src")).unwrap();
    fs::create_dir_all(repo_root.join("tests")).unwrap();

    let src_code = r#"def verify_token(token: str) -> bool:
    """Do not pass empty token."""
    return len(token) == 32
"#;
    // Docstring contains "not ", but executable code does not -> NULL_GUARD must not be falsely triggered
    let archetypes = GraphMemoryManager::detect_ast_archetypes(src_code);
    assert!(
        !archetypes.contains(&"NULL_GUARD".to_string()),
        "Docstring words must not trigger false AST archetypes"
    );

    fs::write(repo_root.join("src/token.py"), src_code).unwrap();

    // Test file imports verify_token in a multi-line import and mentions it in a multi-line docstring,
    // but never actually calls verify_token in executable code!
    let spoof_test = r#"from src.token import (
    verify_token,
)

def test_spoofed():
    """
    This docstring pretends to call verify_token("abcd")
    """
    assert 1 == 1
"#;
    fs::write(repo_root.join("tests/test_token.py"), spoof_test).unwrap();

    let proof = MicroMutationGate::verify_proof(
        repo_root,
        "FUNC:verify_token",
        "tests/test_token.py",
        Some("src/token.py"),
        true,
        5,
        None,
    );
    assert_eq!(proof.proof_status, "fail");
    assert_eq!(
        proof.proof_grade, "NONE",
        "Multi-line imports and multi-line docstrings must not spoof target invocation"
    );
}

#[test]
fn test_multiline_block_comments_and_fn_in_strings_immunity() {
    let rs_code = r#"pub struct PaymentGateway;

pub fn authorize_charge(amount: i32) -> bool {
    /* Multi-line block comment start:
     * Unmatched closing brace } inside multi-line comment!
     * fn fake_commented_fn(x: i32) {
     * fake_multiline_comment_call(amount);
     * if amount >= 0 { return True; }
     */
    let literal = "fn fake_string_fn(a: i32) { fake_str_call(a); }";
    if amount <= 0 {
        return false;
    }
    execute_gateway_charge(amount)
}
"#;
    let parsed = parse_source_content(rs_code, "src/gateway.rs", "rust");
    assert_eq!(parsed.classes, vec!["PaymentGateway".to_string()]);
    assert_eq!(
        parsed.functions.len(),
        1,
        "Functions inside multi-line block comments or string literals must never be parsed"
    );
    let fn_span = &parsed.functions[0];
    assert_eq!(fn_span.name, "authorize_charge");
    assert_eq!(fn_span.start, 3);
    assert_eq!(
        fn_span.end, 15,
        "Unmatched `}}` inside multi-line `/* ... */` comment must not truncate function end line"
    );
    assert_eq!(fn_span.calls, vec!["execute_gateway_charge".to_string()]);

    let mutants =
        MicroMutationGate::collect_mutants_for_target(rs_code, fn_span.start, fn_span.end, 10);
    assert_eq!(
        mutants.len(),
        2,
        "Operators inside multi-line `/* ... */` comments must never be mutated (only executable lines 11 and 12)"
    );
    assert_eq!(mutants[0].line, 11);
    assert_eq!(mutants[1].line, 12);
}

#[test]
fn test_csharp_region_directive_and_multiline_c_signature_support() {
    let cs_code = r#"namespace Demo {
    public class Hub {
        #region Message Types / Interfaces
        public bool Check(int x) {
            return x >= 10;
        }
        #endregion
    }
}
"#;
    let cs_muts = MicroMutationGate::collect_mutants_for_target(cs_code, 1, 9, 10);
    assert_eq!(
        cs_muts.len(),
        1,
        "C# `#region Message Types / Interfaces` directive must never be mutated"
    );
    assert_eq!(cs_muts[0].line, 5);

    let c_code = r#"unsigned char *
b64_decode (const char *src, size_t len) {
    if (len == 0) {
        return 0;
    }
    return 0;
}
"#;
    let parsed_c = parse_source_content(c_code, "decode.c", "c");
    assert_eq!(parsed_c.functions.len(), 1);
    assert_eq!(parsed_c.functions[0].name, "b64_decode");
}

#[test]
fn test_python_dedented_triple_quote_hash_default_arg_and_oneliner() {
    let py_code = r##"def parse_tag(tag: str, sep: str = "#", close: str = ")") -> str:
    r"""Raw docstring mentioning "tag >= 0" and fake_doc_call()."""
    def inner_identity(v: str) -> str: return v
    banner = """Triple-quoted single-line with "tag <= 0" and fake_banner_call()"""
    if len(tag) >= 3:
        return inner_identity(tag)
    return banner

QUERY_SQL = """
SELECT * FROM tags WHERE id >= 100 AND active = True
"""

def clamp_positive(x: int) -> int: return max_val(x)

def after_oneliner(y: int) -> int:
    if y > 0:
        return y
    return 0
"##;
    let parsed = parse_source_content(py_code, "src/tags.py", "python");
    assert_eq!(parsed.functions.len(), 4);

    let parse_tag_fn = &parsed.functions[0];
    assert_eq!(parse_tag_fn.name, "parse_tag");
    assert_eq!(
        parse_tag_fn.signature,
        "def parse_tag(tag: str, sep: str = \"#\", close: str = \")\")"
    );
    assert!(parse_tag_fn.docstring.contains("Raw docstring mentioning"));
    assert_eq!(
        parse_tag_fn.end, 7,
        "Module-level `QUERY_SQL = \"\"\"...\"\"\"` after function must not be swallowed into function span"
    );
    assert!(
        !parse_tag_fn.calls.contains(&"fake_doc_call".to_string())
            && !parse_tag_fn.calls.contains(&"fake_banner_call".to_string()),
        "Calls inside raw docstrings or single-line triple-quoted strings with inner quotes must be masked"
    );

    let mutants = MicroMutationGate::collect_mutants_for_target(
        py_code,
        parse_tag_fn.start,
        parse_tag_fn.end,
        10,
    );
    assert_eq!(
        mutants.len(),
        1,
        "Inner one-liner `def inner_identity` must not block mutation of subsequent lines, and triple-quoted strings with inner quotes must not be mutated"
    );
    assert_eq!(mutants[0].line, 5);

    let clamp_fn = parsed
        .functions
        .iter()
        .find(|f| f.name == "clamp_positive")
        .unwrap();
    assert_eq!(clamp_fn.signature, "def clamp_positive(x: int)");
    assert_eq!(clamp_fn.start, 13);
    assert_eq!(clamp_fn.end, 13);
    assert!(clamp_fn.calls.contains(&"max_val".to_string()));

    let after_fn = parsed
        .functions
        .iter()
        .find(|f| f.name == "after_oneliner")
        .unwrap();
    assert_eq!(after_fn.signature, "def after_oneliner(y: int)");
    assert_eq!(after_fn.start, 15);
    assert_eq!(after_fn.end, 18);
}

#[test]
fn test_backslash_import_spoofing_and_verify_proof_missing_target_rejected() {
    let dir = tempdir().unwrap();
    let outer = dir.path();
    let repo_root = outer.join("repo");
    fs::create_dir_all(repo_root.join("src")).unwrap();
    fs::create_dir_all(repo_root.join("tests")).unwrap();
    fs::write(outer.join("outside.py"), "def secret(): return 1\n").unwrap();

    fs::write(
        repo_root.join("src/token.py"),
        "unrelated_symbol = 1\ndef verify_token(token: str) -> bool:\n    if len(token) >= 8:\n        return True\n    return False\n",
    )
    .unwrap();

    // 3-line backslash import must not spoof invocation on line 3
    let py_spoof_test = "from src.token import \\\n    unrelated_symbol, \\\n    verify_token\n\ndef test_noop():\n    assert 1 == 1\n";
    let valid_test = r#"from src.token import verify_token
ghost_non_existent = lambda: True
def test_real():
    assert verify_token("12345678") is True
    assert ghost_non_existent() is True
"#;
    fs::write(repo_root.join("tests/test_spoof.py"), py_spoof_test).unwrap();
    fs::write(repo_root.join("tests/test_real.py"), valid_test).unwrap();

    let spoof_res = MicroMutationGate::verify_proof(
        &repo_root,
        "FUNC:verify_token",
        "tests/test_spoof.py",
        Some("src/token.py"),
        true,
        5,
        None,
    );
    assert_eq!(spoof_res.proof_status, "fail");
    assert_eq!(spoof_res.proof_grade, "NONE");

    // Unsafe source_file path traversal in verify_proof must be rejected immediately
    let unsafe_src = MicroMutationGate::verify_proof(
        &repo_root,
        "FUNC:verify_token",
        "tests/test_real.py",
        Some("../outside.py"),
        true,
        5,
        None,
    );
    assert_eq!(unsafe_src.proof_status, "fail");
    assert_eq!(unsafe_src.proof_grade, "FAILED");
    assert!(unsafe_src.message.contains("Security boundary violation"));

    // Non-existent function target in existing source file (even when invoked locally in test) must fail
    let ghost_fn = MicroMutationGate::verify_proof(
        &repo_root,
        "FUNC:ghost_non_existent",
        "tests/test_real.py",
        Some("src/token.py"),
        true,
        5,
        None,
    );
    assert_eq!(ghost_fn.proof_status, "fail");
    assert_eq!(ghost_fn.proof_grade, "FAILED");
    assert!(ghost_fn.message.contains("not found in source file"));
}

#[test]
fn test_cls_empirical_lesson_priority_and_scan_test_filter() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    fs::create_dir_all(repo_root.join("src")).unwrap();
    fs::create_dir_all(repo_root.join("tests")).unwrap();

    let code = r#"class QuotaService:
    def check_quota(self, used: int, limit: int) -> bool:
        if used is None or limit <= 0:
            raise ValueError("invalid")
        if used >= limit:
            self.rejected += 1
            return False
        return True
"#;
    fs::write(repo_root.join("src/quota.py"), code).unwrap();
    fs::write(
        repo_root.join("test_root_level.py"),
        "def test_root_helper():\n    assert True\n",
    )
    .unwrap();
    fs::write(
        repo_root.join("tests/test_quota.py"),
        "def test_quota_smoke():\n    assert True\n",
    )
    .unwrap();

    // 1. Extract context -> binds 4 generic JIT_META_AXIOM_BINDING lessons
    let _ctx = extract_scoped_context(
        repo_root,
        Some("FUNC:QuotaService.check_quota"),
        Some("src/quota.py"),
        false,
    );
    // 2. Pin a specific empirical surviving-mutant lesson
    GraphMemoryManager::pin_lesson(
        repo_root,
        "FUNC:QuotaService.check_quota",
        "Assert exact boundary used == limit at line 5.",
        "WEAK_ASSERTION",
        "GtE -> Lt",
    );

    let prompt =
        GraphMemoryManager::get_scoped_memory_prompt(repo_root, "FUNC:QuotaService.check_quota")
            .unwrap();
    assert!(
        prompt.contains("[WEAK_ASSERTION] Assert exact boundary used == limit"),
        "Empirical surviving-mutant lessons must take priority over generic JIT meta-axioms in <80 token prompt"
    );

    // 3. scan_repository_changes must exclude root-level test_*.py and tests/* from missing_coverage
    let scan = scan_repository_changes(repo_root, "main", "HEAD");
    let changed_files = scan["changed_files"].as_array().unwrap();
    assert_eq!(changed_files.len(), 1);
    assert_eq!(changed_files[0].as_str().unwrap(), "src/quota.py");
    let missing = scan["missing_coverage"].as_array().unwrap();
    assert_eq!(missing.len(), 1);
    assert_eq!(
        missing[0]["target_id"].as_str().unwrap(),
        "FUNC:QuotaService.check_quota"
    );
}

#[test]
fn test_empty_target_and_max_mutants_zero_and_file_target_verification() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    fs::create_dir_all(repo_root.join("src")).unwrap();
    fs::create_dir_all(repo_root.join("tests")).unwrap();

    let src_code = r#"def apply_fee(amount: int) -> int:
    if amount >= 100:
        return amount
    if amount <= 0:
        raise ValueError("invalid")
    return amount + 5
"#;
    fs::write(repo_root.join("src/fee.py"), src_code).unwrap();

    let weak_test = r#"from src.fee import apply_fee

def test_weak_fee():
    res = apply_fee(50)
    assert res > 0
"#;
    let titanium_test = r#"import pytest
from src.fee import apply_fee

def test_titanium_fee():
    with pytest.raises(ValueError):
        apply_fee(0)
    with pytest.raises(ValueError):
        apply_fee(-5)
    assert apply_fee(100) == 100
    assert apply_fee(99) == 104
"#;
    fs::write(repo_root.join("tests/test_weak.py"), weak_test).unwrap();
    fs::write(repo_root.join("tests/test_titanium.py"), titanium_test).unwrap();

    // 1. Empty target strings ("", "FUNC:") must never match \b\b or falsely pass
    let empty_ctx = extract_scoped_context(repo_root, Some("FUNC:"), Some("src/fee.py"), false);
    assert_eq!(empty_ctx.status, "error");

    let empty_attn =
        CodeGraphRAG::extract_subgraph_attention(repo_root, "FUNC:", 3, 5, "topological");
    assert!(empty_attn.attended_nodes.is_empty());

    let empty_proof = MicroMutationGate::verify_proof(
        repo_root,
        "FUNC:",
        "tests/test_titanium.py",
        Some("src/fee.py"),
        true,
        3,
        None,
    );
    assert_eq!(empty_proof.proof_status, "fail");
    assert_ne!(empty_proof.proof_grade, "TITANIUM");

    // 2. max_mutants = 0 must be clamped to >= 1 so weak tests cannot bypass the mutation gate
    let zero_mut_proof = MicroMutationGate::verify_proof(
        repo_root,
        "FUNC:apply_fee",
        "tests/test_weak.py",
        Some("src/fee.py"),
        true,
        0,
        None,
    );
    assert_eq!(
        zero_mut_proof.mutation_proof.as_ref().unwrap().mutants_total,
        1
    );
    assert_eq!(
        zero_mut_proof.proof_grade, "SILVER",
        "max_mutants=0 must not allow a weak test to bypass the mutation gate"
    );

    // 3. FILE: target must verify that a function defined in src/fee.py is invoked in the test
    let file_proof = MicroMutationGate::verify_proof(
        repo_root,
        "FILE:src/fee.py",
        "tests/test_titanium.py",
        Some("src/fee.py"),
        true,
        5,
        None,
    );
    assert_eq!(file_proof.proof_status, "pass");
    assert_eq!(file_proof.proof_grade, "TITANIUM");
}

#[test]
fn test_scan_fan_out_and_corrupted_memory_recovery_and_all_wrapper_schemas() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    fs::create_dir_all(repo_root.join("src")).unwrap();
    fs::create_dir_all(repo_root.join(".proven")).unwrap();

    fs::write(
        repo_root.join("src/discount.py"),
        "def compute_discount(price: int) -> int:\n    if price >= 50:\n        return price - 10\n    return price\n",
    )
    .unwrap();

    // 1. Corrupted .proven/memory.json and .proven/meta_axioms.json must recover gracefully without panic
    fs::write(repo_root.join(".proven/memory.json"), "{corrupted json!!!").unwrap();
    fs::write(
        repo_root.join(".proven/meta_axioms.json"),
        "[not valid json",
    )
    .unwrap();

    let lessons = GraphMemoryManager::get_pinned_lessons(repo_root, None);
    assert!(lessons.is_empty());
    let meta = GraphMemoryManager::load_meta_axioms(repo_root);
    assert!(!meta.is_empty());

    // 2. scan_repository_changes_with_options(..., true) must populate fan_out_tasks
    let scan_fan = scan_repository_changes_with_options(repo_root, "main", "HEAD", true);
    let fan_tasks = scan_fan["fan_out_tasks"]
        .as_array()
        .expect("fan_out_tasks must be present when fan_out=true");
    assert_eq!(fan_tasks.len(), 1);
    assert_eq!(fan_tasks[0]["target_id"], "FUNC:compute_discount");

    // 3. All 11 Python wrapper scripts must support --schema and return valid JSON with exit code 0
    let scripts = [
        "scan_impact.py",
        "get_target_context.py",
        "verify_runtime_proof.py",
        "explain_attention.py",
        "query_impact.py",
        "triage_expert.py",
        "run_arena.py",
        "refresh_runtime_map.py",
        "consolidate_memory.py",
        "commit_axiom.py",
        "run_benchmark.py",
    ];
    for script in scripts {
        let out = Command::new("python")
            .args([&format!("skills/proven/scripts/{}", script), "--schema"])
            .output()
            .unwrap_or_else(|_| panic!("Failed to run {} --schema", script));
        assert!(
            out.status.success(),
            "Wrapper {} --schema failed with stderr: {}",
            script,
            String::from_utf8_lossy(&out.stderr)
        );
        let parsed: serde_json::Value = serde_json::from_slice(&out.stdout)
            .unwrap_or_else(|_| panic!("Wrapper {} --schema did not emit valid JSON", script));
        assert_eq!(parsed["engine"], "proven");
    }
}

#[test]
fn test_python_module_dict_and_multiline_bracket_ast_not_misclassified_as_brace() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    fs::create_dir_all(repo_root.join("src")).unwrap();
    fs::create_dir_all(repo_root.join("tests")).unwrap();

    // Starts with a module-level dict `CONFIG = {` and semicolon statement before `class` / `def`,
    // plus an unindented multi-line list inside the method body.
    let py_src = r#"CONFIG = {
    "tax_rate": 2,
    "enabled": True,
}
SEED_A = 1; SEED_B = 2

class BillingEngine:
    def compute_tax(self, amount: int) -> int:
        tiers = [
0,
50,
100
        ]
        if amount >= 100:
            return amount * 2
        return amount + 1
"#;
    fs::write(repo_root.join("src/billing.py"), py_src.trim_start()).unwrap();

    assert!(
        !proven::ast_graph::is_brace_language_source(py_src.trim_start()),
        "Python file with module-level dict `CONFIG = {{` must not be misclassified as a brace language"
    );

    let parsed = parse_source_content(py_src.trim_start(), "src/billing.py", "python");
    assert_eq!(parsed.classes, vec!["BillingEngine".to_string()]);
    assert_eq!(parsed.functions.len(), 1);
    assert_eq!(parsed.functions[0].name, "BillingEngine.compute_tax");
    assert!(
        parsed.functions[0].end >= 15,
        "Unindented lines inside multi-line `[...]` must not prematurely truncate Python function span"
    );

    // Weak test that only checks `is not None` must NOT get TITANIUM
    let weak_test = r#"from src.billing import BillingEngine

def test_weak_billing():
    eng = BillingEngine()
    assert eng.compute_tax(120) is not None
"#;
    fs::write(repo_root.join("tests/test_billing.py"), weak_test).unwrap();

    let proof = MicroMutationGate::verify_proof(
        repo_root,
        "FUNC:BillingEngine.compute_tax",
        "tests/test_billing.py",
        Some("src/billing.py"),
        true,
        6,
        None,
    );
    let m_proof = proof
        .mutation_proof
        .expect("mutation_proof must be present");
    assert!(
        m_proof.mutants_total >= 2,
        "Mutants must be generated for Python function even when file starts with `CONFIG = {{`"
    );
    assert_eq!(
        proof.proof_grade, "SILVER",
        "Weak test must be graded SILVER, never falsely certified as TITANIUM"
    );
}

#[test]
fn test_ts_arrow_functions_methods_and_brace_mutation_operators() {
    let ts_src = r#"export const calcTotal = (price: number, qty: number): number => {
    let valid = true;
    if (price === 0) {
        return 0;
    }
    return price * qty;
};

export class CartService {
    validateItem(count: number): boolean {
        if (count >= 1) {
            return true;
        }
        return false;
    }
}
"#;
    let parsed = parse_source_content(ts_src, "src/cart.ts", "typescript");
    let fn_names: Vec<&str> = parsed.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(
        fn_names.contains(&"calcTotal"),
        "TS arrow function `export const calcTotal = (...) => {{` must be indexed: got {:?}",
        fn_names
    );
    assert!(
        fn_names.contains(&"validateItem"),
        "TS class method `validateItem(count: number): boolean {{` must be indexed: got {:?}",
        fn_names
    );

    let calc_fn = parsed
        .functions
        .iter()
        .find(|f| f.name == "calcTotal")
        .unwrap();
    let mutants =
        MicroMutationGate::collect_mutants_for_target(ts_src, calc_fn.start, calc_fn.end, 10);
    assert!(
        !mutants.is_empty(),
        "Mutants must be collected for TS arrow function"
    );
    for m in &mutants {
        assert!(
            !m.mutated_line.contains(" // "),
            "Brace language mutator must swap ` * ` to ` / `, never ` // ` line comment: got `{}`",
            m.mutated_line
        );
    }
    assert!(
        mutants.iter().any(|m| m.mutation.contains("bool: true -> false")),
        "Brace language mutator must mutate lowercase `= true` -> `= false`"
    );
    assert!(
        mutants.iter().any(|m| m.mutation.contains("StrictEq -> StrictNotEq")),
        "Brace language mutator must mutate `===` -> `!==`"
    );
    assert!(
        mutants.iter().any(|m| m.mutation.contains("Mult -> Div")),
        "Brace language mutator must mutate `*` -> `/`"
    );
}

#[test]
fn test_concurrent_mutation_gate_lock_prevents_race_and_corrupted_source() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    fs::create_dir_all(repo_root.join("src")).unwrap();
    fs::create_dir_all(repo_root.join("tests")).unwrap();

    let shared_src = r#"def calc_alpha(x: int) -> int:
    if x >= 10:
        return x + 5
    return x - 1

def calc_beta(y: int) -> int:
    if y >= 20:
        return y + 10
    return y - 2
"#;
    let test_src = r#"from src.shared import calc_alpha, calc_beta

def test_shared_both():
    assert calc_alpha(10) == 15
    assert calc_alpha(9) == 8
    assert calc_beta(20) == 30
    assert calc_beta(19) == 17
"#;
    fs::write(repo_root.join("src/shared.py"), shared_src).unwrap();
    fs::write(repo_root.join("tests/test_shared.py"), test_src).unwrap();

    std::thread::scope(|s| {
        let mut handles = Vec::new();
        for i in 0..4 {
            let target = if i % 2 == 0 {
                "FUNC:calc_alpha"
            } else {
                "FUNC:calc_beta"
            };
            handles.push(s.spawn(move || {
                MicroMutationGate::verify_proof(
                    repo_root,
                    target,
                    "tests/test_shared.py",
                    Some("src/shared.py"),
                    true,
                    6,
                    None,
                )
            }));
        }
        for h in handles {
            let res = h.join().expect("Worker thread panicked");
            assert_eq!(
                res.proof_grade, "TITANIUM",
                "Concurrent verify_proof failed due to mutation race: {}",
                res.message
            );
        }
    });

    let after_src = fs::read_to_string(repo_root.join("src/shared.py")).unwrap();
    assert_eq!(
        after_src, shared_src,
        "Source file must be identically restored after concurrent mutation gates"
    );
}

#[test]
fn test_nonexistent_target_in_scoped_context_and_verify_proof_rejected() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    fs::create_dir_all(repo_root.join("src")).unwrap();
    fs::create_dir_all(repo_root.join("tests")).unwrap();

    fs::write(
        repo_root.join("src/real.py"),
        "def real_fn(x: int) -> int:\n    return x + 1\n",
    )
    .unwrap();
    fs::write(
        repo_root.join("tests/test_real.py"),
        "from src.real import real_fn\n\ndef test_real():\n    assert real_fn(1) == 2\n",
    )
    .unwrap();

    let ghost_ctx =
        extract_scoped_context(repo_root, Some("FUNC:ghost_fn"), Some("src/real.py"), true);
    assert_eq!(
        ghost_ctx.status, "error",
        "Requesting a non-existent FUNC: target with --file must return status='error'"
    );
    assert!(
        !repo_root.join(".proven/memory.json").exists(),
        "Failed context lookup must not pollute .proven/memory.json"
    );

    let ghost_unprefixed_proof = MicroMutationGate::verify_proof(
        repo_root,
        "ghost_fn",
        "tests/test_real.py",
        Some("src/real.py"),
        true,
        4,
        None,
    );
    assert_eq!(ghost_unprefixed_proof.proof_status, "fail");
    assert_eq!(ghost_unprefixed_proof.proof_grade, "NONE");

    let ghost_class_proof = MicroMutationGate::verify_proof(
        repo_root,
        "CLASS:GhostClass",
        "tests/test_real.py",
        Some("src/real.py"),
        true,
        4,
        None,
    );
    assert_eq!(ghost_class_proof.proof_status, "fail");
    assert_eq!(ghost_class_proof.proof_grade, "NONE");
}

#[test]
fn test_nested_python_classes_and_exact_method_disambiguation() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    fs::create_dir_all(repo_root.join("src")).unwrap();

    let py_code = r#"class URL:
    class RegexMemoizer:
        def __init__(self):
            self._memoized = {}

    _regex = RegexMemoizer()

    def __call__(self, value: str) -> str:
        return value.strip()

class Range:
    def __init__(self, min_val: int, max_val: int):
        self.min = min_val
        self.max = max_val

    def __call__(self, value: int) -> int:
        if value < self.min or value > self.max:
            raise ValueError("Out of bounds")
        return value
"#;
    fs::write(repo_root.join("src/validate.py"), py_code).unwrap();

    let ctx = extract_scoped_context(
        repo_root,
        Some("FUNC:Range.__call__"),
        Some("src/validate.py"),
        true,
    );
    assert_eq!(ctx.status, "success");
    assert!(
        ctx.source_code.contains("Out of bounds"),
        "FUNC:Range.__call__ must match Range.__call__, not URL.__call__"
    );
    assert!(!ctx.source_code.contains("value.strip()"));
}

#[test]
fn test_brace_parser_skips_trait_prototypes_and_dts_declarations() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    fs::create_dir_all(repo_root.join("src")).unwrap();

    let rs_code = r#"pub trait Sealed {
    fn write(self, buf: &mut [u8]) -> &str;
}

impl Sealed for i32 {
    fn write(self, buf: &mut [u8]) -> &str {
        if self < 0 {
            buf[0] = b'-';
        }
        "ok"
    }
}
"#;
    fs::write(repo_root.join("src/lib.rs"), rs_code).unwrap();
    fs::write(
        repo_root.join("index.d.ts"),
        "export function parse(str: string): number;\n",
    )
    .unwrap();
    fs::write(
        repo_root.join("src/index.js"),
        "function parse(str) {\n  if (!str) return 0;\n  return str.length;\n}\n",
    )
    .unwrap();

    let ctx_rs = extract_scoped_context(repo_root, Some("FUNC:write"), Some("src/lib.rs"), false);
    assert_eq!(ctx_rs.status, "success");
    assert!(
        ctx_rs.source_code.contains("if self < 0"),
        "Rust parser must skip trait method prototype ending with ';' and extract the impl body"
    );

    let ctx_js = extract_scoped_context(repo_root, Some("FUNC:parse"), None, false);
    assert_eq!(ctx_js.status, "success");
    assert_eq!(
        ctx_js.source_file, "src/index.js",
        "TypeScript .d.ts declaration files must be skipped in favor of real implementation"
    );
    assert!(repo_root.join(".proven/ast_index.json").exists());
}

#[test]
fn test_oversized_file_excluded_from_changed_files_and_dead_branch_spoof_rejected() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    fs::create_dir_all(repo_root.join("src")).unwrap();
    fs::create_dir_all(repo_root.join("tests")).unwrap();

    fs::write(
        repo_root.join("src/calc.py"),
        "def compute_fee(x: int) -> int:\n    if x <= 0:\n        return 0\n    return x + 5\n",
    )
    .unwrap();

    // Create > 1.5 MB oversized file
    let huge = "x = 1\n".repeat(300_000);
    fs::write(repo_root.join("legacy_dump.py"), huge).unwrap();

    let scan = scan_repository_changes(repo_root, "main", "HEAD");
    let changed_files = scan["changed_files"].as_array().unwrap();
    assert!(
        !changed_files.iter().any(|f| f.as_str() == Some("legacy_dump.py")),
        "Oversized >1MB file must be excluded from changed_files"
    );

    // Dead-branch spoofing test (`if False`)
    fs::write(
        repo_root.join("tests/test_spoof.py"),
        "def test_spoof():\n    res = compute_fee(10) if False else 15\n    assert res == 15\n",
    )
    .unwrap();
    let proof = MicroMutationGate::verify_proof(
        repo_root,
        "FUNC:compute_fee",
        "tests/test_spoof.py",
        Some("src/calc.py"),
        true,
        3,
        None,
    );
    assert_eq!(proof.proof_status, "fail");
    assert_eq!(
        proof.proof_grade, "NONE",
        "Dead conditional branch `if False` must be rejected as target invocation spoofing"
    );
}

#[test]
fn test_memory_record_success_and_cross_repo_cls_transfer() {
    let dir_a = tempdir().unwrap();
    let dir_b = tempdir().unwrap();
    fs::create_dir_all(dir_a.path().join("src")).unwrap();
    fs::create_dir_all(dir_b.path().join("src")).unwrap();

    fs::write(
        dir_a.path().join("src/source.py"),
        "def validate_range(x: int, low: int, high: int) -> bool:\n    if x is None:\n        raise ValueError('null')\n    return x >= low and x <= high\n",
    )
    .unwrap();
    fs::write(
        dir_b.path().join("src/target.py"),
        "def check_bounds(v: int, a: int, b: int) -> bool:\n    if v is None:\n        raise ValueError('empty')\n    return v >= a and v <= b\n",
    )
    .unwrap();

    let rec = GraphMemoryManager::record_success(dir_a.path(), "FUNC:validate_range", 4, 4);
    assert_eq!(rec["status"], "success");
    let cls_path_a = dir_a.path().join(".proven/memory_cls.json");
    assert!(cls_path_a.exists());

    fs::create_dir_all(dir_b.path().join(".proven")).unwrap();
    fs::copy(&cls_path_a, dir_b.path().join(".proven/memory_cls.json")).unwrap();

    let ctx_b = extract_scoped_context(dir_b.path(), Some("FUNC:check_bounds"), None, true);
    assert_eq!(ctx_b.status, "success");
    assert!(!ctx_b.pinned_lessons.is_empty());
    assert!(ctx_b.repo_memory.is_some());
}

#[test]
fn test_logical_bitwise_membership_operators_and_compile_safety_guards() {
    // 1. Python logical connectors (`and`/`or`), membership (`in`), bitwise shifts & compound assignments,
    //    while skipping `for x in items`, comprehensions, DSL `task_a >> task_b`, and `* 1` / `* 0` / `<< 0`.
    let py_code = r#"def check_permissions(role: str, roles: set, flags: int, items: list) -> int:
    for item in items:
        pass
    filtered = [x for x in items if x in roles]
    pipeline_left >> pipeline_right
    noop_scale = flags * 1
    noop_zero = flags * 0
    noop_shift = flags << 0
    if role in roles and has_mfa:
        flags |= 4
        flags <<= 2
        return flags >> 1
    if is_guest or is_trial:
        flags &= 3
    return flags
"#;
    let py_muts = MicroMutationGate::collect_mutants_for_target(py_code, 1, 16, 25);
    assert!(
        !py_muts.iter().any(|m| m.line == 2 || m.line == 4),
        "`for item in items` and list comprehensions must never mutate `in` to `not in`: {:?}",
        py_muts
    );
    assert!(
        !py_muts.iter().any(|m| (5..=8).contains(&m.line)),
        "Standalone `a >> b` DSL statements and `* 1`, `* 0`, `<< 0` identities must be skipped: {:?}",
        py_muts
    );
    assert!(py_muts.iter().any(|m| m.line == 9 && m.mutation == "In -> NotIn"));
    assert!(py_muts.iter().any(|m| m.line == 9 && m.mutation == "And -> Or"));
    assert!(py_muts.iter().any(|m| m.line == 10 && m.mutation == "AugBitOr -> AugBitAnd"));
    assert!(py_muts.iter().any(|m| m.line == 11 && m.mutation == "AugShl -> AugShr"));
    assert!(py_muts.iter().any(|m| m.line == 12 && m.mutation == "Shr -> Shl"));
    assert!(py_muts.iter().any(|m| m.line == 13 && m.mutation == "Or -> And"));
    assert!(py_muts.iter().any(|m| m.line == 14 && m.mutation == "AugBitAnd -> AugBitOr"));

    // 2. Rust & C++ brace languages: `|| -> &&`, `*=/ /=`, `|=/ &=`, `<<=/ >>=`, `<< / >>`,
    //    while skipping Rust zero-arg closures (`|| 42`, `move || 42`) and C++ stream I/O (`cout <<`, `out <<`, `in >>`).
    let brace_code = r#"pub fn MixBits(mut acc: u32, ok_a: bool, ok_b: bool) -> u32 {
    let c1 = || 42;
    let c2 = move || 99;
    std::cout << acc << std::endl;
    out << acc;
    input_stream >> acc;
    let id1 = acc * 1;
    let id0 = acc / 1;
    let sh0 = acc << 0;
    if ok_a || ok_b {
        acc |= 8;
        acc <<= 2;
        acc *= 3;
        return acc << 1;
    }
    return acc;
}
"#;
    let brace_muts = MicroMutationGate::collect_mutants_for_target(brace_code, 1, 18, 25);
    assert!(
        !brace_muts.iter().any(|m| (2..=9).contains(&m.line)),
        "Rust closures, C++ streams, and `* 1` / `/ 1` / `<< 0` identities must never be mutated: {:?}",
        brace_muts
    );
    assert!(brace_muts.iter().any(|m| m.line == 10 && m.mutation == "Or -> And"));
    assert!(brace_muts.iter().any(|m| m.line == 11 && m.mutation == "AugBitOr -> AugBitAnd"));
    assert!(brace_muts.iter().any(|m| m.line == 12 && m.mutation == "AugShl -> AugShr"));
    assert!(brace_muts.iter().any(|m| m.line == 13 && m.mutation == "AugMult -> AugDiv"));
    assert!(brace_muts.iter().any(|m| m.line == 14 && m.mutation == "Shl -> Shr"));

    // 3. CausalReflexionEngine produces specific directives for logical connectors & membership checks
    let logic_detail = MutantEvalDetail {
        mutant_id: 1,
        persona: "Red3_ChaosPoisoner".to_string(),
        line: 9,
        mutation: "And -> Or".to_string(),
        original_code: "if is_admin and is_active:".to_string(),
        mutated_code: "if is_admin or is_active:".to_string(),
        status: "survived".to_string(),
        verdict: "SURVIVED".to_string(),
    };
    let diag = CausalReflexionEngine::diagnose("FUNC:check_permissions", &logic_detail);
    assert_eq!(diag.failure_mode, "WEAK_ASSERTION");
    assert!(diag.actionable_directive.contains("one sub-condition is true and the other is false"));
}

#[test]
fn test_whole_function_stratified_sampling_and_compound_line_verification() {
    // 1. Stratified even-stride sampling across a 15-branch function when max_mutants = 5
    let mut long_fn = String::from("def classify_code(x: int) -> int:\n");
    for i in 1..=15 {
        long_fn.push_str(&format!("    if x == {}:\n        return {}\n", i * 10, i));
    }
    long_fn.push_str("    return 0\n");
    let total_lines = long_fn.lines().count();

    let sampled = MicroMutationGate::collect_mutants_for_target(&long_fn, 1, total_lines, 5);
    assert_eq!(sampled.len(), 5);
    // First mutable `if x == 10:` is line 2; 15th mutable `if x == 150:` is line 30; `return 0` is line 32.
    assert_eq!(
        sampled.first().unwrap().line,
        2,
        "Stratified sampling must include the top guard of the function"
    );
    assert!(
        sampled.last().unwrap().line >= 30,
        "Stratified sampling must reach the bottom of long functions instead of truncating at line 10: got {:?}",
        sampled.iter().map(|m| m.line).collect::<Vec<_>>()
    );
    for w in sampled.windows(2) {
        assert!(w[0].line < w[1].line && w[0].mutant_id + 1 == w[1].mutant_id);
    }

    // 2. End-to-end runtime verification on a compound `>= ... or ...` condition: a partial test
    //    that only tests `(True, True)` and `(False, False)` kills the primary `GtE -> Lt` mutant
    //    but leaves the secondary `Or -> And` mutant alive (`SILVER`), while mixed truth-table
    //    assertions kill all primary and secondary mutants (`TITANIUM`).
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    fs::create_dir_all(repo_root.join("src")).unwrap();
    fs::create_dir_all(repo_root.join("tests")).unwrap();

    fs::write(
        repo_root.join("src/gate.py"),
        "def has_access(score: int, is_override: bool) -> bool:\n    if score >= 100 or is_override:\n        return True\n    return False\n",
    )
    .unwrap();

    fs::write(
        repo_root.join("tests/test_partial.py"),
        "from src.gate import has_access\n\ndef test_partial():\n    assert has_access(100, True) is True\n    assert has_access(50, False) is False\n",
    )
    .unwrap();
    let partial_proof = MicroMutationGate::verify_proof(
        repo_root,
        "FUNC:has_access",
        "tests/test_partial.py",
        Some("src/gate.py"),
        true,
        6,
        None,
    );
    assert_eq!(
        partial_proof.proof_grade, "SILVER",
        "Compound condition `score >= 100 or is_override` must leave secondary `Or -> And` alive when mixed truth-table cases are untested"
    );

    fs::write(
        repo_root.join("tests/test_full.py"),
        "from src.gate import has_access\n\ndef test_full():\n    assert has_access(100, False) is True\n    assert has_access(99, True) is True\n    assert has_access(99, False) is False\n",
    )
    .unwrap();
    let full_proof = MicroMutationGate::verify_proof(
        repo_root,
        "FUNC:has_access",
        "tests/test_full.py",
        Some("src/gate.py"),
        true,
        6,
        None,
    );
    assert_eq!(full_proof.proof_grade, "TITANIUM");
}

#[test]
fn test_pillar1_blind_spec_discovery_brace_docstrings_and_unverified_literals() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    fs::create_dir_all(repo_root.join("src")).unwrap();
    fs::create_dir_all(repo_root.join("pkg")).unwrap();
    fs::create_dir_all(repo_root.join("docs")).unwrap();

    fs::write(
        repo_root.join("README.md"),
        "# Billing Rules\n- `apply_vip_discount` grants a 15% maximum VIP discount on positive subtotals.\n",
    )
    .unwrap();
    fs::write(
        repo_root.join("docs/pricing_spec.md"),
        "Contract: apply_vip_discount must subtract exactly 15% from subtotal.\n",
    )
    .unwrap();
    fs::write(
        repo_root.join(".env"),
        "TOP_SECRET_API_KEY=DO_NOT_LEAK_THIS_SECRET_98765\n",
    )
    .unwrap();

    // Production implementation intentionally has an unverified/buggy 0.20 literal instead of 0.15
    fs::write(
        repo_root.join("src/pricing.py"),
        r#"def apply_vip_discount(subtotal: float) -> float:
    """Apply the 15% VIP loyalty discount to subtotal."""
    if subtotal <= 0.0:
        return 0.0
    return subtotal * (1.0 - 0.20)
"#,
    )
    .unwrap();
    fs::write(
        repo_root.join("src/checkout.py"),
        "from src.pricing import apply_vip_discount\n\ndef finalize_cart(subtotal: float) -> float:\n    return apply_vip_discount(subtotal)\n",
    )
    .unwrap();

    // Initialize git repository so git_commit spec discovery is exercised end-to-end
    let _ = Command::new("git")
        .args(["init"])
        .current_dir(repo_root)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_root)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Proven Test"])
        .current_dir(repo_root)
        .output();
    let _ = Command::new("git")
        .args(["add", "README.md", "docs/pricing_spec.md", "src/pricing.py", "src/checkout.py"])
        .current_dir(repo_root)
        .output();
    let _ = Command::new("git")
        .args([
            "commit",
            "-m",
            "feat(pricing): cap apply_vip_discount at 15% for VIP orders",
        ])
        .current_dir(repo_root)
        .output();

    // 1. Blind mode hides source_code, callees, graph_rag_prompt, unverified_literals,
    //    integration_slice, and concurrency_risk while discovering external specs and callers.
    let blind_ctx = extract_scoped_context_with_options(
        repo_root,
        Some("FUNC:apply_vip_discount"),
        Some("src/pricing.py"),
        true,
        true,
        Some("docs/pricing_spec.md"),
    );
    assert_eq!(blind_ctx.status, "success");
    assert!(blind_ctx.blind_mode);
    assert!(
        blind_ctx.source_code.is_empty(),
        "--blind must never expose implementation source_code"
    );
    assert!(blind_ctx.callees.is_empty());
    assert!(blind_ctx.graph_rag_prompt.is_none());
    assert!(blind_ctx.unverified_literals.is_empty());
    assert!(blind_ctx.integration_slice.is_none());
    assert!(blind_ctx.concurrency_risk.is_none());
    assert!(
        blind_ctx.callers.iter().any(|c| c == "FUNC:finalize_cart"),
        "Cross-file production caller must be discovered in --blind mode: {:?}",
        blind_ctx.callers
    );
    assert!(blind_ctx.discovered_specs.len() <= 6);
    for s in &blind_ctx.discovered_specs {
        assert!(s.excerpt.chars().count() <= 240);
    }
    let kinds: Vec<&str> = blind_ctx
        .discovered_specs
        .iter()
        .map(|s| s.source.as_str())
        .collect();
    assert!(kinds.contains(&"explicit_spec"), "got specs: {:?}", blind_ctx.discovered_specs);
    assert!(kinds.contains(&"docstring"), "got specs: {:?}", blind_ctx.discovered_specs);
    assert!(kinds.contains(&"repo_doc"), "got specs: {:?}", blind_ctx.discovered_specs);
    assert!(kinds.contains(&"git_commit"), "got specs: {:?}", blind_ctx.discovered_specs);
    assert!(kinds.contains(&"caller_usage"), "got specs: {:?}", blind_ctx.discovered_specs);

    // 2. Passing `--spec .env` must never read secret `.env` contents
    assert!(is_secret_env_file(".env"));
    assert!(is_secret_env_file(".env.production"));
    assert!(!is_secret_env_file(".env.example"));
    let secret_attempt = extract_scoped_context_with_options(
        repo_root,
        Some("FUNC:apply_vip_discount"),
        Some("src/pricing.py"),
        false,
        true,
        Some(".env"),
    );
    let serialized_attempt = serde_json::to_string(&secret_attempt).unwrap();
    assert!(
        !serialized_attempt.contains("DO_NOT_LEAK_THIS_SECRET_98765"),
        "--spec .env must never read or leak secret .env values"
    );

    // 3. Full mode flags 0.20 as an unverified business literal (contradicts 15% spec)
    let full_ctx = extract_scoped_context_with_options(
        repo_root,
        Some("FUNC:apply_vip_discount"),
        Some("src/pricing.py"),
        true,
        false,
        None,
    );
    assert!(!full_ctx.blind_mode);
    assert!(full_ctx.source_code.contains("0.20"));
    assert!(
        full_ctx
            .unverified_literals
            .iter()
            .any(|u| u.literal == "0.20"),
        "0.20 contradicts the 15% external spec and must be flagged in unverified_literals: {:?}",
        full_ctx.unverified_literals
    );

    // 4. Negative check: 0.15 backed by `15%` spec, module-level UPPER_SNAKE_CASE constants,
    //    structural -1/0/1/2, and bitshift widths (8/16/32/64) must produce 0 false positives.
    fs::write(
        repo_root.join("src/pricing.py"),
        r#"FALLBACK_CAP = 250.0

def apply_vip_discount(subtotal: float) -> float:
    """Apply the 15% VIP loyalty discount to subtotal."""
    flags = (1 << 16) >> 8
    if subtotal <= 0.0 or flags == -1:
        return 0.0
    discounted = subtotal * (1.0 - 0.15)
    return min(discounted, FALLBACK_CAP)
"#,
    )
    .unwrap();
    let clean_ctx = extract_scoped_context(
        repo_root,
        Some("FUNC:apply_vip_discount"),
        Some("src/pricing.py"),
        false,
    );
    assert!(
        clean_ctx.unverified_literals.is_empty(),
        "Spec-backed 0.15 (15%), UPPER_SNAKE_CASE constants, structural values, and bitshift widths must not trigger false positives: {:?}",
        clean_ctx.unverified_literals
    );

    // 5. Polyglot brace docstring extraction across Rust (`///` above `#[inline]`), TS (`/** */`), and Go (`//`)
    fs::write(
        repo_root.join("src/discounts.rs"),
        "/// Applies 15% tier discount to amount.\n/// Clamps negative values to zero.\n#[inline]\npub fn calc_rust_discount(amount: i64) -> i64 {\n    if amount <= 0 { return 0; }\n    amount * 85 / 100\n}\n",
    )
    .unwrap();
    fs::write(
        repo_root.join("src/discounts.ts"),
        "/**\n * Computes JSDoc 15% rebate for positive totals.\n */\nexport function calcTsRebate(amount: number): number {\n    return amount > 0 ? amount * 0.15 : 0;\n}\n",
    )
    .unwrap();
    fs::write(
        repo_root.join("pkg/discounts.go"),
        "package pkg\n\n// CalcGoFee charges a 15% platform fee.\nfunc CalcGoFee(amount int) int {\n    if amount <= 0 {\n        return 0\n    }\n    return amount * 15 / 100\n}\n",
    )
    .unwrap();

    let rs_ctx = extract_scoped_context_with_options(
        repo_root,
        Some("FUNC:calc_rust_discount"),
        Some("src/discounts.rs"),
        false,
        true,
        None,
    );
    assert!(
        rs_ctx.docstring.contains("Applies 15% tier discount"),
        "Rust /// doc comment above #[inline] must be extracted: {:?}",
        rs_ctx.docstring
    );
    let ts_ctx = extract_scoped_context_with_options(
        repo_root,
        Some("FUNC:calcTsRebate"),
        Some("src/discounts.ts"),
        false,
        true,
        None,
    );
    assert!(
        ts_ctx.docstring.contains("Computes JSDoc 15% rebate"),
        "TypeScript JSDoc must be extracted: {:?}",
        ts_ctx.docstring
    );
    let go_ctx = extract_scoped_context_with_options(
        repo_root,
        Some("FUNC:CalcGoFee"),
        Some("pkg/discounts.go"),
        false,
        true,
        None,
    );
    assert!(
        go_ctx.docstring.contains("CalcGoFee charges a 15% platform fee"),
        "Go doc comment must be extracted: {:?}",
        go_ctx.docstring
    );
}

#[test]
fn test_pillar2_sibling_guard_omission_and_signature_domain_probing() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    fs::create_dir_all(repo_root.join("src")).unwrap();

    // Python payment module: withdraw and transfer enforce `user is None`, `user.is_locked`,
    // and `ensure_kyc_verified(user)`, whereas `pay_invoice` omits `user.is_locked` and `ensure_kyc_verified`.
    fs::write(
        repo_root.join("src/payments.py"),
        r#"def ensure_kyc_verified(user) -> None:
    if not getattr(user, "kyc_ok", False):
        raise PermissionError("KYC required")

def withdraw(user, amount: float) -> float:
    if user is None:
        raise ValueError("user required")
    if user.is_locked:
        raise PermissionError("locked account")
    ensure_kyc_verified(user)
    if amount <= 0:
        raise ValueError("amount must be positive")
    return user.balance - amount

def transfer(user, amount: float, recipient) -> float:
    if user is None:
        raise ValueError("user required")
    if user.is_locked:
        raise PermissionError("locked account")
    ensure_kyc_verified(user)
    if amount <= 0:
        raise ValueError("amount must be positive")
    return user.balance - amount

def pay_invoice(user, amount: float) -> float:
    if user is None:
        raise ValueError("user required")
    if amount <= 0:
        raise ValueError("amount must be positive")
    return user.balance - amount
"#,
    )
    .unwrap();

    let py_blind = extract_scoped_context_with_options(
        repo_root,
        Some("FUNC:pay_invoice"),
        Some("src/payments.py"),
        false,
        true,
        None,
    );
    assert_eq!(py_blind.status, "success");
    assert!(
        py_blind
            .missing_sibling_guards
            .iter()
            .any(|g| g.guard_pattern.contains("is_locked")
                && g.observed_in_siblings.contains(&"FUNC:withdraw".to_string())
                && g.observed_in_siblings.contains(&"FUNC:transfer".to_string())),
        "Missing user.is_locked sibling guard must be detected: {:?}",
        py_blind.missing_sibling_guards
    );
    assert!(
        py_blind
            .missing_sibling_guards
            .iter()
            .any(|g| g.guard_pattern.contains("ensure_kyc_verified")),
        "Missing ensure_kyc_verified sibling helper call must be detected: {:?}",
        py_blind.missing_sibling_guards
    );
    assert!(
        !py_blind
            .missing_sibling_guards
            .iter()
            .any(|g| g.guard_pattern == "user is None"),
        "Guards already present in pay_invoice (`user is None`) must not be flagged as missing: {:?}",
        py_blind.missing_sibling_guards
    );
    assert!(
        py_blind
            .inferred_invariants
            .iter()
            .any(|inv| inv.contains("'user'") && inv.contains("locked")),
        "Entity domain param `user` must generate locked/restricted state probes: {:?}",
        py_blind.inferred_invariants
    );
    assert!(
        py_blind
            .inferred_invariants
            .iter()
            .any(|inv| inv.contains("'amount'") && inv.contains("0") && inv.contains("-1")),
        "Numeric param `amount` must generate 0 and -1 boundary probes: {:?}",
        py_blind.inferred_invariants
    );

    // Brace-language (Rust) sibling guard omission (1 sibling sharing first param `account`)
    // plus zero-arg function edge case (`ping()`).
    fs::write(
        repo_root.join("src/vault.rs"),
        r#"pub struct Account {
    pub is_frozen: bool,
    pub cents: i64,
}

pub fn authorize_account(account: &Account) -> Result<(), String> {
    if account.is_frozen {
        return Err("frozen".to_string());
    }
    Ok(())
}

pub fn close_account(account: &Account) -> Result<(), String> {
    authorize_account(account)?;
    if account.is_frozen {
        return Err("frozen".to_string());
    }
    Ok(())
}

pub fn debit_account(
    account: &Account,
    cents: i64,
    tags: Vec<String>,
) -> Result<i64, String> {
    if cents <= 0 {
        return Err("invalid".to_string());
    }
    let _ = tags.len();
    Ok(account.cents - cents)
}

pub fn ping() -> bool {
    true
}
"#,
    )
    .unwrap();

    let rs_blind = extract_scoped_context_with_options(
        repo_root,
        Some("FUNC:debit_account"),
        Some("src/vault.rs"),
        false,
        true,
        None,
    );
    assert!(
        rs_blind.signature.contains("tags: Vec<String>")
            && rs_blind.signature.contains("-> Result<i64, String>"),
        "Multi-line brace signature must be extracted completely in --blind mode: {:?}",
        rs_blind.signature
    );
    assert!(
        rs_blind
            .missing_sibling_guards
            .iter()
            .any(|g| g.guard_pattern.contains("authorize_account")),
        "Single sibling sharing first domain param `account` must flag missing `authorize_account`: {:?}",
        rs_blind.missing_sibling_guards
    );
    assert!(
        rs_blind
            .inferred_invariants
            .iter()
            .any(|inv| inv.contains("'tags'") && inv.contains("[]")),
        "Vec<String> parameter `tags` must generate empty collection probe: {:?}",
        rs_blind.inferred_invariants
    );

    let ping_ctx = extract_scoped_context_with_options(
        repo_root,
        Some("FUNC:ping"),
        Some("src/vault.rs"),
        false,
        true,
        None,
    );
    assert!(
        ping_ctx
            .inferred_invariants
            .iter()
            .any(|inv| inv.contains("zero-argument")),
        "Zero-parameter function must emit zero-argument idempotency probe: {:?}",
        ping_ctx.inferred_invariants
    );
    let file_ctx = extract_scoped_context_with_options(
        repo_root,
        Some("FILE:src/vault.rs"),
        Some("src/vault.rs"),
        false,
        true,
        None,
    );
    assert!(
        file_ctx.inferred_invariants.is_empty(),
        "FILE: target must return empty inferred_invariants: {:?}",
        file_ctx.inferred_invariants
    );
}

#[test]
fn test_pillar3_cross_layer_slice_mutation_concurrency_hint_and_env_config_audit() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    fs::create_dir_all(repo_root.join("src")).unwrap();
    fs::create_dir_all(repo_root.join("tests")).unwrap();

    fs::write(
        repo_root.join(".env.example"),
        "DATABASE_URL=sqlite:///:memory:\nLOG_LEVEL=INFO\n",
    )
    .unwrap();
    // Secret `.env` file must NEVER be opened or leaked by `proven scan`
    fs::write(
        repo_root.join(".env"),
        "PAYMENT_WEBHOOK_SECRET=SUPER_SECRET_DO_NOT_READ_12345\nSTRIPE_LIVE_KEY=sk_live_999999\n",
    )
    .unwrap();

    // 3-layer architecture: api_checkout (JSON) -> create_order_service (env + domain) -> insert_order_sql (SQLite)
    fs::write(
        repo_root.join("src/repo.py"),
        r#"def insert_order_sql(conn, user_id: str, total_cents: int) -> int:
    if total_cents <= 0:
        raise ValueError("total_cents must be positive")
    cur = conn.execute(
        "INSERT INTO orders (user_id, total_cents) VALUES (?, ?)",
        (user_id, total_cents),
    )
    conn.commit()
    return int(cur.lastrowid)
"#,
    )
    .unwrap();

    fs::write(
        repo_root.join("src/service.py"),
        r#"import os
from src.repo import insert_order_sql

def create_order_service(conn, user_id: str, subtotal_cents: int, fee_cents: int) -> int:
    # os.getenv("COMMENTED_OUT_ENV_KEY")
    _db = os.getenv("DATABASE_URL", ":memory:")
    _secret = os.getenv("PAYMENT_WEBHOOK_SECRET", "dev-secret")
    if subtotal_cents < 0 or fee_cents < 0:
        raise ValueError("invalid amounts")
    return insert_order_sql(conn, user_id, subtotal_cents + fee_cents)
"#,
    )
    .unwrap();

    fs::write(
        repo_root.join("src/api.py"),
        r#"import json
from src.service import create_order_service

def api_checkout(conn, payload_json: str) -> str:
    body = json.loads(payload_json)
    order_id = create_order_service(
        conn,
        body["user_id"],
        int(body["subtotal_cents"]),
        int(body["fee_cents"]),
    )
    return json.dumps({"order_id": order_id, "status": "created"})
"#,
    )
    .unwrap();

    fs::write(
        repo_root.join("src/wallet.py"),
        r#"class Wallet:
    def __init__(self, balance: int) -> None:
        self.balance = balance

    def deduct_balance(self, amount: int) -> int:
        if amount <= 0 or self.balance < amount:
            raise ValueError("insufficient")
        self.balance -= amount
        return self.balance
"#,
    )
    .unwrap();

    // Polyglot env reads in Rust and TypeScript
    fs::write(
        repo_root.join("src/signing.rs"),
        "pub fn load_key() -> String {\n    std::env::var(\"RUST_SIGNING_KEY\").unwrap_or_default()\n}\n",
    )
    .unwrap();
    fs::write(
        repo_root.join("src/client.ts"),
        "export function getHost(): string {\n    return process.env.PUBLIC_API_HOST || \"localhost\";\n}\n",
    )
    .unwrap();

    // 1. Verify `proven scan` config_audit detects undocumented env keys across Python/Rust/TS,
    //    ignores documented keys (`DATABASE_URL`) and commented-out keys (`COMMENTED_OUT_ENV_KEY`),
    //    never leaks `.env` secret values, and emits `blind_context_cmd` in fan-out tasks.
    let scan = scan_repository_changes_with_options(repo_root, "main", "HEAD", true);
    let scan_json = serde_json::to_string(&scan).unwrap();
    assert!(!scan_json.contains("SUPER_SECRET_DO_NOT_READ_12345"));
    assert!(!scan_json.contains("sk_live_999999"));
    let audit: proven::models::ConfigAuditReport =
        serde_json::from_value(scan["config_audit"].clone())
            .expect("config_audit must deserialize into ConfigAuditReport");
    assert_eq!(audit.env_example_files, vec![".env.example".to_string()]);
    let missing_keys: Vec<&str> = audit
        .missing_env_example_keys
        .iter()
        .map(String::as_str)
        .collect();
    assert!(missing_keys.contains(&"PAYMENT_WEBHOOK_SECRET"), "got {:?}", missing_keys);
    assert!(missing_keys.contains(&"RUST_SIGNING_KEY"), "got {:?}", missing_keys);
    assert!(missing_keys.contains(&"PUBLIC_API_HOST"), "got {:?}", missing_keys);
    assert!(!missing_keys.contains(&"DATABASE_URL"));
    assert!(!missing_keys.contains(&"COMMENTED_OUT_ENV_KEY"));
    assert!(
        scan["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w.as_str().unwrap_or("").contains("PAYMENT_WEBHOOK_SECRET"))
    );
    let tasks = scan["fan_out_tasks"].as_array().unwrap();
    assert!(!tasks.is_empty());
    assert!(
        tasks
            .iter()
            .all(|t| t["blind_context_cmd"].as_str().unwrap_or("").contains("--blind")),
        "Every fan-out task must include blind_context_cmd with --blind"
    );

    // 2. Verify `integration_slice` and `concurrency_risk` in full `proven context`
    let repo_ctx = extract_scoped_context(
        repo_root,
        Some("FUNC:insert_order_sql"),
        Some("src/repo.py"),
        false,
    );
    let repo_slice = repo_ctx
        .integration_slice
        .expect("insert_order_sql must report integration_slice");
    assert!(
        repo_slice
            .io_boundaries_detected
            .contains(&"sql_query".to_string())
    );
    assert!(
        repo_slice
            .entry_callers
            .contains(&"FUNC:create_order_service".to_string())
    );

    let api_ctx = extract_scoped_context(
        repo_root,
        Some("FUNC:api_checkout"),
        Some("src/api.py"),
        false,
    );
    let api_slice = api_ctx
        .integration_slice
        .expect("api_checkout must report integration_slice");
    assert!(
        api_slice
            .io_boundaries_detected
            .contains(&"json_serde".to_string())
    );
    assert!(
        api_slice
            .internal_callees
            .contains(&"FUNC:create_order_service".to_string())
    );

    let wallet_ctx = extract_scoped_context(
        repo_root,
        Some("FUNC:Wallet.deduct_balance"),
        Some("src/wallet.py"),
        false,
    );
    let conc = wallet_ctx
        .concurrency_risk
        .expect("Wallet.deduct_balance must report concurrency_risk");
    assert!(
        conc.signals
            .contains(&"shared_state_mutation".to_string()),
        "got {:?}",
        conc
    );

    // Unit-check Go multi-core & `-race` runner helper logic
    let (procs, use_race) = MicroMutationGate::select_go_gomaxprocs_and_race(
        "func TestConcurrent(t *testing.T) { var wg sync.WaitGroup; go func() { wg.Done() }() }",
        Some("-race"),
    );
    assert_eq!(procs, "4");
    assert!(use_race);
    let (plain_procs, plain_race) = MicroMutationGate::select_go_gomaxprocs_and_race(
        "func TestAdd(t *testing.T) { if add(1, 2) != 3 { t.Fatal() } }",
        None,
    );
    assert_eq!(plain_procs, "1");
    assert!(!plain_race);
    assert!(MicroMutationGate::is_go_race_cgo_unavailable_error(
        "go: -race requires cgo; enable cgo by setting CGO_ENABLED=1"
    ));
    assert!(!MicroMutationGate::is_go_race_cgo_unavailable_error(
        "==================\nWARNING: DATA RACE\nWrite at 0x00c000018090 by goroutine 7:\n"
    ));

    // 3. End-to-end 2-hop Transitive Slice Mutation Proof:
    //    `tests/test_checkout_slice.py` ONLY imports and invokes `api_checkout` (2 hops away from
    //    `insert_order_sql`) against a real `:memory:` SQLite schema.
    fs::write(
        repo_root.join("tests/test_checkout_slice.py"),
        r#"import json
import sqlite3
import pytest
from src.api import api_checkout

def _make_db():
    conn = sqlite3.connect(":memory:")
    conn.execute(
        "CREATE TABLE orders (id INTEGER PRIMARY KEY AUTOINCREMENT, user_id TEXT NOT NULL, total_cents INTEGER NOT NULL)"
    )
    return conn

def test_end_to_end_checkout_slice():
    conn = _make_db()
    resp = json.loads(
        api_checkout(conn, json.dumps({"user_id": "u_42", "subtotal_cents": 1500, "fee_cents": 250}))
    )
    assert resp == {"order_id": 1, "status": "created"}
    row = conn.execute("SELECT user_id, total_cents FROM orders WHERE id = 1").fetchone()
    assert row == ("u_42", 1750)

def test_repo_zero_and_one_boundary_via_2hop_api():
    conn = _make_db()
    resp = json.loads(
        api_checkout(conn, json.dumps({"user_id": "u_min", "subtotal_cents": 1, "fee_cents": 0}))
    )
    assert resp["order_id"] == 1
    row = conn.execute("SELECT user_id, total_cents FROM orders WHERE id = 1").fetchone()
    assert row == ("u_min", 1)
    with pytest.raises(ValueError):
        api_checkout(conn, json.dumps({"user_id": "u_zero", "subtotal_cents": 0, "fee_cents": 0}))
"#,
    )
    .unwrap();

    let orig_repo_py = fs::read_to_string(repo_root.join("src/repo.py")).unwrap();
    let slice_proof = MicroMutationGate::verify_proof(
        repo_root,
        "FUNC:insert_order_sql",
        "tests/test_checkout_slice.py",
        Some("src/repo.py"),
        true,
        6,
        None,
    );
    assert_eq!(
        slice_proof.proof_status, "pass",
        "Transitive slice proof must accept test invoking 1-2 hop ancestor callers: {:?}",
        slice_proof.message
    );
    assert_eq!(
        slice_proof.proof_grade, "TITANIUM",
        "All mutants in insert_order_sql must be killed through the real SQLite slice test: {:?}",
        slice_proof.mutation_proof
    );
    assert_eq!(
        fs::read_to_string(repo_root.join("src/repo.py")).unwrap(),
        orig_repo_py,
        "RAII FileRestoreGuard must restore src/repo.py byte-for-byte after slice mutation"
    );

    // 4. Unrelated test file that never invokes `insert_order_sql` or any 1-3 hop caller must still be rejected!
    fs::write(
        repo_root.join("tests/test_unrelated.py"),
        "def test_unrelated_math():\n    assert 2 + 2 == 4\n",
    )
    .unwrap();
    let unrelated_proof = MicroMutationGate::verify_proof(
        repo_root,
        "FUNC:insert_order_sql",
        "tests/test_unrelated.py",
        Some("src/repo.py"),
        true,
        6,
        None,
    );
    assert_eq!(unrelated_proof.proof_status, "fail");
    assert_eq!(unrelated_proof.proof_grade, "NONE");

    // 5. 4-hop chain boundary check: 3-hop caller is accepted, 4-hop caller (> 3 hops) is rejected.
    fs::write(
        repo_root.join("src/hops.py"),
        r#"def leaf_calc(x: int) -> int:
    if x >= 10:
        return x + 5
    return x - 1

def hop1(x: int) -> int:
    return leaf_calc(x)

def hop2(x: int) -> int:
    return hop1(x)

def hop3(x: int) -> int:
    return hop2(x)

def hop4(x: int) -> int:
    return hop3(x)
"#,
    )
    .unwrap();
    fs::write(
        repo_root.join("tests/test_hop3.py"),
        "from src.hops import hop3\n\ndef test_hop3():\n    assert hop3(10) == 15\n    assert hop3(9) == 8\n",
    )
    .unwrap();
    fs::write(
        repo_root.join("tests/test_hop4.py"),
        "from src.hops import hop4\n\ndef test_hop4():\n    assert hop4(10) == 15\n    assert hop4(9) == 8\n",
    )
    .unwrap();
    let hop3_proof = MicroMutationGate::verify_proof(
        repo_root,
        "FUNC:leaf_calc",
        "tests/test_hop3.py",
        Some("src/hops.py"),
        true,
        6,
        None,
    );
    assert_eq!(hop3_proof.proof_grade, "TITANIUM");

    let hop4_proof = MicroMutationGate::verify_proof(
        repo_root,
        "FUNC:leaf_calc",
        "tests/test_hop4.py",
        Some("src/hops.py"),
        false,
        6,
        None,
    );
    assert_eq!(hop4_proof.proof_grade, "NONE");

    // 6. Real 8-worker `threading.Barrier` concurrency test against locked shared state (`mutex_or_atomic`)
    fs::write(
        repo_root.join("src/safe_wallet.py"),
        r#"import threading

class SafeWallet:
    def __init__(self, balance: int) -> None:
        self.balance = balance
        self._lock = threading.Lock()

    def withdraw_atomic(self, amount: int) -> bool:
        if amount <= 0:
            raise ValueError("amount must be positive")
        with self._lock:
            if self.balance >= amount:
                self.balance -= amount
                return True
            return False
"#,
    )
    .unwrap();
    let safe_ctx = extract_scoped_context(
        repo_root,
        Some("FUNC:SafeWallet.withdraw_atomic"),
        Some("src/safe_wallet.py"),
        false,
    );
    let safe_conc = safe_ctx
        .concurrency_risk
        .expect("SafeWallet.withdraw_atomic must report concurrency_risk");
    assert!(safe_conc.signals.contains(&"mutex_or_atomic".to_string()));
    assert!(
        safe_conc
            .signals
            .contains(&"shared_state_mutation".to_string())
    );

    fs::write(
        repo_root.join("tests/test_safe_wallet_barrier.py"),
        r#"import threading
import pytest
from src.safe_wallet import SafeWallet

def test_concurrent_barrier_no_overdraft():
    wallet = SafeWallet(50)
    barrier = threading.Barrier(8)
    outcomes = []
    out_lock = threading.Lock()

    def worker():
        barrier.wait()
        ok = wallet.withdraw_atomic(10)
        with out_lock:
            outcomes.append(ok)

    threads = [threading.Thread(target=worker) for _ in range(8)]
    for t in threads:
        t.start()
    for t in threads:
        t.join()

    assert outcomes.count(True) == 5
    assert outcomes.count(False) == 3
    assert wallet.balance == 0
    with pytest.raises(ValueError):
        wallet.withdraw_atomic(0)
"#,
    )
    .unwrap();
    let barrier_proof = MicroMutationGate::verify_proof(
        repo_root,
        "FUNC:SafeWallet.withdraw_atomic",
        "tests/test_safe_wallet_barrier.py",
        Some("src/safe_wallet.py"),
        true,
        6,
        None,
    );
    assert_eq!(barrier_proof.proof_grade, "TITANIUM");
}

#[test]
fn test_adversarial_oracle_omission_and_config_regressions() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    fs::create_dir_all(repo_root.join("src")).unwrap();

    // 1. >240-char docstring cap (`<= 240` chars), `--blind --spec <source.py>` leak prevention,
    //    local `UPPER_SNAKE_CASE` non-self-verification, `.25` leading-dot float, & `1e-6` scientific notation
    let long_doc = format!(
        "Max discount is 15% with tolerance 1e-6. {}",
        "specification_clause ".repeat(18)
    );
    fs::write(
        repo_root.join("src/pricing.py"),
        format!(
            r#"def apply_promo(price: float, promo_ratio: float) -> float:
    """{}"""
    SECRET_LOCAL_CAP = 0.20
    if abs(promo_ratio - SECRET_LOCAL_CAP) < 1e-6:
        return price * .25
    return price * (1.0 - min(promo_ratio, SECRET_LOCAL_CAP))
"#,
            long_doc
        ),
    )
    .unwrap();

    // Passing `--spec src/pricing.py` in `--blind` mode must NEVER leak implementation source lines
    let blind_self_spec = extract_scoped_context_with_options(
        repo_root,
        Some("FUNC:apply_promo"),
        Some("src/pricing.py"),
        true,
        true,
        Some("src/pricing.py"),
    );
    assert!(blind_self_spec.blind_mode);
    assert!(blind_self_spec.source_code.is_empty());
    assert!(
        blind_self_spec
            .discovered_specs
            .iter()
            .all(|s| s.source != "explicit_spec" && !s.excerpt.contains("SECRET_LOCAL_CAP")),
        "Source files passed via --spec must never leak implementation body in blind mode: {:?}",
        blind_self_spec.discovered_specs
    );
    for snip in &blind_self_spec.discovered_specs {
        assert!(
            snip.excerpt.chars().count() <= 240,
            "Spec snippet exceeded 240 chars ({}): {}",
            snip.excerpt.chars().count(),
            snip.excerpt
        );
    }

    let full_pricing = extract_scoped_context(
        repo_root,
        Some("FUNC:apply_promo"),
        Some("src/pricing.py"),
        false,
    );
    let flagged_lits: Vec<&str> = full_pricing
        .unverified_literals
        .iter()
        .map(|u| u.literal.as_str())
        .collect();
    assert!(
        flagged_lits.contains(&"0.20"),
        "Local constant inside target function body must NOT verify itself: {:?}",
        full_pricing.unverified_literals
    );
    assert!(
        flagged_lits.contains(&".25"),
        "Leading-dot float `.25` must be flagged as unverified: {:?}",
        full_pricing.unverified_literals
    );
    assert!(
        !flagged_lits.contains(&"6") && !flagged_lits.contains(&"1e-6"),
        "Scientific notation `1e-6` backed by docstring must not split into phantom `6`: {:?}",
        full_pricing.unverified_literals
    );

    // 2. Polyglot signature invariant parsing (`pub(crate)`, TS typed arrow, Go grouped types, C `const int64_t`, 6-param coverage)
    let rust_probes = infer_signature_invariants(
        "pub(crate) fn pay_invoice(user: &User, amount: i64, memo: &str, tax: i64, fee: i64, code: &str) -> Result<(), Err>",
        "rust",
    );
    assert_eq!(rust_probes.len(), 6, "got {:?}", rust_probes);
    assert!(
        rust_probes.iter().all(|p| !p.contains("'crate'")),
        "pub(crate) visibility modifier must never parse as parameter 'crate': {:?}",
        rust_probes
    );
    for expected_param in ["'user'", "'amount'", "'memo'", "'tax'", "'fee'", "'code'"] {
        assert!(
            rust_probes.iter().any(|p| p.contains(expected_param)),
            "Missing probe for {} in {:?}",
            expected_param,
            rust_probes
        );
    }

    let ts_arrow_probes = infer_signature_invariants(
        "export const calcFee: (x: number) => number = (amount: number) =>",
        "typescript",
    );
    assert!(
        ts_arrow_probes
            .iter()
            .any(|p| p.contains("numeric parameter 'amount'")),
        "got {:?}",
        ts_arrow_probes
    );

    let go_grouped_probes = infer_signature_invariants(
        "func Transfer(sender, recipient *Account, min, max int64) error",
        "go",
    );
    assert!(
        go_grouped_probes
            .iter()
            .any(|p| p.contains("numeric parameter 'min'"))
            && go_grouped_probes
                .iter()
                .any(|p| p.contains("numeric parameter 'max'")),
        "Go grouped parameters `min, max int64` must inherit trailing int64 type: {:?}",
        go_grouped_probes
    );

    let c_qual_probes = infer_signature_invariants(
        "int settle_batch(const int64_t total_cents)",
        "cpp",
    );
    assert!(
        c_qual_probes
            .iter()
            .any(|p| p.contains("numeric parameter 'total_cents'")),
        "got {:?}",
        c_qual_probes
    );

    // 3. `missing_sibling_guards`: decorators (`@login_required`), `elif` branches, and non-suppression by comments/strings
    fs::write(
        repo_root.join("src/endpoints.py"),
        r#"@login_required
def view_wallet(user):
    ensure_kyc_verified(user)
    if user is None:
        raise ValueError("missing user")
    elif user.is_locked:
        raise PermissionError("locked")
    return user.balance

@login_required
def freeze_wallet(user):
    ensure_kyc_verified(user)
    if user.is_locked:
        raise PermissionError("already locked")
    return True

def drain_wallet(user):
    # TODO: check user.is_locked and @login_required before release
    audit_note = "ensure_kyc_verified bypassed"
    return 0
"#,
    )
    .unwrap();

    let drain_ctx = extract_scoped_context_with_options(
        repo_root,
        Some("FUNC:drain_wallet"),
        Some("src/endpoints.py"),
        false,
        true,
        None,
    );
    let guard_patterns: Vec<&str> = drain_ctx
        .missing_sibling_guards
        .iter()
        .map(|g| g.guard_pattern.as_str())
        .collect();
    assert!(
        guard_patterns.contains(&"@login_required"),
        "Missing @login_required decorator guard must be detected: {:?}",
        guard_patterns
    );
    assert!(
        guard_patterns.contains(&"user.is_locked"),
        "`elif user.is_locked` guard must be detected and not suppressed by `# TODO` comment: {:?}",
        guard_patterns
    );
    assert!(
        guard_patterns.contains(&"ensure_kyc_verified(user)"),
        "Guard helper must not be suppressed by string literal mention: {:?}",
        guard_patterns
    );

    // 4. Multi-line `caller_usage` argument capture & inline trailing `/* note */` non-pollution of brace docstrings
    fs::write(
        repo_root.join("src/billing.rs"),
        r#"const SCHEMA_VER: i32 = 1; /* internal schema tag */
pub(crate) fn compute_tax(subtotal: i64, rate_bps: i64) -> i64 {
    (subtotal * rate_bps) / 10_000
}

pub fn checkout_handler(sub: i64) -> i64 {
    // compute_tax(0, 0) inside comment must be skipped
    let tax = compute_tax(
        sub,
        750,
    );
    sub + tax
}
"#,
    )
    .unwrap();

    let tax_ctx = extract_scoped_context_with_options(
        repo_root,
        Some("FUNC:compute_tax"),
        Some("src/billing.rs"),
        false,
        true,
        None,
    );
    assert!(
        tax_ctx.docstring.is_empty(),
        "Inline trailing block comment on previous const must not pollute docstring: {:?}",
        tax_ctx.docstring
    );
    let caller_spec = tax_ctx
        .discovered_specs
        .iter()
        .find(|s| s.source == "caller_usage")
        .expect("compute_tax must discover caller_usage from checkout_handler");
    assert!(
        caller_spec.excerpt.contains("750"),
        "Multi-line caller_usage must join arguments across lines and skip comments: {:?}",
        caller_spec.excerpt
    );

    // 5. `audit_changed_files_env_keys`: UTF-8 BOM `.env.example`, JS/TS destructuring, multi-line `os.getenv`, single-quoted `getenv('...')`
    fs::write(
        repo_root.join(".env.example"),
        "\u{feff}DATABASE_URL=postgres://localhost/app\nSTRIPE_KEY=sk_test_placeholder\n",
    )
    .unwrap();
    fs::write(
        repo_root.join("src/env_loader.ts"),
        r#"const {
    STRIPE_KEY,
    WEBHOOK_SECRET = "fallback,secret",
    API_HOST: publicHost,
} = process.env;
export const cfg = { STRIPE_KEY, WEBHOOK_SECRET, publicHost };
"#,
    )
    .unwrap();
    fs::write(
        repo_root.join("src/py_env.py"),
        r#"import os
from os import getenv, environ

DB = os.getenv(
    "DATABASE_URL",
    "sqlite:///:memory:",
)
TOK = getenv('SINGLE_QUOTE_TOKEN')
BRK = environ['BRACKET_SINGLE_KEY']
"#,
    )
    .unwrap();

    let audit = audit_changed_files_env_keys(
        repo_root,
        &[
            "src/env_loader.ts".to_string(),
            "src/py_env.py".to_string(),
        ],
    );
    assert!(
        audit
            .declared_env_example_keys
            .contains(&"DATABASE_URL".to_string()),
        "UTF-8 BOM at start of .env.example must not drop first declared key: {:?}",
        audit.declared_env_example_keys
    );
    assert!(
        !audit
            .missing_env_example_keys
            .contains(&"DATABASE_URL".to_string())
            && !audit
                .missing_env_example_keys
                .contains(&"STRIPE_KEY".to_string()),
        "Declared keys must not be flagged missing: {:?}",
        audit.missing_env_example_keys
    );
    for expected_missing in [
        "WEBHOOK_SECRET",
        "API_HOST",
        "SINGLE_QUOTE_TOKEN",
        "BRACKET_SINGLE_KEY",
    ] {
        assert!(
            audit
                .missing_env_example_keys
                .contains(&expected_missing.to_string()),
            "Expected missing env key {} in {:?}",
            expected_missing,
            audit.missing_env_example_keys
        );
    }
}

#[test]
fn test_adversarial_transitive_collisions_and_live_go_race_barrier() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    fs::create_dir_all(repo_root.join("src")).unwrap();
    fs::create_dir_all(repo_root.join("tests")).unwrap();

    // 1. Same-name function `save_row` across two files (`src/orders.py` vs `src/users.py`)
    fs::write(
        repo_root.join("src/orders.py"),
        r#"def save_row(amount: int) -> int:
    if amount <= 0:
        raise ValueError("invalid order amount")
    return amount + 10

def orders_endpoint(amount: int) -> int:
    return save_row(amount)
"#,
    )
    .unwrap();
    fs::write(
        repo_root.join("src/users.py"),
        r#"def save_row(uid: int) -> int:
    return uid * 2

def users_endpoint(uid: int) -> int:
    return save_row(uid)
"#,
    )
    .unwrap();
    fs::write(
        repo_root.join("tests/test_orders_slice.py"),
        r#"import pytest
from src.orders import orders_endpoint

def test_orders_slice_kills_mutants():
    assert orders_endpoint(1) == 11
    assert orders_endpoint(25) == 35
    with pytest.raises(ValueError):
        orders_endpoint(0)
"#,
    )
    .unwrap();

    let orders_proof = MicroMutationGate::verify_proof(
        repo_root,
        "FUNC:save_row",
        "tests/test_orders_slice.py",
        Some("src/orders.py"),
        true,
        6,
        None,
    );
    assert_eq!(
        orders_proof.proof_grade, "TITANIUM",
        "Same-name function in src/users.py must not overwrite src/orders.py::save_row in transitive BFS: {}",
        orders_proof.message
    );
    assert!(orders_proof.message.contains("transitive 1-hop slice"));

    // Cross-file non-invocation: `test_orders_slice.py` does NOT invoke `src/users.py::save_row`
    let users_proof = MicroMutationGate::verify_proof(
        repo_root,
        "FUNC:save_row",
        "tests/test_orders_slice.py",
        Some("src/users.py"),
        false,
        4,
        None,
    );
    assert_eq!(
        users_proof.proof_grade, "NONE",
        "Test calling orders_endpoint must NOT validate src/users.py::save_row: {}",
        users_proof.message
    );

    // 2. Same-name method across two unrelated classes (`ClassA.save -> persist_audit` vs `ClassB.save`)
    fs::write(
        repo_root.join("src/audit.py"),
        r#"def persist_audit(code: int) -> int:
    return code + 100

class ClassA:
    def save(self, code: int) -> int:
        return persist_audit(code)

class ClassB:
    def save(self, code: int) -> int:
        return code
"#,
    )
    .unwrap();
    fs::write(
        repo_root.join("tests/test_unrelated_class_b.py"),
        r#"from src.audit import ClassB

def test_only_class_b():
    b = ClassB()
    assert b.save(7) == 7
"#,
    )
    .unwrap();

    let false_class_proof = MicroMutationGate::verify_proof(
        repo_root,
        "FUNC:persist_audit",
        "tests/test_unrelated_class_b.py",
        Some("src/audit.py"),
        false,
        4,
        None,
    );
    assert_eq!(
        false_class_proof.proof_grade, "NONE",
        "Unrelated ClassB().save() must not falsely match ClassA.save -> persist_audit: {}",
        false_class_proof.message
    );

    // 3. Live Go 16-goroutine concurrency barrier + `--pytest-args "-race"` mutation proof
    let portable_go = std::env::temp_dir()
        .join("go_sdk")
        .join("go")
        .join("bin")
        .join("go.exe");
    let go_available = portable_go.exists()
        || Command::new("go")
            .arg("version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

    if go_available {
        let go_dir = tempdir().unwrap();
        let go_root = go_dir.path();
        fs::write(go_root.join("go.mod"), "module proven_go_race_wallet\n\ngo 1.21\n").unwrap();
        fs::write(
            go_root.join("wallet.go"),
            r#"package wallet

import "sync"

type SafeLedger struct {
	mu      sync.Mutex
	balance int
}

func NewSafeLedger(initial int) *SafeLedger {
	return &SafeLedger{balance: initial}
}

func (w *SafeLedger) Withdraw(amount int) bool {
	if amount <= 0 {
		return false
	}
	w.mu.Lock()
	defer w.mu.Unlock()
	if w.balance >= amount {
		w.balance -= amount
		return true
	}
	return false
}
"#,
        )
        .unwrap();
        fs::write(
            go_root.join("wallet_test.go"),
            r#"package wallet

import (
	"sync"
	"sync/atomic"
	"testing"
)

func TestConcurrentWithdrawBarrier(t *testing.T) {
	ledger := NewSafeLedger(50)
	if ledger.Withdraw(0) {
		t.Fatalf("zero withdrawal must be rejected")
	}
	if ledger.Withdraw(-5) {
		t.Fatalf("negative withdrawal must be rejected")
	}

	const workers = 16
	start := make(chan struct{})
	var wg sync.WaitGroup
	var okCount int32

	for i := 0; i < workers; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			<-start
			if ledger.Withdraw(10) {
				atomic.AddInt32(&okCount, 1)
			}
		}()
	}
	close(start)
	wg.Wait()

	if okCount != 5 {
		t.Fatalf("expected exactly 5 successful withdrawals, got %d", okCount)
	}
	if ledger.balance != 0 {
		t.Fatalf("expected final balance 0, got %d", ledger.balance)
	}
	single := NewSafeLedger(10)
	if !single.Withdraw(10) || single.balance != 0 {
		t.Fatalf("exact boundary balance==amount withdrawal failed")
	}
}
"#,
        )
        .unwrap();

        let go_ctx = extract_scoped_context(
            go_root,
            Some("FUNC:Withdraw"),
            Some("wallet.go"),
            false,
        );
        let go_conc = go_ctx
            .concurrency_risk
            .expect("Go Withdraw with sync.Mutex must report concurrency_risk");
        assert_eq!(
            go_conc.runner_flag_hint.as_deref(),
            Some("--pytest-args \"-race\"")
        );

        let go_proof = MicroMutationGate::verify_proof(
            go_root,
            "FUNC:Withdraw",
            "wallet_test.go",
            Some("wallet.go"),
            true,
            6,
            Some("-race"),
        );
        assert_eq!(
            go_proof.proof_grade, "TITANIUM",
            "Live Go 16-goroutine barrier test with -race flag failed: {}",
            go_proof.message
        );
        let has_gcc = Command::new("gcc")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !has_gcc {
            assert!(
                go_proof
                    .message
                    .contains("[proven note: go test -race unavailable without CGO/C compiler"),
                "Passing Go proof without CGO must surface the [proven note: ...] fallback diagnostic in message: {}",
                go_proof.message
            );
        }
    }
}

#[test]
fn test_rust_lifetime_and_cpp_digit_separator_function_span() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"vault_lifetime\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/lib.rs"),
        r#"#[derive(Clone, Debug)]
pub struct Session<'a> {
    pub label: &'a str,
    pub is_locked: bool,
    pub mfa_verified: bool,
    pub clearance: u32,
}

pub fn rotate_master_key<'a>(session: &'a Session<'a>, slot: u32) -> Result<u32, &'static str> {
    if session.is_locked {
        return Err("SESSION_LOCKED");
    }
    if !session.mfa_verified {
        return Err("MFA_REQUIRED");
    }
    if session.clearance < 3 {
        return Err("INSUFFICIENT_CLEARANCE");
    }
    if slot == 0 || slot > 16 {
        return Err("INVALID_SLOT");
    }
    Ok(slot * 100 + session.clearance)
}

pub fn revoke_subkey<'a>(session: &'a Session<'a>, slot: u32) -> Result<u32, &'static str> {
    if session.is_locked {
        return Err("SESSION_LOCKED");
    }
    if !session.mfa_verified {
        return Err("MFA_REQUIRED");
    }
    if session.clearance < 2 {
        return Err("INSUFFICIENT_CLEARANCE");
    }
    if slot == 0 || slot > 16 {
        return Err("INVALID_SLOT");
    }
    Ok(slot)
}

pub fn export_wrapped_key<'a>(session: &'a Session<'a>, slot: u32) -> Result<u32, &'static str> {
    if session.is_locked {
        return Err("SESSION_LOCKED");
    }
    if session.clearance < 2 {
        return Err("INSUFFICIENT_CLEARANCE");
    }
    if slot == 0 || slot > 16 {
        return Err("INVALID_SLOT");
    }
    Ok(slot * 10 + session.clearance)
}
"#,
    )
    .unwrap();

    let ctx = extract_scoped_context(
        root,
        Some("FUNC:export_wrapped_key"),
        Some("src/lib.rs"),
        false,
    );
    let extracted_lines = ctx.source_code.lines().count();
    assert!(
        extracted_lines >= 11,
        "Rust function with &'static str return type and <'a> lifetimes must span full body, got {} lines:\n{}",
        extracted_lines,
        ctx.source_code
    );
    assert!(
        ctx.missing_sibling_guards
            .iter()
            .any(|g| g.guard_pattern.contains("mfa_verified")),
        "Sibling guard audit across Rust functions with lifetimes must detect missing mfa_verified guard: {:?}",
        ctx.missing_sibling_guards
    );

    fs::create_dir_all(root.join("tests")).unwrap();
    fs::write(
        root.join("tests/vault_test.rs"),
        r#"use vault_lifetime::{export_wrapped_key, Session};

#[test]
fn test_export_wrapped_key_branches() {
    let locked = Session { label: "ops", is_locked: true, mfa_verified: true, clearance: 3 };
    assert_eq!(export_wrapped_key(&locked, 2), Err("SESSION_LOCKED"));

    let low = Session { label: "ops", is_locked: false, mfa_verified: true, clearance: 1 };
    assert_eq!(export_wrapped_key(&low, 2), Err("INSUFFICIENT_CLEARANCE"));

    let ok = Session { label: "ops", is_locked: false, mfa_verified: true, clearance: 2 };
    assert_eq!(export_wrapped_key(&ok, 0), Err("INVALID_SLOT"));
    assert_eq!(export_wrapped_key(&ok, 17), Err("INVALID_SLOT"));
    assert_eq!(export_wrapped_key(&ok, 16), Ok(162));
}
"#,
    )
    .unwrap();

    let proof = MicroMutationGate::verify_proof(
        root,
        "FUNC:export_wrapped_key",
        "tests/vault_test.rs",
        Some("src/lib.rs"),
        true,
        10,
        None,
    );
    let mp = proof
        .mutation_proof
        .as_ref()
        .expect("mutation_proof must be populated");
    assert!(
        mp.mutants_total >= 4,
        "Expected >= 4 AST mutants in export_wrapped_key, got {} ({})",
        mp.mutants_total,
        proof.message
    );
    assert_eq!(
        proof.proof_grade, "TITANIUM",
        "Expected TITANIUM proof for export_wrapped_key, got {}: {}",
        proof.proof_grade, proof.message
    );

    // Also verify C++14 numeric digit separators (`1'000'000`) and JS single-quoted strings do not regress brace spans
    fs::write(
        root.join("limits.cpp"),
        r#"int clamp_budget(int amount) {
    int cap = 1'000'000;
    if (amount > cap) {
        return cap;
    }
    if (amount < 0) {
        return 0;
    }
    return amount;
}
"#,
    )
    .unwrap();
    let cpp_ctx = extract_scoped_context(root, Some("FUNC:clamp_budget"), Some("limits.cpp"), false);
    assert_eq!(cpp_ctx.source_code.lines().count(), 10);

    fs::write(
        root.join("quote.js"),
        r#"function tagMode(x) {
    const label = 'alpha { not a brace';
    if (x > 10) {
        return label;
    }
    return 'beta';
}
"#,
    )
    .unwrap();
    let js_ctx = extract_scoped_context(root, Some("FUNC:tagMode"), Some("quote.js"), false);
    assert_eq!(js_ctx.source_code.lines().count(), 7);
}

#[test]
fn test_java_flat_src_slice_sourcepath_and_context_config_audit() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("tests")).unwrap();
    fs::write(root.join(".env.example"), "LOYALTY_CURRENCY=USD\n").unwrap();

    fs::write(
        root.join("src/LoyaltyEngine.java"),
        r#"public class LoyaltyEngine {
    public static int calculateRedemptionCents(int points, int subtotalCents) {
        String _secret = System.getenv("LOYALTY_SIGNING_KEY");
        if (points < 0 || subtotalCents <= 0) {
            throw new IllegalArgumentException("invalid");
        }
        if (points < 100) {
            return 0;
        }
        int raw = (points / 100) * 125;
        int cap = (subtotalCents * 30) / 100;
        return raw > cap ? cap : raw;
    }
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("src/PosTerminal.java"),
        r#"public class PosTerminal {
    public static int finalPayableCents(int subtotalCents, int points) {
        return subtotalCents - LoyaltyEngine.calculateRedemptionCents(points, subtotalCents);
    }
}
"#,
    )
    .unwrap();

    // 1. Blind context hides config_audit, whereas full context surfaces missing_env_example_keys
    let blind_ctx = extract_scoped_context_with_options(
        root,
        Some("FUNC:calculateRedemptionCents"),
        Some("src/LoyaltyEngine.java"),
        false,
        true,
        None,
    );
    assert!(blind_ctx.config_audit.is_none());

    let full_ctx = extract_scoped_context(
        root,
        Some("FUNC:calculateRedemptionCents"),
        Some("src/LoyaltyEngine.java"),
        false,
    );
    let cfg = full_ctx
        .config_audit
        .expect("Full context must report config_audit when target file reads env keys");
    assert_eq!(
        cfg.missing_env_example_keys,
        vec!["LOYALTY_SIGNING_KEY".to_string()]
    );

    // 2. Java test in tests/ with `// PROVEN_COMPILE: src/LoyaltyEngine.java` that also calls
    //    `PosTerminal` from flat `src/` must resolve `src/PosTerminal.java` via `-sourcepath`
    fs::write(
        root.join("tests/TestLoyaltyEngine.java"),
        r#"// PROVEN_COMPILE: src/LoyaltyEngine.java
public class TestLoyaltyEngine {
    public static void main(String[] args) {
        assert LoyaltyEngine.calculateRedemptionCents(0, 1000) == 0;
        assert LoyaltyEngine.calculateRedemptionCents(99, 1000) == 0;
        assert LoyaltyEngine.calculateRedemptionCents(100, 1000) == 125;
        assert LoyaltyEngine.calculateRedemptionCents(5000, 10000) == 3000;
        assert PosTerminal.finalPayableCents(10000, 200) == 9750;
        try {
            LoyaltyEngine.calculateRedemptionCents(-1, 1000);
            throw new AssertionError("expected negative points rejection");
        } catch (IllegalArgumentException expected) {}
        try {
            LoyaltyEngine.calculateRedemptionCents(100, 0);
            throw new AssertionError("expected zero subtotal rejection");
        } catch (IllegalArgumentException expected) {}
    }
}
"#,
    )
    .unwrap();

    let jbr_javac = std::path::Path::new(
        r"C:\Program Files\JetBrains\PyCharm 2025.3.2.1\jbr\bin\javac.exe",
    );
    let javac_ok = jbr_javac.exists()
        || Command::new("javac")
            .arg("-version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
    if javac_ok {
        let proof = MicroMutationGate::verify_proof(
            root,
            "FUNC:calculateRedemptionCents",
            "tests/TestLoyaltyEngine.java",
            Some("src/LoyaltyEngine.java"),
            true,
            10,
            None,
        );
        assert_eq!(
            proof.proof_grade, "TITANIUM",
            "Java flat-src slice test referencing PosTerminal failed verify_proof: {}",
            proof.message
        );
    }
}





