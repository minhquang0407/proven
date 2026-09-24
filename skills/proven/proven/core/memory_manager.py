"""Topological Graph-Pinned Hierarchical Memory and Sleep Consolidation for Proven.

Implements Complementary Learning Systems (CLS Theory):
- Fast Learning (Hippocampal Episodic): State transitions (Trap + Solution) are crystallized
  into local LESSON nodes upon successful mutant defeat: (FUNC) --[HAS_LESSON]--> (LESSON).
- Slow Learning (Neocortical Sleep Consolidation): Consolidates episodic lessons into parent
  AXIOM nodes: (AXIOM) --[GENERALIZES]--> (LESSON), (FUNC) --[CONSTRAINED_BY]--> (AXIOM).
- Token Shielding: Context retrieval extracts 1-hop ego-memory, querying concise AXIOM rules
  (< 80 tokens) and shielding child lessons from prompt bloat while preserving full graph lineage.
- Vulnerability Flagging: Exhausted rounds create VULNERABILITY nodes: (FUNC) --[HAS_BLINDSPOT]--> (VULNERABILITY).
"""

from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
from typing import Any, Dict, List, Optional


class GraphMemoryManager:
    """Manages hierarchical knowledge graph memory, CLS consolidation, and token shielding."""

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
    def _load_raw_memory(
        cls, repo_path: str = ".", project_name: Optional[str] = None
    ) -> Dict[str, Any]:
        """Load memory store, automatically upgrading legacy schema to hierarchical graph schema."""
        store_file = cls._get_store_file(repo_path, project_name)
        if not store_file.exists():
            return {
                "version": "2.0",
                "target_lessons": {},
                "nodes": {},
                "edges": [],
            }

        try:
            with store_file.open("r", encoding="utf-8") as f:
                raw = json.load(f)
        except Exception:
            return {
                "version": "2.0",
                "target_lessons": {},
                "nodes": {},
                "edges": [],
            }

        # Check if legacy flat dictionary format
        if "version" not in raw and isinstance(raw, dict):
            migrated: Dict[str, Any] = {
                "version": "2.0",
                "target_lessons": raw,
                "nodes": {},
                "edges": [],
            }
            # Reconstruct nodes and edges from legacy records
            for tgt, lessons in raw.items():
                for item in lessons:
                    lid = item.get("lesson_id", f"LES_{hashlib.sha256(str(item).encode()).hexdigest()[:8]}")
                    node_id = f"LESSON:{lid}"
                    migrated["nodes"][node_id] = {
                        "id": node_id,
                        "type": "LESSON",
                        "target_id": tgt,
                        "lesson_id": lid,
                        "lesson": item.get("lesson", ""),
                        "trap": item.get("mutant_desc", ""),
                        "solution": item.get("lesson", ""),
                        "rule": item.get("lesson", ""),
                        "status": "unconsolidated",
                        "timestamp": item.get("timestamp", datetime.now(timezone.utc).isoformat()),
                    }
                    migrated["edges"].append({
                        "source": tgt,
                        "target": node_id,
                        "type": "HAS_LESSON",
                    })
            return migrated

        raw.setdefault("target_lessons", {})
        raw.setdefault("nodes", {})
        raw.setdefault("edges", [])
        return raw

    @classmethod
    def _save_raw_memory(
        cls,
        data: Dict[str, Any],
        repo_path: str = ".",
        project_name: Optional[str] = None,
    ) -> None:
        store_file = cls._get_store_file(repo_path, project_name)
        with store_file.open("w", encoding="utf-8") as f:
            json.dump(data, f, indent=2, ensure_ascii=False)

    @classmethod
    def crystallize_lesson(
        cls,
        target_id: str,
        trap: str,
        solution: str,
        rule: str = "",
        failure_mode: str = "WEAK_ASSERTION",
        mutant_desc: str = "",
        repo_path: str = ".",
        project_name: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Crystallize a state-transition lesson (Trap + Solution) into a LESSON node.

        Called ONLY when Author Agent successfully repairs tests to kill the mutant.
        """
        clean_target = target_id.strip()
        data = cls._load_raw_memory(repo_path, project_name)

        final_rule = rule.strip() or solution.strip()
        final_trap = trap.strip() or mutant_desc.strip()
        hash_seed = f"{clean_target}::{final_trap}::{solution}"
        hash_id = hashlib.sha256(hash_seed.encode("utf-8")).hexdigest()[:8]
        lesson_id = f"LES_{hash_id}"
        node_id = f"LESSON:{lesson_id}"

        now_iso = datetime.now(timezone.utc).isoformat()

        lesson_node = {
            "id": node_id,
            "type": "LESSON",
            "lesson_id": lesson_id,
            "target_id": clean_target,
            "trap": final_trap,
            "solution": solution.strip(),
            "rule": final_rule,
            "lesson": final_rule,
            "failure_mode": failure_mode,
            "mutant_desc": final_trap,
            "status": "unconsolidated",
            "timestamp": now_iso,
        }

        data["nodes"][node_id] = lesson_node

        # Ensure edge exists
        edge_exists = any(
            e.get("source") == clean_target
            and e.get("target") == node_id
            and e.get("type") == "HAS_LESSON"
            for e in data["edges"]
        )
        if not edge_exists:
            data["edges"].append({
                "source": clean_target,
                "target": node_id,
                "type": "HAS_LESSON",
            })

        # Backward compatibility target_lessons
        node_lessons = data["target_lessons"].setdefault(clean_target, [])
        for existing in node_lessons:
            if existing.get("lesson_id") == lesson_id:
                existing["timestamp"] = now_iso
                existing["rule"] = final_rule
                existing["lesson"] = final_rule
                cls._save_raw_memory(data, repo_path, project_name)
                return lesson_node

        node_lessons.append(lesson_node)
        cls._save_raw_memory(data, repo_path, project_name)
        return lesson_node

    @classmethod
    def record_vulnerability(
        cls,
        target_id: str,
        surviving_mutant_desc: str,
        round_exhausted: int = 3,
        message: str = "",
        repo_path: str = ".",
        project_name: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Record an unresolved blindspot as a VULNERABILITY node when max rounds are exhausted."""
        clean_target = target_id.strip()
        data = cls._load_raw_memory(repo_path, project_name)

        hash_seed = f"{clean_target}::VUL::{surviving_mutant_desc}"
        hash_id = hashlib.sha256(hash_seed.encode("utf-8")).hexdigest()[:8]
        vul_id = f"VUL_{hash_id}"
        node_id = f"VULNERABILITY:{vul_id}"

        vul_node = {
            "id": node_id,
            "type": "VULNERABILITY",
            "vul_id": vul_id,
            "target_id": clean_target,
            "surviving_mutant": surviving_mutant_desc,
            "round_exhausted": round_exhausted,
            "message": message or f"Survived {round_exhausted} adversarial rounds without proof of kill.",
            "timestamp": datetime.now(timezone.utc).isoformat(),
        }

        data["nodes"][node_id] = vul_node

        edge_exists = any(
            e.get("source") == clean_target
            and e.get("target") == node_id
            and e.get("type") == "HAS_BLINDSPOT"
            for e in data["edges"]
        )
        if not edge_exists:
            data["edges"].append({
                "source": clean_target,
                "target": node_id,
                "type": "HAS_BLINDSPOT",
            })

        cls._save_raw_memory(data, repo_path, project_name)
        return vul_node

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
        """Backward-compatible helper to pin a lesson."""
        return cls.crystallize_lesson(
            target_id=target_id,
            trap=mutant_desc,
            solution=lesson,
            rule=lesson,
            failure_mode=failure_mode,
            mutant_desc=mutant_desc,
            repo_path=repo_path,
            project_name=project_name,
        )

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
        target_lessons = data.get("target_lessons", {})

        candidates = [clean_target, f"FUNC:{clean_target.replace('FUNC:', '')}", clean_target.replace("FUNC:", "")]
        for c in candidates:
            if c in target_lessons:
                return target_lessons[c]
        return []

    @classmethod
    def get_ego_memory_subgraph(
        cls,
        target_id: str,
        hops: int = 1,
        repo_path: str = ".",
        project_name: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Extract the 1-hop ego-memory subgraph surrounding target_id.

        Implements Token Shielding:
        - If target has AXIOM nodes (via CONSTRAINED_BY), returns AXIOM rules.
        - Only unconsolidated LESSON nodes not yet abstracted by an Axiom are returned as active lessons.
        - Provides full provenance (child lessons under each axiom).
        """
        data = cls._load_raw_memory(repo_path, project_name)
        clean_target = target_id.strip()
        norm_target = clean_target if clean_target.startswith("FUNC:") else f"FUNC:{clean_target}"

        nodes = data.get("nodes", {})
        edges = data.get("edges", [])

        # Find connected node IDs
        connected_axiom_ids = set()
        connected_lesson_ids = set()
        connected_vul_ids = set()

        # Check direct edges and module-level axioms
        mod_prefix = clean_target.replace("FUNC:", "").replace("CLASS:", "").split(".")[0]

        for e in edges:
            src, tgt, etype = e.get("source"), e.get("target"), e.get("type")
            if src in (clean_target, norm_target):
                if etype == "CONSTRAINED_BY" and tgt in nodes:
                    connected_axiom_ids.add(tgt)
                elif etype == "HAS_LESSON" and tgt in nodes:
                    connected_lesson_ids.add(tgt)
                elif etype == "HAS_BLINDSPOT" and tgt in nodes:
                    connected_vul_ids.add(tgt)

        # Also find module-level axioms
        for nid, n in nodes.items():
            if n.get("type") == "AXIOM" and n.get("module") == mod_prefix:
                connected_axiom_ids.add(nid)

        active_axioms = [nodes[aid] for aid in connected_axiom_ids if aid in nodes]
        active_lessons = [nodes[lid] for lid in connected_lesson_ids if lid in nodes]
        vulnerabilities = [nodes[vid] for vid in connected_vul_ids if vid in nodes]

        # Identify which lessons are already subsumed by active axioms
        subsumed_lesson_ids = set()
        for ax in active_axioms:
            for child in ax.get("generalized_lessons", []):
                subsumed_lesson_ids.add(child)

        # Unconsolidated lessons not shielded
        shielded_active_lessons = [
            les for les in active_lessons
            if les.get("status") == "unconsolidated" or les["id"] not in subsumed_lesson_ids
        ]

        return {
            "target_id": clean_target,
            "axioms": active_axioms,
            "unconsolidated_lessons": shielded_active_lessons,
            "subsumed_lessons_count": len(subsumed_lesson_ids),
            "vulnerabilities": vulnerabilities,
        }

    @classmethod
    def get_scoped_memory_prompt(
        cls,
        target_id: str,
        repo_path: str = ".",
        project_name: Optional[str] = None,
        include_axioms: bool = True,
    ) -> str:
        """Build an ultra-compact (< 80 tokens) memory injection block for Author Agent using Ego-Graph."""
        ego = cls.get_ego_memory_subgraph(target_id=target_id, repo_path=repo_path, project_name=project_name)

        axioms = ego["axioms"]
        lessons = ego["unconsolidated_lessons"]
        vulnerabilities = ego["vulnerabilities"]

        # Also load from axioms.md if no graph axioms found yet
        fallback_axioms = []
        if include_axioms and not axioms:
            axioms_file = cls.get_axioms_file(repo_path, project_name)
            if axioms_file.exists():
                try:
                    content = axioms_file.read_text(encoding="utf-8")
                    mod = target_id.replace("FUNC:", "").replace("CLASS:", "").split(".")[0]
                    for line in content.splitlines():
                        if line.startswith(f"- **[Axiom {mod}.") or line.startswith("- **[Axiom"):
                            fallback_axioms.append(line.strip())
                except Exception:
                    pass

        if not axioms and not fallback_axioms and not lessons and not vulnerabilities:
            return ""

        lines = ["### REPO MEMORY & AXIOMS"]

        # 1. Primary Axioms (Token Shield: at most 2 concise rules)
        if axioms:
            for ax in axioms[:2]:
                lines.append(f"- **[Axiom {ax.get('module', 'core')}]**: {ax.get('rule', '')}")
        elif fallback_axioms:
            for f_ax in fallback_axioms[:2]:
                lines.append(f_ax)

        # 2. Unconsolidated Episodic Lessons (at most 2 most recent)
        if lessons:
            for p in lessons[-2:]:
                lines.append(f"- **Pinned Lesson (`{target_id}`)**: {p.get('rule') or p.get('lesson')}")

        # 3. Warning on Known Vulnerabilities
        if vulnerabilities:
            latest_vul = vulnerabilities[-1]
            lines.append(f"- **⚠️ Warning: Known Blindspot**: {latest_vul.get('surviving_mutant')}")

        return "\n".join(lines)

    @classmethod
    def consolidate_axioms(
        cls,
        repo_path: str = ".",
        project_name: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Sleep Consolidation (Neocortical Abstraction):

        1. Groups unconsolidated episodic lessons by module/target.
        2. Promotes them into parent AXIOM nodes: (AXIOM) --[GENERALIZES]--> (LESSON).
        3. Constrains targets: (FUNC) --[CONSTRAINED_BY]--> (AXIOM).
        4. Marks constituent lessons as status='consolidated'.
        5. Writes .proven/axioms.md and persists hierarchical knowledge graph.
        """
        data = cls._load_raw_memory(repo_path, project_name)
        nodes = data.setdefault("nodes", {})
        edges = data.setdefault("edges", [])

        # Collect all LESSON nodes
        lesson_nodes = [n for n in nodes.values() if n.get("type") == "LESSON"]
        if not lesson_nodes:
            return {
                "status": "noop",
                "message": "No pinned lessons to consolidate.",
                "total_axioms": 0,
            }

        # Group by module
        by_module: Dict[str, List[Dict[str, Any]]] = {}
        for lnode in lesson_nodes:
            tgt = lnode.get("target_id", "")
            mod = tgt.replace("FUNC:", "").replace("CLASS:", "").split(".")[0] or "general"
            by_module.setdefault(mod, []).append(lnode)

        now_iso = datetime.now(timezone.utc).isoformat()
        total_axioms = 0
        new_axiom_nodes = {}
        new_edges = []

        axioms_doc = [
            "# Repo-Specific Testing Axioms",
            "",
            "> Auto-generated by Proven Sleep Consolidation. These axioms are injected into Author Agent prompts.",
            "",
        ]

        for mod, l_list in by_module.items():
            axioms_doc.append(f"### Module: `{mod}`")

            # Cluster unique rules
            rule_map: Dict[str, List[str]] = {}
            for lnode in l_list:
                rule_text = lnode.get("rule") or lnode.get("lesson") or ""
                if rule_text:
                    rule_map.setdefault(rule_text, []).append(lnode["id"])

            for idx, (rule_text, child_lesson_ids) in enumerate(list(rule_map.items())[:5], start=1):
                total_axioms += 1
                axiom_id = f"AXIOM:AX_{mod}_{idx}"
                label = f"Axiom {mod}.{idx}"
                axioms_doc.append(f"- **[{label}]**: {rule_text}")

                axiom_node = {
                    "id": axiom_id,
                    "type": "AXIOM",
                    "axiom_id": f"AX_{mod}_{idx}",
                    "module": mod,
                    "label": label,
                    "rule": rule_text,
                    "generalized_lessons": child_lesson_ids,
                    "timestamp": now_iso,
                }
                new_axiom_nodes[axiom_id] = axiom_node

                # Hierarchy edges: (AXIOM) -[GENERALIZES]-> (LESSON)
                for clid in child_lesson_ids:
                    new_edges.append({
                        "source": axiom_id,
                        "target": clid,
                        "type": "GENERALIZES",
                    })
                    # Mark lesson consolidated
                    if clid in nodes:
                        nodes[clid]["status"] = "consolidated"

                # Constraint edges: (FUNC) -[CONSTRAINED_BY]-> (AXIOM)
                for clid in child_lesson_ids:
                    cl_target = nodes.get(clid, {}).get("target_id")
                    if cl_target:
                        new_edges.append({
                            "source": cl_target,
                            "target": axiom_id,
                            "type": "CONSTRAINED_BY",
                        })

            axioms_doc.append("")

        # Merge new nodes and edges avoiding duplicates
        for aid, anode in new_axiom_nodes.items():
            nodes[aid] = anode

        for ne in new_edges:
            if not any(
                e.get("source") == ne["source"]
                and e.get("target") == ne["target"]
                and e.get("type") == ne["type"]
                for e in edges
            ):
                edges.append(ne)

        cls._save_raw_memory(data, repo_path, project_name)

        # Write markdown summary file
        axioms_file = cls.get_axioms_file(repo_path, project_name)
        axioms_file.write_text("\n".join(axioms_doc), encoding="utf-8")

        return {
            "status": "success",
            "message": f"Consolidated {len(lesson_nodes)} lessons into {total_axioms} axioms in {axioms_file.name}.",
            "total_axioms": total_axioms,
            "axioms_file": str(axioms_file),
            "modules": list(by_module.keys()),
        }

    @classmethod
    def export_memory_graph(
        cls,
        repo_path: str = ".",
        project_name: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Export memory graph nodes and edges for Cytoscape visualization."""
        data = cls._load_raw_memory(repo_path, project_name)
        return {
            "nodes": list(data.get("nodes", {}).values()),
            "edges": list(data.get("edges", [])),
        }
