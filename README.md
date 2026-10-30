# Proven

Stop coding agents from writing passing tests around broken code — spec-first AST context, call-graph analysis, and mutation testing CLI.

<p>
  <img src="https://img.shields.io/badge/version-1.0.0-blue.svg" alt="Version 1.0.0"/>
  <img src="https://img.shields.io/badge/license-MIT-green.svg" alt="MIT License"/>
  <img src="https://img.shields.io/badge/languages-Python%20%7C%20Rust%20%7C%20Go%20%7C%20JS%20%7C%20TS%20%7C%20C%20%7C%20C%2B%2B%20%7C%20Java%20%7C%20C%23-0a7ea4.svg" alt="Supported Languages"/>
</p>

---

> **Tip:** State the intended business rule in your prompt or `docs/` when testing new code — no tool can catch wrong logic if the code itself is the only spec.

## Quick Start

### 1. Install the Agent Skill via `npx`
Install `proven` directly into Claude Code, Cursor, Antigravity, Codex, or Windsurf:
```bash
npx skills add minhquang0407/proven
# Or install globally across all projects:
npx skills add minhquang0407/proven -g
```
*(On first execution, the skill wrapper automatically downloads the prebuilt `proven` release binary for Windows, Linux, or macOS, or builds from source via `cargo` if needed.)*

### 2. Install the Standalone CLI (Optional)
Install directly from GitHub via `cargo`:
```bash
cargo install --git https://github.com/minhquang0407/proven
```
Or build from a local clone:
```bash
cargo build --release
```

### 3. Basic CLI Usage
```bash
# 1. Scan git diff and working tree for untested functions and undocumented env keys
proven scan --path . --base main --head HEAD --fan-out --json

# 2. Phase A: Extract external specs, sibling guard omissions, and domain invariants without source_code
proven context --target "FUNC:calculate_order_total" --file "src/pricing.py" --blind --path . --json

# 3. Phase B: Extract target AST, call-graph neighbors, slice/concurrency hints, and unverified literals
proven context --target "FUNC:calculate_order_total" --file "src/pricing.py" --path . --json

# 4. Run tests (direct or 1-3 hop slice) and AST mutation verification
proven verify-proof --target "FUNC:calculate_order_total" --test "tests/test_pricing.py" --mutation-check --json

# 5. Consolidate survived-mutant history into reusable rules
proven sleep --force --meta --path .
```

---

## Overview

Without tooling, coding agents read entire files (wasting **~71%** of context) and write passing tests that miss boundary bugs or lock in broken code as "expected".

`proven` pairs a CLI binary with an agent skill (`skills/proven/`) across **9 languages**:

- **Blind Spec Discovery (`proven context --blind`)** — Extracts requirements from docs, docstrings, `git log`, and sibling guards *before* showing implementation code.
- **Scoped AST & Risk Hints (`proven context`)** — Returns only the target AST, 3-hop call-graph neighbors, and SQL / JSON / concurrency / `.env` risk hints.
- **Live AST Mutation Gate (`proven verify-proof --mutation-check`)** — Mutates operators on disk, runs your test suite (unit or 1–3 hop slice), and auto-restores files via RAII.
- **Failure Memory (`proven sleep`, `proven memory`)** — Distills recurring surviving-mutant patterns into `<80`-token rules reused across repositories.

---

## Verification Workflow

<img width="1024" height="1024" alt="ai-drawing-880433e6-cd5f-4083-8b3d-30f6364ade8d" src="https://github.com/user-attachments/assets/bda0a0ff-c7e5-4c1f-aa99-0132f299a237" />

---

## Benchmark Results (120 OSS + 36 Fault-Seeded Repositories, 9 Languages)

Evaluated across **120 open-source repositories (`831` AST mutants, `N ≥ 10` per language)** and **36 fault-seeded multi-file repositories (`308` AST mutants)**. Every baseline run iterated until `100%` of native tests compiled and passed (`exit code 0`).

### Summary of Findings

