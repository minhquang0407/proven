"""Tests for TriAgentArena stopping criteria and state-transition memory crystallization."""

import pytest
from unittest.mock import MagicMock, patch
from proven.core.swarm.arena import TriAgentArena


def test_arena_titanium_crystallizes_lesson(tmp_path):
    repo_path = str(tmp_path)
    arena = TriAgentArena(repo_path=repo_path)

    mock_vault_res = {"status": "passed", "total_vault_mutants": 0, "killed": 0, "survived": 0}
    mock_proof_res = {
        "status": "success",
        "proof_status": "pass",
        "proof_grade": "TITANIUM",
        "mutation_proof": {
            "mutants_killed": 2,
            "mutants_survived": 0,
        },
    }

    with patch("proven.core.mutant_vault.MutantVault.run_vault_regression", return_value=mock_vault_res), \
         patch.object(arena.svc, "verify_proof", return_value=mock_proof_res):
        
        result = arena.referee_round(
            target_id="FUNC:pricing.calculate_total",
            test_target="tests/test_pricing.py",
            round_no=2,
            max_rounds=3,
        )

        assert result.verdict == "TITANIUM_VICTORY"
        assert result.is_titanium is True
        assert result.crystallized_lesson is not None
        assert result.crystallized_lesson["type"] == "LESSON"
        assert result.crystallized_lesson["target_id"] == "FUNC:pricing.calculate_total"


def test_arena_intermediate_defeat_does_not_create_lesson(tmp_path):
    repo_path = str(tmp_path)
    arena = TriAgentArena(repo_path=repo_path)

    mock_vault_res = {"status": "passed", "total_vault_mutants": 0, "killed": 0, "survived": 0}
    mock_proof_res = {
        "status": "success",
        "proof_status": "pass",
        "proof_grade": "SILVER",
        "mutation_proof": {
            "mutants_killed": 1,
            "mutants_survived": 1,
            "details": [{"status": "survived", "mutation": "amount > 0 -> amount >= 0"}],
            "causal_reflexion": {"reflexion_prompt": "Assert boundary zero value"},
        },
    }

    with patch("proven.core.mutant_vault.MutantVault.run_vault_regression", return_value=mock_vault_res), \
         patch.object(arena.svc, "verify_proof", return_value=mock_proof_res):
        
        result = arena.referee_round(
            target_id="FUNC:pricing.calculate_total",
            test_target="tests/test_pricing.py",
            round_no=1,
            max_rounds=3,
        )

        assert result.verdict == "CONTINUE"
        assert result.is_titanium is False
        assert result.crystallized_lesson is None
        assert result.recorded_vulnerability is None
        assert result.reflexion_prompt == "Assert boundary zero value"


def test_arena_budget_exhaustion_records_vulnerability(tmp_path):
    repo_path = str(tmp_path)
    arena = TriAgentArena(repo_path=repo_path)

    mock_vault_res = {"status": "passed", "total_vault_mutants": 0, "killed": 0, "survived": 0}
    mock_proof_res = {
        "status": "success",
        "proof_status": "pass",
        "proof_grade": "SILVER",
        "mutation_proof": {
            "mutants_killed": 1,
            "mutants_survived": 1,
            "details": [{"status": "survived", "mutation": "amount > 0 -> amount >= 0"}],
        },
    }

    with patch("proven.core.mutant_vault.MutantVault.run_vault_regression", return_value=mock_vault_res), \
         patch.object(arena.svc, "verify_proof", return_value=mock_proof_res):
        
        result = arena.referee_round(
            target_id="FUNC:pricing.calculate_total",
            test_target="tests/test_pricing.py",
            round_no=3,
            max_rounds=3,
        )

        assert result.verdict == "MAX_ROUNDS_EXHAUSTED"
        assert result.is_titanium is False
        assert result.recorded_vulnerability is not None
        assert result.recorded_vulnerability["type"] == "VULNERABILITY"
        assert result.recorded_vulnerability["round_exhausted"] == 3


def test_arena_stagnation_detected(tmp_path):
    repo_path = str(tmp_path)
    arena = TriAgentArena(repo_path=repo_path)

    mock_vault_res = {"status": "passed", "total_vault_mutants": 0, "killed": 0, "survived": 0}
    mock_proof_res = {
        "status": "success",
        "proof_status": "pass",
        "proof_grade": "SILVER",
        "mutation_proof": {
            "mutants_killed": 1,
            "mutants_survived": 1,
            "details": [{"status": "survived", "mutation": "amount > 0 -> amount >= 0"}],
        },
    }

    with patch("proven.core.mutant_vault.MutantVault.run_vault_regression", return_value=mock_vault_res), \
         patch.object(arena.svc, "verify_proof", return_value=mock_proof_res):
        
        result = arena.referee_round(
            target_id="FUNC:pricing.calculate_total",
            test_target="tests/test_pricing.py",
            round_no=2,
            max_rounds=3,
            previous_survived_mutant="amount > 0 -> amount >= 0",
        )

        assert result.verdict == "STAGNATION_DETECTED"
        assert "Stagnation detected" in result.message
