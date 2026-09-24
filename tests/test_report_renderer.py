import types

from proven.core.report_renderer import build_generate_report_payload, render_html_report, save_html_report
from proven.infrastructure.pipelines.runtime_coverage_mapper import RuntimeCoverageEdge


def test_render_html_report_escapes_input():
    payload = {
        'project': '<script>alert(1)</script>',
        'created_at': 'now',
        'summary': {'changed_files': 1, 'proof_pass': 1},
        'scan': {'changed_nodes': []},
        'plans': [],
        'verification': [],
        'runtime_edges': [],
    }
    html = render_html_report(payload)
    assert '<script>alert(1)</script>' not in html
    assert '&lt;script&gt;alert(1)&lt;/script&gt;' in html


def test_build_report_payload_includes_proof_counts():
    proof_edge = RuntimeCoverageEdge(
        test_id='TEST:tests/test_foo.py::test_calls_target',
        target_id='FUNC:target',
        relation='executes_runtime',
        confidence=1.0,
        source_file='foo.py',
        covered_lines=[1, 2],
        function_range=[1, 3],
        coverage_context='tests/test_foo.py::test_calls_target',
        covered_line_count=2,
        function_line_count=3,
        covered_fraction=0.6667,
        mode='per-test',
    )
    verification = types.SimpleNamespace(
        target_id='FUNC:target',
        test_file='tests/test_foo.py',
        pytest_target='tests/test_foo.py::test_calls_target',
        returncode=0,
        output='passed',
        status='kept',
        repair_attempts=[],
        proof_status='pass',
        proof_edges=[proof_edge],
        proof_message='edge confirmed',
    )
    apply_result = types.SimpleNamespace(
        plans=[],
        verification_results=[verification],
        runtime_result=types.SimpleNamespace(runtime_edges=[proof_edge]),
        files_written=['tests/test_foo.py'],
        warnings=[],
        failures=[],
    )
    payload = build_generate_report_payload('demo', apply_result=apply_result, repo_path='.')
    assert payload['summary']['proof_pass'] == 1
    assert payload['summary']['proof_fail'] == 0
    assert payload['summary']['runtime_edges'] == 1
    assert payload['verification'][0]['proof_edges'][0]['target_id'] == 'FUNC:target'


def test_save_html_report_writes_latest(tmp_path, monkeypatch):
    import proven.config.settings as settings
    import proven.core.report_renderer as renderer

    def fake_paths(project):
        reports = tmp_path / project / 'reports'
        reports.mkdir(parents=True, exist_ok=True)
        return {
            'REPORTS_DIR': reports,
            'LATEST_REPORT_PATH': reports / 'latest_report.html',
        }

    monkeypatch.setattr(renderer, 'get_project_paths', fake_paths)
    payload = {'project': 'demo', 'created_at': 'now', 'summary': {}, 'scan': {}, 'plans': [], 'verification': [], 'runtime_edges': []}
    report_path, latest_path = save_html_report('demo', payload, report_id='run1')
    assert report_path.endswith('run1_report.html')
    assert latest_path.endswith('latest_report.html')
    assert 'SoftGNN Report' in (tmp_path / 'demo' / 'reports' / 'latest_report.html').read_text(encoding='utf-8')
