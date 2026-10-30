#!/usr/bin/env python3
"""Thin CLI wrapper -> `proven explain-attention`."""
import argparse
import json
import sys
from _rust_bridge import run_proven


def main() -> int:
    parser = argparse.ArgumentParser(description="Explain GraphRAG topological attention via Proven CLI")
    parser.add_argument("--target", default=None)
    parser.add_argument("--hops", type=int, default=3)
    parser.add_argument("--top-k", type=int, default=5)
    parser.add_argument("--mode", default="topological")
    parser.add_argument("--path", "--repo", dest="path", default=".")
    parser.add_argument("--visualize", action="store_true")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--schema", action="store_true")
    args = parser.parse_args()

    if args.schema:
        print(
            json.dumps(
                {
                    "tool": "explain_attention",
                    "engine": "proven",
                    "args": ["--target", "--hops", "--top-k", "--mode", "--path", "--visualize", "--json"],
                },
                indent=2,
            )
        )
        return 0

    if not args.target:
        parser.error("--target is required unless --schema is specified")

    cmd = [
        "explain-attention",
        "--target",
        args.target,
        "--hops",
        str(args.hops),
        "--top-k",
        str(args.top_k),
        "--mode",
        args.mode,
        "--path",
        args.path,
    ]
    if args.visualize:
        cmd.append("--visualize")
    if args.json:
        cmd.append("--json")
    return run_proven(cmd)


if __name__ == "__main__":
    sys.exit(main())
