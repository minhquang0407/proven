import os
from pathlib import Path

from proven.core.test_generation_agent import GeneratedTestPlan, TestGenerationAgent


class DummyScanner:
    repo_path = ""

    def scan(self, *args, **kwargs):
        raise AssertionError("scan should not be called")


def _agent(tmp_path, monkeypatch):
    monkeypatch.setattr("proven.core.test_generation_agent.PRScanner", lambda project, repo_path=None: DummyScanner())
    agent = TestGenerationAgent("tmp", repo_path=str(tmp_path))
    agent.llm_provider.available = False
    return agent


def test_block_rollback_keeps_passing_block_in_same_new_file(tmp_path, monkeypatch):
    agent = _agent(tmp_path, monkeypatch)
    calls = []

    def fake_run_pytest(files, stream=False):
        calls.append(files)
        target = files[0]
        if target.endswith("::test_fail"):
            return 1, "FAILED test_fail - AssertionError"
        return 0, "passed"

    monkeypatch.setattr(agent, "_run_pytest", fake_run_pytest)
    plan_fail = GeneratedTestPlan("FUNC:fail", "tests/test_generated.py", ["test_fail"], "", "def test_fail():\n    assert False")
    plan_pass = GeneratedTestPlan("FUNC:pass", "tests/test_generated.py", ["test_pass"], "", "def test_pass():\n    assert True")

    result = agent.apply_saved_plans([plan_fail, plan_pass], verify=True, repair_iters=0, refresh_runtime=False)

    test_file = tmp_path / "tests" / "test_generated.py"
    content = test_file.read_text(encoding="utf-8")
    assert 'target="FUNC:fail"' not in content
    assert 'target="FUNC:pass"' in content
    assert "def test_pass" in content
    assert {item.target_id: item.status for item in result.verification_results} == {
        "FUNC:fail": "rolled_back",
        "FUNC:pass": "kept",
    }
    assert str(test_file).endswith(os.path.join("tests", "test_generated.py"))
    assert calls == [["tests/test_generated.py::test_fail"], ["tests/test_generated.py::test_pass"]]


def test_block_rollback_preserves_existing_user_content(tmp_path, monkeypatch):
    agent = _agent(tmp_path, monkeypatch)
    tests_dir = tmp_path / "tests"
    tests_dir.mkdir()
    test_file = tests_dir / "test_existing.py"
    test_file.write_text("def test_user_existing():\n    assert True\n", encoding="utf-8")

    monkeypatch.setattr(agent, "_run_pytest", lambda files, stream=False: (1, "FAILED test_bad - AssertionError"))
    plan = GeneratedTestPlan("FUNC:bad", "tests/test_existing.py", ["test_bad"], "", "def test_bad():\n    assert False")

    result = agent.apply_saved_plans([plan], verify=True, repair_iters=0, refresh_runtime=False)

    content = test_file.read_text(encoding="utf-8")
    assert "def test_user_existing" in content
    assert 'target="FUNC:bad"' not in content
    assert result.verification_results[0].status == "rolled_back"
