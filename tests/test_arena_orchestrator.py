"""Tests for TriAgentArena orchestrator."""

import pytest
from unittest.mock import MagicMock, patch
from proven.core.swarm.arena import TriAgentArena, ArenaRoundResult


def test_arena_get_author_brief():
    arena = TriAgentArena(repo_path=".")
    with patch.object(arena.svc, "get_context", return_value={
        "status": "success",
        "target_id": "FUNC:calculate_total",
        "signature": "def calculate_total(items)",
        "source_file": "pricing.py",
        "source_code": "def calculate_total(items): return sum(items)",
        "suggested_test_file": "tests/test_pricing.py",
    }):
        brief = arena.get_author_brief("FUNC:calculate_total")
        assert brief["status"] == "success"
        assert brief["persona"] == "Author Agent (Defensive Tester)"
        assert "def calculate_total" in brief["signature"]
        assert "defensive tests" in brief["directive"].lower()


def test_arena_get_adversary_brief(tmp_path):
    arena = TriAgentArena(repo_path=str(tmp_path))
    test_file = tmp_path / "tests" / "test_pricing.py"
    test_file.parent.mkdir(parents=True, exist_ok=True)
    test_file.write_text("def test_pricing(): assert True\n", encoding="utf-8")

    with patch.object(arena.svc, "get_context", return_value={
        "status": "success",
        "target_id": "FUNC:calculate_total",
        "source_file": "pricing.py",
        "source_code": "def calculate_total(items): return sum(items)",
    }):
        brief = arena.get_adversary_brief("FUNC:calculate_total", test_target="tests/test_pricing.py")
        assert brief["status"] == "success"
        assert brief["persona"] == "Adversary Agent (Semantic Bugmaker)"
        assert "test_pricing" in brief["author_test_code"]
        assert "semantic mutation" in brief["directive"].lower()


def test_arena_referee_round_achieves_titanium():
    arena = TriAgentArena(repo_path=".")
    
    mock_vault_res = {"status": "passed", "total_vault_mutants": 1, "killed": 1, "survived": 0}
    mock_proof_res = {
        "status": "success",
        "proof_status": "pass",
        "proof_grade": "TITANIUM",
        "mutation_proof": {
            "mutants_killed": 3,
            "mutants_survived": 0,
        },
    }

    with patch("proven.core.mutant_vault.MutantVault.run_vault_regression", return_value=mock_vault_res), \
         patch.object(arena.svc, "verify_proof", return_value=mock_proof_res):
        
        result = arena.referee_round(
            target_id="FUNC:calculate_total",
            test_target="tests/test_pricing.py",
            round_no=1,
        )

        assert isinstance(result, ArenaRoundResult)
        assert result.is_titanium is True
        assert result.proof_grade == "TITANIUM"
        assert result.mutants_killed == 3
        assert result.mutants_survived == 0
        assert "TITANIUM GRADE ACHIEVED" in result.message


def test_arena_referee_round_emits_reflexion_on_defeat():
    arena = TriAgentArena(repo_path=".")
    
    mock_vault_res = {"status": "passed", "total_vault_mutants": 0, "killed": 0, "survived": 0}
    mock_reflexion = {
        "failure_mode": "WEAK_ASSERTION",
        "execution_status": "HIT",
        "reflexion_prompt": "### CRITIC REFLEXION MEMORY (Round 1 Defeat)",
    }
    mock_proof_res = {
        "status": "success",
        "proof_status": "pass",
        "proof_grade": "SILVER",
        "mutation_proof": {
            "mutants_killed": 1,
            "mutants_survived": 1,
            "causal_reflexion": mock_reflexion,
        },
        "self_healing": {
            "reflexion_prompt": "### CRITIC REFLEXION MEMORY (Round 1 Defeat)",
        },
    }

    with patch("proven.core.mutant_vault.MutantVault.run_vault_regression", return_value=mock_vault_res), \
         patch.object(arena.svc, "verify_proof", return_value=mock_proof_res):
        
        result = arena.referee_round(
            target_id="FUNC:calculate_total",
            test_target="tests/test_pricing.py",
            round_no=1,
        )

        assert result.is_titanium is False
        assert result.proof_grade == "SILVER"
        assert result.mutants_survived == 1
        assert "Round 1 Defeat" in result.message
        assert "CRITIC REFLEXION MEMORY" in result.reflexion_prompt
