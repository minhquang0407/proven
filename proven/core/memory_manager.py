"""Topological Graph-Pinned Memory and Sleep Consolidation for Proven.

Pins episodic reflexion lessons directly to AST nodes/subgraphs to eliminate
context window bloat (< 80 tokens per prompt), and periodically consolidates
fine-grained lessons into high-level repo testing axioms (.proven/axioms.md).
"""

from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
from typing import Any, Dict, List, Optional


class GraphMemoryManager:
    """Manages graph-pinned memory and sleep consolidation into repo axioms."""

    @classmethod
    def get_memory_dir(cls, repo_path: str = ".", project_name: Optional[str] = None) -> Path:
        """Resolve memory directory location (.proven/memory in repo or global fallback)."""
        local_dir = Path(repo_path) / ".proven" / "memory"
        try:
            local_dir.mkdir(parents=True, exist_ok=True)
            return local_dir
        except Exception:
            fallback = Path.home() / ".proven" / (project_name or "default") / "memory"
            fallback.mkdir(parents=True, exist_ok=True)
            return fallback

    @classmethod
    def get_axioms_file(cls, repo_path: str = ".", project_name: Optional[str] = None) -> Path:
        """Resolve path to .proven/axioms.md."""
        mem_dir = cls.get_memory_dir(repo_path, project_name)
        return mem_dir / "axioms.md"

    @classmethod
    def _get_store_file(cls, repo_path: str = ".", project_name: Optional[str] = None) -> Path:
        mem_dir = cls.get_memory_dir(repo_path, project_name)
        return mem_dir / "graph_memory.json"

    @classmethod
    def _load_raw_memory(cls, repo_path: str = ".", project_name: Optional[str] = None) -> Dict[str, List[Dict[str, Any]]]:
        store_file = cls._get_store_file(repo_path, project_name)
        if not store_file.exists():
            return {}
        try:
            with store_file.open("r", encoding="utf-8") as f:
                return json.load(f)
        except Exception:
            return {}

    @classmethod
    def _save_raw_memory(
        cls,
        data: Dict[str, List[Dict[str, Any]]],
        repo_path: str = ".",
        project_name: Optional[str] = None,
    ) -> None:
        store_file = cls._get_store_file(repo_path, project_name)
        with store_file.open("w", encoding="utf-8") as f:
            json.dump(data, f, indent=2, ensure_ascii=False)

    @classmethod
    def pin_lesson(
        cls,
        target_id: str,
        lesson: str,
        failure_mode: str = "WEAK_ASSERTION",
        mutant_desc: str = "",
        repo_path: str = ".",
        project_name: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Pin a specific reflexion lesson directly to a graph target node."""
        clean_target = target_id.strip()
        data = cls._load_raw_memory(repo_path, project_name)
        node_lessons = data.setdefault(clean_target, [])

        # Avoid exact duplicate lessons on the same target
        lesson_hash = hashlib.sha256(f"{clean_target}::{lesson}".encode("utf-8")).hexdigest()[:8]
        lesson_id = f"LES_{lesson_hash}"

        for existing in node_lessons:
            if existing.get("lesson_id") == lesson_id:
                existing["timestamp"] = datetime.now(timezone.utc).isoformat()
                cls._save_raw_memory(data, repo_path, project_name)
                return existing

        entry = {
            "lesson_id": lesson_id,
            "target_id": clean_target,
            "failure_mode": failure_mode,
            "mutant_desc": mutant_desc,
            "lesson": lesson.strip(),
            "timestamp": datetime.now(timezone.utc).isoformat(),
        }
        node_lessons.append(entry)
        cls._save_raw_memory(data, repo_path, project_name)
        return entry

    @classmethod
    def get_pinned_lessons(
        cls,
        target_id: str,
        repo_path: str = ".",
        project_name: Optional[str] = None,
    ) -> List[Dict[str, Any]]:
        """Retrieve lessons pinned to a specific target node."""
        data = cls._load_raw_memory(repo_path, project_name)
        clean_target = target_id.strip()
        candidates = [clean_target, f"FUNC:{clean_target.replace('FUNC:', '')}", clean_target.replace("FUNC:", "")]
        for c in candidates:
            if c in data:
                return data[c]
        return []

    @classmethod
    def get_scoped_memory_prompt(
        cls,
        target_id: str,
        repo_path: str = ".",
        project_name: Optional[str] = None,
        include_axioms: bool = True,
    ) -> str:
        """Build an ultra-compact (< 80 tokens) memory injection block for Author Agent."""
        pinned = cls.get_pinned_lessons(target_id, repo_path, project_name)
        module_name = target_id.replace("FUNC:", "").replace("CLASS:", "").split(".")[0]

        axioms = []
        if include_axioms:
            axioms_file = cls.get_axioms_file(repo_path, project_name)
            if axioms_file.exists():
                try:
                    content = axioms_file.read_text(encoding="utf-8")
                    # Extract axioms relevant to current module or general axioms
                    for line in content.splitlines():
                        if line.startswith("- **[Axiom"):
                            axioms.append(line.strip())
                except Exception:
                    pass

        if not pinned and not axioms:
            return ""

        lines = ["### REPO MEMORY & AXIOMS"]
        if pinned:
            for p in pinned[-2:]:  # Keep at most 2 most recent lessons
                lines.append(f"- **Pinned Lesson (`{target_id}`)**: {p['lesson']}")
        if axioms:
            for ax in axioms[:2]:  # Keep at most 2 core axioms
                lines.append(ax)

        return "\n".join(lines)

    @classmethod
    def consolidate_axioms(
        cls,
        repo_path: str = ".",
        project_name: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Sleep Consolidation: Synthesize fine-grained lessons into immutable repo axioms.

        Groups all node lessons by module and distills them into .proven/axioms.md.
        """
        data = cls._load_raw_memory(repo_path, project_name)
        if not data:
            return {
                "status": "noop",
                "message": "No pinned lessons to consolidate.",
                "total_axioms": 0,
            }

        # Group by module/prefix
        by_module: Dict[str, List[str]] = {}
        for target, lessons in data.items():
            mod = target.replace("FUNC:", "").replace("CLASS:", "").split(".")[0]
            if not mod:
                mod = "general"
            for les in lessons:
                by_module.setdefault(mod, []).append(les["lesson"])

        axioms_doc = [
            "# Repo-Specific Testing Axioms",
            "",
            "> Auto-generated by Proven Sleep Consolidation. These axioms are injected into Author Agent prompts.",
            "",
        ]

        total_axioms = 0
        for mod, lesson_list in by_module.items():
            axioms_doc.append(f"### Module: `{mod}`")
            # Deduplicate and consolidate
            unique_lessons = list(dict.fromkeys(lesson_list))
            for i, l in enumerate(unique_lessons[:5], start=1):
                total_axioms += 1
                axioms_doc.append(f"- **[Axiom {mod}.{i}]**: {l}")
            axioms_doc.append("")

        axioms_file = cls.get_axioms_file(repo_path, project_name)
        axioms_file.write_text("\n".join(axioms_doc), encoding="utf-8")

        return {
            "status": "success",
            "message": f"Consolidated {len(data)} nodes into {total_axioms} axioms in {axioms_file.name}.",
            "total_axioms": total_axioms,
            "axioms_file": str(axioms_file),
            "modules": list(by_module.keys()),
        }
