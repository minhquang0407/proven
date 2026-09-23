#!/usr/bin/env python3
"""Standalone script for Coding Agents: Scan Git diff/PR impact and output missing runtime coverage gaps as JSON."""

import argparse
import json
import sys
import os

# Ensure package root is in sys.path
repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from softgnn_advisor.core.agent_service import AgentService


def main():
    parser = argparse.ArgumentParser(description="Scan PR impact and find uncovered functions")
    parser.add_argument("--project", default=None, help="Project name (defaults to repository folder name)")
    parser.add_argument("--path", default=".", help="Repository path")
    parser.add_argument("--base", default="main", help="Base git ref")
    parser.add_argument("--head", default="HEAD", help="Head git ref")
    parser.add_argument("--source", default="auto", choices=["auto", "git", "filesystem", "full-scan"], help="Diff source")
    parser.add_argument("--lang", default=None, help="Language override (python, typescript, go, etc.)")
    parser.add_argument("--fan-out", action="store_true", help="Generate parallel Sub-Agent worker tasks")
    args = parser.parse_args()

    try:
        svc = AgentService(project=args.project, repo_path=args.path, language=args.lang)
        if args.fan_out:
            result = svc.generate_fanout_tasks(base_ref=args.base, head_ref=args.head, lang=args.lang)
        else:
            result = svc.scan(base=args.base, head=args.head, change_source=args.source, lang=args.lang)
        print(json.dumps(result, indent=2, ensure_ascii=False))
    except Exception as e:
        err = {"status": "error", "message": str(e)}
        print(json.dumps(err, indent=2, ensure_ascii=False))
        sys.exit(1)


if __name__ == "__main__":
    main()
