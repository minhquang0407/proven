# Proven CLI & Skill Scripts API Reference

All 11 skill scripts in `skills/proven/scripts/` are thin wrappers around the binary (`proven`). Every script supports `--schema` and `--json`.

---

## 1. Core Verification Loop (Tier 1)

### `scan_impact.py` -> `proven scan`
Scans git diffs (`base..head`), working tree changes, and untracked files to identify untested functions and audit environment variable drift against `.env.example` / `.env.sample` / `.env.template`.
```bash
python skills/proven/scripts/scan_impact.py --path . --base main --head HEAD [--fan-out] --json
```
- **Key Output Fields**: `status`, `track`, `language`, `changed_count`, `missing_count`, `changed_files`, `changed_nodes`, `missing_coverage`, `fan_out_tasks` (when `--fan-out` is passed; includes `blind_context_cmd`, `context_cmd`, and `verify_cmd`), `warnings`, `config_audit` (`ConfigAuditReport`: `env_example_files`, `referenced_env_keys`, `declared_env_example_keys`, `missing_env_example_keys`).

### `get_target_context.py` -> `proven context`
Extracts external specifications, sibling guard omissions, signature domain invariants, scoped AST source slice, 1-hop callers/callees, 3-hop call-graph PageRank context, integration/concurrency hints, and `<80`-token failure rules.
```bash
python skills/proven/scripts/get_target_context.py --target "FUNC:my_func" [--file "src/mod.py"] [--blind] [--spec "SPEC.md"] --path . --json
```
- **Key Output Fields** (`TargetContext`): `status`, `track`, `target_id`, `language`, `source_file`, `module_path`, `signature`, `docstring`, `source_code` (empty when `--blind`), `imports`, `functions`, `callers`, `callees`, `suggested_test_file`, `existing_test_file_exists`, `existing_test_preview`, `repo_memory`, `graph_rag_prompt`, `pinned_lessons`, `blind_mode`, `discovered_specs`, `inferred_invariants`, `missing_sibling_guards`, `unverified_literals`, `integration_slice`, `concurrency_risk`.

### `verify_runtime_proof.py` -> `proven verify-proof`
Executes the test suite (supporting direct target invocation or 1–3 hop transitive slice tests, plus Go `GOMAXPROCS=4` / `-race`) and runs the 3-Persona Red Team AST Mutation Gate.
```bash
python skills/proven/scripts/verify_runtime_proof.py --target "FUNC:my_func" --test "tests/test_mod.py" --mutation-check [--max-mutants 6] [--pytest-args "-race"] --json
```
- **Key Output Fields** (`VerifyProofResult`): `status` (`success` | `error`), `track`, `proof_status` (`pass` | `fail`), `proof_grade` (`TITANIUM` | `SILVER` | `NONE`), `target_id`, `test_target`, `pytest_passed`, `pytest_returncode`, `message`, `covered_fraction`, `covered_lines`, `covered_line_count`, `function_line_count`, `mutation_proof` (`MutationGateResult`).

### `run_arena.py` -> `proven arena`
Referees a single adversarial round between Blue Team (Test Author) and Red Team (3-Persona AST Mutator).
```bash
python skills/proven/scripts/run_arena.py --target "FUNC:my_func" --test "tests/test_mod.py" --round 1 --json
```
- **Key Output Fields** (`ArenaRoundResult`): `target_id`, `test_target`, `round`, `max_rounds`, `stop_condition` (`TITANIUM_ACHIEVED` | `CONTINUE_ARENA` | `MAX_ROUNDS_REACHED`), `is_titanium`, `proof_grade`, `mutation_score`, `message`, `reflexion_prompt`, `red_swarm_summary`.

---

## 2. Topological GraphRAG, Impact & Memory (Tier 2)

### `explain_attention.py` -> `proven explain-attention`
Computes deterministic multi-hop Personalized PageRank + degree-normalized decay over the AST call graph.
```bash
python skills/proven/scripts/explain_attention.py --target "FUNC:my_func" --hops 3 --top-k 5 [--visualize] [--json]
```
- **Key Output Fields** (`SubgraphAttentionResult`): `target_id`, `target_label`, `hops`, `mode`, `engine`, `total_subgraph_nodes`, `attended_nodes`, `prompt_block`, `ascii_tree`.

### `query_impact.py` -> `proven impact`
Predicts the change impact blast radius, direct dependents, and latent risk candidates.
```bash
python skills/proven/scripts/query_impact.py --target "FUNC:my_func" --path . --json
```
- **Key Output Fields** (`ImpactPredictionResult`): `status`, `project`, `target`, `target_type`, `mode`, `direct_impact_count`, `direct_dependents`, `latent_risk_candidates`, `warnings`.

### `triage_expert.py` -> `proven triage`
Ranks suspect files and recommended reviewers using AST symbol matching + `git log` ownership topology.
```bash
python skills/proven/scripts/triage_expert.py --query "Null pointer in payment webhook" --path . --json
```
- **Key Output Fields** (`TriageResult`): `status`, `project`, `query`, `top_engineers`, `related_files`.

### `consolidate_memory.py` -> `proven sleep`
Runs Neocortex Sleep Consolidation to compress `.proven/memory.json` into deduplicated axioms (`<= 45` words) and optionally promote universal Meta-Axioms (`--meta`).
```bash
python skills/proven/scripts/consolidate_memory.py --path . --force --meta --json
```
- **Key Output Fields** (`SleepConsolidationResult`): `status`, `consolidated_lessons`, `axioms_count`, `promoted_meta_axioms`, `axioms_file`, `message`.

### `commit_axiom.py` -> `proven memory`
Lists or pins lessons into `.proven/memory.json`, or manages `.proven/meta_axioms.json`.
```bash
python skills/proven/scripts/commit_axiom.py --target "FUNC:my_func" --lesson "Assert exact boundary equality" --failure-mode "DIV_ZERO_OR_BOUNDARY" --json
python skills/proven/scripts/commit_axiom.py --list --json
python skills/proven/scripts/commit_axiom.py --meta-promote --json
python skills/proven/scripts/commit_axiom.py --meta-list --json
```

### `refresh_runtime_map.py` -> `proven refresh`
Re-indexes the repository AST graph and refreshes the verification status map.
```bash
python skills/proven/scripts/refresh_runtime_map.py --path . --json
```

### `run_benchmark.py` -> `proven benchmark`
Runs the 4-Regime Empirical Benchmark or initializes/grades the 4-Regime Agent Challenge Arena.
```bash
python skills/proven/scripts/run_benchmark.py --regime all --json
python skills/proven/scripts/run_benchmark.py --init-arena .proven/benchmark_arena --json
python skills/proven/scripts/run_benchmark.py --grade-arena .proven/benchmark_arena --report docs/SCORE.md --json
```
