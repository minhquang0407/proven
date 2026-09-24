#!/usr/bin/env python3
"""Sync softgnn_advisor package into skills/softgnn-advisor/softgnn_advisor bundle."""

import os
import shutil
import sys
from pathlib import Path

def sync_bundle():
    root = Path(__file__).resolve().parent.parent
    src = root / "softgnn_advisor"
    dst = root / "skills" / "softgnn-advisor" / "softgnn_advisor"

    print(f"[SYNC] Source: {src}")
    print(f"[SYNC] Target: {dst}")

    if not src.exists():
        print(f"[ERROR] Source package {src} not found!")
        sys.exit(1)

    if dst.exists():
        shutil.rmtree(dst)

    def ignore_patterns(folder, contents):
        return [c for c in contents if c in ("__pycache__", ".pytest_cache") or c.endswith((".pyc", ".pyo"))]

    shutil.copytree(src, dst, ignore=ignore_patterns)
    print(f"[SYNC] Successfully bundled softgnn_advisor into {dst}")

if __name__ == "__main__":
    sync_bundle()
