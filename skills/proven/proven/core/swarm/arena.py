"""Tri-Agent Adversarial Arena for Proven.

Orchestrates the co-evolutionary debate between:
- 🔵 Author Agent (Defensive Tester)
- 🔴 Adversary Agent (Semantic Bug Injector)
- 🧠 Critic Agent (Ground Truth Referee & Reflexion Synthesizer)
"""

from dataclasses import asdict, dataclass
import os
from pathlib import Path
import time
from typing import Any, Dict, List, Optional

from proven.core.agent_service import AgentService
from proven.core.memory_manager import GraphMemoryManager
from proven.core.mutant_vault import MutantVault
from proven.core.reflexion_engine import CausalReflexionEngine


@dataclass
class ArenaRoundResult:
    round_no: int
    target_id: str
    test_target: str
    proof_status: str
    proof_grade: str
    is_titanium: bool
    vault_status: str
    mutants_killed: int
    mutants_survived: int
    causal_reflexion: Optional[Dict[str, Any]]
    reflexion_prompt: Optional[str]
    message: str

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


class TriAgentArena:
    """Manages multi-agent adversarial evaluation and referee loop."""

    def __init__(
        self,
        repo_path: str = ".",
        project_name: Optional[str] = None,
        language: Optional[str] = None,
    ):
        self.repo_path = os.path.abspath(repo_path)
        self.svc = AgentService(project=project_name, repo_path=self.repo_path, language=language)
        self.project = self.svc.project

    def get_author_brief(self, target_id: str, source_file: Optional[str] = None) -> Dict[str, Any]:
        """Generate tailored instruction brief and scoped memory for Author Agent."""
        context = self.svc.get_context(target_id=target_id, source_file=source_file)
        memory_prompt = GraphMemoryManager.get_scoped_memory_prompt(
            target_id=target_id, repo_path=self.repo_path, project_name=self.project
        )
        return {
            "status": "success",
            "persona": "Author Agent (Defensive Tester)",
            "role": "Synthesize comprehensive, runtime-proven tests with strict behavioral assertions.",
            "target_id": target_id,
            "signature": context.get("signature"),
            "source_file": context.get("source_file"),
            "source_code": context.get("source_code"),
            "suggested_test_file": context.get("suggested_test_file"),
            "scoped_memory": memory_prompt,
            "directive": (
                "Write defensive tests covering happy paths, error boundaries, and state changes. "
                "Always assert exact return values and side-effects to resist adversarial mutation."
            ),
        }

    def get_adversary_brief(
        self,
        target_id: str,
        test_target: str,
        source_file: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Generate tailored instruction brief for Adversary Agent to inspect test gaps."""
        context = self.svc.get_context(target_id=target_id, source_file=source_file)
        test_content = ""
        abs_test = Path(self.repo_path) / test_target.replace("/", os.sep)
        if abs_test.exists():
            test_content = abs_test.read_text(encoding="utf-8", errors="replace")

        return {
            "status": "success",
            "persona": "Adversary Agent (Semantic Bugmaker)",
            "role": "Identify specification blindspots and weak assertions in Author's test suite.",
            "target_id": target_id,
            "source_file": context.get("source_file"),
            "source_code": context.get("source_code"),
            "author_test_target": test_target,
            "author_test_code": test_content[:4000],
            "directive": (
                "Analyze the Author's test assertions. What return attributes or side-effects are NOT verified? "
                "Formulate a subtle semantic mutation (e.g., invert a condition, alter an edge case return) "
                "that changes behavior but still allows the Author's test to pass."
            ),
        }

    def referee_round(
        self,
        target_id: str,
        test_target: str,
        round_no: int = 1,
        pytest_args: Optional[str] = None,
    ) -> ArenaRoundResult:
        """Referee a single adversarial round between Author and Adversary."""
        # 1. Zero-Token Vault Regression Check
        vault_res = MutantVault.run_vault_regression(
            target_id=target_id,
            test_target=test_target,
            repo_path=self.repo_path,
            project_name=self.project,
            pytest_args=pytest_args,
        )
        vault_passed = vault_res.get("status") == "passed"
        vault_status = vault_res.get("status", "passed")

        # 2. Execute Runtime Proof Gate with Micro-Mutation Testing
        proof_res = self.svc.verify_proof(
            target_id=target_id,
            test_target=test_target,
            mutation_check=True,
            pytest_args=pytest_args,
        )

        proof_status = proof_res.get("proof_status", "fail")
        proof_grade = proof_res.get("proof_grade", "SILVER")
        mut_proof = proof_res.get("mutation_proof") or {}
        killed = mut_proof.get("mutants_killed", 0)
        survived = mut_proof.get("mutants_survived", 0)

        causal_reflexion = mut_proof.get("causal_reflexion")
        self_healing = proof_res.get("self_healing") or {}
        reflexion_prompt = self_healing.get("reflexion_prompt") or (
            causal_reflexion.get("reflexion_prompt") if causal_reflexion else None
        )

        is_titanium = (proof_status == "pass" and proof_grade == "TITANIUM" and vault_passed)
        if is_titanium:
            final_grade = "TITANIUM"
            msg = (
                f"TITANIUM GRADE ACHIEVED in Round {round_no}! "
                f"All {killed} micro-mutants killed and all historical vault regressions passed."
            )
        else:
            final_grade = "SILVER" if proof_status == "pass" else "FAILED"
            reasons = []
            if survived > 0:
                reasons.append(f"{survived} mutant(s) SURVIVED (Weak assertion detected)")
            if not vault_passed:
                reasons.append(f"{vault_res.get('survived', 1)} historical vault mutant(s) SURVIVED")
            if proof_status != "pass":
                reasons.append("Test did not establish physical runtime coverage on target")
            msg = f"Round {round_no} Defeat: {'; '.join(reasons)}."

        return ArenaRoundResult(
            round_no=round_no,
            target_id=target_id,
            test_target=test_target,
            proof_status=proof_status,
            proof_grade=final_grade,
            is_titanium=is_titanium,
            vault_status=vault_status,
            mutants_killed=killed,
            mutants_survived=survived,
            causal_reflexion=causal_reflexion,
            reflexion_prompt=reflexion_prompt,
            message=msg,
        )