- **`Flash + Proven` beats `Pro Vanilla` (`120` OSS repos)** — **`99.40%`** vs. `93.14%` mutant kill while reading **`71.2%` fewer source lines** (`26W–0L–94T`).
- **Tooling > Model Scaling (`84`-repo `2×2` matrix)** — Adding `proven` to `Flash` delivers **`2.08×` the kill gain** (`+14.68%` vs. `+7.05%`) of upgrading `Flash` to `Pro`.
- **Full Convergence on `Pro + Proven`** — **`97.83%`** Turn-1 kill before feedback, reaching **`100.00%` (`831/831`)** after the mutation gate.
- **CLI Gate beats Prompt-Only Skills (`36` clean repos)** — Markdown TDD skills plateau at **`87.71%`** kill on `Flash`; `proven` reaches **`98.98%`** with `63%–73%` fewer lines read.
- **Zero False-Green Bug Legalization (`36` buggy repos)** — `Flash Vanilla` legalized **`44.4%`** of spec-mismatch bugs; `Flash + Proven` caught **`36/36` (`100%`)** real bugs (`0%` False-Green) and **`66.7%`** of missing `.env` keys.

> **Benchmark Documentation & Raw Data**:
> - Full Per-Repository Results & Statistical Tests: [`docs/EMPIRICAL_BENCHMARK_RESULTS.md`](docs/EMPIRICAL_BENCHMARK_RESULTS.md)
> - Benchmark Specification & Protocol: [`docs/empirical-benchmark-specification.md`](docs/empirical-benchmark-specification.md)
> - Raw Paired-Mutant Datasets: [`docs/ab_blind_agent_120_max10.json`](docs/ab_blind_agent_120_max10.json), [`docs/ab_2x2_matrix_84_max10.json`](docs/ab_2x2_matrix_84_max10.json), [`docs/ab_multiskill_flash_36_max10.json`](docs/ab_multiskill_flash_36_max10.json), [`docs/ab_fault_seeded_flash_36.json`](docs/ab_fault_seeded_flash_36.json), [`docs/empirical-benchmark-data.json`](docs/empirical-benchmark-data.json)

### 1. 84-Repository `2×2` Cross-Model Matrix (`681` AST Mutants)

*Note: `TITANIUM` denotes a repository target where `100%` of non-equivalent AST mutants were killed (`0` survived).*

| Model \ Configuration | `Vanilla` (`100%` Native Pass) | `+ Proven` (Turn 1, Pre-Feedback) | `+ Proven` (Final, Post-Gate) | `Proven` Delta (`Final − Vanilla`) |
| :--- | :---: | :---: | :---: | :---: |
| **`Gemini Flash`** | `576 / 681` (**`84.58%`**)<br/>`43 / 84` (`51.19%` Titanium) | `557 / 681` (`81.79%` raw;<br/>`>92%` on compile-first Waves 2–3) | **`676 / 681` (`99.27%`)**<br/>**`80 / 84` (`95.24%` Titanium)** | **`+14.68%` Kill** ($p = 5.39 \times 10^{-28}$)<br/>**`+44.05%` Titanium** (`39W–0L–45T`) |
| **`Gemini Pro`** | `624 / 681` (**`91.63%`**)<br/>`56 / 84` (`66.67%` Titanium) | `663 / 681` (**`97.36%`**)<br/>`68 / 84` (`80.95%` Titanium) | **`681 / 681` (`100.00%`)**<br/>**`84 / 84` (`100.00%` Titanium)** | **`+8.37%` Kill** ($p = 2.78 \times 10^{-17}$)<br/>**`+33.33%` Titanium** (`28W–0L–56T`) |
| **Model Delta (`Pro − Flash`)** | `+7.05%` Kill / `+15.48%` Tit<br/>($p = 2.92 \times 10^{-12}$) | — | `+0.73%` Kill / `+4.76%` Tit | **`2.08×` Kill & `2.85×` Titanium vs. Model Scaling** |

### 2. 120-Repository 6-Regime Breakdown (`831` AST Mutants)

