#!/usr/bin/env python3
"""Sync proven package and backward-compatible shim into skills/proven bundle."""

import os
import shutil
import sys
from pathlib import Path

def sync_bundle():
    root = Path(__file__).resolve().parent.parent
    src_proven = root / "proven"
    dst_proven = root / "skills" / "proven" / "proven"
    src_compat = root / "softgnn_advisor"
    dst_compat = root / "skills" / "proven" / "softgnn_advisor"

    print(f"[SYNC] Source: {src_proven}")
    print(f"[SYNC] Target: {dst_proven}")

    if not src_proven.exists():
        print(f"[ERROR] Source package {src_proven} not found!")
        sys.exit(1)

    def ignore_patterns(folder, contents):
        return [c for c in contents if c in ("__pycache__", ".pytest_cache") or c.endswith((".pyc", ".pyo"))]

    # Sync proven
    if dst_proven.exists():
        shutil.rmtree(dst_proven)
    shutil.copytree(src_proven, dst_proven, ignore=ignore_patterns)
    print(f"[SYNC] Successfully bundled proven into {dst_proven}")

    # Sync backward-compatibility shim
    if dst_compat.exists():
        shutil.rmtree(dst_compat)
    shutil.copytree(src_compat, dst_compat, ignore=ignore_patterns)
    print(f"[SYNC] Successfully bundled backward-compat shim into {dst_compat}")

if __name__ == "__main__":
    sync_bundle()
