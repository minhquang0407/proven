"""Tests for SoftGNN GitHub Action runner and PR commenting."""

import json
import os
import sys
from unittest.mock import MagicMock, patch
import pytest

from proven.scripts.ci.github_action import (
    COMMENT_MARKER,
    format_audit_markdown,
    parse_event_payload,
    update_or_create_comment,
    write_github_outputs,
    write_step_summary,
    main,
)


def test_format_audit_markdown_with_missing_coverage():
    scan_result = {
        "changed_count": 2,
        "contract_diff_count": 1,
        "missing_count": 2,
        "language": "python",
        "missing_coverage": [
            {"target_id": "FUNC:process_payment", "file": "src/payments.py", "risk": 0.85},
            {"target_id": "FUNC:validate_token", "file": "src/auth.py", "risk": 0.72},
        ],
    }

    md = format_audit_markdown(scan_result, repo_full_name="org/repo")

    assert COMMENT_MARKER in md
    assert "SoftGNN Advisor — Untested Changes Detected" in md
    assert "FUNC:process_payment" in md
    assert "FUNC:validate_token" in md
    assert "src/payments.py" in md
    assert "0.85" in md
    assert "Coding Agent Guidance" in md
    assert "get_target_context.py" in md
    assert "verify_runtime_proof.py" in md


def test_format_audit_markdown_with_zero_missing_coverage():
    scan_result = {
        "changed_count": 3,
        "contract_diff_count": 0,
        "missing_count": 0,
        "language": "typescript",
        "missing_coverage": [],
    }

    md = format_audit_markdown(scan_result, repo_full_name="org/repo")

    assert COMMENT_MARKER in md
    assert "SoftGNN Advisor — All Changed Functions Have Runtime Proof!" in md
    assert "100%" in md
    assert "typescript" in md
    assert "Coding Agent Guidance" not in md


def test_parse_event_payload(tmp_path):
    assert parse_event_payload(None) == {}
    assert parse_event_payload(str(tmp_path / "nonexistent.json")) == {}

    event_file = tmp_path / "event.json"
    event_file.write_text('{"pull_request": {"number": 42}}', encoding="utf-8")
    parsed = parse_event_payload(str(event_file))
    assert parsed.get("pull_request", {}).get("number") == 42

    # Malformed json
    bad_file = tmp_path / "bad.json"
    bad_file.write_text("invalid json", encoding="utf-8")
    assert parse_event_payload(str(bad_file)) == {}


def test_write_step_summary_and_outputs(tmp_path):
    summary_file = tmp_path / "summary.md"
    output_file = tmp_path / "output.txt"

    write_step_summary(str(summary_file), "# Report")
    assert summary_file.read_text(encoding="utf-8").strip() == "# Report"

    write_github_outputs(str(output_file), {
        "missing-count": 2,
        "has-gaps": "true"
    })
    output_content = output_file.read_text(encoding="utf-8")
    assert "missing-count=2\n" in output_content
    assert "has-gaps=true\n" in output_content


def test_update_comment_in_place_when_comment_exists():
    mock_api = MagicMock()
    # List comments returns one comment containing COMMENT_MARKER
    mock_api.side_effect = [
        [
            {"id": 101, "body": "Some other comment"},
            {"id": 202, "body": f"Old report {COMMENT_MARKER}"}
        ],
        {"id": 202, "body": "updated"} # Result of PATCH
    ]

    with patch("proven.scripts.ci.github_action.github_api_request", mock_api):
        update_or_create_comment(
            repo_full_name="test-org/test-repo",
            pr_number=42,
            token="gh_fake_token",
            markdown_body="New body with marker"
        )

        assert mock_api.call_count == 2
        # First call is GET list comments
        call1 = mock_api.call_args_list[0]
        assert "issues/42/comments" in call1[0][0]
        assert call1[1]["method"] == "GET"

        # Second call is PATCH existing comment 202
        call2 = mock_api.call_args_list[1]
        assert "issues/comments/202" in call2[0][0]
        assert call2[1]["method"] == "PATCH"
        assert call2[1]["data"] == {"body": "New body with marker"}


def test_create_new_comment_when_none_exists():
    mock_api = MagicMock()
    mock_api.side_effect = [
        [{"id": 101, "body": "Unrelated comment"}], # List comments
        {"id": 303, "body": "created"}             # Result of POST
    ]

    with patch("proven.scripts.ci.github_action.github_api_request", mock_api):
        update_or_create_comment(
            repo_full_name="test-org/test-repo",
            pr_number=42,
            token="gh_fake_token",
            markdown_body="New body with marker"
        )

        assert mock_api.call_count == 2
        # First call is GET
        assert mock_api.call_args_list[0][1]["method"] == "GET"
        # Second call is POST to issues/42/comments
        call2 = mock_api.call_args_list[1]
        assert "issues/42/comments" in call2[0][0]
        assert call2[1]["method"] == "POST"
        assert call2[1]["data"] == {"body": "New body with marker"}


def test_main_cli_execution_fail_on_missing_strict(tmp_path):
    summary_path = tmp_path / "summary.md"
    output_path = tmp_path / "output.txt"

    mock_scan_result = {
        "changed_count": 1,
        "contract_diff_count": 0,
        "missing_count": 1,
        "language": "python",
        "missing_coverage": [{"target_id": "FUNC:foo", "file": "src/foo.py", "risk": 0.9}],
    }

    test_args = [
        "github_action.py",
        "--fail-on-missing", "true",
        "--comment-pr", "false",
        "--step-summary", str(summary_path),
        "--github-output", str(output_path),
    ]

    with patch("sys.argv", test_args):
        with patch("proven.core.agent_service.AgentService.scan", return_value=mock_scan_result):
            with pytest.raises(SystemExit) as exc:
                main()
            assert exc.value.code == 1

    assert "missing-count=1" in output_path.read_text(encoding="utf-8")
    assert "has-gaps=true" in output_path.read_text(encoding="utf-8")
    assert "FUNC:foo" in summary_path.read_text(encoding="utf-8")


def test_main_cli_execution_advisory_mode_passes(tmp_path):
    summary_path = tmp_path / "summary.md"
    output_path = tmp_path / "output.txt"

    mock_scan_result = {
        "changed_count": 1,
        "contract_diff_count": 0,
        "missing_count": 1,
        "language": "python",
        "missing_coverage": [{"target_id": "FUNC:foo", "file": "src/foo.py", "risk": 0.9}],
    }

    test_args = [
        "github_action.py",
        "--fail-on-missing", "false", # Advisory only
        "--comment-pr", "false",
        "--step-summary", str(summary_path),
        "--github-output", str(output_path),
    ]

    with patch("sys.argv", test_args):
        with patch("proven.core.agent_service.AgentService.scan", return_value=mock_scan_result):
            with pytest.raises(SystemExit) as exc:
                main()
            assert exc.value.code == 0
