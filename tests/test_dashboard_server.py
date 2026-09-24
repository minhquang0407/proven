import pytest

from proven.core.dashboard_server import ALLOWED_ACTIONS, _start_job


def test_dashboard_action_allowlist_rejects_unknown(tmp_path):
    with pytest.raises(ValueError):
        _start_job('demo', str(tmp_path), {'action': 'rm -rf .'})


def test_dashboard_allowlist_contains_expected_actions():
    assert {'refresh', 'scan', 'pr_scan', 'impact', 'map_runtime', 'generate_file'} <= ALLOWED_ACTIONS
