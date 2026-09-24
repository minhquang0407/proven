"""Tests for Sleep Consolidation, Hybrid Triggers, and Heuristic Clustering."""

import pytest
from pathlib import Path
from proven.core.memory_manager import GraphMemoryManager
from proven.core.swarm.arena import TriAgentArena


def test_check_sleep_trigger_threshold(tmp_path):
    repo_path = str(tmp_path)

    # Initially 0 lessons
    res0 = GraphMemoryManager.check_sleep_trigger(threshold=10, repo_path=repo_path)
    assert res0["needs_sleep"] is False
    assert res0["unconsolidated_count"] == 0

    # Add 9 lessons
    for i in range(9):
        GraphMemoryManager.crystallize_lesson(
            target_id=f"FUNC:mod.func_{i}",
            trap=f"Boundary trap {i}",
            solution=f"Solution {i}",
            repo_path=repo_path,
        )

    res9 = GraphMemoryManager.check_sleep_trigger(threshold=10, repo_path=repo_path)
    assert res9["needs_sleep"] is False
    assert res9["unconsolidated_count"] == 9

    # Add 10th lesson -> trigger fires
    GraphMemoryManager.crystallize_lesson(
        target_id="FUNC:mod.func_9",
        trap="Boundary trap 9",
        solution="Solution 9",
        repo_path=repo_path,
    )

    res10 = GraphMemoryManager.check_sleep_trigger(threshold=10, repo_path=repo_path)
    assert res10["needs_sleep"] is True
    assert res10["unconsolidated_count"] == 10


def test_heuristic_clustering_categories(tmp_path):
    repo_path = str(tmp_path)

    # 1. Boundary trap
    GraphMemoryManager.crystallize_lesson(
        target_id="FUNC:auth.check_rate_limit",
        trap="Mutant inverted `>=` to `>` boundary off-by-one check",
        solution="Assert boundary zero and max integer limit",
        repo_path=repo_path,
    )

    # 2. Null guard trap
    GraphMemoryManager.crystallize_lesson(
        target_id="FUNC:auth.parse_token",
        trap="Mutant removed null check causing AttributeError when header is None",
        solution="Add guard clause checking if token is None or empty string",
        repo_path=repo_path,
    )

    # 3. State effect trap
    GraphMemoryManager.crystallize_lesson(
        target_id="FUNC:auth.session_rollback",
        trap="Mutant omitted cache invalidation dirty state rollback",
        solution="Assert session state is cleaned up after error",
        repo_path=repo_path,
    )

    # 4. Chaos exception trap
    GraphMemoryManager.crystallize_lesson(
        target_id="FUNC:auth.handle_crash",
        trap="Mutant caused unhandled KeyError crash exception",
        solution="Catch KeyError and return 400 Bad Request",
        repo_path=repo_path,
    )

    # Force cluster with 4 items
    cluster_res = GraphMemoryManager.cluster_unconsolidated_lessons(
        threshold=10, force=True, repo_path=repo_path
    )
    assert cluster_res["status"] == "NEEDS_SYNTHESIS"
    assert cluster_res["count"] == 4

    categories = {c["category"] for c in cluster_res["clusters"]}
    assert "BOUNDARY_OFF_BY_ONE" in categories
    assert "NULL_EMPTY_GUARD" in categories
    assert "STATE_MUTATION_EFFECT" in categories
    assert "CHAOS_UNHANDLED_EXCEPTION" in categories


def test_threshold_gate_and_force(tmp_path):
    repo_path = str(tmp_path)

    for i in range(3):
        GraphMemoryManager.crystallize_lesson(
            target_id=f"FUNC:orders.checkout_{i}",
            trap=f"Trap {i}",
            solution=f"Solution {i}",
            repo_path=repo_path,
        )

    # Threshold 10 not met -> threshold_not_met
    gate_res = GraphMemoryManager.cluster_unconsolidated_lessons(
        threshold=10, force=False, repo_path=repo_path
    )
    assert gate_res["status"] == "threshold_not_met"
    assert gate_res["count"] == 3
    assert len(gate_res["clusters"]) == 0

    # With force=True -> succeeds
    force_res = GraphMemoryManager.cluster_unconsolidated_lessons(
        threshold=10, force=True, repo_path=repo_path
    )
    assert force_res["status"] == "NEEDS_SYNTHESIS"
    assert len(force_res["clusters"]) >= 1