| Regime | Cohort Description | Repos (`Mutants`) | `Gemini Pro Vanilla` | `Gemini Flash + Proven` | `Gemini Pro + Proven` (`T1` → `Final`) | Delta (`Pro+Proven` vs `Pro Vanilla`) |
| :--- | :--- | :---: | :---: | :---: | :---: | :--- |
| **Regime 1** | Small & clean libraries (`werkzeug`, `marshmallow`, `pyjwt`, `packaging`, `dateutil`, `tabwriter`, `ipaddr.js`, …) | `10` (`58`) | `79.31%` (`46/58`)<br/>`7/10` Titanium | **`98.28%` (`57/58`)**<br/>`9/10` Titanium | `94.83%` → **`100.00%`**<br/>`10/10` Titanium | `+20.69%` Kill (`+18.97%` Flash+Proven) |
| **Regime 2** | Large monorepos (`ruff`, `pydantic`, `flask`, `fastapi`, `gin`, `cobra`, `zstd`, `pugixml`, `lodash`, `zod`, `guava`, `cecil`, …) | `18` (`99`) | `84.85%` (`84/99`)<br/>`13/18` Titanium | **`97.98%` (`97/99`)**<br/>`17/18` Titanium | `96.97%` → **`100.00%`**<br/>`18/18` Titanium | `+15.15%` Kill; `99.4%–99.99%` CCR |
| **Regime 3** | Legacy / zero-test repos & corrupted `>1 MB` files | `16` (`108`) | `95.37%` (`103/108`)<br/>`12/16` Titanium | **`100.00%` (`108/108`)**<br/>`16/16` Titanium | `99.07%` → **`100.00%`**<br/>`16/16` Titanium | `+4.63%` Kill; `0` parser crashes |
| **Regime 4** | Cross-repo & cross-language rule transfer (`8` domain clusters across 9 languages) | `24` (`150`) | `96.67%` (`145/150`)<br/>`19/24` Titanium | **`98.67%` (`148/150`)**<br/>`22/24` Titanium | `98.00%` → **`100.00%`**<br/>`24/24` Titanium | `98.00%` Turn-1 kill via `<80`-tok rules |
| **Regime 5** | Multi-branch algorithmic & stateful code (`urllib3`, `tenacity`, `xxhash`, `strsim-rs`, `http-parser`, `QR-Code-generator`, `jBCrypt`, …) | `28` (`231`) | `96.54%` (`223/231`)<br/>`24/28` Titanium | **`100.00%` (`231/231`)**<br/>`28/28` Titanium | `98.27%` → **`100.00%`**<br/>`28/28` Titanium | `+3.46%` Kill (`28/28` full-kill repos) |
| **Regime 6** | Systems & enterprise libraries (`cJSON`, `tinyxml2`, `hashmap.c`, `commons-lang`, `commons-codec`, `Newtonsoft.Json`, `TinyIoC`, …) | `24` (`185`) | `93.51%` (`173/185`)<br/>`17/24` Titanium | **`100.00%` (`185/185`)**<br/>`24/24` Titanium | `97.84%` → **`100.00%`**<br/>`24/24` Titanium | `+6.49%` Kill (`+29.17%` Titanium) |
| **Total** | **All 120 Repositories (`N ≥ 10` per language)** | **`120` (`831`)** | **`93.14%` (`774/831`)**<br/>`92/120` (`76.67%`) Tit<br/>`51,632` lines | **`99.40%` (`826/831`)**<br/>`116/120` (`96.67%`) Tit<br/>`14,851` lines (`-71.2%`) | **`97.83%`** → **`100.00%`**<br/>`120/120` (`100.0%`) Tit<br/>`15,278` lines (`-70.4%`) | **Flash+Proven `+6.26%`** ($p = 6.11 \times 10^{-15}$)<br/>**Pro+Proven `+6.86%`** ($p = 2.78 \times 10^{-17}$) |

### 3. Breakdown by Language (`N ≥ 10` Repositories per Language)

