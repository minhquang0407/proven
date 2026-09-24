import os

from proven.core.change_provider import (
    FilesystemSnapshotChangeProvider,
    FullScanChangeProvider,
    build_filesystem_snapshot,
    save_filesystem_snapshot,
)


def test_full_scan_detects_python_files(tmp_path):
    src = tmp_path / 'src'
    src.mkdir()
    (src / 'calculator.py').write_text('def add(a, b):\n    return a + b\n', encoding='utf-8')
    (src / 'README.md').write_text('ignore me', encoding='utf-8')

    changes = FullScanChangeProvider(str(tmp_path)).detect_changes()

    assert changes.source == 'full-scan'
    assert [f.path for f in changes.files] == ['src/calculator.py']
    assert changes.files[0].status == 'added'
    assert changes.files[0].hunks[0].start_line == 1


def test_filesystem_snapshot_detects_added_modified_deleted(tmp_path):
    src = tmp_path / 'src'
    src.mkdir()
    old_file = src / 'old.py'
    modified_file = src / 'modified.py'
    old_file.write_text('def old():\n    return 1\n', encoding='utf-8')
    modified_file.write_text('def value():\n    return 1\n', encoding='utf-8')

    snapshot_path = tmp_path / 'snapshot.json'
    save_filesystem_snapshot(str(snapshot_path), build_filesystem_snapshot(str(tmp_path)))

    old_file.unlink()
    modified_file.write_text('def value():\n    return 2\n', encoding='utf-8')
    (src / 'new.py').write_text('def new():\n    return 3\n', encoding='utf-8')

    changes = FilesystemSnapshotChangeProvider(str(tmp_path), str(snapshot_path)).detect_changes()
    by_path = {changed.path: changed for changed in changes.files}

    assert changes.source == 'filesystem'
    assert by_path['src/new.py'].status == 'added'
    assert by_path['src/modified.py'].status == 'modified'
    assert by_path['src/old.py'].status == 'deleted'

