#!/usr/bin/env python3
"""Thin CLI wrapper -> `proven memory`."""
import argparse
import json
import sys
from _rust_bridge import run_proven


def main() -> int:
    parser = argparse.ArgumentParser(description="Commit or list CLS memory axioms via Proven CLI")
    parser.add_argument("--target", default=None)
    parser.add_argument("--lesson", default=None)
    parser.add_argument("--failure-mode", default="BOUNDARY_OFF_BY_ONE")
    parser.add_argument("--mutant-desc", default="MANUAL_PIN")
    parser.add_argument("--list", action="store_true")
    parser.add_argument("--meta-promote", action="store_true")
    parser.add_argument("--meta-list", action="store_true")
    parser.add_argument("--path", "--repo", dest="path", default=".")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--schema", action="store_true")
    args = parser.parse_args()

    if args.schema:
        print(
            json.dumps(
                {
                    "tool": "commit_axiom",
                    "engine": "proven",
                    "args": [
                        "--target",
                        "--lesson",
                        "--failure-mode",
                        "--mutant-desc",
                        "--list",
                        "--meta-promote",
                        "--meta-list",
                        "--path",
                        "--json",
                    ],
                },
                indent=2,
            )
        )
        return 0

    if args.meta_promote:
        cmd = ["memory", "meta-promote", "--path", args.path]
    elif args.meta_list:
        cmd = ["memory", "meta-list", "--path", args.path]
    elif args.list or not args.lesson:
        cmd = ["memory", "list", "--path", args.path]
        if args.target:
            cmd.extend(["--target", args.target])
    else:
        cmd = [
            "memory",
            "pin",
            "--target",
            args.target or "GLOBAL",
            "--lesson",
            args.lesson,
            "--failure-mode",
            args.failure_mode,
            "--mutant-desc",
            args.mutant_desc,
            "--path",
            args.path,
        ]
    if args.json:
        cmd.append("--json")
    return run_proven(cmd)


if __name__ == "__main__":
    sys.exit(main())
