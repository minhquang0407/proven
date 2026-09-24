"""Tests for CausalReflexionEngine."""

import pytest
from unittest.mock import MagicMock
from proven.core.reflexion_engine import CausalReflexionEngine, CausalReflexion


def test_reflexion_engine_diagnoses_weak_assertion_when_line_hit():
    target_id = "FUNC:calculate_discount"
    mutant_info = {
        "mutant_id": 1,
        "line": 42,
        "mutation": "Eq -> NotEq",
        "original_code": "if is_vip == True:",
        "mutated_code": "if is_vip != True:",
    }
    # Line 42 WAS executed by the test
    covered_lines = [40, 41, 42, 43, 45]

    reflexion = CausalReflexionEngine.diagnose(
        target_id=target_id,
        mutant_info=mutant_info,
        covered_lines=covered_lines,
        round_no=1,
    )

    assert isinstance(reflexion, CausalReflexion)
    assert reflexion.target_id == target_id
    assert reflexion.failure_mode == "WEAK_ASSERTION"
    assert reflexion.execution_status == "HIT"
    assert "physically executed" in reflexion.causal_root_cause
    assert "Strengthen test assertions" in reflexion.actionable_directive
    assert "CRITIC REFLEXION MEMORY" in reflexion.reflexion_prompt
    assert "WEAK_ASSERTION" in reflexion.reflexion_prompt


def test_reflexion_engine_diagnoses_uncovered_branch_when_line_missed():
    target_id = "FUNC:calculate_discount"
    mutant_info = {
        "mutant_id": 2,
        "line": 50,
        "mutation": "Gt -> LtE",
        "original_code": "if price > 1000:",
        "mutated_code": "if price <= 1000:",
    }
    # Line 50 was NOT executed by the test
    covered_lines = [40, 41, 42]

    reflexion = CausalReflexionEngine.diagnose(
        target_id=target_id,
        mutant_info=mutant_info,
        covered_lines=covered_lines,
        round_no=2,
    )

    assert reflexion.failure_mode == "UNCOVERED_BRANCH"
    assert reflexion.execution_status == "MISSED"
    assert "NOT executed" in reflexion.causal_root_cause
    assert "satisfies the branch conditions" in reflexion.actionable_directive
    assert "MISSED" in reflexion.reflexion_prompt


def test_reflexion_engine_incorporates_graph_topology():
    target_id = "FUNC:checkout"
    mutant_info = {
        "mutant_id": 1,
        "line": 15,
        "mutation": "Add -> Sub",
        "original_code": "total = price + tax",
    }
    mock_graph = MagicMock()
    mock_graph.predecessors.return_value = ["FUNC:api_order_create", "FUNC:batch_processor"]
    mock_graph.nodes = {
        "FUNC:checkout": {"type": "Function"},
        "FUNC:api_order_create": {"type": "Function"},
        "FUNC:batch_processor": {"type": "Function"},
    }
    mock_graph_repo = MagicMock(graph=mock_graph)

    reflexion = CausalReflexionEngine.diagnose(
        target_id=target_id,
        mutant_info=mutant_info,
        covered_lines=[15],
        graph_repo=mock_graph_repo,
    )

    assert len(reflexion.downstream_callers) == 2
    assert reflexion.downstream_callers[0]["caller_id"] == "FUNC:api_order_create"
    assert "FUNC:api_order_create" in reflexion.reflexion_prompt


def test_verify_proof_attaches_causal_reflexion_on_survived_mutant(tmp_path):
    from unittest.mock import patch
    from proven.core.agent_service import AgentService
    from proven.infrastructure.pipelines.runtime_coverage_mapper import RuntimeCoverageEdge

    svc = AgentService(project="test_proj", repo_path=str(tmp_path))
    
    mock_edge = RuntimeCoverageEdge(
        test_id="tests/test_foo.py::test_calc",
        target_id="FUNC:pricing.calc",
        relation="executes",
        confidence=1.0,
        source_file="pricing.py",
        covered_lines=[10, 11],
        function_range=[10, 15],
        coverage_context="line",
        covered_line_count=2,
        function_line_count=5,
        covered_fraction=0.4,
        mode="pytest",
    )
    mock_mut_res = {
        "status": "success",
        "proof_grade": "SILVER",
        "mutants_survived": 1,
        "details": [
            {
                "status": "survived",
                "line": 10,
                "mutation": "Eq -> NotEq",
                "original_code": "if x == 1:",
                "mutated_code": "if x != 1:",
            }
        ],
        "message": "Weak assertion detected",
    }

    with patch.object(svc, "ensure_initialized", return_value=True), \
         patch("subprocess.run", return_value=MagicMock(returncode=0, stdout=".", stderr="")), \
         patch("proven.infrastructure.pipelines.runtime_coverage_mapper.RuntimeCoverageMapper.map_runtime_coverage", return_value=MagicMock(runtime_edges=[mock_edge])), \
         patch.object(svc, "_resolve_source_file", return_value="pricing.py"), \
         patch("proven.core.mutation_gate.MicroMutationGate.evaluate_mutations", return_value=mock_mut_res):
        
        res = svc.verify_proof(
            target_id="FUNC:pricing.calc",
            test_target="tests/test_foo.py",
            mutation_check=True,
        )

        assert res["status"] == "success"
        assert res["proof_grade"] == "SILVER"
        assert "causal_reflexion" in res["mutation_proof"]
        reflexion = res["mutation_proof"]["causal_reflexion"]
        assert reflexion["failure_mode"] == "WEAK_ASSERTION"
        assert reflexion["execution_status"] == "HIT"
        assert "CRITIC REFLEXION MEMORY" in reflexion["reflexion_prompt"]
        assert "reflexion_prompt" in res["self_healing"]