| Language | Test Runner / Compiler | Repos ($N$) | Mutants ($M$) | `Gemini Pro Vanilla` | `Gemini Flash + Proven` | `Gemini Pro + Proven` (`T1` → `Final`) | `2×2` Subset: `Flash Vanilla` → `Flash + Proven` |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **Python** | `pytest` | **40** | **205** | `88.29%` (`181/205`, `29/40`) | **`99.02%`** (`203/205`, `38/40`) | `97.56%` → **`100.00%`** (`40/40`) | $N=13$: `67.07%` → **`97.56%`** |
| **C** | `zig cc` / native binary | **10** | **86** | `87.21%` (`75/86`, `7/10`) | **`100.00%`** (`86/86`, `10/10`) | `97.67%` → **`100.00%`** (`10/10`) | $N=10$: `72.09%` → **`100.00%`** |
| **C++** | `zig c++` / native binary | **10** | **84** | `97.62%` (`82/84`, `8/10`) | **`100.00%`** (`84/84`, `10/10`) | `97.62%` → **`100.00%`** (`10/10`) | $N=10$: `96.43%` → **`100.00%`** |
| **TypeScript** | `tsx` / `node --test` | **10** | **82** | `95.12%` (`78/82`, `7/10`) | **`100.00%`** (`82/82`, `10/10`) | `97.56%` → **`100.00%`** (`10/10`) | $N=10$: `82.93%` → **`100.00%`** |
| **Rust** | `cargo test` / `rustc --test` | **10** | **81** | `96.30%` (`78/81`, `9/10`) | **`100.00%`** (`81/81`, `10/10`) | `97.53%` → **`100.00%`** (`10/10`) | $N=7$: `91.43%` → **`100.00%`** |
| **C#** | `csc` / `.NET` runner | **10** | **77** | `97.40%` (`75/77`, `8/10`) | **`100.00%`** (`77/77`, `10/10`) | `98.70%` → **`100.00%`** (`10/10`) | $N=10$: `92.21%` → **`100.00%`** |
| **Java** | `javac` / `java` | **10** | **74** | `94.59%` (`70/74`, `8/10`) | **`98.65%`** (`73/74`, `9/10`) | `98.65%` → **`100.00%`** (`10/10`) | $N=10$: `93.24%` → **`98.65%`** |
| **JavaScript** | `node` / `vitest` / `jest` | **10** | **74** | `97.30%` (`72/74`, `8/10`) | **`100.00%`** (`74/74`, `10/10`) | `97.30%` → **`100.00%`** (`10/10`) | $N=7$: `87.10%` → **`100.00%`** |
| **Go** | `go test` | **10** | **68** | `92.65%` (`63/68`, `8/10`) | **`97.06%`** (`66/68`, `9/10`) | `98.53%` → **`100.00%`** (`10/10`) | $N=7$: `81.25%` → **`96.88%`** |
| **Total** | **All 9 Languages** | **120** | **831** | **`93.14%` (`774/831`, `92/120`)** | **`99.40%` (`826/831`, `116/120`)** | **`97.83%` → `100.00%` (`120/120`)** | **$N=84$: `84.58%` → `99.27%`** |

### 4. Comparison Against Prompt-Only Testing Skills (`36` Repositories, `9` Languages, `Gemini Flash`)

6-arm comparison on `Gemini Flash` across **36 symmetric repositories (`4` × `9` languages, `293` paired AST mutants, `36/36` native pass in all arms)**:

