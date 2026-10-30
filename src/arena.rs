use crate::memory_cls::GraphMemoryManager;
use crate::models::{ArenaRoundResult, CausalReflexionReport, MutantEvalDetail};
use crate::mutation_gate::MicroMutationGate;
use std::collections::HashMap;
use std::path::Path;

pub struct CausalReflexionEngine;

impl CausalReflexionEngine {
    pub fn diagnose(target_id: &str, survived_mutant: &MutantEvalDetail) -> CausalReflexionReport {
        let mut_desc = &survived_mutant.mutation;
        let orig = &survived_mutant.original_code;

        let (failure_mode, root_cause, directive) = if mut_desc.contains("bool:")
            || mut_desc.contains("Aug")
            || orig.contains("self.")
            || orig.contains("this.")
            || orig.contains(".append(")
            || orig.contains(".push(")
            || orig.contains(".insert(")
        {
            (
                "UNCHECKED_SIDE_EFFECT",
                format!(
                    "Line {} (`{}`) mutated via {} but all assertions still passed.",
                    survived_mutant.line, orig, mut_desc
                ),
                format!(
                    "Add an assertion verifying the state mutation or boolean flag produced at line {}.",
                    survived_mutant.line
                ),
            )
        } else if orig.contains("raise ")
            || orig.contains("throw ")
            || orig.contains("panic!(")
            || orig.contains("None")
            || orig.contains("null")
            || orig.contains("nil")
        {
            (
                "MISSING_EXCEPTION_CONTRACT",
                format!(
                    "Guard condition at line {} (`{}`) was inverted ({}) without triggering a test failure.",
                    survived_mutant.line, orig, mut_desc
                ),
                format!(
                    "Add a test case passing invalid/boundary inputs or None to verify the guard/exception at line {}.",
                    survived_mutant.line
                ),
            )
        } else if mut_desc == "And -> Or" || mut_desc == "Or -> And" {
            (
                "WEAK_ASSERTION",
                format!(
                    "Logical connector at line {} (`{}`) was flipped ({}) and survived.",
                    survived_mutant.line, orig, mut_desc
                ),
                format!(
                    "Add test cases where one sub-condition is true and the other is false at line {}.",
                    survived_mutant.line
                ),
            )
        } else if mut_desc == "In -> NotIn" || mut_desc == "NotIn -> In" {
            (
                "WEAK_ASSERTION",
                format!(
                    "Membership check at line {} (`{}`) was inverted ({}) and survived.",
                    survived_mutant.line, orig, mut_desc
                ),
                format!(
                    "Add test cases covering both present and absent membership keys at line {}.",
                    survived_mutant.line
                ),
            )
        } else {
            (
                "WEAK_ASSERTION",
                format!(
                    "Boundary/arithmetic operator at line {} (`{}`) was mutated ({}) and survived.",
                    survived_mutant.line, orig, mut_desc
                ),
                format!(
                    "Add an exact boundary test (e.g. testing the exact threshold value at line {}) and assert exact return values.",
                    survived_mutant.line
                ),
            )
        };

        let prompt = format!(
            "### CAUSAL REFLEXION [{}]\n- Target: `{}` (Line {})\n- Survived Mutant: `{}` ({})\n- Fix Directive: {}",
            failure_mode, target_id, survived_mutant.line, orig, mut_desc, directive
        );

        CausalReflexionReport {
            target_id: target_id.to_string(),
            failure_mode: failure_mode.to_string(),
            root_cause,
            actionable_directive: directive,
            reflexion_prompt: prompt,
        }
    }
}

pub struct TriAgentArena;

impl TriAgentArena {
    pub const MAX_ROUNDS: usize = 3;

    pub fn referee_round(
        repo_path: &Path,
        target_id: &str,
        test_target: &str,
        round_no: usize,
        pytest_args: Option<&str>,
    ) -> ArenaRoundResult {
        let proof = MicroMutationGate::verify_proof(
            repo_path,
            target_id,
            test_target,
            None,
            true,
            6,
            pytest_args,
        );

        let mut red_swarm_summary: HashMap<String, usize> = HashMap::new();
        red_swarm_summary.insert("Red1_BoundaryHunter".to_string(), 0);
        red_swarm_summary.insert("Red2_SideEffectNullifier".to_string(), 0);
        red_swarm_summary.insert("Red3_ChaosPoisoner".to_string(), 0);

        let mut mutation_score = 0.0;
        let mut reflexion_prompt = None;

        if let Some(m_proof) = &proof.mutation_proof {
            mutation_score = m_proof.mutation_score;
            for d in &m_proof.details {
                *red_swarm_summary.entry(d.persona.clone()).or_insert(0) += 1;
            }
            if let Some(ref_rep) = &m_proof.causal_reflexion {
                reflexion_prompt = Some(ref_rep.reflexion_prompt.clone());
            }
        }

        let is_titanium = proof.proof_grade == "TITANIUM";
        let stop_condition = if is_titanium {
            "TITANIUM_VICTORY".to_string()
        } else if round_no >= Self::MAX_ROUNDS {
            GraphMemoryManager::pin_lesson(
                repo_path,
                target_id,
                "Unresolved surviving mutants after 3 adversarial rounds; flagged as VULNERABILITY.",
                "VULNERABILITY",
                "MAX_ROUNDS_EXCEEDED",
            );
            "MAX_ROUNDS_EXCEEDED_VULNERABILITY".to_string()
        } else {
            "CONTINUE_NEXT_ROUND".to_string()
        };

        ArenaRoundResult {
            target_id: target_id.to_string(),
            test_target: test_target.to_string(),
            round: round_no,
            max_rounds: Self::MAX_ROUNDS,
            stop_condition,
            is_titanium,
            proof_grade: proof.proof_grade,
            mutation_score,
            message: proof.message,
            reflexion_prompt,
            red_swarm_summary,
        }
    }
}
