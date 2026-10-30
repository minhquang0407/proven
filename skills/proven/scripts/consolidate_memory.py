#!/usr/bin/env python3
"""Thin CLI wrapper -> `proven sleep`."""
import argparse
import json
import sys
from _rust_bridge import run_proven


def main() -> int:
    parser = argparse.ArgumentParser(description="Run CLS Neocortex Sleep Consolidation via Proven CLI")
    parser.add_argument("--path", "--repo", dest="path", default=".")
    parser.add_argument("--threshold", type=int, default=10)
    parser.add_argument("--force", action="store_true")
    parser.add_argument("--meta", action="store_true")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--schema", action="store_true")
    args = parser.parse_args()

    if args.schema:
        print(
            json.dumps(
                {
                    "tool": "consolidate_memory",
                    "engine": "proven",
                    "args": ["--path", "--threshold", "--force", "--meta", "--json"],
                },
                indent=2,
            )
        )
        return 0

    cmd = ["sleep", "--path", args.path, "--threshold", str(args.threshold)]
    if args.force:
        cmd.append("--force")
    if args.meta:
        cmd.append("--meta")
    if args.json:
        cmd.append("--json")
    return run_proven(cmd)


if __name__ == "__main__":
    sys.exit(main())
