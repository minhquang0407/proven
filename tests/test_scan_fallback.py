import subprocess
from pathlib import Path

from proven.core.scan_fallback import (
    generate_same_branch_fallback,
    git_changed_files,
    git_worktree_dirty,
    resolve_read_only_scan_range,
)


def _git(repo: Path, *args: str) -> str:
    result = subprocess.run(
        ['git', *args],
        cwd=repo,
        capture_output=True,
        text=True,
        encoding='utf-8',
        errors='replace',
        check=True,
    )
    return result.stdout.strip()


def _init_repo(tmp_path: Path) -> Path:
    repo = tmp_path / 'repo'
    repo.mkdir()
    _git(repo, 'init')
    _git(repo, 'config', 'user.email', 'test@example.com')
    _git(repo, 'config', 'user.name', 'Test User')
    (repo / 'app.py').write_text('def one():\n    return 1\n', encoding='utf-8')
    _git(repo, 'add', '.')
    _git(repo, 'commit', '-m', 'initial')
    return repo


def test_git_changed_files_empty_for_same_ref(tmp_path):
    repo = _init_repo(tmp_path)
    assert git_changed_files(str(repo), 'HEAD', 'HEAD') == []


def test_git_changed_files_returns_changed_files(tmp_path):
    repo = _init_repo(tmp_path)
    (repo / 'app.py').write_text('def one():\n    return 2\n', encoding='utf-8')
    _git(repo, 'add', '.')
    _git(repo, 'commit', '-m', 'change app')
    assert git_changed_files(str(repo), 'HEAD~1', 'HEAD') == ['app.py']


def test_resolve_read_only_scan_range_preserves_non_empty_original(tmp_path):
    repo = _init_repo(tmp_path)
    (repo / 'app.py').write_text('def one():\n    return 2\n', encoding='utf-8')
    _git(repo, 'add', '.')
    _git(repo, 'commit', '-m', 'change app')
    decision = resolve_read_only_scan_range(str(repo), 'HEAD~1', 'HEAD')
    assert decision.base == 'HEAD~1'
    assert decision.head == 'HEAD'
    assert decision.reason == 'original'
    assert decision.changed_files == ['app.py']


def test_generate_same_branch_fallback_detects_last_commit(tmp_path):
    repo = _init_repo(tmp_path)
    (repo / 'app.py').write_text('def one():\n    return 2\n', encoding='utf-8')
    _git(repo, 'add', '.')
    _git(repo, 'commit', '-m', 'change app')
    decision = generate_same_branch_fallback(str(repo), 'main', 'HEAD')
    assert decision.reason == 'same-branch-commit'
    assert decision.base == 'HEAD~1'
    assert decision.head == 'HEAD'
    assert decision.changed_files == ['app.py']


def test_generate_same_branch_fallback_detects_dirty_worktree(tmp_path):
    repo = _init_repo(tmp_path)
    (repo / 'app.py').write_text('def one():\n    return 3\n', encoding='utf-8')
    assert git_worktree_dirty(str(repo)) is True
    decision = generate_same_branch_fallback(str(repo), 'HEAD', 'HEAD')
    assert decision.reason == 'dirty-worktree'
