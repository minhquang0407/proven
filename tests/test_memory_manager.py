"""Tests for GraphMemoryManager and Sleep Consolidation."""

import pytest
from pathlib import Path
from proven.core.memory_manager import GraphMemoryManager


def test_graph_memory_pin_and_retrieve(tmp_path):
    repo_path = str(tmp_path)
    target_id = "FUNC:pricing.calculate_discount"
    lesson = "Always assert exact decimal output and test VIP boundary."

    entry = GraphMemoryManager.pin_lesson(
        target_id=target_id,
        lesson=lesson,
        failure_mode="WEAK_ASSERTION",
        mutant_desc="Eq -> NotEq",
        repo_path=repo_path,
    )

    assert entry["lesson_id"].startswith("LES_")
    assert entry["target_id"] == target_id
    assert entry["lesson"] == lesson

    # Retrieve lessons
    lessons = GraphMemoryManager.get_pinned_lessons(target_id=target_id, repo_path=repo_path)
    assert len(lessons) == 1
    assert lessons[0]["lesson"] == lesson


def test_scoped_memory_prompt_is_concise(tmp_path):
    repo_path = str(tmp_path)
    target_id = "FUNC:pricing.calculate_discount"
    GraphMemoryManager.pin_lesson(
        target_id=target_id,
        lesson="Assert exact value, avoid >= 0.",
        repo_path=repo_path,
    )

    prompt = GraphMemoryManager.get_scoped_memory_prompt(target_id=target_id, repo_path=repo_path)
    assert "REPO MEMORY & AXIOMS" in prompt
    assert "Assert exact value, avoid >= 0." in prompt
    # Check length: must be ultra-compact (< 300 chars, ~60 tokens)
    assert len(prompt) < 300


def test_sleep_consolidation_into_axioms(tmp_path):
    repo_path = str(tmp_path)
    # Pin lessons to multiple functions in pricing and auth modules
    GraphMemoryManager.pin_lesson(
        target_id="FUNC:pricing.calc_discount",
        lesson="Assert exact discount values.",
        repo_path=repo_path,
    )
    GraphMemoryManager.pin_lesson(
        target_id="FUNC:pricing.apply_tax",
        lesson="Round tax to 2 decimal places.",
        repo_path=repo_path,
    )
    GraphMemoryManager.pin_lesson(
        target_id="FUNC:auth.login",
        lesson="Verify JWT token expiration.",
        repo_path=repo_path,
    )

    res = GraphMemoryManager.consolidate_axioms(repo_path=repo_path)
    assert res["status"] == "success"
    assert res["total_axioms"] == 3
    assert "pricing" in res["modules"]
    assert "auth" in res["modules"]

    axioms_file = Path(res["axioms_file"])
    assert axioms_file.exists()
    content = axioms_file.read_text(encoding="utf-8")
    assert "### Module: `pricing`" in content
    assert "### Module: `auth`" in content
    assert "[Axiom pricing.1]" in content
    assert "[Axiom auth.1]" in content
