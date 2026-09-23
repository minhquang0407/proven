#!/usr/bin/env python3
"""Standalone script for Coding Agents: Query direct dependents and latent HGT blast radius for a symbol."""

import argparse
import json
import os
import sys

# Ensure package root is in sys.path
repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from softgnn_advisor.core.agent_service import AgentService


def main():
    parser = argparse.ArgumentParser(description="Query blast radius and latent GNN impact for a target symbol")
    parser.add_argument("--target", required=True, help="Target symbol ID (e.g. FUNC:module.function)")
    parser.add_argument("--mode", default="hybrid", choices=["hybrid", "graph", "gnn"], help="Impact analysis mode")
    parser.add_argument("--threshold", type=float, default=0.1, help="Score threshold")
    parser.add_argument("--project", default=None, help="Project name")
    parser.add_argument("--path", default=".", help="Repository path")
    args = parser.parse_args()

    try:
        svc = AgentService(project=args.project, repo_path=args.path)
        result = svc.predict_impact(target_symbol=args.target, mode=args.mode, threshold=args.threshold)
        print(json.dumps(result, indent=2, ensure_ascii=False))
    except Exception as e:
        err = {"status": "error", "message": str(e)}
        print(json.dumps(err, indent=2, ensure_ascii=False))
        sys.exit(1)


if __name__ == "__main__":
    main()
