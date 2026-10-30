#!/usr/bin/env python3
"""
Proven CLI wrapper bridge -> compiled `proven` binary.
Locates an existing `proven` binary or downloads/builds it automatically on first run
(so `npx skills add minhquang0407/proven` works out of the box across platforms).
"""

import hashlib
import os
import platform
import shutil
import stat
import subprocess
import sys
import urllib.request
from pathlib import Path

GITHUB_REPO = "minhquang0407/proven"
RELEASE_BASE_URL = f"https://github.com/{GITHUB_REPO}/releases/latest/download"

if hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass


def _detect_release_asset_name() -> str | None:
    sys_name = platform.system().lower()
    machine = platform.machine().lower()

    if sys_name == "windows" and machine in ("amd64", "x86_64"):
        return "proven-windows-x86_64.exe"
    if sys_name == "linux" and machine in ("x86_64", "amd64"):
        return "proven-linux-x86_64"
    if sys_name == "darwin":
        if machine in ("arm64", "aarch64"):
            return "proven-macos-aarch64"
        if machine in ("x86_64", "amd64"):
            return "proven-macos-x86_64"
    return None


def _download_prebuilt_binary(dest_bin: Path) -> bool:
    asset_name = _detect_release_asset_name()
    if not asset_name:
        return False

    dest_bin.parent.mkdir(parents=True, exist_ok=True)
    tmp_bin = dest_bin.with_suffix(dest_bin.suffix + f".tmp.{os.getpid()}")
    asset_url = f"{RELEASE_BASE_URL}/{asset_name}"
    sums_url = f"{RELEASE_BASE_URL}/SHA256SUMS.txt"

    try:
        req = urllib.request.Request(asset_url, headers={"User-Agent": "proven-skill-bootstrap"})
        with urllib.request.urlopen(req, timeout=30) as resp:
            data = resp.read()
        if not data:
            return False

        # Verify SHA256 checksum if SHA256SUMS.txt is published on the release
        try:
            sums_req = urllib.request.Request(sums_url, headers={"User-Agent": "proven-skill-bootstrap"})
            with urllib.request.urlopen(sums_req, timeout=10) as sums_resp:
                sums_txt = sums_resp.read().decode("utf-8", errors="replace")
            expected = None
            for line in sums_txt.splitlines():
                parts = line.strip().split()
                if len(parts) >= 2 and parts[-1].lstrip("*") == asset_name:
                    expected = parts[0].lower()
                    break
            if expected:
                actual = hashlib.sha256(data).hexdigest().lower()
                if actual != expected:
                    return False
        except Exception:
            pass

        tmp_bin.write_bytes(data)
        if os.name != "nt":
            tmp_bin.chmod(tmp_bin.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
        os.replace(tmp_bin, dest_bin)
        return dest_bin.is_file()
    except Exception:
        if tmp_bin.exists():
            try:
                tmp_bin.unlink()
            except OSError:
                pass
        return False


def _cargo_bootstrap_binary(skill_root: Path, dest_bin: Path) -> bool:
    cargo_bin = shutil.which("cargo")
    if not cargo_bin:
        return False
    try:
        proc = subprocess.run(
            [
                cargo_bin,
                "install",
                "--git",
                f"https://github.com/{GITHUB_REPO}.git",
                "--root",
                str(skill_root),
                "--force",
                "--quiet",
            ],
            check=False,
        )
        return proc.returncode == 0 and dest_bin.is_file()
    except Exception:
        return False


def find_proven_binary() -> str:
    exe_name = "proven.exe" if os.name == "nt" else "proven"
    script_dir = Path(__file__).resolve().parent
    skill_root = script_dir.parent
    bundled_bin = skill_root / "bin" / exe_name

    candidates = [
        # 1. Bundled or cached inside the skill directory: <skill_dir>/bin/proven(.exe)
        bundled_bin,
        # 2. Repo root target/release or target/debug (2 levels above skills/proven)
        skill_root.parent.parent / "target" / "release" / exe_name,
        skill_root.parent.parent / "target" / "debug" / exe_name,
        # 3. Repo root target/release (3 levels above .agents/skills/proven)
        skill_root.parent.parent / "target" / "release" / exe_name,
        skill_root.parent.parent / "target" / "debug" / exe_name,
        # 4. User's ~/.cargo/bin/proven(.exe)
        Path.home() / ".cargo" / "bin" / exe_name,
    ]
    for candidate in candidates:
        if candidate.is_file():
            return str(candidate)

    which_bin = shutil.which("proven")
    if which_bin:
        return which_bin

    # 5. Auto-bootstrap when installed standalone (e.g. via `npx skills add minhquang0407/proven`)
    if _download_prebuilt_binary(bundled_bin):
        return str(bundled_bin)

    if _cargo_bootstrap_binary(skill_root, bundled_bin):
        return str(bundled_bin)

    raise FileNotFoundError(
        f"CLI binary '{exe_name}' not found. Install it via "
        f"`cargo install --git https://github.com/{GITHUB_REPO}` or download a release binary."
    )


def run_proven(subcommand_args: list[str]) -> int:
    binary = find_proven_binary()
    proc = subprocess.run(
        [binary, *subcommand_args],
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    return proc.returncode
