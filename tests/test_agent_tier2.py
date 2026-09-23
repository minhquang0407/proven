import sys
from unittest.mock import MagicMock, patch
import pytest
from click.testing import CliRunner

from softgnn_advisor.core.agent_service import AgentService
from softgnn_advisor.core.triage_engine import TriageEngine
from softgnn_advisor.cli import cli


def test_agent_service_predict_impact():
    svc = AgentService(project="test_proj", repo_path=".", language="python")
    
    mock_target = MagicMock(full_id="FUNC:test_func", node_type="Function")
    mock_direct_cand = MagicMock(
        label="FUNC:caller_func",
        node_type="Function",
        final_score=0.85,
        rule_score=0.85,
        gnn_score=0.0,
        tiers=["Direct"],
        relations=["direct caller"],
        paths=["tests/test_foo.py"],
    )
    mock_latent_cand = MagicMock(
        label="FILE:remote_service.py",
        node_type="File",
        final_score=0.72,
        rule_score=0.2,
        gnn_score=0.91,
        tiers=["GNN", "Historical"],
        relations=["git co-change"],
        paths=["remote_service.py"],
    )
    mock_result = MagicMock(
        target=mock_target,
        internal_members=[],
        candidates=[mock_direct_cand, mock_latent_cand],
        mode="hybrid",
        direct_count=1,
        warnings=[],
    )

    with patch.object(svc, "ensure_initialized", return_value=True):
        with patch("softgnn_advisor.core.impact_engine.ImpactEngine") as mock_engine_cls:
            mock_engine_cls.return_value.analyze.return_value = mock_result
            res = svc.predict_impact("FUNC:test_func", mode="hybrid")
            assert res["status"] == "success"
            assert res["target"] == "FUNC:test_func"
            assert res["direct_impact_count"] == 1
            assert len(res["direct_dependents"]) == 1
            assert res["direct_dependents"][0]["symbol"] == "FUNC:caller_func"
            assert len(res["latent_risk_candidates"]) == 1
            assert res["latent_risk_candidates"][0]["symbol"] == "FILE:remote_service.py"
            assert res["latent_risk_candidates"][0]["gnn_score"] == 0.91


def test_agent_service_predict_impact_non_python():
    svc = AgentService(project="test_proj", repo_path=".", language="typescript")
    res = svc.predict_impact("FUNC:processPayment")
    assert res["status"] == "warning"
    assert "typescript" in res["message"]


def test_triage_engine_missing_dependency():
    engine = TriageEngine("test_proj")
    with patch.dict(sys.modules, {"torch": None}):
        res = engine.triage("Critical bug")
        assert res["status"] == "error"
        assert res["error_type"] == "MISSING_DEPENDENCY"


def test_triage_engine_model_not_found(tmp_path):
    engine = TriageEngine("non_existent_proj", str(tmp_path))
    m = MagicMock()
    mock_dict = {
        "torch": m,
        "torch.nn": m,
        "torch.nn.functional": m,
        "torch_geometric": m,
        "torch_geometric.transforms": m,
        "pandas": m,
        "softgnn_advisor.core.ai.predicter": m,
        "softgnn_advisor.core.ai.gnn_architecture": m,
        "softgnn_advisor.infrastructure.pipelines.feature_encoder": m,
    }
    with patch.dict(sys.modules, mock_dict):
        res = engine.triage("Critical payment timeout bug")
        assert res["status"] == "error"
        assert res["error_type"] == "MODEL_NOT_FOUND"


def test_agent_service_triage_bug():
    svc = AgentService(project="test_proj", repo_path=".", language="python")
    expected_triage = {
        "status": "success",
        "project": "test_proj",
        "query": "Fix auth vulnerability",
        "top_engineers": [
            {
                "rank": 1,
                "developer": "Alice",
                "final_score": 0.92,
                "gnn_score": 0.95,
                "git_score": 0.88,
                "sem_score": 0.90,
                "evidence": "Direct: auth.py (10)",
            }
        ],
        "related_files": [
            {
                "rank": 1,
                "file": "auth.py",
                "relevance": 0.94,
                "semantic_score": 0.95,
                "lexical_score": 0.92,
            }
        ],
    }

    with patch.object(svc, "ensure_initialized", return_value=True):
        with patch.object(TriageEngine, "triage", return_value=expected_triage):
            res = svc.triage_bug("Fix auth vulnerability")
            assert res["status"] == "success"
            assert len(res["top_engineers"]) == 1
            assert res["top_engineers"][0]["developer"] == "Alice"


def test_agent_service_train_gnn():
    svc = AgentService(project="test_proj", repo_path=".", language="python")
    mock_train_module = MagicMock(run_optimization=MagicMock(return_value=None))
    with patch.object(svc, "ensure_initialized", return_value=True):
        with patch.dict(sys.modules, {"softgnn_advisor.scripts.train_model": mock_train_module}):
            with patch("softgnn_advisor.core.agent_service.load_metadata", return_value={"best_val_auc": 0.89, "test_auc": 0.87}):
                res = svc.train_gnn()
                assert res["status"] == "success"
                assert res["test_auc"] == 0.87


def test_cli_agent_impact():
    runner = CliRunner()
    mock_res = {
        "status": "success",
        "target": "FUNC:foo",
        "direct_impact_count": 0,
        "direct_dependents": [],
        "latent_risk_candidates": [],
    }
    with patch("softgnn_advisor.core.agent_service.AgentService.predict_impact", return_value=mock_res):
        result = runner.invoke(cli, ["agent", "impact", "--target", "FUNC:foo", "--json"])
        assert result.exit_code == 0
        assert '"FUNC:foo"' in result.output


def test_cli_agent_triage():
    runner = CliRunner()
    mock_res = {
        "status": "success",
        "query": "bug description",
        "top_engineers": [{"rank": 1, "developer": "Bob", "final_score": 0.8}],
        "related_files": [],
    }
    with patch("softgnn_advisor.core.agent_service.AgentService.triage_bug", return_value=mock_res):
        result = runner.invoke(cli, ["agent", "triage", "bug description", "--json"])
        assert result.exit_code == 0
        assert '"Bob"' in result.output


def test_cli_agent_train():
    runner = CliRunner()
    mock_res = {
        "status": "success",
        "message": "Trained successfully in 5.2s.",
        "test_auc": 0.9,
    }
    with patch("softgnn_advisor.core.agent_service.AgentService.train_gnn", return_value=mock_res):
        result = runner.invoke(cli, ["agent", "train", "--json"])
        assert result.exit_code == 0
        assert '"Trained successfully in 5.2s."' in result.output
