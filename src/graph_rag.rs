use crate::ast_graph::{build_repo_graph_parallel, extract_scoped_context, read_bounded_source_file};
use crate::models::{
    AttendedNode, ImpactDependent, ImpactPredictionResult, RepoGraph, SubgraphAttentionResult,
    TriageEngineer, TriageFile, TriageResult,
};
use regex::Regex;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

pub struct CodeGraphRAG;

impl CodeGraphRAG {
    pub fn relation_weight(rel: &str) -> f64 {
        match rel.to_lowercase().as_str() {
            "calls" => 0.85,
            "instantiates" => 0.80,
            "defines" => 0.75,
            "uses" => 0.70,
            "modifies" | "co-change" => 0.60,
            "imports" => 0.50,
            _ => 0.55,
        }
    }

    pub fn generate_rationale(relation: &str, hop: usize, node_type: &str) -> String {
        let rel = relation.to_lowercase();
        if rel.contains("call") {
            if hop == 1 {
                return "Direct callee; high execution failure blast radius".to_string();
            }
            return format!(
                "Transitive call dependency (hop {}); potential regression propagation",
                hop
            );
        }
        if rel.contains("use") || node_type == "Class" {
            return "Shared state or data model container; concurrency & dirty state sensitive"
                .to_string();
        }
        if rel.contains("co-change") || rel.contains("modifies") {
            return format!(
                "Historical co-change correlation (hop {}); high commit coupling",
                hop
            );
        }
        if rel.contains("instantiate") {
            return "Direct class instantiation; lifecycle and constructor dependency".to_string();
        }
        format!("Structural dependency (hop {}) via relation '{}'", hop, relation)
    }

