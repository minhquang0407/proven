use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphNode {
    pub id: String,
    pub node_type: String,
    pub name: String,
    pub kind: String,
    pub is_project_defined: bool,
    pub defined_in: Option<String>,
    pub module: Option<String>,
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphEdge {
    pub src: String,
    pub dst: String,
    pub relation: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepoGraph {
    pub nodes: HashMap<String, GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSpan {
    pub function_id: String,
    pub name: String,
    pub start: usize,
    pub end: usize,
    pub signature: String,
    pub docstring: String,
    pub source: String,
    pub language: String,
    pub calls: Vec<String>,
    pub uses: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpecSnippet {
    pub source: String,
    pub location: String,
    pub excerpt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnverifiedLiteral {
    pub line: usize,
    pub literal: String,
    pub code_line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SiblingGuardHint {
    pub guard_pattern: String,
    pub observed_in_siblings: Vec<String>,
    pub suggested_probe: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntegrationSliceHint {
    pub entry_callers: Vec<String>,
    pub internal_callees: Vec<String>,
    pub io_boundaries_detected: Vec<String>,
    pub directive: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConcurrencyRiskHint {
    pub signals: Vec<String>,
    pub recommended_harness: String,
    pub runner_flag_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnvKeyUsage {
    pub key: String,
    pub file: String,
    pub line: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigAuditReport {
    pub env_example_files: Vec<String>,
    pub referenced_env_keys: Vec<EnvKeyUsage>,
    pub declared_env_example_keys: Vec<String>,
    pub missing_env_example_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetContext {
    pub status: String,
    pub track: String,
    pub target_id: String,
    pub language: String,
    pub source_file: String,
    pub module_path: String,
    pub signature: String,
    pub docstring: String,
    pub source_code: String,
    pub imports: Vec<String>,
    pub functions: Vec<String>,
    pub callers: Vec<String>,
    pub callees: Vec<String>,
    pub suggested_test_file: String,
    pub existing_test_file_exists: bool,
    pub existing_test_preview: Option<String>,
    pub repo_memory: Option<String>,
    pub graph_rag_prompt: Option<String>,
    pub pinned_lessons: Vec<LessonEntry>,
    #[serde(default)]
    pub blind_mode: bool,
    #[serde(default)]
    pub discovered_specs: Vec<SpecSnippet>,
    #[serde(default)]
    pub inferred_invariants: Vec<String>,
    #[serde(default)]
    pub missing_sibling_guards: Vec<SiblingGuardHint>,
    #[serde(default)]
    pub unverified_literals: Vec<UnverifiedLiteral>,
    #[serde(default)]
    pub integration_slice: Option<IntegrationSliceHint>,
    #[serde(default)]
    pub concurrency_risk: Option<ConcurrencyRiskHint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_audit: Option<ConfigAuditReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttendedNode {
    pub symbol: String,
    pub node_type: String,
    pub attention_score: f64,
    pub hop_distance: usize,
    pub relation: String,
    pub path: String,
    pub semantic_rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubgraphAttentionResult {
    pub target_id: String,
    pub target_label: String,
    pub hops: usize,
    pub mode: String,
    pub engine: String,
    pub total_subgraph_nodes: usize,
    pub attended_nodes: Vec<AttendedNode>,
    pub prompt_block: String,
    pub ascii_tree: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactDependent {
    pub symbol: String,
    pub node_type: String,
    pub score: f64,
    pub relation: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactPredictionResult {
    pub status: String,
    pub project: String,
    pub target: String,
    pub target_type: String,
    pub mode: String,
    pub direct_impact_count: usize,
    pub direct_dependents: Vec<ImpactDependent>,
    pub latent_risk_candidates: Vec<ImpactDependent>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageEngineer {
    pub rank: usize,
    pub developer: String,
    pub final_score: f64,
    pub git_score: f64,
    pub sem_score: f64,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageFile {
    pub rank: usize,
    pub file: String,
    pub relevance: f64,
    pub semantic_score: f64,
    pub lexical_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageResult {
    pub status: String,
    pub project: String,
    pub query: String,
    pub top_engineers: Vec<TriageEngineer>,
    pub related_files: Vec<TriageFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutantSpec {
    pub mutant_id: usize,
    pub persona: String,
    pub line: usize,
    pub mutation: String,
    pub original_line: String,
    pub mutated_line: String,
    pub mutated_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutantEvalDetail {
    pub mutant_id: usize,
    pub persona: String,
    pub line: usize,
    pub mutation: String,
    pub original_code: String,
    pub mutated_code: String,
    pub status: String,
    pub verdict: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalReflexionReport {
    pub target_id: String,
    pub failure_mode: String,
    pub root_cause: String,
    pub actionable_directive: String,
    pub reflexion_prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationGateResult {
    pub status: String,
    pub mutation_gate_status: String,
    pub mutants_total: usize,
    pub mutants_killed: usize,
    pub mutants_survived: usize,
    pub mutation_score: f64,
    pub proof_grade: String,
    pub message: String,
    pub details: Vec<MutantEvalDetail>,
    pub causal_reflexion: Option<CausalReflexionReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyProofResult {
    pub status: String,
    pub track: String,
    pub proof_status: String,
    pub proof_grade: String,
    pub target_id: String,
    pub test_target: String,
    pub pytest_passed: bool,
    pub pytest_returncode: i32,
    pub message: String,
    pub covered_fraction: f64,
    pub covered_lines: Vec<usize>,
    pub covered_line_count: usize,
    pub function_line_count: usize,
    pub mutation_proof: Option<MutationGateResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaRoundResult {
    pub target_id: String,
    pub test_target: String,
    pub round: usize,
    pub max_rounds: usize,
    pub stop_condition: String,
    pub is_titanium: bool,
    pub proof_grade: String,
    pub mutation_score: f64,
    pub message: String,
    pub reflexion_prompt: Option<String>,
    pub red_swarm_summary: HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LessonEntry {
    pub id: String,
    pub target_id: String,
    pub lesson: String,
    pub failure_mode: String,
    pub mutant_desc: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxiomEntry {
    pub id: String,
    pub failure_mode: String,
    pub axiom: String,
    pub lesson_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaAxiomEntry {
    pub id: String,
    pub archetype: String,
    pub label: String,
    pub rule: String,
    pub source_axiom_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SleepConsolidationResult {
    pub status: String,
    pub consolidated_lessons: usize,
    pub axioms_count: usize,
    pub promoted_meta_axioms: usize,
    pub axioms_file: String,
    pub message: String,
}