| Configuration (`Gemini Flash`) | Skill Prompt Tokens | Source Lines Read (`36` Repos) | Mutants Killed (`293`) | Full-Kill Repos (`36`) | vs. `Flash Vanilla` (Exact McNemar) | `Flash + Proven` vs. Arm (Exact McNemar / Repo W–L–T) |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **`Flash Vanilla`** *(no skill)* | `0` | `23,888` | `243 / 293` (`82.94%`) | `17 / 36` (`47.2%`) | — | **`+16.04%`** (`+49 / -2`, $p = 1.18 \times 10^{-12}$, **`19W–0L–17T`**) |
| **`+ addyosmani/agent-skills`** *(`test-driven-development` + `debugging-and-error-recovery`, `75.8K` installs)* | `~6,892` | `17,084` | `243 / 293` (`82.94%`) | `20 / 36` (`55.6%`) | `+0.00%` (`+8 / -8`, $p = 1.00$) | **`+16.04%`** (`+48 / -1`, $p = 1.78 \times 10^{-13}$, **`16W–0L–20T`**) |
| **`+ riekelt/principal-engineer` + `dotnet/skills`** *(`writing-unit-tests` + `testing-changes` + `assertion-quality`, `42.1K` installs)* | `~7,111` | `17,554` | `249 / 293` (`84.98%`) | `20 / 36` (`55.6%`) | `+2.05%` (`+13 / -7`, $p = 0.263$) | **`+13.99%`** (`+42 / -1`, $p = 1.00 \times 10^{-11}$, **`15W–0L–21T`**) |
| **`+ mattpocock/skills@tdd`** *(`SKILL.md` + `mocking.md` + `tests.md`, `1.2M` installs)* | `~1,831` | `12,919` | `253 / 293` (`86.35%`) | `22 / 36` (`61.1%`) | `+3.41%` (`+13 / -3`, $p = 0.0213$) | **`+12.63%`** (`+39 / -2`, $p = 7.84 \times 10^{-10}$, **`14W–0L–22T`**) |
| **`+ obra/superpowers`** *(`test-driven-development` + `writing-good-tests` + `verification-before-completion` + `systematic-debugging`, `313K` installs)* | `~7,755` | `13,249` | `257 / 293` (`87.71%`) | `22 / 36` (`61.1%`) | `+4.78%` (`+17 / -3`, $p = 0.00258$) | **`+11.26%`** (`+34 / -1`, $p = 2.10 \times 10^{-9}$, **`13W–0L–23T`**) |
| **`+ proven`** *(`minhquang0407/proven`, CLI + Skill)* | **`~1,417`** | **`4,791`** | **`290 / 293` (`98.98%`)** | **`34 / 36` (`94.4%`)** | **`+16.04%`** (`+49 / -2`, $p = 1.18 \times 10^{-12}$) | — |

*Markdown TDD skills plateau near `87.7%` because prompt instructions cannot execute runtime AST mutations to expose surviving branches; `proven` can also be installed alongside `obra/superpowers` or `mattpocock/skills@tdd`.*

### 5. Fault-Seeded Real-Bug & False-Green Benchmark (`36` Multi-File Repositories, `9` Languages, `Gemini Flash`, `v1.0.0`)

Evaluated across **36 fault-seeded multi-file repositories (`4` defect regimes × `9` languages, `14–17` files / `650–1,278` LOC each, `308` ground-truth AST mutants)** under natural prompts (*"Write automated tests for `<target_func>`"*):

| Configuration (`Gemini Flash`) | Skill Prompt Tokens | Metric 1: `Real Bug Catch Rate` (`Fail-to-Pass`) | Metric 2: `False-Green Legalization` *(Pre-Fix Mutant Kill)* | Metric 3: `Post-Fix AST Mutant Kill` (`308` Mutants) | Unprompted `.env` Drift (`Regime C`) | Source Lines Read (`36` Repos) |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **`Flash Vanilla`** *(no skill)* | `0` | `32 / 36` (`88.89%`) | `4 / 36` (`11.11%`; `44.4%` in `A_ORACLE`)<br/>*(`36/36 = 100.0%` pre-fix kill)* | `263 / 308` (`85.39%` uncond. / `96.69%` cond., `28/36` full-kill) | `1 / 9` (`11.1%`) | `4,580` (`127.2`/repo) |
| **`+ obra/superpowers`** | `~7,755` | **`36 / 36` (`100.00%`)** | **`0 / 36` (`0.00%`)** | **`297 / 308` (`96.43%`**, `+11.04%`, $p = 0.0290$, `30/36` full-kill) | `2 / 9` (`22.2%`) | `4,646` (`129.1`/repo) |
| **`+ proven v1.0.0`** | **`~1,417`** | **`36 / 36` (`100.00%`)** | **`0 / 36` (`0.00%`)** | **`297 / 308` (`96.43%`**, `+11.04%`, $p = 0.0296$, **`31/36` full-kill**) | **`6 / 9` (`66.7%`)** | **`3,706` (`102.9`/repo)** |

#### Key Empirical Takeaways

