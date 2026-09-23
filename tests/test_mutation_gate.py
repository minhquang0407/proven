"""Tests for MicroMutationGate and Fan-Out Sub-Agent Tasks."""

import os
from pathlib import Path
import pytest

from softgnn_advisor.core.mutation_gate import MicroMutationGate
from softgnn_advisor.core.agent_service import AgentService


def test_mutation_gate_kills_mutants_with_strong_assertions(tmp_path):
    source_file = tmp_path / "pricing.py"
    source_code = (
        "def calculate_total(price, quantity):\n"
        "    if quantity > 10:\n"
        "        return price * quantity * 0.9\n"
        "    return price * quantity\n"
    )
    source_file.write_text(source_code, encoding="utf-8")

    # A strong test verifying exact outputs for both > 10 and <= 10
    test_file = tmp_path / "test_pricing.py"
    test_code = (
        "from pricing import calculate_total\n"
        "\n"
        "def test_pricing_discount():\n"
        "    assert calculate_total(100, 20) == 1800.0\n"
        "    assert calculate_total(100, 5) == 500.0\n"
    )
    test_file.write_text(test_code, encoding="utf-8")

    res = MicroMutationGate.evaluate_mutations(
        target_id="FUNC:calculate_total",
        source_file="pricing.py",
        test_target=str(test_file),
        repo_path=str(tmp_path),
        max_mutants=2,
    )

    assert res["status"] == "success"
    assert res["mutants_total"] > 0
    assert res["mutants_killed"] > 0
    assert res["mutants_survived"] == 0
    assert res["proof_grade"] == "TITANIUM"
    assert res["mutation_gate_status"] == "passed"
    assert "TITANIUM PROOF CONFIRMED" in res["message"]

    # Verify original file was cleanly restored
    assert source_file.read_text(encoding="utf-8") == source_code


def test_mutation_gate_catches_weak_assertion(tmp_path):
    source_file = tmp_path / "auth.py"
    source_code = (
        "def is_admin(user_role):\n"
        "    if user_role == 'admin':\n"
        "        return True\n"
        "    return False\n"
    )
    source_file.write_text(source_code, encoding="utf-8")

    # A weak test that only asserts result is not None (assertion trap!)
    test_file = tmp_path / "test_auth.py"
    test_code = (
        "from auth import is_admin\n"
        "\n"
        "def test_is_admin_weak():\n"
        "    # Weak assertion: checks not None instead of True\n"
        "    assert is_admin('admin') is not None\n"
    )
    test_file.write_text(test_code, encoding="utf-8")

    res = MicroMutationGate.evaluate_mutations(
        target_id="FUNC:is_admin",
        source_file="auth.py",
        test_target=str(test_file),
        repo_path=str(tmp_path),
        max_mutants=2,
    )

    assert res["status"] == "success"
    # When == is mutated to !=, is_admin('admin') returns False, which is STILL not None!
    # The mutant survives because the assertion is too weak!
    assert res["mutants_survived"] > 0
    assert res["proof_grade"] == "SILVER"
    assert res["mutation_gate_status"] == "weak_assertions"
    assert "WEAK ASSERTION DETECTED" in res["message"]

    # Verify original file was cleanly restored
    assert source_file.read_text(encoding="utf-8") == source_code


def test_generate_fanout_tasks(tmp_path):
    repo_root = Path(__file__).resolve().parent.parent
    svc = AgentService(project="test-fanout", repo_path=str(repo_root))

    # Fanout returns list of task dicts
    tasks_res = svc.generate_fanout_tasks(base="HEAD", head="HEAD")
    assert tasks_res["status"] == "success"
    assert "tasks" in tasks_res
    assert isinstance(tasks_res["tasks"], list)
