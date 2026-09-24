"""Tests for Zero-Token MutantVault."""

import pytest
from pathlib import Path
from proven.core.mutant_vault import MutantVault


def test_mutant_vault_store_and_list(tmp_path):
    repo_path = str(tmp_path)
    target_id = "FUNC:pricing.calculate_total"
    
    stored = MutantVault.store_mutant(
        target_id=target_id,
        source_file="pricing.py",
        line=10,
        mutation_desc="Add -> Sub",
        original_code="return a + b",
        mutated_code="return a - b",
        defeated_test="tests/test_pricing.py::test_case_1",
        repo_path=repo_path,
    )

    assert stored["mutant_id"].startswith("mut_")
    assert stored["line"] == 10
    assert "tests/test_pricing.py::test_case_1" in stored["defeated_tests"]

    # List mutants
    all_mutants = MutantVault.list_mutants(target_id=target_id, repo_path=repo_path)
    assert len(all_mutants) == 1
    assert all_mutants[0]["mutant_id"] == stored["mutant_id"]

    # Clear mutants
    deleted = MutantVault.clear_vault(target_id=target_id, repo_path=repo_path)
    assert deleted == 1
    assert len(MutantVault.list_mutants(target_id=target_id, repo_path=repo_path)) == 0


def test_mutant_vault_run_regression_kills_mutants_with_good_test(tmp_path):
    repo_path = tmp_path
    src_file = repo_path / "calc.py"
    src_file.write_text("def add(a, b):\n    return a + b\n", encoding="utf-8")

    test_file = repo_path / "test_calc.py"
    # Strong assertion that kills 'return a - b'
    test_file.write_text("from calc import add\ndef test_add():\n    assert add(2, 3) == 5\n", encoding="utf-8")

    target_id = "FUNC:calc.add"
    MutantVault.store_mutant(
        target_id=target_id,
        source_file="calc.py",
        line=2,
        mutation_desc="Add -> Sub",
        original_code="    return a + b",
        mutated_code="    return a - b",
        repo_path=str(repo_path),
    )

    res = MutantVault.run_vault_regression(
        target_id=target_id,
        test_target=str(test_file),
        repo_path=str(repo_path),
    )

    assert res["status"] == "passed"
    assert res["total_vault_mutants"] == 1
    assert res["killed"] == 1
    assert res["survived"] == 0
    # Verify source file was restored
    assert src_file.read_text(encoding="utf-8") == "def add(a, b):\n    return a + b\n"


def test_mutant_vault_run_regression_catches_surviving_mutant(tmp_path):
    repo_path = tmp_path
    src_file = repo_path / "calc.py"
    src_file.write_text("def add(a, b):\n    return a + b\n", encoding="utf-8")

    test_file = repo_path / "test_calc.py"
    # Weak assertion that will NOT catch 'return a + b + 0' or similar
    # Let's say test asserts add(0, 0) >= 0 which passes even if return a - b
    test_file.write_text("from calc import add\ndef test_add():\n    assert add(0, 0) == 0\n", encoding="utf-8")

    target_id = "FUNC:calc.add"
    MutantVault.store_mutant(
        target_id=target_id,
        source_file="calc.py",
        line=2,
        mutation_desc="Add -> Sub",
        original_code="    return a + b",
        mutated_code="    return a - b",
        repo_path=str(repo_path),
    )

    res = MutantVault.run_vault_regression(
        target_id=target_id,
        test_target=str(test_file),
        repo_path=str(repo_path),
    )

    assert res["status"] == "regression_detected"
    assert res["total_vault_mutants"] == 1
    assert res["killed"] == 0
    assert res["survived"] == 1
    assert len(res["survived_mutants"]) == 1
    # Verify source file was restored
    assert src_file.read_text(encoding="utf-8") == "def add(a, b):\n    return a + b\n"