1. **`100%` mutant kill ≠ bug-free** — `Flash Vanilla`'s 4 bug-legalizing suites still killed **`36/36` (`100%`)** pre-fix mutants, proving `--blind` spec discovery must precede mutation checks.
2. **False-Green is a multi-file context gap** — `Flash Vanilla` had `0%` False-Green on `~49`-LOC micro-repos, jumping to **`44.4%`** on multi-file repos when specs lived in `docs/`.
3. **Spec-First + Mutation Gate work in two stages** — Spec-First catches pre-existing bugs (`36/36` Fail-to-Pass); once code passes, the Mutation Gate lifts kill rate from `87.71%` to **`98.98%`**.
4. **Static AST hints beat prompt reminders** — `proven context` caught **`3×–6×`** more undocumented `.env.example` keys and enforced deterministic `Barrier` race tests under CPU load.
5. **Small model + AST CLI beats big model solo** — `Gemini Flash + Proven` uses **`5.47×` fewer** skill tokens and **`19%–71%` fewer** source lines while outperforming unassisted `Gemini Pro`.

---

## CLI Subcommands (`proven --help`)

Every subcommand can be called directly (`proven <subcommand>`) or under `agent` (`proven agent <subcommand>`):

| Subcommand | Description |
| :--- | :--- |
| `proven scan` | Scan git diff and working tree for untested functions and `.env.example` config drift (`--fan-out`, `--json`) |
| `proven refresh` | Re-index repository AST and refresh verification status |
| `proven context` | Extract external specs (`--blind`, `--spec`), sibling guard omissions, function AST, 3-hop PageRank context, slice/concurrency/env-audit hints, and `<80`-token rules |
| `proven verify-proof` | Run test suite (direct or 1–3 hop transitive slice) and execute AST mutation testing (`--mutation-check`) |
| `proven arena` | Run a single mutation-feedback round and log surviving mutants |
| `proven explain-attention` | Inspect multi-hop Personalized PageRank call-graph scores (`--hops`, `--top-k`, `--visualize`) |
| `proven impact` | Estimate change blast radius (`HIGH` / `MEDIUM` / `LOW`), callers, and co-modified files |
| `proven triage` | Rank candidate files and reviewers from AST symbols and `git log` history |
| `proven sleep` | Consolidate `.proven/memory.json` entries into `.proven/axioms.md` and `.proven/meta_axioms.json` |
| `proven memory` | Manage stored failure lessons (`list`, `pin`, `consolidate`, `meta-promote`, `meta-list`) |
| `proven benchmark` | Run built-in local self-benchmark (`run`, `init`, `grade`, `report`) |

---

## Project Layout

```text
├── Cargo.toml                 # Crate manifest (proven library + proven CLI)
├── docs/
│   ├── EMPIRICAL_BENCHMARK_RESULTS.md        # 120-repo, 84-repo 2×2, 36-repo multi-skill & 36-repo fault-seeded report
│   ├── empirical-benchmark-specification.md  # Multi-regime benchmark specification
│   ├── ab_blind_agent_120_max10.json         # Paired mutant results across 120 repositories
│   ├── ab_2x2_matrix_84_max10.json           # Paired mutant results for 84-repo 2×2 matrix
│   ├── ab_multiskill_flash_36_max10.json     # 6-arm multi-skill comparison across 36 repos (Gemini Flash)
│   ├── ab_fault_seeded_flash_36.json    # 3-arm fault-seeded real-bug & false-green dataset across 36 repos (v1.0.0)
│   └── empirical-benchmark-data.json         # Aggregated benchmark summary data
├── src/
│   ├── lib.rs                 # Library exports
│   ├── main.rs                # CLI entry point
│   ├── models.rs              # JSON schemas
│   ├── oracle.rs              # External spec discovery, sibling guard checks, slice/concurrency hints & env audit
│   ├── ast_graph.rs           # 9-language AST, import, and call-graph indexer
│   ├── graph_rag.rs           # Personalized PageRank call-graph scoring & git ownership triage
│   ├── mutation_gate.rs       # AST mutator, workspace lockfile, and RAII file restoration
│   ├── memory_cls.rs          # Failure memory and cross-repository rule consolidation
│   ├── arena.rs               # Iterative mutation-feedback referee
│   └── benchmark.rs           # Built-in local self-benchmark
├── tests/
│   └── integration_test.rs    # End-to-end integration, concurrency, path-safety, and polyglot tests
└── skills/proven/             # Coding agent skill definitions and thin CLI wrapper scripts
```

---

## Development & Tests

```bash
cargo clippy --all-targets -- -D warnings
cargo test
```
