#!/usr/bin/env python3
"""Thin CLI wrapper -> `proven triage`."""
import argparse
import json
import sys
from _rust_bridge import run_proven


def main() -> int:
    parser = argparse.ArgumentParser(description="Deterministic Graph Bug Triage via Proven CLI")
    parser.add_argument("query_pos", nargs="?", default=None)
    parser.add_argument("--query", default=None)
    parser.add_argument("--max-devs", type=int, default=3)
    parser.add_argument("--max-files", type=int, default=5)
    parser.add_argument("--path", "--repo", dest="path", default=".")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--schema", action="store_true")
    args = parser.parse_args()

    if args.schema:
        print(
            json.dumps(
                {
                    "tool": "triage_expert",
                    "engine": "proven",
                    "args": ["--query", "--max-devs", "--max-files", "--path", "--json"],
                },
                indent=2,
            )
        )
        return 0

    query = args.query or args.query_pos or ""
    cmd = [
        "triage",
        "--query",
        query,
        "--max-devs",
        str(args.max_devs),
        "--max-files",
        str(args.max_files),
        "--path",
        args.path,
    ]
    if args.json:
        cmd.append("--json")
    return run_proven(cmd)


if __name__ == "__main__":
    sys.exit(main())
