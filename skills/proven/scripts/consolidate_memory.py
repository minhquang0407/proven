#!/usr/bin/env python3
"""Standalone script for Coding Agents: Run Sleep Consolidation on graph-pinned memory as JSON."""

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

from proven.core.memory_manager import GraphMemoryManager

SCHEMA = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "title": "ConsolidateMemoryOutput",
    "description": "Output schema of consolidate_memory.py for sleep consolidation",
    "type": "object",
    "properties": {
        "status": {"type": "string"},
        "message": {"type": "string"},
        "total_axioms": {"type": "integer"},
        "axioms_file": {"type": "string"},
        "modules": {"type": "array", "items": {"type": "string"}}
    },
    "required": ["status", "message", "total_axioms"]
}


def main():
    parser = argparse.ArgumentParser(description="Consolidate pinned graph lessons into .proven/axioms.md")
    parser.add_argument("--project", default=None, help="Project name")
    parser.add_argument("--path", default=".", help="Repository path")
    parser.add_argument("--schema", action="store_true", help="Print JSON Schema for output and exit")
    args = parser.parse_args()

    if args.schema:
        print(json.dumps(SCHEMA, indent=2))
        sys.exit(0)

    try:
        res = GraphMemoryManager.consolidate_axioms(repo_path=args.path, project_name=args.project)
        print(json.dumps(res, indent=2, ensure_ascii=False))
        sys.exit(0)
    except Exception as e:
        err = {"status": "error", "message": str(e)}
        print(json.dumps(err, indent=2, ensure_ascii=False))
        sys.exit(1)


if __name__ == "__main__":
    main()
