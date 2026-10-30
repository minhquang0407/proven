#!/usr/bin/env python3
"""Thin CLI wrapper -> `proven arena`."""
import argparse
import json
import sys
from _rust_bridge import run_proven


def main() -> int:
    parser = argparse.ArgumentParser(description="Referee Tri-Agent Adversarial Arena round via Proven CLI")
    parser.add_argument("--target", default=None)
    parser.add_argument("--test", default=None)
    parser.add_argument("--round", type=int, default=1)
    parser.add_argument("--mutation-check", action="store_true")
    parser.add_argument("--pytest-args", default=None)
    parser.add_argument("--path", "--repo", dest="path", default=".")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--schema", action="store_true")
    args = parser.parse_args()

    if args.schema:
        print(
            json.dumps(
                {
                    "tool": "run_arena",
                    "engine": "proven",
                    "args": [
                        "--target",
                        "--test",
                        "--round",
                        "--mutation-check",
                        "--pytest-args",
                        "--path",
                        "--json",
                    ],
                },
                indent=2,
            )
        )
        return 0

    if not args.target or not args.test:
        parser.error("--target and --test are required unless --schema is specified")

    cmd = [
        "arena",
        "--target",
        args.target,
        "--test",
        args.test,
        "--round",
        str(args.round),
        "--path",
        args.path,
    ]
    if args.mutation_check:
        cmd.append("--mutation-check")
    if args.pytest_args:
        cmd.extend(["--pytest-args", args.pytest_args])
    if args.json:
        cmd.append("--json")
    return run_proven(cmd)


if __name__ == "__main__":
    sys.exit(main())
