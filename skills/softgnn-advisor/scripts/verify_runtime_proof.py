#!/usr/bin/env python3
"""Standalone script for Coding Agents: Verify runtime execution proof against target function.

Supports:
- Track 1 (Native Python): via --test tests/test_*.py (runs pytest + coverage.py)
- Track 2 (Universal Engine): via --lcov <path> or --test-cmd "<custom test command>"
"""

import argparse
import json
import sys
import os

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from softgnn_advisor.core.agent_service import AgentService


def main():
    parser = argparse.ArgumentParser(description="Verify runtime proof of a written test for a target function")
    parser.add_argument("--target", required=True, help="Target ID (e.g. FUNC:my_func)")
    parser.add_argument("--test", default=None, help="Test target file or function (e.g. tests/test_foo.py)")
    parser.add_argument("--pytest-args", default=None, help="Additional pytest arguments (Track 1)")
    parser.add_argument("--lcov", default=None, help="Path to lcov.info or coverage.out (Track 2 Universal)")
    parser.add_argument("--test-cmd", default=None, help="Custom test execution command (e.g. npm test -- --coverage)")
    parser.add_argument("--lang", default=None, help="Language override (python, typescript, go, etc.)")
    parser.add_argument("--mutation-check", action="store_true", help="Run MicroMutationGate to prove assertion strength (Titanium Proof)")
    parser.add_argument("--max-mutants", type=int, default=2, help="Maximum AST mutants to test (default 2)")
    parser.add_argument("--project", default=None, help="Project name")
    parser.add_argument("--path", default=".", help="Repository path")
    args = parser.parse_args()

    try:
        svc = AgentService(project=args.project, repo_path=args.path, language=args.lang)
        result = svc.verify_proof(
            target_id=args.target,
            test_target=args.test,
            pytest_args=args.pytest_args,
            lcov_path=args.lcov,
            test_cmd=args.test_cmd,
            lang=args.lang,
            mutation_check=args.mutation_check,
            max_mutants=args.max_mutants,
        )
        print(json.dumps(result, indent=2, ensure_ascii=False))
        if result.get("proof_status") != "pass":
            sys.exit(2)
    except Exception as e:
        err = {"status": "error", "message": str(e), "proof_status": "fail"}
        print(json.dumps(err, indent=2, ensure_ascii=False))
        sys.exit(1)


if __name__ == "__main__":
    main()
