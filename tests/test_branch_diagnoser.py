"""Tests for BranchDiagnoser and Self-Healing Proof Gate."""

import os
import sys
from pathlib import Path
import pytest

from proven.core.branch_diagnoser import BranchDiagnoser
from proven.core.agent_service import AgentService


def test_diagnose_mocking_detected(tmp_path):
    source_file = tmp_path / "payments.py"
    source_file.write_text("""
def process_payment(amount, token):
    if token is None:
        return {"error": "no token"}
    # core business logic
    gateway_charge(amount)
    return {"status": "ok"}
""", encoding="utf-8")

    test_file = tmp_path / "test_payments.py"
    test_file.write_text("""
from unittest.mock import patch

@patch("payments.process_payment")
def test_payment(mock_fn):
    mock_fn.return_value = {"status": "mocked"}
    assert True
""", encoding="utf-8")

    diag = BranchDiagnoser.diagnose_failure(
        target_id="FUNC:process_payment",
        source_file=str(source_file),
        test_file=str(test_file),
        covered_lines=[],
        function_range=[2, 8],
        repo_path=str(tmp_path),
    )

    assert diag["status"] == "diagnosed"
    assert diag["diagnosis_type"] == "MOCKED_OUT"
    assert "process_payment" in diag["explanation"]
    assert "mocked" in diag["explanation"]
    assert "Remove mock" in diag["actionable_suggestion"]


def test_diagnose_early_branch_guard_clause(tmp_path):
    source_file = tmp_path / "order_service.py"
    # Lines:
    # 1: 
    # 2: def validate_order(order_id, user=None):
    # 3:     if user is None:
    # 4:         return False
    # 5:     # Core logic
    # 6:     item_count = count_items(order_id)
    # 7:     return item_count > 0
    source_code = (
        "\n"
        "def validate_order(order_id, user=None):\n"
        "    if user is None:\n"
        "        return False\n"
        "    item_count = count_items(order_id)\n"
        "    return item_count > 0\n"
    )
    source_file.write_text(source_code, encoding="utf-8")

    # Suppose test only executed lines 2, 3, 4 (stopped at line 4)
    diag = BranchDiagnoser.diagnose_failure(
        target_id="FUNC:validate_order",
        source_file=str(source_file),
        covered_lines=[2, 3, 4],
        function_range=[2, 6],
        repo_path=str(tmp_path),
    )

    assert diag["status"] == "diagnosed"
    assert diag["diagnosis_type"] == "EARLY_BRANCH"
    assert diag["branch_line"] == 3
    assert "user is None" in diag["condition"]
    assert "user is None" in diag["explanation"]
    assert ("Main body" in diag["explanation"] or "not reached" in diag["explanation"])
    assert "mock data" in diag["actionable_suggestion"] or "inputs" in diag["actionable_suggestion"]


def test_diagnose_caller_early_branch(tmp_path):
    caller_file = tmp_path / "checkout.py"
    caller_code = (
        "def checkout(cart):\n"
        "    if cart is None:\n"
        "        return False\n"
        "    return process_payment(cart)\n"
    )
    caller_file.write_text(caller_code, encoding="utf-8")

    source_file = tmp_path / "payments.py"
    source_file.write_text("def process_payment(cart):\n    return True\n", encoding="utf-8")

    # Caller only executed lines 1, 2, 3 (line 4 where process_payment is called was NOT reached)
    all_covered = {
        "checkout.py": {1, 2, 3}
    }

    diag = BranchDiagnoser.diagnose_failure(
        target_id="FUNC:process_payment",
        source_file="payments.py",
        covered_lines=[],
        function_range=[1, 2],
        all_covered_files=all_covered,
        repo_path=str(tmp_path),
    )

    assert diag["status"] == "diagnosed"
    assert diag["diagnosis_type"] == "CALLER_EARLY_BRANCH"
    assert diag["caller_name"] == "checkout"
    assert diag["branch_line"] == 2
    assert "process_payment" in diag["explanation"]
    assert "checkout" in diag["explanation"]


def test_diagnose_never_called(tmp_path):
    test_file = tmp_path / "test_unrelated.py"
    test_file.write_text("def test_nothing(): assert True\n", encoding="utf-8")

    source_file = tmp_path / "calc.py"
    source_file.write_text("def multiply(a, b):\n    return a * b\n", encoding="utf-8")

    diag = BranchDiagnoser.diagnose_failure(
        target_id="FUNC:multiply",
        source_file=str(source_file),
        test_file=str(test_file),
        covered_lines=[],
        function_range=[1, 2],
        repo_path=str(tmp_path),
    )

    assert diag["status"] == "diagnosed"
    assert diag["diagnosis_type"] == "NEVER_CALLED"
    assert "never referenced or called" in diag["explanation"]


def test_verify_proof_includes_self_healing(tmp_path):
    repo_root = Path(__file__).resolve().parent.parent
    svc = AgentService(project="test-diag", repo_path=str(repo_root))

    # Passing test that touches a dummy assert, never executing build_llm_provider
    passing_test = tmp_path / "test_dummy.py"
    passing_test.write_text("def test_dummy(): assert 1 + 1 == 2\n", encoding="utf-8")

    res = svc.verify_proof(
        target_id="FUNC:build_llm_provider",
        test_target=str(passing_test),
    )

    assert res["proof_status"] == "fail"
    assert res["covered_fraction"] == 0.0
    assert "self_healing" in res
    assert res["self_healing"] is not None
    assert res["self_healing"]["status"] == "diagnosed"
    assert "Self-Healing Diagnosis" in res["message"]


def test_diagnose_multilang_early_branch_typescript(tmp_path):
    ts_file = tmp_path / "order.ts"
    ts_code = (
        "export function processOrder(orderId: string, amount: number): boolean {\n"
        "  if (amount <= 0) return false;\n"
        "  const fee = amount * 0.1;\n"
        "  return saveOrder(orderId, fee);\n"
        "}\n"
    )
    ts_file.write_text(ts_code, encoding="utf-8")

    diag = BranchDiagnoser.diagnose_failure(
        target_id="FUNC:processOrder",
        source_file="order.ts",
        covered_lines=[1, 2],
        function_range=[1, 5],
        repo_path=str(tmp_path),
    )

    assert diag["status"] == "diagnosed"
    assert diag["diagnosis_type"] == "EARLY_BRANCH"
    assert diag["branch_line"] == 2
    assert "amount <= 0" in diag["condition"]
    assert "branched early at line 2" in diag["explanation"]


def test_diagnose_input_guard_detected(tmp_path):
    py_file = tmp_path / "auth.py"
    py_code = (
        "def verify_token(token, secret):\n"
        "    if not token:\n"
        "        raise ValueError('Empty token')\n"
        "    return decode(token, secret)\n"
    )
    py_file.write_text(py_code, encoding="utf-8")

    test_file = tmp_path / "test_auth.py"
    test_file.write_text("from auth import verify_token\n# never actually called\n", encoding="utf-8")

    diag = BranchDiagnoser.diagnose_failure(
        target_id="FUNC:verify_token",
        source_file="auth.py",
        test_file="test_auth.py",
        covered_lines=[],
        function_range=[1, 4],
        repo_path=str(tmp_path),
    )

    assert diag["status"] == "diagnosed"
    assert diag["diagnosis_type"] == "INPUT_GUARD_DETECTED"
    assert diag["branch_line"] == 2
    assert "not token" in diag["condition"]