    pub fn extract_subgraph_attention(
        repo_path: &Path,
        target_id: &str,
        hops: usize,
        top_k: usize,
        mode: &str,
    ) -> SubgraphAttentionResult {
        let clean_target = target_id.trim().to_string();
        let graph = build_repo_graph_parallel(repo_path);

        let resolved_id = Self::resolve_node_id(&graph, &clean_target);
        let mut attended_nodes = Vec::new();
        let mut total_subgraph_nodes = 0;
        let mut engine_used = "topological_ppr".to_string();

        if let Some(root_id) = resolved_id {
            let mut visited: HashSet<String> = HashSet::new();
            visited.insert(root_id.clone());

            let mut scores: HashMap<String, (f64, usize, String, String)> = HashMap::new();
            let mut queue: VecDeque<(String, usize, f64, Vec<String>)> = VecDeque::new();
            queue.push_back((root_id.clone(), 0, 1.0, vec![root_id.clone()]));

            while let Some((curr_id, curr_hop, curr_w, curr_path)) = queue.pop_front() {
                if curr_hop >= hops {
                    continue;
                }

                for edge in &graph.edges {
                    if edge.src == curr_id && edge.dst != root_id {
                        if let Some(dst_node) = graph.nodes.get(&edge.dst)
                            && matches!(
                                dst_node.node_type.as_str(),
                                "Function" | "Class" | "File"
                            ) {
                                let w = Self::relation_weight(&edge.relation);
                                let decayed = curr_w * w * 0.75_f64.powi(curr_hop as i32);
                                let mut next_path = curr_path.clone();
                                next_path.push(format!("--({})--> {}", edge.relation, edge.dst));

                                let entry = scores.entry(edge.dst.clone()).or_insert((
                                    0.0,
                                    curr_hop + 1,
                                    edge.relation.clone(),
                                    next_path.join(" "),
                                ));
                                entry.0 += decayed;

                                if !visited.contains(&edge.dst) && curr_hop + 1 < hops {
                                    visited.insert(edge.dst.clone());
                                    queue.push_back((
                                        edge.dst.clone(),
                                        curr_hop + 1,
                                        decayed,
                                        next_path,
                                    ));
                                }
                            }
                    } else if edge.dst == curr_id && edge.src != root_id
                        && let Some(src_node) = graph.nodes.get(&edge.src)
                            && matches!(
                                src_node.node_type.as_str(),
                                "Function" | "Class" | "File"
                            ) {
                                let w = Self::relation_weight(&edge.relation);
                                let decayed = curr_w * w * 0.75_f64.powi(curr_hop as i32);
                                let mut next_path =
                                    vec![format!("{} --({})-->", edge.src, edge.relation)];
                                next_path.extend(curr_path.clone());

                                let entry = scores.entry(edge.src.clone()).or_insert((
                                    0.0,
                                    curr_hop + 1,
                                    edge.relation.clone(),
                                    next_path.join(" "),
                                ));
                                entry.0 += decayed;

                                if !visited.contains(&edge.src) && curr_hop + 1 < hops {
                                    visited.insert(edge.src.clone());
                                    queue.push_back((
                                        edge.src.clone(),
                                        curr_hop + 1,
                                        decayed,
                                        next_path,
                                    ));
                                }
                            }
                }
            }

            total_subgraph_nodes = scores.len();
            let mut ranked: Vec<(String, f64, usize, String, String)> = scores
                .into_iter()
                .map(|(k, (sc, h, r, p))| (k, sc.clamp(0.05, 1.0), h, r, p))
                .collect();
            ranked.sort_by(|a, b| {
                b.1.partial_cmp(&a.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.0.cmp(&b.0))
            });

            for (node_id, attn, hop, rel, path) in ranked.into_iter().take(top_k) {
                let ntype = graph
                    .nodes
                    .get(&node_id)
                    .map(|n| n.node_type.clone())
                    .unwrap_or_else(|| "Function".to_string());
                attended_nodes.push(AttendedNode {
                    symbol: node_id,
                    node_type: ntype.clone(),
                    attention_score: (attn * 100.0).round() / 100.0,
                    hop_distance: hop,
                    relation: rel.clone(),
                    path,
                    semantic_rationale: Self::generate_rationale(&rel, hop, &ntype),
                });
            }
        }

        if attended_nodes.is_empty() && hops > 0 && !clean_target.is_empty() {
            let ctx = extract_scoped_context(repo_path, Some(&clean_target), None, false);
            for (idx, callee) in ctx.callees.iter().take(top_k).enumerate() {
                let score = (0.90 - (idx as f64 * 0.08)).max(0.10);
                attended_nodes.push(AttendedNode {
                    symbol: format!("FUNC:{}", callee),
                    node_type: "Function".to_string(),
                    attention_score: (score * 100.0).round() / 100.0,
                    hop_distance: 1,
                    relation: "calls".to_string(),
                    path: format!("{} --(calls)--> {}", clean_target, callee),
                    semantic_rationale: "Direct callee (hop 1); high failure blast radius"
                        .to_string(),
                });
            }
            total_subgraph_nodes = attended_nodes.len();
            engine_used = "ast_heuristic".to_string();
        }

        let target_label = clean_target
            .replace("FUNC:", "")
            .replace("CLASS:", "")
            .replace("FILE:", "");
        let prompt_block = Self::render_prompt_block(&clean_target, &attended_nodes, 120);
        let ascii_tree =
            Self::format_attention_tree(&target_label, &engine_used, hops, &attended_nodes);

        SubgraphAttentionResult {
            target_id: clean_target,
            target_label,
            hops,
            mode: mode.to_string(),
            engine: engine_used,
            total_subgraph_nodes,
            attended_nodes,
            prompt_block,
            ascii_tree,
        }
    }

    pub fn build_prompt_from_scoped(
        target_id: &str,
        callees: &[String],
        max_tokens: usize,
    ) -> String {
        let mut nodes = Vec::new();
        for (idx, callee) in callees.iter().take(3).enumerate() {
            let score = (0.90 - (idx as f64 * 0.08)).max(0.10);
            nodes.push(AttendedNode {
                symbol: format!("FUNC:{}", callee),
                node_type: "Function".to_string(),
                attention_score: score,
                hop_distance: 1,
                relation: "calls".to_string(),
                path: format!("{} --(calls)--> {}", target_id, callee),
                semantic_rationale: "Direct callee; high execution failure blast radius"
                    .to_string(),
            });
        }
        Self::render_prompt_block(target_id, &nodes, max_tokens)
    }

    pub fn render_prompt_block(
        target_id: &str,
        attended_nodes: &[AttendedNode],
        max_tokens: usize,
    ) -> String {
        if attended_nodes.is_empty() {
            return String::new();
        }
        let mut lines = vec![
            "### CODE GRAPHRAG (Topological Attention Context)".to_string(),
            format!("Target: `{}`", target_id),
        ];
        for node in attended_nodes.iter().take(4) {
            lines.push(format!(
                "- **[Attn: {:.2}]** `{}` ({} | {})",
                node.attention_score, node.symbol, node.relation, node.semantic_rationale
            ));
        }
        let full = lines.join("\n");
        let max_chars = max_tokens * 4;
        if full.len() > max_chars {
            lines.truncate(3);
            let joined = lines.join("\n");
            if joined.len() > max_chars {
                joined.chars().take(max_chars).collect()
            } else {
                joined
            }
        } else {
            full
        }
    }

