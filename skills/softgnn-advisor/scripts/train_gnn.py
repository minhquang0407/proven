#!/usr/bin/env python3
"""Standalone script for Coding Agents: Trigger offline HGT Graph AI training for the codebase."""

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
    parser = argparse.ArgumentParser(description="Train HGT Graph AI model for codebase link prediction")
    parser.add_argument("--project", default=None, help="Project name")
    parser.add_argument("--path", default=".", help="Repository path")
    args = parser.parse_args()

    try:
        svc = AgentService(project=args.project, repo_path=args.path)
        result = svc.train_gnn()
        print(json.dumps(result, indent=2, ensure_ascii=False))
    except Exception as e:
        err = {"status": "error", "message": str(e)}
        print(json.dumps(err, indent=2, ensure_ascii=False))
        sys.exit(1)


if __name__ == "__main__":
    main()
