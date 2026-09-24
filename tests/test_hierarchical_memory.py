"""Tests for Hierarchical Memory Graph, CLS Consolidation, and Token Shielding."""

import pytest
from pathlib import Path
from proven.core.memory_manager import GraphMemoryManager


def test_crystallize_lesson_and_ego_subgraph(tmp_path):
    repo_path = str(tmp_path)
    target_id = "FUNC:payments.process_charge"

    # Fast learning: crystallize lesson upon successful repair
    entry = GraphMemoryManager.crystallize_lesson(
        target_id=target_id,
        trap="Mutant changed `amount > 0` to `amount >= 0`",
        solution="Assert ValueError when amount == 0",
        rule="Always assert boundary zero values for payments",
        repo_path=repo_path,
    )

    assert entry["id"].startswith("LESSON:LES_")
    assert entry["type"] == "LESSON"
    assert entry["status"] == "unconsolidated"
    assert entry["target_id"] == target_id
    assert "ValueError" in entry["solution"]

    # Ego-graph retrieval
    ego = GraphMemoryManager.get_ego_memory_subgraph(target_id=target_id, repo_path=repo_path)
    assert ego["target_id"] == target_id
    assert len(ego["unconsolidated_lessons"]) == 1
    assert ego["unconsolidated_lessons"][0]["id"] == entry["id"]
    assert len(ego["axioms"]) == 0


def test_record_vulnerability(tmp_path):
    repo_path = str(tmp_path)
    target_id = "FUNC:auth.oauth_callback"

    vul = GraphMemoryManager.record_vulnerability(
        target_id=target_id,
        surviving_mutant_desc="Bypassed state nonce verification",
        round_exhausted=3,
        repo_path=repo_path,
    )

    assert vul["id"].startswith("VULNERABILITY:VUL_")
    assert vul["type"] == "VULNERABILITY"
    assert vul["round_exhausted"] == 3

    ego = GraphMemoryManager.get_ego_memory_subgraph(target_id=target_id, repo_path=repo_path)
    assert len(ego["vulnerabilities"]) == 1
    assert "state nonce" in ego["vulnerabilities"][0]["surviving_mutant"]

    prompt = GraphMemoryManager.get_scoped_memory_prompt(target_id=target_id, repo_path=repo_path)
    assert "Known Blindspot" in prompt


def test_hierarchical_sleep_consolidation(tmp_path):
    repo_path = str(tmp_path)
    target_1 = "FUNC:payments.charge"
    target_2 = "FUNC:payments.refund"

    # Crystallize 2 lessons in payments module
    GraphMemoryManager.crystallize_lesson(
        target_id=target_1,
        trap="Mutant inverted idempotency check",
        solution="Assert 409 Conflict on duplicate idempotency key",
        rule="Enforce idempotency verification on all charge mutations",
        repo_path=repo_path,
    )
    GraphMemoryManager.crystallize_lesson(
        target_id=target_2,
        trap="Mutant skipped balance check",
        solution="Assert balance >= refund amount",
        rule="Validate account balance before refunding",
        repo_path=repo_path,
    )

    # Perform Sleep Consolidation
    res = GraphMemoryManager.consolidate_axioms(repo_path=repo_path)
    assert res["status"] == "success"
    assert res["total_axioms"] >= 1

    # Check memory graph structure
    mem_graph = GraphMemoryManager.export_memory_graph(repo_path=repo_path)
    nodes = {n["id"]: n for n in mem_graph["nodes"]}
    edges = mem_graph["edges"]

    # Check that parent AXIOM exists
    axiom_nodes = [n for n in nodes.values() if n["type"] == "AXIOM"]
    assert len(axiom_nodes) >= 1
    ax = axiom_nodes[0]
    assert ax["module"] == "payments"

    # Check hierarchy edges
    generalizes_edges = [e for e in edges if e["type"] == "GENERALIZES"]
    assert len(generalizes_edges) >= 1
    assert generalizes_edges[0]["source"].startswith("AXIOM:")
    assert generalizes_edges[0]["target"].startswith("LESSON:")

    # Check constraint edges
    constrained_edges = [e for e in edges if e["type"] == "CONSTRAINED_BY"]
    assert len(constrained_edges) >= 1
    assert constrained_edges[0]["source"].startswith("FUNC:payments")

    # Check lessons marked consolidated
    for lnode in [n for n in nodes.values() if n["type"] == "LESSON"]:
        assert lnode["status"] == "consolidated"

    # Verify Token Shielding in Ego-Graph
    ego = GraphMemoryManager.get_ego_memory_subgraph(target_id=target_1, repo_path=repo_path)
    assert len(ego["axioms"]) >= 1
    # Prompt should include Axiom and stay ultra-compact (< 300 chars)
    prompt = GraphMemoryManager.get_scoped_memory_prompt(target_id=target_1, repo_path=repo_path)
    assert "[Axiom payments" in prompt
    assert len(prompt) < 300
