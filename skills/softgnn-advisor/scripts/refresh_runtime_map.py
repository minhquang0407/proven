#!/usr/bin/env python3
"""Standalone script for Coding Agents: Refresh runtime test coverage graph across repository."""

import argparse
import json
import sys
import os

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from softgnn_advisor.core.agent_service import AgentService


def main():
    parser = argparse.ArgumentParser(description="Refresh runtime coverage map across tests")
    parser.add_argument("--pytest-args", default="tests", help="Pytest targets to map")
    parser.add_argument("--project", default=None, help="Project name")
    parser.add_argument("--path", default=".", help="Repository path")
    args = parser.parse_args()

    try:
        svc = AgentService(project=args.project, repo_path=args.path)
        result = svc.refresh_runtime(pytest_args=args.pytest_args)
        print(json.dumps(result, indent=2, ensure_ascii=False))
    except Exception as e:
        err = {"status": "error", "message": str(e)}
        print(json.dumps(err, indent=2, ensure_ascii=False))
        sys.exit(1)


if __name__ == "__main__":
    main()
