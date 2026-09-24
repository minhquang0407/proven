import types

from proven.core.plan_cache import save_plan_bundle, load_plan_bundle, validate_plan_bundle


class Result:
    pass


def _plan(target_id='FUNC:add', source_file='src/calc.py'):
    return types.SimpleNamespace(
        target_id=target_id,
        source_file=source_file,
        test_file='tests/test_calc.py',
        test_names=['test_add'],
        rationale='cover add',
        code='def test_add():\n    assert 1 + 1 == 2\n',
        assumptions=[],
    )


def _target():
    return types.SimpleNamespace(
        node_id='FUNC:add',
        source_file='src/calc.py',
        reason='missing coverage',
        priority=100.0,
        evidence=['unit'],
        suggested_file='tests/test_calc.py',
    )


def test_save_load_and_validate_plan_bundle(tmp_path, monkeypatch):
    repo = tmp_path / 'repo'
    (repo / 'src').mkdir(parents=True)
    (repo / 'src' / 'calc.py').write_text('def add(a, b):\n    return a + b\n', encoding='utf-8')

    result = Result()
    result.targets = [_target()]
    result.plans = [_plan()]
    result.warnings = []

    plan_path, latest_path, bundle = save_plan_bundle('cache-test', result, str(repo), change_source='full-scan', plan_id='plan1')
    loaded, loaded_path = load_plan_bundle('cache-test')

    assert loaded['plan_id'] == 'plan1'
    assert loaded_path.endswith('latest_plan.json')
    assert validate_plan_bundle(loaded, str(repo)).valid

    (repo / 'src' / 'calc.py').write_text('def add(a, b):\n    return a - b\n', encoding='utf-8')
    validation = validate_plan_bundle(loaded, str(repo))

    assert not validation.valid
    assert any('src/calc.py' in warning for warning in validation.warnings)

