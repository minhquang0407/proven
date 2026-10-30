#!/usr/bin/env python3
"""Thin CLI wrapper -> `proven impact`."""
import argparse
import json
import sys
from _rust_bridge import run_proven


def main() -> int:
    parser = argparse.ArgumentParser(description="Predict change impact blast radius via Proven CLI")
    parser.add_argument("--target", default=None)
    parser.add_argument("--mode", default="deterministic")
    parser.add_argument("--path", "--repo", dest="path", default=".")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--schema", action="store_true")
    args = parser.parse_args()

    if args.schema:
        print(
            json.dumps(
                {
                    "tool": "query_impact",
                    "engine": "proven",
                    "args": ["--target", "--mode", "--path", "--json"],
                },
                indent=2,
            )
        )
        return 0

    if not args.target:
        parser.error("--target is required unless --schema is specified")

    cmd = ["impact", "--target", args.target, "--mode", args.mode, "--path", args.path]
    if args.json:
        cmd.append("--json")
    return run_proven(cmd)


if __name__ == "__main__":
    sys.exit(main())