    pub fn format_attention_tree(
        target_label: &str,
        engine: &str,
        hops: usize,
        attended: &[AttendedNode],
    ) -> String {
        let mut lines = vec![format!(
            "Target: {} (Root) [Engine: {}, Hops: {}]",
            target_label, engine, hops
        )];
        if attended.is_empty() {
            lines.push("  +-- (No multi-hop dependencies found in graph)".to_string());
            return lines.join("\n");
        }
        for (idx, item) in attended.iter().enumerate() {
            let is_last = idx + 1 == attended.len();
            let connector = if is_last { "+--" } else { "|--" };
            let pct = (item.attention_score * 100.0).round() as i32;
            lines.push(format!(
                "  {} [Attn: {}%] --({})--> {}",
                connector, pct, item.relation, item.symbol
            ));
            let indent = if is_last { "      " } else { "  |   " };
            lines.push(format!("{}Rationale: {}", indent, item.semantic_rationale));
        }
        lines.join("\n")
    }

    fn resolve_node_id(graph: &RepoGraph, query: &str) -> Option<String> {
        let q_trim = query.trim();
        if q_trim.is_empty() {
            return None;
        }
        if graph.nodes.contains_key(q_trim) {
            return Some(q_trim.to_string());
        }
        let clean = q_trim
            .strip_prefix("FUNC:")
            .or_else(|| q_trim.strip_prefix("CLASS:"))
            .or_else(|| q_trim.strip_prefix("FILE:"))
            .unwrap_or(q_trim)
            .trim();
        let short = clean.split('.').next_back().unwrap_or(clean).trim();
        if clean.is_empty() || short.is_empty() {
            return None;
        }

        for exact_cand in [
            format!("FUNC:{}", clean),
            format!("CLASS:{}", clean),
            format!("FILE:{}", clean),
            format!("FUNC:{}", short),
            format!("CLASS:{}", short),
        ] {
            if graph.nodes.contains_key(&exact_cand) {
                return Some(exact_cand);
            }
        }

        let mut sorted_ids: Vec<&String> = graph.nodes.keys().collect();
        sorted_ids.sort();

        for id in sorted_ids {
            if id.eq_ignore_ascii_case(q_trim)
                || id.ends_with(&format!(":{}", clean))
                || id.ends_with(&format!(".{}", short))
                || id.ends_with(&format!(":{}", short))
            {
                return Some(id.clone());
            }
        }
        None
    }

    pub fn predict_impact(repo_path: &Path, target_symbol: &str, mode: &str) -> ImpactPredictionResult {
        let project = repo_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("default")
            .to_string();
        let attn = Self::extract_subgraph_attention(repo_path, target_symbol, 3, 10, mode);
        let mut direct_dependents = Vec::new();
        let mut latent_risk_candidates = Vec::new();

        for node in attn.attended_nodes {
            let dep = ImpactDependent {
                symbol: node.symbol,
                node_type: node.node_type,
                score: node.attention_score,
                relation: node.relation,
                path: node.path,
            };
            if node.hop_distance <= 1 {
                direct_dependents.push(dep);
            } else {
                latent_risk_candidates.push(dep);
            }
        }

        let target_type = if attn.target_id.starts_with("CLASS:") {
            "Class".to_string()
        } else if attn.target_id.starts_with("FILE:") {
            "File".to_string()
        } else {
            "Function".to_string()
        };

        ImpactPredictionResult {
            status: "success".to_string(),
            project,
            target: attn.target_id,
            target_type,
            mode: mode.to_string(),
            direct_impact_count: direct_dependents.len(),
            direct_dependents,
            latent_risk_candidates,
            warnings: Vec::new(),
        }
    }

