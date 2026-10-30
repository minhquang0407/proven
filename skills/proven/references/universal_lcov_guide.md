# Polyglot Verification Guide (Python, Rust, TypeScript/JS, Go, Java, C/C++, C#)

Proven (`proven`) natively parses and verifies **7 programming language families** in (`src/ast_graph.rs` and `src/mutation_gate.rs`) without requiring external coverage servers or language-specific daemons.

---

## Supported Languages & Test Runners

| Language | File Extensions | Native Test Runner Invoked by `verify-proof` |
| :--- | :--- | :--- |
| **Python** | `.py` | `python -B -m pytest <test_file> -q` |
| **Rust** | `.rs` | `cargo test --quiet` |
| **Go** | `.go` | `go test <pkg_dir>` |
| **JavaScript** | `.js`, `.mjs`, `.cjs` | `node <test_file>` (or `npx --no-install jest <test_file>` for `.test.js`/`.spec.js`) |
| **TypeScript / JSX** | `.ts`, `.tsx`, `.jsx` | `npx --no-install jest <test_file>` |
| **Java / C / C++ / C#** | `.java`, `.c`, `.cpp`, `.cc`, `.h`, `.hpp`, `.cs` | Custom runner args via `--pytest-args` or project test harness |

---

## Polyglot AST Parsing & Mutation
1. **Parallel AST Indexing**: `build_repo_graph_parallel` indexes Python (`def`, `async def`, `class` with multi-line parenthesis/bracket depth tracking) and brace-delimited languages (`fn`, `function`, `func`, TS/JS arrow functions `const f = (...) => {`, TS/JS class methods, `class`, `struct`, and typed Java/C++/C# methods) in parallel using `rayon`.
2. **Comment & String Masking**: Both single-line comments (`#`, `//`), block comments (`/* ... */`), and string/triple-quoted literals are masked prior to call extraction and operator mutation so comments or string literals are never falsely mutated.
3. **Polyglot Red Team Mutants**:
   - `Red1_BoundaryHunter` mutates `>=` <-> `<`, `<=` <-> `>`, `>` <-> `<=`, `<` <-> `>=`, `==` <-> `!=`, and `===` <-> `!==` across all languages.
   - `Red2_SideEffectNullifier` mutates augmented assignments (`+=` <-> `-=`) and boolean flags (`= True` <-> `= False`, `= true` <-> `= false`).
   - `Red3_ChaosPoisoner` mutates arithmetic operators (`+` <-> `-`, and `*` -> `//` in Python or `*` -> `/` in brace languages so `//` never creates an unintended comment).
