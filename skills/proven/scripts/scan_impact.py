#!/usr/bin/env python3
"""Thin CLI wrapper -> `proven scan`."""
import argparse
import json
import sys
from _rust_bridge import run_proven


def main() -> int:
    parser = argparse.ArgumentParser(description="Scan repository diff via Proven CLI")
    parser.add_argument("--path", "--repo", dest="path", default=".")
    parser.add_argument("--base", default="main")
    parser.add_argument("--head", default="HEAD")
    parser.add_argument("--fan-out", action="store_true")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--schema", action="store_true")
    args = parser.parse_args()

    if args.schema:
        print(
            json.dumps(
                {
                    "tool": "scan_impact",
                    "engine": "proven",
                    "args": ["--path", "--base", "--head", "--fan-out", "--json"],
                },
                indent=2,
            )
        )
        return 0

    cmd = ["scan", "--path", args.path, "--base", args.base, "--head", args.head]
    if args.fan_out:
        cmd.append("--fan-out")
    if args.json:
        cmd.append("--json")
    return run_proven(cmd)


if __name__ == "__main__":
    sys.exit(main())