    pub fn triage_bug(
        repo_path: &Path,
        query: &str,
        max_devs: usize,
        max_files: usize,
    ) -> TriageResult {
        let project = repo_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("default")
            .to_string();
        let graph = build_repo_graph_parallel(repo_path);

        let terms: Vec<String> = match Regex::new(r"[A-Za-z_]{3,}") {
            Ok(word_re) => word_re
                .find_iter(&query.to_lowercase())
                .map(|m| m.as_str().to_string())
                .filter(|w| !matches!(w.as_str(), "the" | "and" | "for" | "with" | "bug" | "fix"))
                .collect(),
            Err(_) => Vec::new(),
        };

        let mut file_scores: Vec<(String, f64, f64, f64)> = Vec::new();
        for node in graph.nodes.values() {
            if node.node_type == "File" {
                let rel = node.name.clone();
                let abs_p = repo_path.join(&rel);
                let content = read_bounded_source_file(&abs_p)
                    .unwrap_or_default()
                    .to_lowercase();
                let hits = terms.iter().filter(|t| content.contains(t.as_str())).count();
                let path_hits = terms
                    .iter()
                    .filter(|t| rel.to_lowercase().contains(t.as_str()))
                    .count();

                let denom = (terms.len() as f64 * 1.5).max(3.0);
                let lex = ((hits * 2 + path_hits) as f64 / denom).min(1.0);
                let sem = ((path_hits * 2 + hits) as f64 / denom).min(1.0);
                let combined = 0.55 * sem + 0.45 * lex;
                file_scores.push((rel, combined, sem, lex));
            }
        }
        file_scores.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        file_scores.truncate(max_files);

        let mut commit_to_dev: HashMap<String, String> = HashMap::new();
        for e in &graph.edges {
            if e.relation == "authored_by" {
                commit_to_dev.insert(e.dst.clone(), e.src.replace("DEV:", ""));
            }
        }

        let mut dev_touches: HashMap<String, HashMap<String, usize>> = HashMap::new();
        for e in &graph.edges {
            if e.relation == "modifies"
                && let Some(dev) = commit_to_dev.get(&e.src) {
                    let f = e.dst.replace("FILE:", "");
                    *dev_touches
                        .entry(dev.clone())
                        .or_default()
                        .entry(f)
                        .or_insert(0) += 1;
                }
        }

        let top_file_set: HashSet<String> = file_scores.iter().map(|x| x.0.clone()).collect();
        let mut dev_ranked: Vec<(String, f64, f64, f64, String)> = Vec::new();

        for (dev, f_map) in dev_touches {
            let total_touches: usize = f_map.values().sum();
            let direct_touches: usize = f_map
                .iter()
                .filter(|(f, _)| top_file_set.contains(*f))
                .map(|(_, c)| *c)
                .collect::<Vec<_>>()
                .into_iter()
                .sum();
            let git_score = ((direct_touches * 2 + total_touches) as f64 / 20.0).min(1.0);
            let sem_score = if top_file_set.is_empty() {
                0.0
            } else {
                let overlap = f_map.keys().filter(|f| top_file_set.contains(*f)).count();
                overlap as f64 / top_file_set.len() as f64
            };
            let final_score = 0.65 * git_score + 0.35 * sem_score;
            let mut ev_list: Vec<(&String, &usize)> = f_map.iter().collect();
            ev_list.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
            let ev_str = ev_list
                .into_iter()
                .take(3)
                .map(|(f, c)| format!("{} ({})", f, c))
                .collect::<Vec<_>>()
                .join(", ");
            dev_ranked.push((
                dev,
                final_score,
                git_score,
                sem_score,
                format!("Direct: {}", ev_str),
            ));
        }
        dev_ranked.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        dev_ranked.truncate(max_devs);

        let top_engineers = dev_ranked
            .into_iter()
            .enumerate()
            .map(|(idx, (dev, fin, git, sem, ev))| TriageEngineer {
                rank: idx + 1,
                developer: dev,
                final_score: (fin * 10000.0).round() / 10000.0,
                git_score: (git * 10000.0).round() / 10000.0,
                sem_score: (sem * 10000.0).round() / 10000.0,
                evidence: ev,
            })
            .collect();

        let related_files = file_scores
            .into_iter()
            .enumerate()
            .map(|(idx, (f, rel, sem, lex))| TriageFile {
                rank: idx + 1,
                file: f,
                relevance: (rel * 10000.0).round() / 10000.0,
                semantic_score: (sem * 10000.0).round() / 10000.0,
                lexical_score: (lex * 10000.0).round() / 10000.0,
            })
            .collect();

        TriageResult {
            status: "success".to_string(),
            project,
            query: query.to_string(),
            top_engineers,
            related_files,
        }
    }
}
