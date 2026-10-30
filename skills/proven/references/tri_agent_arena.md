# Tri-Agent Adversarial Arena Guide

The **Tri-Agent Adversarial Arena** (`src/arena.rs`, invoked via `proven arena` or `run_arena.py`) pits a **Blue Team Agent** (Test Author) against a **Red Team Agent** (3-Persona AST Mutator) under the supervision of a deterministic **Referee**.

---

## How the Arena Loop Works

### Step 1: Blue Team Writes Initial Test
The Blue Team extracts scoped context via `proven context` and writes a test file (e.g., `tests/test_pricing.py`) targeting `FUNC:calculate_order_total`.

### Step 2: Referee Runs Round 1 (`proven arena`)
```bash
proven arena --target "FUNC:calculate_order_total" --test "tests/test_pricing.py" --round 1 --json
```
Under the hood, the Referee executes:
1. Standard test runner verification (`pytest` / `cargo test` / `go test` / `node` / `jest`).
2. Static test-body invocation check.
3. The 3-Persona Red Team AST Mutation Gate (`Red1_BoundaryHunter`, `Red2_SideEffectNullifier`, `Red3_ChaosPoisoner`).

### Step 3: Outcome & Automatic Memory Pinning
- **`TITANIUM_IMMUNITY`**: All tests pass and 100% of Red Team AST mutants are killed (`mutation_score == 1.0`). The loop terminates in 1 round!
- **`ROUND_FAILED`** (or **`MAX_ROUNDS_EXCEEDED`** when `round >= 3`): One or more Red Team mutants survived (`mutation_score < 1.0`), or the test suite failed.
  - Every survived mutant is automatically diagnosed by `CausalReflexionEngine` (`UNCHECKED_SIDE_EFFECT`, `MISSING_EXCEPTION_CONTRACT`, or `WEAK_ASSERTION`) and pinned into `.proven/memory.json` (Hippocampus Tier 1).
  - `proven arena` generates a ready-to-use `reflexion_prompt` and `causal_diagnoses` listing the exact survived mutants so the Blue Team can immediately add the missing boundary, side-effect, or exception assertions in Round 2.
