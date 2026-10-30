# Proven Roadmap

## Completed (v1.0.0)

- [x] **9-Language AST & Call Graph (`src/ast_graph.rs`)**: Parallel AST indexing across Python, Rust, Go, JavaScript, TypeScript, C, C++, Java, and C# with comment/string masking and scoped context extraction.
- [x] **Runtime & AST Mutation Verification (`src/mutation_gate.rs`)**: Test execution verification, test-body invocation checks, and AST mutation testing with `FileRestoreGuard` atomic rollback and `RepoMutationLockGuard` concurrency locking.
- [x] **Multi-Hop Call-Graph Context (`src/graph_rag.rs`)**: 3-hop Personalized PageRank (`explain-attention`), change impact analysis (`impact`), and commit-history bug triage (`triage`).
- [x] **Failure Memory & Cross-Repo Rules (`src/memory_cls.rs`)**: Survived-mutant tracking, rule consolidation (`proven sleep`), and `<80`-token cross-repository AST rules.
- [x] **120-Repository Empirical Benchmark (`docs/EMPIRICAL_BENCHMARK_RESULTS.md`)**: Blind A/B evaluation across 120 open-source repositories (831 AST mutants) and an 84-repository 2×2 factorial matrix across all 9 languages.
- [x] **Input & Path Safety (`src/mutation_gate.rs` & `src/ast_graph.rs`)**: Path traversal and symlink escape checks (`resolve_safe_repo_path`), CLI argument sanitization, shell-free subprocess execution, and a 1 MB file size limit.

---

## Planned (v1.1+)

- [ ] **Incremental File Watcher (`proven watch`)**: Resident file watcher updating modified files on save.
- [ ] **Additional Build Tool Presets**: Built-in runner presets for Maven/Gradle (`mvn test`, `gradle test`), `.NET` (`dotnet test`), and CTest (`ctest`).
- [ ] **GitHub SARIF Output (`proven scan --sarif`)**: SARIF v2.1.0 output for GitHub Code Scanning annotations.
