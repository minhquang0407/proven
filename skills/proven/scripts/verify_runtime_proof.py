#!/usr/bin/env python3
"""Thin CLI wrapper -> `proven verify-proof`."""
import argparse
import json
import sys
from _rust_bridge import run_proven


def main() -> int:
    parser = argparse.ArgumentParser(description="Verify runtime proof and 3-Persona mutation gate via Proven CLI")
    parser.add_argument("--target", default=None)
    parser.add_argument("--test", default=None)
    parser.add_argument("--file", default=None)
    parser.add_argument("--path", "--repo", dest="path", default=".")
    parser.add_argument("--mutation-check", action="store_true")
    parser.add_argument("--max-mutants", type=int, default=3)
    parser.add_argument("--pytest-args", default=None)
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--schema", action="store_true")
    args = parser.parse_args()

    if args.schema:
        print(
            json.dumps(
                {
                    "tool": "verify_runtime_proof",
                    "engine": "proven",
                    "args": [
                        "--target",
                        "--test",
                        "--file",
                        "--path",
                        "--mutation-check",
                        "--max-mutants",
                        "--pytest-args",
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
        "verify-proof",
        "--target",
        args.target,
        "--test",
        args.test,
        "--path",
        args.path,
        "--max-mutants",
        str(args.max_mutants),
    ]
    if args.file:
        cmd.extend(["--file", args.file])
    if args.mutation_check:
        cmd.append("--mutation-check")
    if args.pytest_args:
        cmd.extend(["--pytest-args", args.pytest_args])
    if args.json:
        cmd.append("--json")
    return run_proven(cmd)


if __name__ == "__main__":
    sys.exit(main())
