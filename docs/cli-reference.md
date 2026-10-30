# Proven CLI Reference (`proven` v1.0.0)

`proven` provides AST/call-graph indexing, scoped context extraction, AST mutation verification, and failure-rule consolidation through a single CLI binary.

> **Note**: Every subcommand can be called directly (`proven <subcommand>`) or under the `agent` namespace (`proven agent <subcommand>`).

---

## 1. `proven scan`
Scans a repository's git diff (`--base` to `--head`), working tree modifications, and untracked files to list untested functions.

```bash
proven scan --path . --base main --head HEAD [--fan-out] [--json]
```
- `--path` / `--repo`: Path to repository root (default: `.`).
- `--base`: Base git ref (default: `main`).
- `--head`: Head git ref (default: `HEAD`).
- `--fan-out`: Generate `fan_out_tasks` for parallel subagent execution.
- `--json`: Format output as JSON.

---

## 2. `proven refresh`
Re-indexes the repository AST graph and refreshes the verification status map.

```bash
proven refresh --path . [--base main] [--head HEAD] [--json]
```

---

## 3. `proven context`
Extracts scoped AST source lines, 1-hop callers/callees, 3-hop Personalized PageRank call-graph neighbors, and `<80`-token failure rules for a target symbol or file.

```bash
proven context --target "FUNC:calculate_order_total" [--file "src/pricing.py"] --path . [--json]
```

---

## 4. `proven verify-proof`
Runs the target test suite, verifies target symbol invocation, and optionally runs AST mutation checks (`--mutation-check`).

```bash
proven verify-proof \
  --target "FUNC:calculate_order_total" \
  --test "tests/test_pricing.py" \
  [--file "src/pricing.py"] \
  --path . \
  [--mutation-check] \
  [--max-mutants 3] \
  [--pytest-args "-k test_vip"] \
  [--json]
```

---

## 5. `proven arena`
Runs a mutation-verification round and logs any surviving mutants to `.proven/memory.json` with diagnostic repair hints.

```bash
proven arena --target "FUNC:calculate_order_total" --test "tests/test_pricing.py" --round 1 [--json]
```

---

## 6. `proven explain-attention`
Computes multi-hop Personalized PageRank scores over the AST call graph.

```bash
proven explain-attention --target "FUNC:calculate_order_total" [--hops 3] [--top-k 5] [--visualize] [--json]
```

---

## 7. `proven impact`
Estimates structural blast radius (`HIGH` / `MEDIUM` / `LOW`), direct callers, and co-evolving files for a target symbol.

```bash
proven impact --target "FUNC:calculate_order_total" --path . [--json]
```

---

## 8. `proven triage`
Ranks candidate files and reviewers for a bug description using AST symbol matching and `git log` commit history.

```bash
proven triage --query "discount calculation off by one" [--max-devs 3] [--max-files 5] --path . [--json]
```

---

## 9. `proven sleep` & `proven memory`
Manages failure-lesson records in `.proven/memory.json` and cross-repository rules in `.proven/meta_axioms.json`.

```bash
# Consolidate recorded failures into repository axioms and cross-repo rules
proven sleep --path . [--threshold 10] [--force] [--meta] [--json]

# List recorded lessons
proven memory list [--target "FUNC:calculate_order_total"] --path . [--json]

# Manually record a failure lesson
proven memory pin --target "FUNC:calculate_order_total" --lesson "Assert exact boundary equality" --failure-mode "DIV_ZERO_OR_BOUNDARY" --path .

# Promote & list cross-repository rules
proven memory meta-promote --path . [--json]
proven memory meta-list --path . [--json]
```

---

## 10. `proven benchmark`
Runs the built-in local self-benchmark or manages the local benchmark arena.

```bash
proven benchmark run [--regime all] [--report docs/BENCHMARK_REPORT.md] [--json]
proven benchmark init --dir .proven/benchmark_arena [--json]
proven benchmark grade --dir .proven/benchmark_arena [--report docs/ARENA_SCORE.md] [--json]
proven benchmark report --output docs/BENCHMARK_REPORT.md [--json]
```
