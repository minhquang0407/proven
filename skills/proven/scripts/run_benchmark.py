#!/usr/bin/env python3
"""Thin CLI wrapper -> `proven benchmark`."""
import argparse
import json
import sys
from _rust_bridge import run_proven


def main() -> int:
    parser = argparse.ArgumentParser(description="Run 4-Regime Empirical Benchmark via Proven CLI")
    parser.add_argument("--path", "--repo", dest="path", default=".")
    parser.add_argument("--regime", default="all", choices=["all", "small", "large", "dirty", "cross-repo"])
    parser.add_argument("--external-repo", default=None)
    parser.add_argument("--init-arena", default=None, help="Initialize 4-regime agent arena at dir")
    parser.add_argument("--grade-arena", default=None, help="Grade 4-regime agent arena at dir")
    parser.add_argument("--report", default=None)
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--schema", action="store_true")
    args = parser.parse_args()

    if args.schema:
        print(
            json.dumps(
                {
                    "tool": "run_benchmark",
                    "engine": "proven",
                    "args": [
                        "--path",
                        "--regime",
                        "--external-repo",
                        "--init-arena",
                        "--grade-arena",
                        "--report",
                        "--json",
                    ],
                },
                indent=2,
            )
        )
        return 0

    if args.init_arena:
        cmd = ["benchmark", "init", "--dir", args.init_arena]
    elif args.grade_arena:
        cmd = ["benchmark", "grade", "--dir", args.grade_arena]
        if args.report:
            cmd.extend(["--report", args.report])
    else:
        cmd = ["benchmark", "run", "--path", args.path, "--regime", args.regime]
        if args.external_repo:
            cmd.extend(["--external-repo", args.external_repo])
        if args.report:
            cmd.extend(["--report", args.report])

    if args.json:
        cmd.append("--json")
    return run_proven(cmd)


if __name__ == "__main__":
    sys.exit(main())
