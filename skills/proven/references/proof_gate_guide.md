# Runtime Execution & AST Mutation Gate Guide

## Why Line Coverage Is Not Enough
Traditional coverage tools only check whether a line of code was *executed* during a test run. An agent can write `def test_foo(): foo(1); assert True` to achieve 100% line coverage while catching 0% of actual bugs.

Proven enforces a 2-Stage Verification Gate (`src/mutation_gate.rs`):

---

## Stage 1: Execution & Static Invocation Proof
When you run:
```bash
proven verify-proof --target "FUNC:calculate_total" --test "tests/test_billing.py" --json
```
Proven verifies:
1. **Path Security**: Both `--test` and `--file` must reside inside the repository root (symlink escape and `..` path traversal are strictly rejected) and must not start with `-` (argument injection prevention).
2. **Test Suite Execution**: Runs the language-appropriate test runner (`python -B -m pytest`, `cargo test --quiet`, `go test`, `node` / `npx --no-install jest`) without a shell (`Command::new`) so shell metacharacters cannot execute arbitrary commands.
3. **Body Invocation Proof**: Parses the test file (stripping comments, docstrings, and `import`/`from`/`use` statements) to verify the target symbol (or a function defined in a `FILE:` / `CLASS:` target) is actually invoked inside the test body.

---

## Stage 2: AST Mutation Gate (`--mutation-check`)
When `--mutation-check` is enabled:
```bash
proven verify-proof --target "FUNC:calculate_total" --test "tests/test_billing.py" --mutation-check --max-mutants 6 --json
```
Proven's AST mutator generates up to `--max-mutants` (clamped to `>= 1`) targeted mutants sampled evenly across the target function body (plus secondary operators on compound conditions when budget allows) across three operator groups:
- **`Red1_BoundaryHunter`**: Flips boundary and comparison operators (`>=` <-> `<`, `<=` <-> `>`, `>` <-> `<=`, `<` <-> `>=`, `==` <-> `!=`, `===` <-> `!==`), arithmetic operators (`+` <-> `-`, `*` <-> `//` in Python, `*` <-> `/` in brace languages), and bitwise shifts (`<<` <-> `>>`, `>>> 0` -> `>>> 1`).
- **`Red2_SideEffectNullifier`**: Mutates augmented assignments (`+=` <-> `-=`, `*=` <-> `/=`, `//=` -> `*=`, `|=` <-> `&=`, `<<=` <-> `>>=`), boolean assignments/literals (`= True` <-> `= False`, `= true` <-> `= false`), and scalar sentinel returns (`return 0` -> `return 1`, `return -1` -> `return 1`).
- **`Red3_ChaosPoisoner`**: Mutates logical connectors (`and` <-> `or`, `&&` <-> `||`), membership and null guards (`in` <-> `not in`, `is None` <-> `is not None`), and negated branch guards (`if not ` -> `if `, `if (!` -> `if (`).

### Crash-Safe & Thread-Safe Atomic Restoration (`FileRestoreGuard` + `RepoMutationLockGuard`)
Before mutating a source file on disk, `RepoMutationLockGuard` serializes concurrent mutation checks across threads and processes (`.proven/.mutation.lock`), and `FileRestoreGuard` captures the exact original bytes in memory and guarantees restoration in its `Drop` implementation—even if the test runner times out or panics. If the source file is read-only and cannot be written, the mutant is safely skipped rather than falsely counted as survived.

---

## Understanding Proof Grades
- **`TITANIUM`** (`proof_status: "pass"`): Tests pass, target is invoked in the test body, and **100% of tested AST mutants are killed** (`mutation_score == 1.0`, or no mutable operators exist in the target).
- **`SILVER`** (`proof_status: "pass"`): Tests pass and target is invoked in the test body, but one or more mutants survived (`mutation_score < 1.0`). Survived mutants are automatically diagnosed (`causal_reflexion`) and recorded in `.proven/memory.json`.
- **`NONE`**: Either `--mutation-check` was omitted (`proof_status: "pass"` for execution + invocation only), or verification failed (`proof_status: "fail"` when tests failed, target was never called in the test body, target does not exist in the source file, or paths were invalid).
