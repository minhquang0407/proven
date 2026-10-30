use crate::ast_graph::{
    is_brace_language_source, mask_brace_line, mask_python_line, strip_python_string_prefix,
    update_triple_quote_state,
};
use crate::models::{AxiomEntry, LessonEntry, MetaAxiomEntry, SleepConsolidationResult};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

pub struct GraphMemoryManager;

impl GraphMemoryManager {
    pub fn memory_file(repo_path: &Path) -> PathBuf {
        repo_path.join(".proven").join("memory.json")
    }

    pub fn axioms_json_file(repo_path: &Path) -> PathBuf {
        repo_path.join(".proven").join("axioms.json")
    }

    pub fn axioms_md_file(repo_path: &Path) -> PathBuf {
        repo_path.join(".proven").join("axioms.md")
    }

    pub fn meta_axioms_file(repo_path: &Path) -> PathBuf {
        repo_path.join(".proven").join("meta_axioms.json")
    }

    pub fn memory_cls_file(repo_path: &Path) -> PathBuf {
        repo_path.join(".proven").join("memory_cls.json")
    }

    pub fn pin_lesson(
        repo_path: &Path,
        target_id: &str,
        lesson: &str,
        failure_mode: &str,
        mutant_desc: &str,
    ) -> LessonEntry {
        let mem_path = Self::memory_file(repo_path);
        if let Some(parent) = mem_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let mut raw_map: BTreeMap<String, Vec<LessonEntry>> = fs::read_to_string(&mem_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let entries = raw_map.entry(target_id.to_string()).or_default();
        if let Some(existing) = entries
            .iter()
            .find(|e| e.lesson == lesson && e.failure_mode == failure_mode)
        {
            return existing.clone();
        }

        let id = format!("LESSON:{}:{}", target_id, entries.len() + 1);
        let new_entry = LessonEntry {
            id,
            target_id: target_id.to_string(),
            lesson: lesson.to_string(),
            failure_mode: failure_mode.to_string(),
            mutant_desc: mutant_desc.to_string(),
            status: "active".to_string(),
        };
        entries.push(new_entry.clone());

        if let Ok(serialized) = serde_json::to_string_pretty(&raw_map) {
            let _ = fs::write(&mem_path, serialized);
        }
        new_entry
    }

    pub fn get_pinned_lessons(repo_path: &Path, target_id: Option<&str>) -> Vec<LessonEntry> {
        let mem_path = Self::memory_file(repo_path);
        let raw_map: HashMap<String, Vec<LessonEntry>> = fs::read_to_string(&mem_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        if let Some(tid) = target_id {
            if let Some(exact) = raw_map.get(tid) {
                return exact.clone();
            }
            let alt = if let Some(stripped) = tid.strip_prefix("FUNC:") {
                stripped.to_string()
            } else {
                format!("FUNC:{}", tid)
            };
            raw_map.get(&alt).cloned().unwrap_or_default()
        } else {
            let mut all: Vec<LessonEntry> = raw_map.into_values().flatten().collect();
            all.sort_by(|a, b| a.id.cmp(&b.id));
            all
        }
    }

    pub fn consolidate_axioms(
        repo_path: &Path,
        threshold: usize,
        force: bool,
        meta_promote: bool,
    ) -> SleepConsolidationResult {
        let mem_path = Self::memory_file(repo_path);
        let mut raw_map: BTreeMap<String, Vec<LessonEntry>> = fs::read_to_string(&mem_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let total_lessons: usize = raw_map.values().map(|v| v.len()).sum();
        if !force && total_lessons < threshold {
            return SleepConsolidationResult {
                status: "skipped".to_string(),
                consolidated_lessons: 0,
                axioms_count: 0,
                promoted_meta_axioms: 0,
                axioms_file: Self::axioms_md_file(repo_path).to_string_lossy().to_string(),
                message: format!(
                    "Sleep consolidation skipped: {} lessons (< threshold {}). Use --force to consolidate.",
                    total_lessons, threshold
                ),
            };
        }

        let mut by_mode: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for lessons in raw_map.values_mut() {
            for item in lessons.iter_mut() {
                item.status = "consolidated".to_string();
                by_mode
                    .entry(item.failure_mode.clone())
                    .or_default()
                    .push(item.id.clone());
            }
        }

        if let Ok(serialized) = serde_json::to_string_pretty(&raw_map) {
            let _ = fs::write(&mem_path, serialized);
        }

        let mut axioms = Vec::new();
        for (idx, (mode, lesson_ids)) in by_mode.into_iter().enumerate() {
            let axiom_rule = Self::synthesize_axiom_for_mode(&mode);
            axioms.push(AxiomEntry {
                id: format!("AXIOM:{}", idx + 1),
                failure_mode: mode,
                axiom: axiom_rule,
                lesson_ids,
            });
        }

        if axioms.is_empty() {
            axioms.push(AxiomEntry {
                id: "AXIOM:1".to_string(),
                failure_mode: "BOUNDARY_AND_CONTRACT".to_string(),
                axiom: "Assert exact boundary equality, state mutations, and exception contracts."
                    .to_string(),
                lesson_ids: Vec::new(),
            });
        }

        let proven_dir = repo_path.join(".proven");
        let _ = fs::create_dir_all(&proven_dir);

        if let Ok(ax_json) = serde_json::to_string_pretty(&axioms) {
            let _ = fs::write(Self::axioms_json_file(repo_path), ax_json);
        }

        let mut md_lines = vec![
            "# Repository Testing Axioms (Neocortex Consolidated)".to_string(),
            String::new(),
        ];
        for ax in &axioms {
            md_lines.push(format!(
                "- **[{}]** ({}): {}",
                ax.id, ax.failure_mode, ax.axiom
            ));
        }
        let md_path = Self::axioms_md_file(repo_path);
        let _ = fs::write(&md_path, md_lines.join("\n"));

        let promoted = if meta_promote {
            Self::promote_to_meta_axioms(repo_path).len()
        } else {
            0
        };

        SleepConsolidationResult {
            status: "success".to_string(),
            consolidated_lessons: total_lessons,
            axioms_count: axioms.len(),
            promoted_meta_axioms: promoted,
            axioms_file: md_path.to_string_lossy().to_string(),
            message: format!(
                "Consolidated {} lessons into {} Neocortex axioms (Promoted {} Meta-Axioms).",
                total_lessons,
                axioms.len(),
                promoted
            ),
        }
    }

    pub fn synthesize_axiom_for_mode(mode: &str) -> String {
        match mode {
            "WEAK_ASSERTION" | "DIV_ZERO_OR_BOUNDARY" => {
                "Assert exact boundary equality (x == limit) and arithmetic return values."
                    .to_string()
            }
            "UNCHECKED_SIDE_EFFECT" | "STATE_SIDE_EFFECT" => {
                "Verify mutated object attributes, collection lengths, and emitted side effects."
                    .to_string()
            }
            "MISSING_EXCEPTION_CONTRACT" | "EXCEPTION_CONTRACT" => {
                "Explicitly test invalid/negative inputs with pytest.raises or error assertions."
                    .to_string()
            }
            "NULL_GUARD" => {
                "Test None/empty inputs explicitly to verify early-return guard clauses."
                    .to_string()
            }
            _ => "Assert exact return values, boundary thresholds, and state transitions."
                .to_string(),
        }
    }

    pub fn default_meta_axioms() -> Vec<MetaAxiomEntry> {
        vec![
            MetaAxiomEntry {
                id: "META_AXIOM:NULL_GUARD".to_string(),
                archetype: "NULL_GUARD".to_string(),
                label: "Universal Null/Empty Guard Axiom".to_string(),
                rule: "Test None/empty inputs to verify early-return guard clauses.".to_string(),
                source_axiom_id: "AXIOM:DEFAULT:1".to_string(),
            },
            MetaAxiomEntry {
                id: "META_AXIOM:DIV_ZERO_OR_BOUNDARY".to_string(),
                archetype: "DIV_ZERO_OR_BOUNDARY".to_string(),
                label: "Universal Boundary & Division Axiom".to_string(),
                rule: "Test exact threshold boundaries (x == limit) and zero-denominators."
                    .to_string(),
                source_axiom_id: "AXIOM:DEFAULT:2".to_string(),
            },
            MetaAxiomEntry {
                id: "META_AXIOM:STATE_SIDE_EFFECT".to_string(),
                archetype: "STATE_SIDE_EFFECT".to_string(),
                label: "Universal State Mutation Axiom".to_string(),
                rule: "Assert post-call state attributes and collection mutations.".to_string(),
                source_axiom_id: "AXIOM:DEFAULT:3".to_string(),
            },
            MetaAxiomEntry {
                id: "META_AXIOM:EXCEPTION_CONTRACT".to_string(),
                archetype: "EXCEPTION_CONTRACT".to_string(),
                label: "Universal Exception Contract Axiom".to_string(),
                rule: "Assert raised exceptions on invalid or out-of-range arguments.".to_string(),
                source_axiom_id: "AXIOM:DEFAULT:4".to_string(),
            },
        ]
    }

    pub fn promote_to_meta_axioms(repo_path: &Path) -> Vec<MetaAxiomEntry> {
        let meta = Self::default_meta_axioms();
        let meta_file = Self::meta_axioms_file(repo_path);
        if let Some(parent) = meta_file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let payload = json!({ "meta_axioms": meta });
        if let Ok(s) = serde_json::to_string_pretty(&payload) {
            let _ = fs::write(&meta_file, &s);
            let cls_file = Self::memory_cls_file(repo_path);
            if !cls_file.exists() {
                let _ = fs::write(&cls_file, s);
            }
        }
        meta
    }

    pub fn load_meta_axioms(repo_path: &Path) -> Vec<MetaAxiomEntry> {
        for path in [Self::memory_cls_file(repo_path), Self::meta_axioms_file(repo_path)] {
            if let Ok(content) = fs::read_to_string(&path)
                && let Ok(val) = serde_json::from_str::<Value>(&content)
                    && let Some(arr) = val.get("meta_axioms")
                        && let Ok(parsed) =
                            serde_json::from_value::<Vec<MetaAxiomEntry>>(arr.clone())
                            && !parsed.is_empty() {
                                return parsed;
                            }
        }
        Self::default_meta_axioms()
    }

    pub fn compute_structural_archetype(source_code: &str, callees: &[String]) -> Value {
        let first_line = source_code.lines().next().unwrap_or("");
        let param_count = if let (Some(open), Some(close)) =
            (first_line.find('('), first_line.find(')'))
        {
            if close > open {
                let inner = &first_line[open + 1..close];
                inner
                    .split(',')
                    .map(|p| p.trim())
                    .filter(|p| {
                        !p.is_empty()
                            && *p != "self"
                            && *p != "cls"
                            && *p != "&self"
                            && *p != "&mut self"
                    })
                    .count()
            } else {
                0
            }
        } else {
            0
        };

        let has_return_annotation = first_line.contains("->");
        let lower = source_code.to_lowercase();
        let cyclomatic_branches = ["if ", "elif ", "else", "for ", "while ", "try:", "except", "match "]
            .iter()
            .map(|kw| lower.matches(kw).count())
            .sum::<usize>();
        let raises_exceptions = lower.contains("raise ")
            || lower.contains("throw ")
            || lower.contains("error(")
            || lower.contains("err(")
            || lower.contains("panic!(");
        let mutates_state = source_code.contains("self.")
            || source_code.contains("this.")
            || source_code.contains("+=")
            || source_code.contains("-=")
            || source_code.contains(".append(")
            || source_code.contains(".insert(")
            || source_code.contains(".push(")
            || source_code.contains(".update(");
        let callee_degree = callees.len();

        let vec6 = [
            (param_count as f64 / 6.0).min(1.0),
            if has_return_annotation { 1.0 } else { 0.5 },
            (cyclomatic_branches as f64 / 10.0).min(1.0),
            if raises_exceptions { 1.0 } else { 0.0 },
            if mutates_state { 1.0 } else { 0.0 },
            (callee_degree as f64 / 6.0).min(1.0),
        ];

        json!({
            "param_count": param_count,
            "has_return_annotation": has_return_annotation,
            "cyclomatic_branches": cyclomatic_branches,
            "raises_exceptions": raises_exceptions,
            "mutates_state": mutates_state,
            "callee_degree": callee_degree,
            "vector": vec6,
        })
    }

    pub fn archetype_cosine_similarity(arch_a: &Value, arch_b: &Value) -> f64 {
        let va = arch_a
            .get("vector")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|x| x.as_f64().unwrap_or(0.0))
                    .collect::<Vec<f64>>()
            })
            .unwrap_or_default();
        let vb = arch_b
            .get("vector")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|x| x.as_f64().unwrap_or(0.0))
                    .collect::<Vec<f64>>()
            })
            .unwrap_or_default();
        if va.len() != 6 || vb.len() != 6 {
            return 0.0;
        }
        let dot: f64 = va.iter().zip(vb.iter()).map(|(a, b)| a * b).sum();
        let na: f64 = va.iter().map(|a| a * a).sum::<f64>().sqrt();
        let nb: f64 = vb.iter().map(|b| b * b).sum::<f64>().sqrt();
        if na == 0.0 || nb == 0.0 {
            0.0
        } else {
            ((dot / (na * nb)) * 10000.0).round() / 10000.0
        }
    }

    pub fn record_success(
        repo_path: &Path,
        function_id: &str,
        mutants_killed: usize,
        total_mutants: usize,
    ) -> Value {
        let ctx = crate::ast_graph::extract_scoped_context(
            repo_path,
            Some(function_id),
            None,
            false,
        );
        let structural_archetype =
            Self::compute_structural_archetype(&ctx.source_code, &ctx.callees);
        let mut archetypes = Self::detect_ast_archetypes(&ctx.source_code);
        if archetypes.is_empty() {
            archetypes.push("DIV_ZERO_OR_BOUNDARY".to_string());
        }

        let effective_tid = if ctx.target_id.is_empty() {
            function_id.to_string()
        } else {
            ctx.target_id.clone()
        };

        for arch in &archetypes {
            let rule = Self::synthesize_axiom_for_mode(arch);
            Self::pin_lesson(
                repo_path,
                &effective_tid,
                &rule,
                arch,
                &format!("KILLED_{}_OF_{}_MUTANTS", mutants_killed, total_mutants),
            );
        }

        let sleep_res = Self::consolidate_axioms(repo_path, 1, true, true);
        let all_meta = Self::default_meta_axioms();
        let mut distilled_meta: Vec<MetaAxiomEntry> = all_meta
            .iter()
            .filter(|m| archetypes.contains(&m.archetype))
            .cloned()
            .collect();
        if distilled_meta.is_empty() {
            distilled_meta = all_meta;
        }

        let cls_payload = json!({
            "status": "success",
            "source_function": effective_tid,
            "source_file": ctx.source_file,
            "mutants_killed": mutants_killed,
            "total_mutants": total_mutants,
            "structural_archetype": structural_archetype,
            "detected_archetypes": archetypes,
            "meta_axioms": distilled_meta,
            "consolidated_axioms": sleep_res.axioms_count,
        });

        let cls_file = Self::memory_cls_file(repo_path);
        if let Some(parent) = cls_file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(serialized) = serde_json::to_string_pretty(&cls_payload) {
            let _ = fs::write(&cls_file, &serialized);
        }
        let meta_file = Self::meta_axioms_file(repo_path);
        if let Ok(serialized) = serde_json::to_string_pretty(&json!({ "meta_axioms": distilled_meta }))
        {
            let _ = fs::write(&meta_file, serialized);
        }

        cls_payload
    }

    pub fn detect_ast_archetypes(source_code: &str) -> Vec<String> {
        let mut archetypes = Vec::new();
        let mut in_triple: Option<&str> = None;
        let mut in_block_comment = false;
        let is_brace = is_brace_language_source(source_code);
        let mut masked_lines = Vec::new();

        for line in source_code.lines() {
            if !is_brace {
                let was_triple = in_triple.is_some();
                in_triple = update_triple_quote_state(line, in_triple);
                if was_triple || in_triple.is_some() {
                    continue;
                }
                let trimmed = strip_python_string_prefix(line.trim());
                if trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''") {
                    continue;
                }
                masked_lines.push(mask_python_line(line));
            } else {
                masked_lines.push(mask_brace_line(line, &mut in_block_comment));
            }
        }

        let code_only = masked_lines.join("\n");
        let lower = code_only.to_lowercase();

        if lower.contains("is none")
            || lower.contains("== none")
            || lower.contains("!= none")
            || lower.contains("== null")
            || lower.contains("!= null")
            || lower.contains("=== null")
            || lower.contains("!== null")
            || lower.contains("== nil")
            || lower.contains("!= nil")
            || lower.contains("not ")
            || lower.contains(".is_empty()")
            || lower.contains("is_none()")
        {
            archetypes.push("NULL_GUARD".to_string());
        }

        if code_only.contains(">=")
            || code_only.contains("<=")
            || code_only.contains(" > ")
            || code_only.contains(" < ")
            || code_only.contains(" / ")
            || code_only.contains(" // ")
        {
            archetypes.push("DIV_ZERO_OR_BOUNDARY".to_string());
        }

        if code_only.contains("self.")
            || code_only.contains("this.")
            || code_only.contains(".append(")
            || code_only.contains(".insert(")
            || code_only.contains(".push(")
            || code_only.contains(".update(")
            || code_only.contains("+=")
            || code_only.contains("-=")
        {
            archetypes.push("STATE_SIDE_EFFECT".to_string());
        }

        if lower.contains("raise ")
            || lower.contains("throw ")
            || lower.contains("error(")
            || lower.contains("err(")
            || lower.contains("panic!(")
        {
            archetypes.push("EXCEPTION_CONTRACT".to_string());
        }

        archetypes
    }

    pub fn bind_meta_axioms_to_target(
        repo_path: &Path,
        target_id: &str,
        source_code: &str,
    ) -> Vec<MetaAxiomEntry> {
        let archetypes = Self::detect_ast_archetypes(source_code);
        let available = Self::load_meta_axioms(repo_path);
        let mut matched = Vec::new();

        for item in available {
            if archetypes.contains(&item.archetype) {
                Self::pin_lesson(
                    repo_path,
                    target_id,
                    &item.rule,
                    &item.archetype,
                    "JIT_META_AXIOM_BINDING",
                );
                matched.push(item);
            }
        }
        matched
    }

    pub fn get_scoped_memory_prompt(repo_path: &Path, target_id: &str) -> Option<String> {
        let mut lessons = Self::get_pinned_lessons(repo_path, Some(target_id));
        if lessons.is_empty() {
            return None;
        }

        // Prioritize empirical surviving-mutant / vulnerability lessons over generic JIT meta-axioms
        lessons.sort_by_key(|l| usize::from(l.mutant_desc == "JIT_META_AXIOM_BINDING"));

        let mut lines = vec!["### REPO MEMORY (Neocortex Axiom)".to_string()];
        for l in lessons.iter().take(2) {
            lines.push(format!("- [{}] {}", l.failure_mode, l.lesson));
        }

        let mut out = lines.join("\n");
        // Token Shield: strictly < 80 tokens (~260 chars), UTF-8 char-boundary safe
        if out.chars().count() > 260 {
            out = out.chars().take(257).collect::<String>();
            out.push_str("...");
        }
        Some(out)
    }
}
