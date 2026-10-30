# Tier 2 Advanced Capabilities Guide

Proven runs **100% natively in Rust** with zero Python ML dependencies (`torch`, `torch_geometric`, or external vector databases are never required). All Tier 2 topological and memory capabilities execute deterministically in `<5ms`.

---

## 1. Multi-Hop Code GraphRAG (`explain-attention`)
Instead of dumping entire files into an LLM prompt, `GraphRAGEngine` (`src/graph_rag.rs`) builds the repository's AST + Import + Call graph in parallel using `rayon` and computes multi-hop topological attention weights using:
- **Edge-type priors**: `CALLS` (`0.95`), `DEFINES` (`0.85`), `IMPORTS` (`0.75`), `TESTS` (`0.80`).
- **Hop decay**: `0.72^(hop - 1)` normalized by node degree penalty so hub nodes do not drown out specific callees.

```bash
proven explain-attention --target "FUNC:calculate_order_total" --hops 3 --top-k 5 --visualize
```

---

## 2. Change Impact Blast Radius (`impact`)
Computes upstream callers, downstream callees, structural risk score, and co-evolving files for any symbol or file:
```bash
proven impact --target "FUNC:calculate_order_total" --path . --json
```
Returns `risk_level` (`HIGH`, `MEDIUM`, `LOW`), `risk_score`, `direct_callers`, `co_evolving_files`, and `recommended_actions`.

---

## 3. Graph Bug Triage (`triage`)
Combines lexical + AST symbol matching with `git log` commit frequency to rank suspect files and identify the best engineers to review a fix:
```bash
proven triage --query "discount calculation off by one on VIP orders" --path . --json
```

---

## 4. Complementary Learning Systems (CLS) Memory & Cross-Repo Meta-Axioms (`sleep` & `memory`)
Proven implements a 3-tier biological memory hierarchy in `src/memory_cls.rs`:
1. **Tier 1 — Hippocampus (`.proven/memory.json` -> `lessons`)**: Automatically records survived Red Team mutants whenever a test fails the mutation gate.
2. **Tier 2 — Neocortex (`.proven/memory.json` -> `axioms`)**: Consolidates recurring failure modes into compact repo-specific rules (`<= 45` words) via `proven sleep --force`.
3. **Tier 3 — Cross-Repo Meta-Axioms (`.proven/meta_axioms.json`)**: Abstracts repo-specific axioms into domain-independent AST structural archetypes (`NULL_GUARD`, `DIV_ZERO_OR_BOUNDARY`, `STATE_SIDE_EFFECT`, `EXCEPTION_CONTRACT`) via `proven sleep --force --meta` or `proven memory meta-promote`.

When `proven context` runs on a brand-new repository, `detect_ast_archetypes` inspects the target function's masked AST features (`NULL_GUARD`, `DIV_ZERO_OR_BOUNDARY`, `STATE_SIDE_EFFECT`, `EXCEPTION_CONTRACT`) and automatically binds matching Meta-Axioms into the prompt while strictly enforcing a **<80 token memory shield**.