def test_commit_synthesized_axiom_and_token_shielding(tmp_path):
    repo_path = str(tmp_path)
    target = "FUNC:billing.invoice"

    # Add 3 lessons
    l1 = GraphMemoryManager.crystallize_lesson(
        target_id=target,
        trap="Mutant flipped price check",
        solution="Assert invoice total > 0",
        rule="Enforce positive total",
        repo_path=repo_path,
    )
    l2 = GraphMemoryManager.crystallize_lesson(
        target_id=target,
        trap="Mutant allowed None customer",
        solution="Assert customer is not None",
        rule="Guard customer reference",
        repo_path=repo_path,
    )

    # Pre-sleep ego check
    ego_pre = GraphMemoryManager.get_ego_memory_subgraph(target_id=target, repo_path=repo_path)
    assert len(ego_pre["unconsolidated_lessons"]) == 2
    assert len(ego_pre["axioms"]) == 0

    # Agent commits synthesized axiom
    commit_res = GraphMemoryManager.commit_synthesized_axiom(
        cluster_id="billing_integrity_1234",
        module="billing",
        rule="Always enforce positive pricing and customer presence guards.",
        lesson_ids=[l1["id"], l2["id"]],
        repo_path=repo_path,
    )
    assert commit_res["status"] == "success"
    assert commit_res["consolidated_lessons_count"] == 2

    # Post-sleep ego check: Token Shielding active
    ego_post = GraphMemoryManager.get_ego_memory_subgraph(target_id=target, repo_path=repo_path)
    assert len(ego_post["axioms"]) == 1
    assert ego_post["axioms"][0]["rule"] == "Always enforce positive pricing and customer presence guards."
    # Lessons should now be consolidated and subsumed
    assert len(ego_post["unconsolidated_lessons"]) == 0
    assert ego_post["subsumed_lessons_count"] == 2

    # Verify markdown file exists and has content
    axioms_md = GraphMemoryManager.get_axioms_file(repo_path)
    assert axioms_md.exists()
    content = axioms_md.read_text(encoding="utf-8")
    assert "### Module: `billing`" in content
    assert "positive pricing" in content


def test_arena_memory_notice_on_threshold(tmp_path, monkeypatch):
    repo_path = str(tmp_path)
    arena = TriAgentArena(repo_path=repo_path)

    # Pre-populate 10 unconsolidated lessons
    for i in range(10):
        GraphMemoryManager.crystallize_lesson(
            target_id=f"FUNC:core.worker_{i}",
            trap=f"Worker trap {i}",
            solution=f"Solution {i}",
            repo_path=repo_path,
        )

    # Mock svc.verify_proof to return TITANIUM
    monkeypatch.setattr(
        arena.svc,
        "verify_proof",
        lambda *args, **kwargs: {
            "proof_status": "pass",
            "proof_grade": "TITANIUM",
            "mutation_proof": {
                "mutants_killed": 4,
                "mutants_survived": 0,
                "details": [],
                "causal_reflexion": None,
            },
            "self_healing": {},
        },
    )
    monkeypatch.setattr(
        "proven.core.mutant_vault.MutantVault.run_vault_regression",
        lambda *args, **kwargs: {"status": "passed", "survived": 0},
    )

    result = arena.referee_round(
        target_id="FUNC:core.worker_0",
        test_target="tests/test_worker.py",
        round_no=1,
    )

    assert result.is_titanium is True
    assert result.verdict == "TITANIUM_VICTORY"
    assert result.memory_notice is not None
    assert result.memory_notice["needs_sleep"] is True
    assert result.memory_notice["unconsolidated_count"] >= 10
    assert result.memory_notice["action_required"] == "RUN_SLEEP_CONSOLIDATION"


def test_cli_sleep_command(tmp_path):
    from click.testing import CliRunner
    from proven.cli import cli

    repo_path = str(tmp_path)
    # Add 1 lesson
    GraphMemoryManager.crystallize_lesson(
        target_id="FUNC:api.handle",
        trap="Mutant removed status check",
        solution="Assert status == 200",
        rule="Check 200 OK",
        repo_path=repo_path,
    )

    runner = CliRunner()
    # Without force and threshold 10 -> threshold not met
    res1 = runner.invoke(cli, ["sleep", "--path", repo_path, "--threshold", "10", "--json"])
    assert res1.exit_code == 0
    assert "threshold_not_met" in res1.output

    # With --force -> consolidates
    res2 = runner.invoke(cli, ["sleep", "--path", repo_path, "--force", "--json"])
    assert res2.exit_code == 0
    assert "success" in res2.output

