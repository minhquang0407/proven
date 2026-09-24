"""Automated tests for SoftGNN Agent Skill and CLI agent group."""

import json
import os
import subprocess
import sys
import pytest
from pathlib import Path

from proven.core.agent_service import AgentService


@pytest.fixture
def repo_root():
    return str(Path(__file__).resolve().parent.parent)


def test_agent_service_init_and_infer_project(repo_root):
    svc = AgentService(repo_path=repo_root)
    assert svc.project is not None
    assert os.path.exists(svc.repo_path)


def test_agent_service_get_context(repo_root):
    svc = AgentService(project="proven-test", repo_path=repo_root)
    # Test on a known function in the codebase
    target_id = "FUNC:build_llm_provider"
    source_file = "proven/core/llm_provider.py"
    context = svc.get_context(target_id=target_id, source_file=source_file)

    assert context["status"] == "success"
    assert context["target_id"] == target_id
    assert "build_llm_provider" in context["source_code"]
    assert "def build_llm_provider(" in context["signature"]
    assert context["suggested_test_file"].startswith("tests/")


def test_agent_service_verify_proof_failure_when_test_fails(repo_root, tmp_path):
    svc = AgentService(project="proven-test", repo_path=repo_root)
    # Create a failing test in tmp_path
    failing_test = tmp_path / "test_failing.py"
    failing_test.write_text("def test_broken(): assert 1 == 2\n", encoding="utf-8")

    res = svc.verify_proof(
        target_id="FUNC:build_llm_provider",
        test_target=str(failing_test),
    )
    assert res["proof_status"] == "fail"
    assert res["pytest_passed"] is False
    assert res["pytest_returncode"] != 0


def test_agent_service_verify_proof_failure_when_target_not_executed(repo_root, tmp_path):
    svc = AgentService(project="proven-test", repo_path=repo_root)
    # Create a test that passes but never touches build_llm_provider
    passing_test = tmp_path / "test_dummy.py"
    passing_test.write_text("def test_dummy(): assert 1 + 1 == 2\n", encoding="utf-8")

    res = svc.verify_proof(
        target_id="FUNC:build_llm_provider",
        test_target=str(passing_test),
    )
    # Pytest passes, but proof MUST fail because 0 lines of build_llm_provider were executed!
    assert res["pytest_passed"] is True
    assert res["proof_status"] == "fail"
    assert res["covered_fraction"] == 0.0
    assert "did NOT execute target" in res["message"]


def test_cli_agent_context(repo_root):
    cmd = [
        sys.executable,
        "-m",
        "proven.cli",
        "agent",
        "context",
        "--target",
        "FUNC:build_llm_provider",
        "--file",
        "proven/core/llm_provider.py",
        "--path",
        repo_root,
        "--json",
    ]
    proc = subprocess.run(cmd, cwd=repo_root, capture_output=True, text=True, encoding="utf-8")
    assert proc.returncode == 0
    data = json.loads(proc.stdout)
    assert data["status"] == "success"
    assert "build_llm_provider" in data["source_code"]


def test_standalone_script_get_context(repo_root):
    script_path = os.path.join(repo_root, "skills", "proven", "scripts", "get_target_context.py")
    cmd = [
        sys.executable,
        script_path,
        "--target",
        "FUNC:build_llm_provider",
        "--file",
        "proven/core/llm_provider.py",
        "--path",
        repo_root,
    ]
    proc = subprocess.run(cmd, cwd=repo_root, capture_output=True, text=True, encoding="utf-8")
    assert proc.returncode == 0
    data = json.loads(proc.stdout)
    assert data["status"] == "success"
    assert data["target_id"] == "FUNC:build_llm_provider"


def test_cli_agent_scan(repo_root):
    cmd = [
        sys.executable,
        "-m",
        "proven.cli",
        "agent",
        "scan",
        "--path",
        repo_root,
        "--json",
    ]
    proc = subprocess.run(cmd, cwd=repo_root, capture_output=True, text=True, encoding="utf-8")
    assert proc.returncode == 0
    data = json.loads(proc.stdout)
    assert data["status"] == "success"
    assert "changed_files" in data
    assert "missing_coverage" in data


def test_standalone_script_scan_impact(repo_root):
    script_path = os.path.join(repo_root, "skills", "proven", "scripts", "scan_impact.py")
    cmd = [
        sys.executable,
        script_path,
        "--path",
        repo_root,
    ]
    proc = subprocess.run(cmd, cwd=repo_root, capture_output=True, text=True, encoding="utf-8")
    assert proc.returncode == 0
    data = json.loads(proc.stdout)
    assert data["status"] == "success"
    assert "changed_nodes" in data


def test_agent_service_verify_proof_success_when_target_executed(repo_root):
    svc = AgentService(project="proven-test", repo_path=repo_root)
    # Write a test inside tests/ that actually calls build_llm_provider
    test_code = """
import pytest
from proven.core.llm_provider import build_llm_provider, LLMConfig

def test_calls_build_llm_provider():
    config = LLMConfig(provider="template", model="none", base_url="http://none")
    provider = build_llm_provider(config)
    assert provider is not None
"""
    passing_test = Path(repo_root) / "tests" / "test_temp_verify_call.py"
    passing_test.write_text(test_code, encoding="utf-8")

    try:
        res = svc.verify_proof(
            target_id="FUNC:build_llm_provider",
            test_target=str(passing_test),
        )
        assert res["pytest_passed"] is True
        assert res["proof_status"] == "pass"
        assert res["covered_fraction"] > 0.0
        assert "Runtime proof CONFIRMED" in res["message"]
    finally:
        if passing_test.exists():
            passing_test.unlink()


def test_standalone_scripts_schema_flag(repo_root):
    scripts = [
        "scan_impact.py",
        "get_target_context.py",
        "verify_runtime_proof.py",
        "refresh_runtime_map.py",
        "query_impact.py",
        "triage_expert.py",
        "train_gnn.py",
    ]
    for script_name in scripts:
        script_path = os.path.join(repo_root, "skills", "proven", "scripts", script_name)
        proc = subprocess.run(
            [sys.executable, script_path, "--schema"],
            cwd=repo_root,
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        assert proc.returncode == 0, f"{script_name} --schema failed: {proc.stderr}"
        schema = json.loads(proc.stdout)
        assert "$schema" in schema
        assert "title" in schema
        assert schema.get("type") == "object"


def test_agent_service_refresh_runtime_targeted(repo_root):
    svc = AgentService(repo_path=repo_root)
    res = svc.refresh_runtime(tests="tests/test_scan_fallback.py")
    assert res["status"] == "success"
    assert res["target_used"] == "tests/test_scan_fallback.py"
    assert res["passed_tests"] > 0
    assert "mode_used" in res


def test_backward_compatibility_softgnn_advisor_shim():
    import softgnn_advisor
    from softgnn_advisor.core.agent_service import AgentService as CompatAgentService
    from proven.core.agent_service import AgentService as ProvenAgentService

    assert CompatAgentService is ProvenAgentService
    assert hasattr(softgnn_advisor, "__version__")

