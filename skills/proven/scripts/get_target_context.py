#!/usr/bin/env python3
"""Thin CLI wrapper -> `proven context`."""
import argparse
import json
import sys
from _rust_bridge import run_proven


def main() -> int:
    parser = argparse.ArgumentParser(description="Extract scoped AST + GraphRAG context via Proven CLI")
    parser.add_argument("--target", default=None)
    parser.add_argument("--file", default=None)
    parser.add_argument("--path", "--repo", dest="path", default=".")
    parser.add_argument("--blind", action="store_true", help="Omit target source_code for spec-first test authoring")
    parser.add_argument("--spec", default=None, help="Explicit external specification text or relative file path")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--schema", action="store_true")
    args = parser.parse_args()

    if args.schema:
        print(
            json.dumps(
                {
                    "tool": "get_target_context",
                    "engine": "proven",
                    "args": ["--target", "--file", "--path", "--blind", "--spec", "--json"],
                },
                indent=2,
            )
        )
        return 0

    cmd = ["context", "--path", args.path]
    if args.target:
        cmd.extend(["--target", args.target])
    if args.file:
        cmd.extend(["--file", args.file])
    if args.blind:
        cmd.append("--blind")
    if args.spec:
        cmd.extend(["--spec", args.spec])
    if args.json:
        cmd.append("--json")
    return run_proven(cmd)


if __name__ == "__main__":
    sys.exit(main())
