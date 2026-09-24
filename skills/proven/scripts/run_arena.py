#!/usr/bin/env python3
"""Standalone script for Coding Agents: Run an Adversarial Arena round for a target function as JSON."""

import argparse
import json
import os
import sys

_script_dir = os.path.dirname(os.path.abspath(__file__))
_skill_root = os.path.abspath(os.path.join(_script_dir, ".."))
_repo_root = os.path.abspath(os.path.join(_script_dir, "..", ".."))
for p in (_skill_root, _repo_root):
    if p not in sys.path:
        sys.path.insert(0, p)

from proven.core.swarm.arena import TriAgentArena

SCHEMA = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "title": "RunArenaOutput",
    "description": "Output schema of run_arena.py for multi-agent adversarial evaluation",
    "type": "object",
    "properties": {
        "round_no": {"type": "integer"},
        "target_id": {"type": "string"},
        "test_target": {"type": "string"},
        "proof_status": {"type": "string", "enum": ["pass", "fail"]},
        "proof_grade": {"type": "string", "enum": ["TITANIUM", "SILVER", "FAILED"]},
        "is_titanium": {"type": "boolean"},
        "vault_status": {"type": "string"},
        "mutants_killed": {"type": "integer"},
        "mutants_survived": {"type": "integer"},
        "causal_reflexion": {"type": ["object", "null"]},
        "reflexion_prompt": {"type": ["string", "null"]},
        "message": {"type": "string"}
    },
    "required": ["round_no", "target_id", "test_target", "proof_grade", "is_titanium", "message"]
}


def main():
    parser = argparse.ArgumentParser(description="Referee an Adversarial Arena round between Author and Adversary")
    parser.add_argument("--target", default=None, help="Target ID (e.g. FUNC:my_func)")
    parser.add_argument("--test", default=None, help="Path to test file (e.g. tests/test_my_func.py)")
    parser.add_argument("--round", type=int, default=1, help="Round number (default: 1)")
    parser.add_argument("--project", default=None, help="Project name")
    parser.add_argument("--path", default=".", help="Repository path")
    parser.add_argument("--pytest-args", default=None, help="Extra pytest arguments")
    parser.add_argument("--schema", action="store_true", help="Print JSON Schema for output and exit")
    args = parser.parse_args()

    if args.schema:
        print(json.dumps(SCHEMA, indent=2))
        sys.exit(0)

    if not args.target or not args.test:
        parser.error("--target and --test are required unless --schema is specified")

    try:
        arena = TriAgentArena(repo_path=args.path, project_name=args.project)
        result = arena.referee_round(
            target_id=args.target,
            test_target=args.test,
            round_no=args.round,
            pytest_args=args.pytest_args,
        )
        print(json.dumps(result.to_dict(), indent=2, ensure_ascii=False))
        sys.exit(0 if result.is_titanium else 1)
    except Exception as e:
        err = {"status": "error", "message": str(e)}
        print(json.dumps(err, indent=2, ensure_ascii=False))
        sys.exit(2)


if __name__ == "__main__":
    main()
