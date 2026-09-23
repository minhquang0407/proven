#!/usr/bin/env python3
"""Standalone script for Coding Agents: Extract surgical AST context for a target function as JSON."""

import argparse
import json
import sys
import os

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from softgnn_advisor.core.agent_service import AgentService


def main():
    parser = argparse.ArgumentParser(description="Extract surgical AST context for a target function")
    parser.add_argument("--target", required=True, help="Target ID (e.g. FUNC:my_func)")
    parser.add_argument("--file", default=None, help="Source file path if known")
    parser.add_argument("--project", default=None, help="Project name")
    parser.add_argument("--path", default=".", help="Repository path")
    args = parser.parse_args()

    try:
        svc = AgentService(project=args.project, repo_path=args.path)
        result = svc.get_context(target_id=args.target, source_file=args.file)
        print(json.dumps(result, indent=2, ensure_ascii=False))
    except Exception as e:
        err = {"status": "error", "message": str(e)}
        print(json.dumps(err, indent=2, ensure_ascii=False))
        sys.exit(1)


if __name__ == "__main__":
    main()
