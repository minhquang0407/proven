# Changelog

All notable changes to **Proven** are documented in this file.

## [1.0.0] - 2026-10-30

### Added
- **9-Language Scoped AST & Call-Graph Engine (`src/ast_graph.rs`, `src/graph_rag.rs`)**: Parallel AST, import, and call-graph indexing across Python, Rust, Go, JavaScript, TypeScript, C, C++, Java, and C# with 3-hop Personalized PageRank context extraction (`context`, `explain-attention`, `impact`, `triage`) and 1 MB file safety guards.
- **Blind External Specification Discovery & Unverified Literal Audit (`src/oracle.rs`, `src/main.rs`)**: Added `proven context --blind [--spec <SPEC>]` to extract external requirements (`discovered_specs`) from docstrings across all 9 languages, repository docs (`README*`, `SPEC*`, `PRD*`, `docs/**`, `openapi.*`), `git log` history, and production caller usage before exposing `source_code`, plus `unverified_literals` checks in full context mode.
- **Sibling Guard Omission & Signature Invariant Probing (`src/oracle.rs`)**: Added `missing_sibling_guards` (detecting security, state, permission, and null guards enforced by sibling functions in the module) and `inferred_invariants` (signature-derived boundary probes).
- **Live AST Mutation & 1–3 Hop Transitive Slice Gate (`src/mutation_gate.rs`, `src/arena.rs`)**: Whole-function stratified AST mutation across boundary comparisons, logical connectors (`and`/`or`, `&&`/`||`), membership checks (`in`/`not in`), bitwise shifts (`<<`/`>>`), compound assignments, and state/arithmetic expressions with compile-safety filters, 1–3 hop reverse call-graph slice verification, RAII `FileRestoreGuard` rollback, and `RepoMutationLockGuard` workspace locking.
- **Integration Slice, Concurrency Barrier & `.env` Config Drift Hints (`src/oracle.rs`, `src/ast_graph.rs`)**: Added `integration_slice`, `concurrency_risk` (with Go `GOMAXPROCS=4` and `-race` scheduling), and `config_audit` comparing environment variable reads against `.env.example` / `.env.sample` / `.env.template`.
- **Failure Memory & Cross-Repository Rules (`src/memory_cls.rs`)**: Survived-mutant logging (`.proven/memory.json`), rule consolidation (`proven sleep`), and compact (`<80`-token) reusable rules (`.proven/meta_axioms.json`).
- **Empirical Benchmarks (`docs/`)**: Full evaluation across 120 open-source repositories (`831` AST mutants), an 84-repository `2×2` cross-model matrix (`681` mutants), a 36-repository 6-arm multi-skill comparison (`293` mutants), and a 36-repository fault-seeded real-bug & false-green benchmark (`308` mutants).
