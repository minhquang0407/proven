#!/usr/bin/env python3
"""
Cross-Agent Skill Installer for Proven.
Installs the skill folder into Claude Code, Cursor/Codex/Windsurf, OpenClaw, or local project directories.
"""

import argparse
import os
import shutil
import sys
from pathlib import Path

SKILL_NAME = "proven"

AGENT_TARGETS = {
    "claude": Path.home() / ".claude" / "skills" / SKILL_NAME,
    "openclaw": Path.home() / ".openclaw" / "skills" / SKILL_NAME,
    "cursor": Path.cwd() / ".agents" / "skills" / SKILL_NAME,
    "codex": Path.cwd() / ".agents" / "skills" / SKILL_NAME,
    "windsurf": Path.cwd() / ".windsurf" / "skills" / SKILL_NAME,
}


def get_skill_source_dir() -> Path:
    script_dir = Path(__file__).resolve().parent
    skill_dir = script_dir.parent
    if not (skill_dir / "SKILL.md").exists():
        print(f"Error: SKILL.md not found in {skill_dir}", file=sys.stderr)
        sys.exit(1)
    return skill_dir


def install_to_target(skill_source: Path, target: str, use_symlink: bool = False) -> None:
    targets_to_install = (
        list(AGENT_TARGETS.items())
        if target == "all"
        else [(target, AGENT_TARGETS[target])]
    )

    installed_paths: set[str] = set()
    source_resolved = skill_source.resolve()

    for name, dest in targets_to_install:
        dest_resolved = dest.resolve()
        dest_str = str(dest_resolved)
        if dest_str in installed_paths:
            continue
        installed_paths.add(dest_str)

        if dest_resolved == source_resolved:
            print(f"[{name}] Source and destination are identical ({dest}); skipping self-copy.")
            continue

        dest.parent.mkdir(parents=True, exist_ok=True)
        if dest.exists() or dest.is_symlink():
            if dest.is_symlink() or dest.is_file():
                dest.unlink()
            else:
                shutil.rmtree(dest)

        if use_symlink:
            try:
                os.symlink(skill_source, dest, target_is_directory=True)
                print(f"[{name}] Symlinked skill -> {dest}")
                continue
            except OSError as e:
                print(f"[{name}] Symlink failed ({e}), falling back to copy...")

        shutil.copytree(
            skill_source,
            dest,
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc", ".DS_Store"),
        )
        print(f"[{name}] Installed skill -> {dest}")


def main():
    parser = argparse.ArgumentParser(
        description="Install Proven skill into coding agent directories."
    )
    parser.add_argument(
        "--target",
        choices=["claude", "openclaw", "cursor", "codex", "windsurf", "all"],
        default="all",
        help="Target agent platform to install the skill into (default: all)",
    )
    parser.add_argument(
        "--symlink",
        action="store_true",
        help="Create a symbolic link instead of copying files (useful for development)",
    )
    args = parser.parse_args()

    skill_source = get_skill_source_dir()
    print(f"Installing Proven skill from: {skill_source}")
    install_to_target(skill_source, args.target, use_symlink=args.symlink)
    print("\nDone! Your coding agent will now auto-discover Proven.")


if __name__ == "__main__":
    main()
