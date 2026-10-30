# Proven: 120-Repository Polyglot Blind A/B, 84-Repository 2×2 Cross-Model, 36-Repository Multi-Skill, & 36-Repository Fault-Seeded Real-Bug Benchmark Results

This document reports the comprehensive empirical evaluation of **Proven** (`target/release/proven.exe`, AST Context Extraction, Call-Graph Personalized PageRank, External Spec & Sibling-Guard Discovery, Complementary Learning Systems [CLS] Cross-Repository Memory Consolidation, and 3-Persona Red-Team Mutation Verification Engine) across **120 real-world open-source repositories (`831` non-equivalent AST mutants at `--max-mutants 10`)** spanning **all 9 supported languages with `N ≥ 10` independent repositories per language (`Python=40`, `Rust=10`, `Go=10`, `JavaScript=10`, `TypeScript=10`, `C=10`, `C++=10`, `Java=10`, `C#=10`)**, plus **36 fault-seeded multi-file polyglot repositories (`308` ground-truth AST mutants across `4` defect regimes × `9` languages)**.

To eliminate experimenter bias, language-sample skew, and synthetic baseline inflation, the benchmark executes four experimental tracks (`588 / 588 = 100.0%` native test runner pass across Tracks 1–3 on clean OSS repositories, plus `216` isolated `Gemini Flash` evaluations across fault-seeded repositories in Track 4):

1. **Track 1 — Full 120-Repository 3-Agent Polyglot Benchmark (`Argon Pro Vanilla` vs. `Gemini Flash + Proven` vs. `Argon Pro + Proven`, `831` Non-Equivalent AST Mutants across All 9 Languages)**:
   - **Pro Control Group (`Argon / Gemini Pro Vanilla` — No `proven`)**: Operated in isolated workspaces without access to `proven` or mutation feedback, reading **`51,632` source lines** across 120 repositories and iterating against native test runners (`pytest`, `go test`, `cargo test`, `node`, `zig cc/c++`, `javac/java`, `Add-Type / csc`) until **`120 / 120` (`100.0%`)** test suites passed (`exit code 0`), killing **`774 / 831` (`93.14%`)** mutants (**`92 / 120 = 76.67%` Titanium**).
   - **Lightweight Treatment Group (`Gemini Flash + Proven` — `Model = "flash"`)**: Operated across **all `120 / 120` repositories** using `proven context` (**`14,851` lines extracted — `-71.24%` context line reduction**) and `proven verify-proof --mutation-check --max-mutants 10`, killing **`826 / 831` (`99.40%`)** AST mutants (**`116 / 120 = 96.67%` Titanium**, including `100.00%` across `C`, `C#`, `C++`, `JavaScript`, `Rust`, and `TypeScript`) and strictly dominating `Argon Pro Vanilla` (**`26 Wins – 0 Losses – 94 Ties`**, exact McNemar $p = 6.11 \times 10^{-15}$).
   - **Pro Treatment Group (`Argon / Gemini Pro + Proven`)**: Extracted **`15,278` scoped context lines (`-70.41%` reduction)**, killing **`813 / 831` (`97.83%`, `104 / 120` Titanium) on Turn 1 (zero mutation feedback)** and **`831 / 831` (`100.00%`, `120 / 120` Titanium) post-reflexion** (**`28 Wins – 0 Losses – 92 Ties`**, exact McNemar $p = 2.78 \times 10^{-17}$).
2. **Track 2 — 84-Repository 2×2 Cross-Model Factorial Matrix (`Gemini Flash Vanilla` vs. `Gemini Flash + Proven` vs. `Argon Pro Vanilla` vs. `Argon Pro + Proven`, `681` AST Mutants)**:
   - Across **84 Repositories (`681` non-equivalent AST mutants, `84 × 4 = 336 / 336` native runner passes)** spanning all 6 Regimes and all 9 languages, `Proven` lifts `Gemini Flash` from **`84.58%` (`43/84` Titanium)** to **`99.27%` (`80/84` Titanium; `+14.68%` kill, `+44.05%` Titanium, `39W–0L–45T`, McNemar $p = 5.39 \times 10^{-28}$)** — delivering **`2.08×` the kill-rate gain and `2.85×` the Titanium certification gain of frontier model scaling alone** (`+7.05%` kill, `+15.48%` Titanium).
3. **Track 3 — 36-Repository 6-Arm Multi-Skill Comparison on `Gemini Flash` (`4` Repositories × `9` Languages = `36` Repositories, `293` Paired Non-Equivalent AST Mutants)**:
   - Evaluates `Gemini Flash` across `6` isolated configurations (`36 × 6 = 216 / 216` native runner passes): `Flash Vanilla` (no skill), `+ mattpocock/skills@tdd` (`1.2M` installs), `+ obra/superpowers` (`313K` installs), `+ addyosmani/agent-skills` (`75.8K` installs), `+ riekelt/principal-engineer + dotnet/skills` (`42.1K` installs), and `+ proven`.
   - Prompt-only Markdown testing skills lift `Gemini Flash` from **`82.94%` (`17/36` Titanium)** up to **`86.35%` (`mattpocock/skills@tdd`, `22/36` Titanium)** and **`87.71%` (`obra/superpowers`, `22/36` Titanium)**, whereas `Gemini Flash + Proven` reaches **`98.98%` (`290/293`, `34/36 = 94.4%` Titanium)** with **`0` repository losses** against every arm ($p \le 2.10 \times 10^{-9}$) while reading **`62.9%–72.7%` fewer source lines** (`4,791` vs. `12,919–17,554`).
4. **Track 4 — 36-Repository Fault-Seeded Real-Bug Discovery & False-Green Legalization Benchmark on `Gemini Flash` (`v1.0.0`, `4` Defect Regimes × `9` Languages = `36` Multi-File Repositories, `308` Ground-Truth AST Mutants, `216` Total Runs)**:
   - Evaluates `Flash Vanilla` (`0` skill tokens), `Flash + obra/superpowers` (`~7,755` skill tokens), and `Flash + Proven v1.0.0` (`~1,417` skill tokens) under natural developer prompts (*"Write automated tests for `<target_func>`"*) on `14–17`-file (`650–1,278` LOC, 5-commit Git history) fault-seeded repositories spanning **Oracle Spec Discrepancies (`A_ORACLE`)**, **Sibling-Guard Omissions (`B_OMISSION`)**, **Cross-Layer Slice / TOCTOU Concurrency / `.env.example` Drift (`C_SLICE_CONCURRENCY_ENV`)**, and **Deep Multi-Branch Compound-Operator Bugs (`D_DEEP_COMPOUND`)**.
   - `Flash Vanilla` catches **`32 / 36` (`88.89%`)** real bugs and falls into **`4 / 36` (`11.11%` overall; `4 / 9 = 44.44%` in `A_ORACLE`) False-Green Bug Legalization** — where its passing test suites assert the buggy implementation constants and simultaneously score **`36 / 36` (`100.0%`) pre-fix AST mutant kill on `buggy_target`** (`263 / 308 = 85.39%` unconditional post-fix mutant kill on `clean_target`).
   - `Flash + Proven v1.0.0` eliminates False-Green legalization (**`0 / 36 = 0.00%`**), catches **`36 / 36` (`100.00%`)** real bugs (`Fail-to-Pass`), reaches **`297 / 308` (`96.43%`, `+11.04%` over `Flash Vanilla`, $p = 0.0296$, `31/36` Titanium)** Turn-1 post-fix AST mutant kill prior to any mutation-feedback loop, detects **`6 / 9` (`66.7%`)** undocumented `.env.example` keys unprompted (`6.0×` `Flash Vanilla`, `3.0×` `obra/superpowers`), and reads **`19.1%–20.2%` fewer source lines** (`3,706` vs. `4,580–4,646`) at a **`5.47×` smaller skill prompt budget** than `obra/superpowers`.

All raw paired-mutant telemetry is permanently archived in [`docs/ab_blind_agent_120_max10.json`](ab_blind_agent_120_max10.json), [`docs/ab_2x2_matrix_84_max10.json`](ab_2x2_matrix_84_max10.json), [`docs/ab_multiskill_flash_36_max10.json`](ab_multiskill_flash_36_max10.json), [`docs/ab_fault_seeded_flash_36.json`](ab_fault_seeded_flash_36.json), and [`docs/empirical-benchmark-data.json`](empirical-benchmark-data.json).

---

## 1. Executive Summary Across All 6 Regimes (120 Repositories, 9 Languages, `831` AST Mutants)

| Regime / Track | Operational Focus | Repos & Mutants | Native Pass | Argon (Pro) Vanilla Baseline | Gemini Flash + Proven (Final) | Argon (Pro) + Proven (Turn-1 -> Final) | Measured Gain & Exact Statistical Significance |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :--- |
| **All 120 Blind A/B Targets** | Full 6-Regime Polyglot Benchmark (`N ≥ 10` in all 9/9 Languages) | **120 Repos (`831` Mutants)** | `100.0%` (`120/120`) | **`93.14%` (`774/831`)** \| **`76.67%` Tit (`92/120`)** \| `51,632` lines | **`99.40%` (`826/831`)** \| **`96.67%` Tit (`116/120`)** \| `14,851` lines | **`97.83%` (`813/831`)** -> **`100.00%` (`831/831`, `120/120` Tit)** | **Flash+Proven `+6.26%` over Pro Van** ($p = 6.11 \times 10^{-15}$, `26W–0L`); **Pro+Proven `+6.86%`** ($p = 2.78 \times 10^{-17}$, `28W–0L`) |
| **84-Repo 2×2 Factorial Matrix** | `Flash` vs. `Argon (Pro)` × `Vanilla` vs. `+ Proven` across All 9 Languages | **84 Repos (`681` Mutants)** | `100.0%` (`336/336`) | **Flash Van: `84.58%` (`576/681`, `43/84` Tit)** \| **Pro Van: `91.63%` (`624/681`, `56/84` Tit)** | **`99.27%` (`676/681`)** \| **`95.24%` Tit (`80/84`)** | **`97.36%` (`663/681`)** -> **`100.00%` (`681/681`, `84/84` Tit)** | **`Proven` on Flash `+14.68%` Kill / `+44.05%` Tit** ($p = 5.39 \times 10^{-28}$, `39W–0L`) = **`2.08×` Kill & `2.85×` Tit of Model Scaling** |
| **Regime 1** | Small & Clean Repositories (Assertion Strength & Boundary Mutants) | 10 (`58` Mutants) | `100.0%` (`10/10`) | **`79.31%` (`46/58`)** (`7/10` Titanium) | **`98.28%` (`57/58`)** (`9/10` Titanium) | **`94.83%` (`55/58`)** -> **`100.00%` (`58/58`, `10/10` Tit)** | **`+20.69%` Kill Rate** (`+18.97%` by `Flash+Proven`); **`+30.0%` Titanium Rate** |
| **Regime 2** | Large & Enterprise Monorepos (`ruff`, `gin`, `cobra`, `pugixml`, `cecil`, `lodash`, `zod`, `guava`, `zstd`) | 18 (`99` Mutants) | `100.0%` (`18/18`) | **`84.85%` (`84/99`)** (`13/18` Titanium) | **`97.98%` (`97/99`)** (`17/18` Titanium) | **`96.97%` (`96/99`)** -> **`100.00%` (`99/99`, `18/18` Tit)** | **`+15.15%` Kill Rate** (`+13.13%` by `Flash+Proven`); **`99.4%–99.99%` CCR** |
| **Regime 3** | Dirty, Zero-Test & Legacy Repositories (Adversarial Noise & 1.5MB Corrupted Artifacts) | 16 (`108` Mutants) | `100.0%` (`16/16`) | **`95.37%` (`103/108`)** (`12/16` Titanium) | **`100.00%` (`108/108`)** (`16/16` Titanium) | **`99.07%` (`107/108`)** -> **`100.00%` (`108/108`, `16/16` Tit)** | **`100%` (`16/16` Titanium) by both `Flash+Proven` & `Argon+Proven`**; **`0` crashes** |
| **Regime 4 + 4-Ext** | Cross-Repository & Cross-Language CLS Memory Transfer (8 Domain Clusters: `Source A -> Target B1, B2`) | 24 (`150` Mutants across 9 Langs) | `100.0%` (`24/24`) | **`96.67%` (`145/150`)** (`19/24` Titanium) | **`98.67%` (`148/150`)** (`22/24` Titanium) | **`98.00%` (`147/150`)** -> **`100.00%` (`150/150`, `24/24` Tit)** | **`98.00%` Turn-1 Zero-Shot Kill** via `<80`-token CLS Meta-Axioms (`+20.8%` Titanium) |
| **Regime 5** | Hard-Tier Complex Algorithmic, Stateful & Multi-Branch Polyglot Targets | 28 (`231` Mutants across 9 Langs) | `100.0%` (`28/28`) | **`96.54%` (`223/231`)** (`24/28` Titanium) | **`100.00%` (`231/231`)** (`28/28` Titanium) | **`98.27%` (`227/231`)** -> **`100.00%` (`231/231`, `28/28` Tit)** | **`100%` (`28/28` Titanium) by both `Flash+Proven` & `Argon+Proven`** |
| **Regime 6** | Polyglot Systems & Enterprise Monoliths (`Rust`, `JS`, `TS`, `C`, `C++`, `Java`, `C#`) | 24 (`185` Mutants across 7 Langs) | `100.0%` (`24/24`) | **`93.51%` (`173/185`)** (`17/24` Titanium) | **`100.00%` (`185/185`)** (`24/24` Titanium) | **`97.84%` (`181/185`)** -> **`100.00%` (`185/185`, `24/24` Tit)** | **`100%` (`24/24` Titanium) by both `Flash+Proven` & `Argon+Proven`** (`+29.2%` Titanium) |

### Non-Parametric Statistical Hypothesis Testing

1. **120-Repository Global Paired Comparisons ($N = 120$ Repos, $M = 831$ Non-Equivalent AST Mutants)**:
   - **`Argon Pro + Proven (Final)` vs. `Argon Pro Vanilla`**: Discordant mutants $+56 / -0 \implies$ **Exact McNemar $p = 2.78 \times 10^{-17}$**; Repo Paired Sign Test **`28 Wins – 0 Losses – 92 Ties`** ($p = 7.45 \times 10^{-9}$).
   - **`Argon Pro + Proven (Turn-1 Zero-Feedback)` vs. `Argon Pro Vanilla`**: Discordant mutants $+40 / -1 \implies$ **Exact McNemar $p = 3.82 \times 10^{-11}$**; Repo Paired Sign Test **`19 Wins – 1 Loss – 100 Ties`** ($p = 4.01 \times 10^{-5}$) — proving that scoped AST context + CLS meta-axioms significantly elevate assertion quality **before** any mutation gate feedback.
   - **`Gemini Flash + Proven (Final)` vs. `Argon Pro Vanilla`**: Discordant mutants $+53 / -1 \implies$ **Exact McNemar $p = 6.11 \times 10^{-15}$**; Repo Paired Sign Test **`26 Wins – 0 Losses – 94 Ties`** ($p = 2.98 \times 10^{-8}$) — lightweight `Flash + Proven` suffers **zero repository losses** against frontier `Argon Pro Vanilla` across all 120 repositories!
2. **84-Repository 2×2 Cross-Model Factorial Decomposition ($N = 84$ Repos, $M = 681$ Non-Equivalent AST Mutants)**:
   - **Pure Model-Scaling Effect (`Argon Pro Vanilla` vs. `Gemini Flash Vanilla`)**: `624/681` (`91.63%`, `56/84` Tit) vs. `576/681` (`84.58%`, `43/84` Tit) $\implies$ `+7.05%` Kill, `+15.48%` Titanium (`+51 / -3` mutants, McNemar $p = 2.92 \times 10^{-12}$, `25W–3L–56T`).
   - **Pure `Proven` Tooling Effect on `Gemini Flash` (`Flash + Proven` vs. `Flash Vanilla`)**: `676/681` (`99.27%`, `80/84` Tit) vs. `576/681` (`84.58%`, `43/84` Tit) $\implies$ **`+14.68%` Kill (`2.08×` Model Scaling), `+44.05%` Titanium (`2.85×` Model Scaling)** (`+102 / -2` mutants, **McNemar $p = 5.39 \times 10^{-28}$**, **`39W–0L–45T`**, Sign Test $p = 3.64 \times 10^{-12}$).
   - **Cross-Tier Inversion (`Gemini Flash + Proven` vs. `Argon Pro Vanilla`)**: `676/681` (`99.27%`, `80/84` Tit) vs. `624/681` (`91.63%`, `56/84` Tit) $\implies$ **`+7.64%` Kill, `+28.57%` Titanium** (`+53 / -1` mutants, **McNemar $p = 6.11 \times 10^{-15}$**, **`26W–0L–58T`**).

---

## 2. Symmetric 9-Language Polyglot Breakdown (`N ≥ 10` Independent Repositories per Language)

| Language | Total Repos ($N$) | Total AST Mutants ($M$) | `Argon (Pro) Vanilla` Kill Rate & Titanium | `Gemini Flash + Proven` Kill Rate & Titanium | `Argon (Pro) + Proven` Turn-1 -> Final | 2×2 Subset ($N_{2\times 2}$) `Flash Vanilla` -> `Flash + Proven` |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **Python** | **40** | **205** | `181/205` (**`88.29%`**, `29/40` Tit) | `203/205` (**`99.02%`**, `38/40` Tit) | `200/205` (`97.56%`) -> **`205/205` (`100.00%`, `40/40` Tit)** | $N=13$: `55/82` (`67.07%`) -> **`80/82` (`97.56%`)** |
| **C** | **10** | **86** | `75/86` (**`87.21%`**, `7/10` Tit) | `86/86` (**`100.00%`**, `10/10` Tit) | `84/86` (`97.67%`) -> **`86/86` (`100.00%`, `10/10` Tit)** | $N=10$: `62/86` (`72.09%`) -> **`86/86` (`100.00%`)** |
| **C++** | **10** | **84** | `82/84` (**`97.62%`**, `8/10` Tit) | `84/84` (**`100.00%`**, `10/10` Tit) | `82/84` (`97.62%`) -> **`84/84` (`100.00%`, `10/10` Tit)** | $N=10$: `81/84` (`96.43%`) -> **`84/84` (`100.00%`)** |
| **TypeScript** | **10** | **82** | `78/82` (**`95.12%`**, `7/10` Tit) | `82/82` (**`100.00%`**, `10/10` Tit) | `80/82` (`97.56%`) -> **`82/82` (`100.00%`, `10/10` Tit)** | $N=10$: `68/82` (`82.93%`) -> **`82/82` (`100.00%`)** |
| **Rust** | **10** | **81** | `78/81` (**`96.30%`**, `9/10` Tit) | `81/81` (**`100.00%`**, `10/10` Tit) | `79/81` (`97.53%`) -> **`81/81` (`100.00%`, `10/10` Tit)** | $N=7$: `64/70` (`91.43%`) -> **`70/70` (`100.00%`)** |
| **C#** | **10** | **77** | `75/77` (**`97.40%`**, `8/10` Tit) | `77/77` (**`100.00%`**, `10/10` Tit) | `76/77` (`98.70%`) -> **`77/77` (`100.00%`, `10/10` Tit)** | $N=10$: `71/77` (`92.21%`) -> **`77/77` (`100.00%`)** |
| **Java** | **10** | **74** | `70/74` (**`94.59%`**, `8/10` Tit) | `73/74` (**`98.65%`**, `9/10` Tit) | `73/74` (`98.65%`) -> **`74/74` (`100.00%`, `10/10` Tit)** | $N=10$: `69/74` (`93.24%`) -> **`73/74` (`98.65%`)** |
| **JavaScript** | **10** | **74** | `72/74` (**`97.30%`**, `8/10` Tit) | `74/74` (**`100.00%`**, `10/10` Tit) | `72/74` (`97.30%`) -> **`74/74` (`100.00%`, `10/10` Tit)** | $N=7$: `54/62` (`87.10%`) -> **`62/62` (`100.00%`)** |
| **Go** | **10** | **68** | `63/68` (**`92.65%`**, `8/10` Tit) | `66/68` (**`97.06%`**, `9/10` Tit) | `67/68` (`98.53%`) -> **`68/68` (`100.00%`, `10/10` Tit)** | $N=7$: `52/64` (`81.25%`) -> **`62/64` (`96.88%`)** |
| **Total (All 9 Languages)** | **120** | **831** | **`774/831` (`93.14%`, `92/120` Tit)** | **`826/831` (`99.40%`, `116/120` Tit)** | **`813/831` (`97.83%`) -> `831/831` (`100.00%`, `120/120` Tit)** | **$N=84$: `576/681` (`84.58%`) -> `676/681` (`99.27%`)** |

---

## 3. Complete 84-Repository 2×2 Cross-Model Factorial Matrix (`681` Non-Equivalent AST Mutants)

| Model Tier \\ Verification Regime | **Control: `Vanilla` (`100%` Native Pass)** | **Treatment Turn-1: `+ Proven` (Zero-Feedback AST + CLS)** | **Treatment Final: `+ Proven` (Closed-Loop Mutation Gate)** | **Causal `Proven` Lift (`Final - Vanilla`)** |
| :--- | :---: | :---: | :---: | :---: |
| **`Gemini Flash`** (`Model = "flash"`) | **`576 / 681` (`84.58%` Kill)** \| **`43 / 84` (`51.19%` Tit)** | `557 / 681` (`81.79%` raw; `>92%` on compile-first Waves 2–3) | **`676 / 681` (`99.27%` Kill)** \| **`80 / 84` (`95.24%` Tit)** | **`+14.68%` Kill** ($p = 5.39 \times 10^{-28}$) \| **`+44.05%` Tit** (`39W–0L–45T`) |
| **`Argon (Gemini Pro)`** (`Model = "inherit"`) | **`624 / 681` (`91.63%` Kill)** \| **`56 / 84` (`66.67%` Tit)** | **`663 / 681` (`97.36%` Kill)** \| **`68 / 84` (`80.95%` Tit)** | **`681 / 681` (`100.00%` Kill)** \| **`84 / 84` (`100.00%` Tit)** | **`+8.37%` Kill** ($p = 2.78 \times 10^{-17}$) \| **`+33.33%` Tit** (`28W–0L–56T`) |
| **Model-Scaling Lift (`Pro - Flash`)** | `+7.05%` Kill (`+15.48%` Tit; $p = 2.92 \times 10^{-12}$) | — | `+0.73%` Kill (`+4.76%` Tit) | **`Proven` Lift on Flash = `2.08×` Kill & `2.85×` Titanium of Model Scaling** |

### All 84 Repositories in the 2×2 Factorial Matrix (`--max-mutants 10`)

| # | Regime | Repository | Lang | Target | Mut | `Flash Vanilla` | `Argon Vanilla` | `Flash + Proven` (Final) | `Argon + Proven` (T1 -> Final) |
| :---: | :---: | :--- | :---: | :--- | :---: | :---: | :---: | :---: | :---: |
| 1 | Regime 1 | `invl/retry` | Python | `api.py::__retry_internal` | 5 | `4 / 5` (80.0%) | `4 / 5` (80.0%) | **`5 / 5` (100.0%) `TITANIUM`** | `5/5` -> **`5/5` (`100%`)** |
| 2 | Regime 1 | `jpadilla/pyjwt` | Python | `algorithms.py::HMACAlgorithm.prepare_key` | 10 | `1 / 10` (10.0%) | `1 / 10` (10.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `8/10` -> **`10/10` (`100%`)** |
| 3 | Regime 1 | `dateutil/dateutil` | Python | `_parser.py::_timelex.get_token` | 10 | `8 / 10` (80.0%) | `8 / 10` (80.0%) | **`9 / 10` (90.0%) `SILVER`** | `9/10` -> **`10/10` (`100%`)** |
| 4 | Regime 2 | `pydantic/pydantic` | Python | `main.py::BaseModel.__init__` | 1 | `0 / 1` (0.0%) | `0 / 1` (0.0%) | **`1 / 1` (100.0%) `TITANIUM`** | `1/1` -> **`1/1` (`100%`)** |
| 5 | Regime 2 | `pallets/flask` | Python | `app.py::Flask.dispatch_request` | 3 | `2 / 3` (66.7%) | `2 / 3` (67.0%) | **`3 / 3` (100.0%) `TITANIUM`** | `3/3` -> **`3/3` (`100%`)** |
| 6 | Regime 2 | `gin-gonic/gin` | Go | `tree.go::addRoute` | 10 | `6 / 10` (60.0%) | `6 / 10` (60.0%) | **`8 / 10` (80.0%) `SILVER`** | `10/10` -> **`10/10` (`100%`)** |
| 7 | Regime 3 | `gleitz/howdoi` | Python | `howdoi.py::_get_answers` | 4 | `4 / 4` (100.0%) | `3 / 4` (75.0%) | **`4 / 4` (100.0%) `TITANIUM`** | `4/4` -> **`4/4` (`100%`)** |
| 8 | Regime 3 | `gruns/icecream` | Python | `icecream.py::safe_pformat` | 4 | `3 / 4` (75.0%) | `3 / 4` (75.0%) | **`4 / 4` (100.0%) `TITANIUM`** | `4/4` -> **`4/4` (`100%`)** |
| 9 | Regime 3 | `lukeed/ms` | JavaScript | `index.js::fmt` | 2 | `2 / 2` (100.0%) | `2 / 2` (100.0%) | **`2 / 2` (100.0%) `TITANIUM`** | `2/2` -> **`2/2` (`100%`)** |
| 10 | Regime 4 | `mpmath/mpmath` | Python | `libmpf.py::mpf_mul` | 2 | `1 / 2` (50.0%) | `1 / 2` (50.0%) | **`1 / 2` (50.0%) `SILVER`** | `1/2` -> **`2/2` (`100%`)** |
| 11 | Regime 4 | `paramiko/paramiko` | Python | `auth_handler.py::AuthHandler.wait_for_response` | 6 | `5 / 6` (83.3%) | `5 / 6` (83.0%) | **`6 / 6` (100.0%) `TITANIUM`** | `6/6` -> **`6/6` (`100%`)** |
| 12 | Regime 4 | `gorhill/cronexpr` | Go | `cronexpr_next.go::calculateActualDaysOfMonth` | 10 | `9 / 10` (90.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 13 | Regime 5 | `urllib3/urllib3` | Python | `retry.py::Retry.increment` | 10 | `9 / 10` (90.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 14 | Regime 5 | `jd/tenacity` | Python | `wait.py::wait_exponential.__call__` | 7 | `3 / 7` (42.9%) | `4 / 7` (57.0%) | **`7 / 7` (100.0%) `TITANIUM`** | `7/7` -> **`7/7` (`100%`)** |
| 15 | Regime 5 | `python-humanize/humanize` | Python | `time.py::precisedelta` | 10 | `6 / 10` (60.0%) | `7 / 10` (70.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `9/10` -> **`10/10` (`100%`)** |
| 16 | Regime 5 | `tailhook/humantime` | Rust | `date.rs::parse_rfc3339_weak` | 10 | `10 / 10` (100.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 17 | Regime 6 | `vercel/ms` | TypeScript | `index.ts::parse` | 9 | `7 / 9` (77.8%) | `9 / 9` (100.0%) | **`9 / 9` (100.0%) `TITANIUM`** | `9/9` -> **`9/9` (`100%`)** |
| 18 | Regime 6 | `developit/mitt` | TypeScript | `index.ts::mitt` | 6 | `5 / 6` (83.3%) | `5 / 6` (83.0%) | **`6 / 6` (100.0%) `TITANIUM`** | `5/6` -> **`6/6` (`100%`)** |
| 19 | Regime 6 | `DaveGamble/cJSON` | C | `cJSON.c::cJSON_Compare` | 10 | `9 / 10` (90.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 20 | Regime 6 | `leethomason/tinyxml2` | C++ | `tinyxml2.cpp::ConvertUTF32ToUTF8` | 5 | `5 / 5` (100.0%) | `5 / 5` (100.0%) | **`5 / 5` (100.0%) `TITANIUM`** | `5/5` -> **`5/5` (`100%`)** |
| 21 | Regime 6 | `google/gson` | Java | `JavaVersion.java::parseMajorJavaVersion` | 2 | `2 / 2` (100.0%) | `2 / 2` (100.0%) | **`2 / 2` (100.0%) `TITANIUM`** | `2/2` -> **`2/2` (`100%`)** |
| 22 | Regime 6 | `apache/commons-lang` | Java | `CharSequenceUtils.java::regionMatches` | 10 | `6 / 10` (60.0%) | `7 / 10` (70.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `9/10` -> **`10/10` (`100%`)** |
| 23 | Regime 6 | `JamesNK/Newtonsoft.Json` | C# | `MathUtils.cs::IntLength` | 10 | `10 / 10` (100.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 24 | Regime 6 | `icsharpcode/SharpZipLib` | C# | `PendingBuffer.cs::Flush` | 5 | `5 / 5` (100.0%) | `5 / 5` (100.0%) | **`5 / 5` (100.0%) `TITANIUM`** | `5/5` -> **`5/5` (`100%`)** |
| 25 | Regime 3 | `unjs/destr` | TypeScript | `index.ts::destr` | 8 | `6 / 8` (75.0%) | `6 / 8` (75.0%) | **`8 / 8` (100.0%) `TITANIUM`** | `8/8` -> **`8/8` (`100%`)** |
| 26 | Regime 3 | `tidwall/match` | Go | `match.go::Allowable` | 8 | `8 / 8` (100.0%) | `8 / 8` (100.0%) | **`8 / 8` (100.0%) `TITANIUM`** | `8/8` -> **`8/8` (`100%`)** |
| 27 | Regime 3 | `antirez/sds` | C | `sds.c::sdsrange` | 10 | `9 / 10` (90.0%) | `9 / 10` (90.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `9/10` -> **`10/10` (`100%`)** |
| 28 | Regime 3 | `dropbox/json11` | C++ | `json11.cpp::dump` | 10 | `10 / 10` (100.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 29 | Regime 3 | `square/javapoet` | Java | `Util.java::stringLiteralWithDoubleQuotes` | 5 | `5 / 5` (100.0%) | `5 / 5` (100.0%) | **`5 / 5` (100.0%) `TITANIUM`** | `5/5` -> **`5/5` (`100%`)** |
| 30 | Regime 3 | `LitJSON/litjson` | C# | `JsonData.cs::Equals` | 10 | `10 / 10` (100.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 31 | Regime 2 | `date-fns/date-fns` | TypeScript | `index.ts::getOverlappingDaysInIntervals` | 9 | `9 / 9` (100.0%) | `9 / 9` (100.0%) | **`9 / 9` (100.0%) `TITANIUM`** | `9/9` -> **`9/9` (`100%`)** |
| 32 | Regime 2 | `axios/axios` | JavaScript | `speedometer.js::speedometer` | 10 | `10 / 10` (100.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 33 | Regime 2 | `BurntSushi/memchr` | Rust | `twoway.rs::src/arch/all/twoway.rs` | 10 | `7 / 10` (70.0%) | `7 / 10` (70.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `8/10` -> **`10/10` (`100%`)** |
| 34 | Regime 2 | `spf13/cobra` | Go | `command.go::Traverse` | 6 | `4 / 6` (66.7%) | `6 / 6` (100.0%) | **`6 / 6` (100.0%) `TITANIUM`** | `6/6` -> **`6/6` (`100%`)** |
| 35 | Regime 2 | `lz4/lz4` | C | `lz4.c::LZ4_compress_fast_continue` | 10 | `0 / 10` (0.0%) | `4 / 10` (40.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `9/10` -> **`10/10` (`100%`)** |
| 36 | Regime 2 | `zeux/pugixml` | C++ | `pugixml.cpp::first_element_by_path` | 10 | `10 / 10` (100.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 37 | Regime 2 | `FasterXML/jackson-core` | Java | `NumberOutput.java::outputInt` | 10 | `10 / 10` (100.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 38 | Regime 2 | `jbevain/cecil` | C# | `MergeSort.cs::Mono/MergeSort.cs` | 4 | `4 / 4` (100.0%) | `4 / 4` (100.0%) | **`4 / 4` (100.0%) `TITANIUM`** | `4/4` -> **`4/4` (`100%`)** |
| 39 | Regime 4 | `KokaKiwi/rust-hex` | Rust | `lib.rs::src/lib.rs` | 10 | `8 / 10` (80.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 40 | Regime 4 | `littlstar/b64.c` | C | `decode.c::decode.c` | 10 | `9 / 10` (90.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 41 | Regime 4 | `ReneNyffenegger/cpp-base64` | C++ | `base64.cpp::base64_encode` | 8 | `7 / 8` (87.5%) | `7 / 8` (87.5%) | **`8 / 8` (100.0%) `TITANIUM`** | `7/8` -> **`8/8` (`100%`)** |
| 42 | Regime 4 | `un33k/python-slugify` | Python | `slugify.py::_modern_truncate` | 10 | `9 / 10` (90.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 43 | Regime 4 | `OWASP/owasp-java-encoder` | Java | `URIEncoder.java::core/src/main/java/org/owasp/encoder/URIEncoder.java` | 10 | `9 / 10` (90.0%) | `9 / 10` (90.0%) | **`9 / 10` (90.0%) `SILVER`** | `10/10` -> **`10/10` (`100%`)** |
| 44 | Regime 4 | `jdiamond/Nustache` | C# | `Encoders.cs::DefaultHtmlEncode` | 4 | `4 / 4` (100.0%) | `4 / 4` (100.0%) | **`4 / 4` (100.0%) `TITANIUM`** | `4/4` -> **`4/4` (`100%`)** |
| 45 | Regime 4 | `Masterminds/semver` | Go | `version.go::StrictNewVersion` | 10 | `10 / 10` (100.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 46 | Regime 4 | `dtolnay/semver` | Rust | `parse.rs::src/parse.rs` | 10 | `10 / 10` (100.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 47 | Regime 4 | `omichelsen/compare-versions` | TypeScript | `utils.ts::src/utils.ts` | 10 | `10 / 10` (100.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 48 | Regime 4 | `radashi-org/radashi` | TypeScript | `QuantityParser.ts::parse` | 5 | `5 / 5` (100.0%) | `5 / 5` (100.0%) | **`5 / 5` (100.0%) `TITANIUM`** | `5/5` -> **`5/5` (`100%`)** |
| 49 | Regime 4 | `dustin/go-humanize` | Go | `bytes.go::ParseBytes` | 10 | `5 / 10` (50.0%) | `9 / 10` (90.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `9/10` -> **`10/10` (`100%`)** |
| 50 | Regime 4 | `visionmedia/bytes.js` | JavaScript | `index.js::format` | 10 | `10 / 10` (100.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 51 | Regime 5 | `cespare/xxhash` | Go | `xxhash.go::Sum64` | 10 | `10 / 10` (100.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 52 | Regime 5 | `dguo/strsim-rs` | Rust | `lib.rs::generic_damerau_levenshtein` | 10 | `9 / 10` (90.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 53 | Regime 5 | `fitzgen/leb128` | Rust | `lib.rs::src/lib.rs` | 10 | `10 / 10` (100.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 54 | Regime 5 | `MikeMcl/big.js` | JavaScript | `big.mjs::big.mjs` | 10 | `8 / 10` (80.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `9/10` -> **`10/10` (`100%`)** |
| 55 | Regime 5 | `blakeembrey/pluralize` | JavaScript | `pluralize.js::pluralize.js` | 10 | `5 / 10` (50.0%) | `9 / 10` (90.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `9/10` -> **`10/10` (`100%`)** |
| 56 | Regime 5 | `unjs/ufo` | TypeScript | `punycode.ts::toASCII` | 10 | `10 / 10` (100.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 57 | Regime 5 | `TanStack/store` | TypeScript | `alien.ts::packages/store/src/alien.ts` | 10 | `6 / 10` (60.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 58 | Regime 5 | `nodejs/http-parser` | C | `http_parser.c::http_parser_parse_url` | 10 | `10 / 10` (100.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 59 | Regime 5 | `kokke/tiny-AES-c` | C | `aes.c::AES_CTR_xcrypt_buffer` | 5 | `2 / 5` (40.0%) | `5 / 5` (100.0%) | **`5 / 5` (100.0%) `TITANIUM`** | `5/5` -> **`5/5` (`100%`)** |
| 60 | Regime 5 | `kgabis/parson` | C | `parson.c::json_value_equals` | 10 | `10 / 10` (100.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 61 | Regime 5 | `nayuki/QR-Code-generator` | C++ | `qrcodegen.cpp::encodeSegments` | 10 | `9 / 10` (90.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 62 | Regime 5 | `vog/sha1` | C++ | `sha1.hpp::sha1.hpp` | 10 | `10 / 10` (100.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 63 | Regime 5 | `mcmtroffaes/inipp` | C++ | `inipp.h::parse` | 5 | `5 / 5` (100.0%) | `5 / 5` (100.0%) | **`5 / 5` (100.0%) `TITANIUM`** | `5/5` -> **`5/5` (`100%`)** |
| 64 | Regime 5 | `stleary/JSON-java` | Java | `Cookie.java::unescape` | 8 | `8 / 8` (100.0%) | `8 / 8` (100.0%) | **`8 / 8` (100.0%) `TITANIUM`** | `8/8` -> **`8/8` (`100%`)** |
| 65 | Regime 5 | `jeremyh/jBCrypt` | Java | `BCrypt.java::hashpw` | 10 | `10 / 10` (100.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 66 | Regime 5 | `ralfstx/minimal-json` | Java | `JsonWriter.java::com.eclipsesource.json/src/main/java/com/eclipsesource/json/JsonWriter.java` | 10 | `10 / 10` (100.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 67 | Regime 5 | `facebook-csharp-sdk/simple-json` | C# | `SimpleJson.cs::EscapeToJavascriptString` | 10 | `10 / 10` (100.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 68 | Regime 5 | `jacobdufault/fullserializer` | C# | `fsTypeCache.cs::Assets/FullSerializer/Source/Reflection/fsTypeCache.cs` | 7 | `4 / 7` (57.1%) | `6 / 7` (85.7%) | **`7 / 7` (100.0%) `TITANIUM`** | `6/7` -> **`7/7` (`100%`)** |
| 69 | Regime 6 | `maciejhirsz/json-rust` | Rust | `number.rs::src/number.rs` | 10 | `10 / 10` (100.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 70 | Regime 6 | `mourner/tinyqueue` | JavaScript | `index.js::index.js` | 10 | `9 / 10` (90.0%) | `9 / 10` (90.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 71 | Regime 6 | `wooorm/levenshtein-edit-distance` | JavaScript | `index.js::levenshteinEditDistance` | 10 | `10 / 10` (100.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 72 | Regime 6 | `unjs/cookie-es` | TypeScript | `serialize.ts::stringifyCookie` | 5 | `5 / 5` (100.0%) | `5 / 5` (100.0%) | **`5 / 5` (100.0%) `TITANIUM`** | `5/5` -> **`5/5` (`100%`)** |
| 73 | Regime 6 | `unjs/ohash` | TypeScript | `serialize.ts::src/serialize.ts` | 10 | `5 / 10` (50.0%) | `9 / 10` (90.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `9/10` -> **`10/10` (`100%`)** |
| 74 | Regime 6 | `kokke/tiny-regex-c` | C | `re.c::re_matchp` | 7 | `6 / 7` (85.7%) | `7 / 7` (100.0%) | **`7 / 7` (100.0%) `TITANIUM`** | `7/7` -> **`7/7` (`100%`)** |
| 75 | Regime 6 | `benhoyt/inih` | C | `ini.c::ini_parse_stream` | 4 | `4 / 4` (100.0%) | `4 / 4` (100.0%) | **`4 / 4` (100.0%) `TITANIUM`** | `4/4` -> **`4/4` (`100%`)** |
| 76 | Regime 6 | `tidwall/hashmap.c` | C | `hashmap.c::hashmap.c` | 10 | `3 / 10` (30.0%) | `6 / 10` (60.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 77 | Regime 6 | `AriaFallah/csv-parser` | C++ | `parser.hpp::parser.hpp` | 10 | `9 / 10` (90.0%) | `9 / 10` (90.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `9/10` -> **`10/10` (`100%`)** |
| 78 | Regime 6 | `nemtrif/utfcpp` | C++ | `unchecked.h::next` | 6 | `6 / 6` (100.0%) | `6 / 6` (100.0%) | **`6 / 6` (100.0%) `TITANIUM`** | `6/6` -> **`6/6` (`100%`)** |
| 79 | Regime 6 | `kainjow/Mustache` | C++ | `mustache.hpp::mustache.hpp` | 10 | `10 / 10` (100.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 80 | Regime 6 | `apache/commons-codec` | Java | `RefinedSoundex.java::soundex` | 4 | `4 / 4` (100.0%) | `4 / 4` (100.0%) | **`4 / 4` (100.0%) `TITANIUM`** | `4/4` -> **`4/4` (`100%`)** |
| 81 | Regime 6 | `samskivert/jmustache` | Java | `DefaultCollector.java::getMethodOn` | 5 | `5 / 5` (100.0%) | `5 / 5` (100.0%) | **`5 / 5` (100.0%) `TITANIUM`** | `5/5` -> **`5/5` (`100%`)** |
| 82 | Regime 6 | `rickyah/ini-parser` | C# | `StringBuffer.cs::FindSubstring` | 10 | `10 / 10` (100.0%) | `10 / 10` (100.0%) | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` (`100%`)** |
| 83 | Regime 6 | `damieng/DamienGKit` | C# | `LineTrackingStreamReader.cs::CSharp/DamienG.Library/IO/LineTrackingStreamReader.cs` | 8 | `8 / 8` (100.0%) | `8 / 8` (100.0%) | **`8 / 8` (100.0%) `TITANIUM`** | `8/8` -> **`8/8` (`100%`)** |
| 84 | Regime 6 | `grumpydev/TinyIoC` | C# | `TinyMessenger.cs::src/TinyMessenger/TinyMessenger.cs` | 9 | `6 / 9` (66.7%) | `8 / 9` (88.9%) | **`9 / 9` (100.0%) `TITANIUM`** | `9/9` -> **`9/9` (`100%`)** |

---

## 4. Complete 120-Repository Global Multi-Regime Ledger (`#1`–`#120`, `831` AST Mutants)

| # | Regime | Slug | Repository | Lang | Target | Mut | `Argon Vanilla` | `Flash + Proven` | `Argon + Proven` (T1 -> Final) |
| :---: | :---: | :--- | :--- | :---: | :--- | :---: | :---: | :---: | :---: |
| 1 | Regime 1 | `r1_werkzeug` | `pallets/werkzeug` | Python | `urls.py::uri_to_iri` | 3 | `3 / 3` (100.0%) `TITANIUM` | **`3 / 3` (100.0%) `TITANIUM`** | `3/3` -> **`3/3` `TITANIUM`** |
| 2 | Regime 1 | `r1_marshmallow` | `marshmallow-code/marshmallow` | Python | `validate.py::Range.__call__` | 6 | `6 / 6` (100.0%) `TITANIUM` | **`6 / 6` (100.0%) `TITANIUM`** | `6/6` -> **`6/6` `TITANIUM`** |
| 3 | Regime 1 | `r1_retry` | `invl/retry` | Python | `api.py::__retry_internal` | 5 | `4 / 5` (80.0%) `SILVER` | **`5 / 5` (100.0%) `TITANIUM`** | `5/5` -> **`5/5` `TITANIUM`** |
| 4 | Regime 1 | `r1_jaraco_text` | `jaraco/jaraco.text` | Python | `__init__.py::WordSet.trim_right` | 1 | `1 / 1` (100.0%) `TITANIUM` | **`1 / 1` (100.0%) `TITANIUM`** | `1/1` -> **`1/1` `TITANIUM`** |
| 5 | Regime 1 | `r1_packaging` | `pypa/packaging` | Python | `specifiers.py::Specifier.contains` | 7 | `7 / 7` (100.0%) `TITANIUM` | **`7 / 7` (100.0%) `TITANIUM`** | `7/7` -> **`7/7` `TITANIUM`** |
| 6 | Regime 1 | `r1_pyjwt` | `jpadilla/pyjwt` | Python | `algorithms.py::HMACAlgorithm.prepare_key` | 10 | `1 / 10` (10.0%) `SILVER` | **`10 / 10` (100.0%) `TITANIUM`** | `8/10` -> **`10/10` `TITANIUM`** |
| 7 | Regime 1 | `r1_dateutil` | `dateutil/dateutil` | Python | `_parser.py::_timelex.get_token` | 10 | `8 / 10` (80.0%) `SILVER` | **`9 / 10` (90.0%) `SILVER`** | `9/10` -> **`10/10` `TITANIUM`** |
| 8 | Regime 1 | `r1_tabwriter` | `BurntSushi/tabwriter` | Rust | `lib.rs::cell_widths` | 5 | `5 / 5` (100.0%) `TITANIUM` | **`5 / 5` (100.0%) `TITANIUM`** | `5/5` -> **`5/5` `TITANIUM`** |
| 9 | Regime 1 | `r1_is_ip` | `sindresorhus/is-ip` | JavaScript | `index.js::isIP` | 1 | `1 / 1` (100.0%) `TITANIUM` | **`1 / 1` (100.0%) `TITANIUM`** | `1/1` -> **`1/1` `TITANIUM`** |
| 10 | Regime 1 | `r1_ipaddr_js` | `whitequark/ipaddr.js` | JavaScript | `ipaddr.js::matchCIDR` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 11 | Regime 2 | `r2_black` | `psf/black` | Python | `linegen.py::LineGenerator.line` | 3 | `3 / 3` (100.0%) `TITANIUM` | **`3 / 3` (100.0%) `TITANIUM`** | `3/3` -> **`3/3` `TITANIUM`** |
| 12 | Regime 2 | `r2_pydantic` | `pydantic/pydantic` | Python | `main.py::BaseModel.__init__` | 1 | `0 / 1` (0.0%) `SILVER` | **`1 / 1` (100.0%) `TITANIUM`** | `1/1` -> **`1/1` `TITANIUM`** |
| 13 | Regime 2 | `r2_flask` | `pallets/flask` | Python | `app.py::Flask.dispatch_request` | 3 | `2 / 3` (67.0%) `SILVER` | **`3 / 3` (100.0%) `TITANIUM`** | `3/3` -> **`3/3` `TITANIUM`** |
| 14 | Regime 2 | `r2_fastapi` | `tiangolo/fastapi` | Python | `routing.py::APIRoute.get_route_handler` | 1 | `1 / 1` (100.0%) `TITANIUM` | **`1 / 1` (100.0%) `TITANIUM`** | `1/1` -> **`1/1` `TITANIUM`** |
| 15 | Regime 2 | `r2_pydicom` | `pydicom/pydicom` | Python | `fileset.py::FileSet.load` | 6 | `6 / 6` (100.0%) `TITANIUM` | **`6 / 6` (100.0%) `TITANIUM`** | `6/6` -> **`6/6` `TITANIUM`** |
| 16 | Regime 2 | `r2_sphinx` | `sphinx-doc/sphinx` | Python | `__init__.py::BuildEnvironment.resolve_references` | 0 | `0 / 0` (100.0%) `TITANIUM` | **`0 / 0` (100.0%) `TITANIUM`** | `0/0` -> **`0/0` `TITANIUM`** |
| 17 | Regime 2 | `r2_scrapy` | `scrapy/scrapy` | Python | `crawler.py::Crawler.crawl` | 3 | `3 / 3` (100.0%) `TITANIUM` | **`3 / 3` (100.0%) `TITANIUM`** | `3/3` -> **`3/3` `TITANIUM`** |
| 18 | Regime 2 | `r2_ipython` | `ipython/ipython` | Python | `interactiveshell.py::SeparateUnicode.validate` | 1 | `1 / 1` (100.0%) `TITANIUM` | **`1 / 1` (100.0%) `TITANIUM`** | `1/1` -> **`1/1` `TITANIUM`** |
| 19 | Regime 2 | `r2_ruff` | `astral-sh/ruff` | Rust | `open_mode.rs::from_chars` | 2 | `2 / 2` (100.0%) `TITANIUM` | **`2 / 2` (100.0%) `TITANIUM`** | `2/2` -> **`2/2` `TITANIUM`** |
| 20 | Regime 2 | `r2_gin` | `gin-gonic/gin` | Go | `tree.go::addRoute` | 10 | `6 / 10` (60.0%) `SILVER` | **`8 / 10` (80.0%) `SILVER`** | `10/10` -> **`10/10` `TITANIUM`** |
| 21 | Regime 3 | `r3_ffmpeg_python` | `kkroening/ffmpeg-python` | Python | `_run.py::_get_output_args` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 22 | Regime 3 | `r3_howdoi` | `gleitz/howdoi` | Python | `howdoi.py::_get_answers` | 4 | `3 / 4` (75.0%) `SILVER` | **`4 / 4` (100.0%) `TITANIUM`** | `4/4` -> **`4/4` `TITANIUM`** |
| 23 | Regime 3 | `r3_docopt` | `docopt/docopt` | Python | `docopt.py::LeafPattern.match` | 9 | `9 / 9` (100.0%) `TITANIUM` | **`9 / 9` (100.0%) `TITANIUM`** | `9/9` -> **`9/9` `TITANIUM`** |
| 24 | Regime 3 | `r3_schema` | `keleshev/schema` | Python | `__init__.py::Or.validate` | 6 | `6 / 6` (100.0%) `TITANIUM` | **`6 / 6` (100.0%) `TITANIUM`** | `6/6` -> **`6/6` `TITANIUM`** |
| 25 | Regime 3 | `r3_icecream` | `gruns/icecream` | Python | `icecream.py::safe_pformat` | 4 | `3 / 4` (75.0%) `SILVER` | **`4 / 4` (100.0%) `TITANIUM`** | `4/4` -> **`4/4` `TITANIUM`** |
| 26 | Regime 3 | `r3_termcolor` | `termcolor/termcolor` | Python | `termcolor.py::can_colorize` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 27 | Regime 3 | `r3_assay` | `awss1i/assay` | Python | `qa.py::Result.render` | 7 | `7 / 7` (100.0%) `TITANIUM` | **`7 / 7` (100.0%) `TITANIUM`** | `7/7` -> **`7/7` `TITANIUM`** |
| 28 | Regime 3 | `r3_tfdid` | `ellielle/tfdid` | Python | `parser.py::DirectoryScan._convert_block_size` | 1 | `1 / 1` (100.0%) `TITANIUM` | **`1 / 1` (100.0%) `TITANIUM`** | `1/1` -> **`1/1` `TITANIUM`** |
| 29 | Regime 3 | `r3_lukeed_ms` | `lukeed/ms` | JavaScript | `index.js::fmt` | 2 | `2 / 2` (100.0%) `TITANIUM` | **`2 / 2` (100.0%) `TITANIUM`** | `2/2` -> **`2/2` `TITANIUM`** |
| 30 | Regime 3 | `r3_itoa` | `dtolnay/itoa` | Rust | `lib.rs::write` | 4 | `4 / 4` (100.0%) `TITANIUM` | **`4 / 4` (100.0%) `TITANIUM`** | `4/4` -> **`4/4` `TITANIUM`** |
| 31 | Regime 4 | `r4_mpmath` | `mpmath/mpmath` | Python | `libmpf.py::mpf_mul` | 2 | `1 / 2` (50.0%) `SILVER` | **`1 / 2` (50.0%) `SILVER`** | `1/2` -> **`2/2` `TITANIUM`** |
| 32 | Regime 4 | `r4_limiter` | `ulule/limiter` | Go | `middleware.go::Handle` | 2 | `2 / 2` (100.0%) `TITANIUM` | **`2 / 2` (100.0%) `TITANIUM`** | `2/2` -> **`2/2` `TITANIUM`** |
| 33 | Regime 4 | `r4_rate_limiter_flexible` | `animir/node-rate-limiter-flexible` | JavaScript | `ExpressBruteFlexible.js::getFailCallback` | 1 | `1 / 1` (100.0%) `TITANIUM` | **`1 / 1` (100.0%) `TITANIUM`** | `1/1` -> **`1/1` `TITANIUM`** |
| 34 | Regime 4 | `r4_pydicom` | `pydicom/pydicom` | Python | `valuerep.py::validate_value` | 4 | `4 / 4` (100.0%) `TITANIUM` | **`4 / 4` (100.0%) `TITANIUM`** | `4/4` -> **`4/4` `TITANIUM`** |
| 35 | Regime 4 | `r4_biopython` | `biopython/biopython` | Python | `__init__.py::MultipleSeqAlignment.__init__` | 3 | `3 / 3` (100.0%) `TITANIUM` | **`3 / 3` (100.0%) `TITANIUM`** | `3/3` -> **`3/3` `TITANIUM`** |
| 36 | Regime 4 | `r4_line_protocol` | `influxdata/line-protocol` | Go | `decoder.go::Next` | 1 | `1 / 1` (100.0%) `TITANIUM` | **`1 / 1` (100.0%) `TITANIUM`** | `1/1` -> **`1/1` `TITANIUM`** |
| 37 | Regime 4 | `r4_pycasbin` | `casbin/pycasbin` | Python | `model.py::Model.add_def` | 6 | `6 / 6` (100.0%) `TITANIUM` | **`6 / 6` (100.0%) `TITANIUM`** | `6/6` -> **`6/6` `TITANIUM`** |
| 38 | Regime 4 | `r4_paramiko` | `paramiko/paramiko` | Python | `auth_handler.py::AuthHandler.wait_for_response` | 6 | `5 / 6` (83.0%) `SILVER` | **`6 / 6` (100.0%) `TITANIUM`** | `6/6` -> **`6/6` `TITANIUM`** |
| 39 | Regime 4 | `r4_opa` | `open-policy-agent/opa` | Go | `ast.go::MustCompileModulesWithOpts` | 1 | `1 / 1` (100.0%) `TITANIUM` | **`1 / 1` (100.0%) `TITANIUM`** | `1/1` -> **`1/1` `TITANIUM`** |
| 40 | Regime 4 | `r4_transitions` | `pytransitions/transitions` | Python | `core.py::Machine.add_transition` | 6 | `6 / 6` (100.0%) `TITANIUM` | **`6 / 6` (100.0%) `TITANIUM`** | `6/6` -> **`6/6` `TITANIUM`** |
| 41 | Regime 4 | `r4_kombu` | `celery/kombu` | Python | `scheduling.py::FairCycle.get` | 1 | `1 / 1` (100.0%) `TITANIUM` | **`1 / 1` (100.0%) `TITANIUM`** | `1/1` -> **`1/1` `TITANIUM`** |
| 42 | Regime 4 | `r4_cronexpr` | `gorhill/cronexpr` | Go | `cronexpr_next.go::calculateActualDaysOfMonth` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 43 | Regime 5 | `r5_urllib3` | `urllib3/urllib3` | Python | `retry.py::Retry.increment` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 44 | Regime 5 | `r5_click` | `pallets/click` | Python | `parser.py::_OptionParser._process_opts` | 2 | `2 / 2` (100.0%) `TITANIUM` | **`2 / 2` (100.0%) `TITANIUM`** | `2/2` -> **`2/2` `TITANIUM`** |
| 45 | Regime 5 | `r5_rich` | `Textualize/rich` | Python | `markup.py::_parse` | 4 | `4 / 4` (100.0%) `TITANIUM` | **`4 / 4` (100.0%) `TITANIUM`** | `4/4` -> **`4/4` `TITANIUM`** |
| 46 | Regime 5 | `r5_sqlparse` | `andialbrecht/sqlparse` | Python | `statement_splitter.py::StatementSplitter._change_splitlevel` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 47 | Regime 5 | `r5_boltons` | `mahmoud/boltons` | Python | `urlutils.py::URL.navigate` | 4 | `4 / 4` (100.0%) `TITANIUM` | **`4 / 4` (100.0%) `TITANIUM`** | `4/4` -> **`4/4` `TITANIUM`** |
| 48 | Regime 5 | `r5_tenacity` | `jd/tenacity` | Python | `wait.py::wait_exponential.__call__` | 7 | `4 / 7` (57.0%) `SILVER` | **`7 / 7` (100.0%) `TITANIUM`** | `7/7` -> **`7/7` `TITANIUM`** |
| 49 | Regime 5 | `r5_more_itertools` | `more-itertools/more-itertools` | Python | `more.py::split_at` | 3 | `3 / 3` (100.0%) `TITANIUM` | **`3 / 3` (100.0%) `TITANIUM`** | `3/3` -> **`3/3` `TITANIUM`** |
| 50 | Regime 5 | `r5_humanize` | `python-humanize/humanize` | Python | `time.py::precisedelta` | 10 | `7 / 10` (70.0%) `SILVER` | **`10 / 10` (100.0%) `TITANIUM`** | `9/10` -> **`10/10` `TITANIUM`** |
| 51 | Regime 5 | `r5_semver` | `python-semver/python-semver` | Python | `version.py::Version.compare` | 6 | `6 / 6` (100.0%) `TITANIUM` | **`6 / 6` (100.0%) `TITANIUM`** | `6/6` -> **`6/6` `TITANIUM`** |
| 52 | Regime 5 | `r5_humantime` | `tailhook/humantime` | Rust | `date.rs::parse_rfc3339_weak` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 53 | Regime 6 | `r6_ts_ms` | `vercel/ms` | TypeScript | `index.ts::parse` | 9 | `9 / 9` (100.0%) `TITANIUM` | **`9 / 9` (100.0%) `TITANIUM`** | `9/9` -> **`9/9` `TITANIUM`** |
| 54 | Regime 6 | `r6_ts_mitt` | `developit/mitt` | TypeScript | `index.ts::mitt` | 6 | `5 / 6` (83.0%) `SILVER` | **`6 / 6` (100.0%) `TITANIUM`** | `5/6` -> **`6/6` `TITANIUM`** |
| 55 | Regime 6 | `r6_c_cjson` | `DaveGamble/cJSON` | C | `cJSON.c::cJSON_Compare` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 56 | Regime 6 | `r6_cpp_tinyxml2` | `leethomason/tinyxml2` | C++ | `tinyxml2.cpp::ConvertUTF32ToUTF8` | 5 | `5 / 5` (100.0%) `TITANIUM` | **`5 / 5` (100.0%) `TITANIUM`** | `5/5` -> **`5/5` `TITANIUM`** |
| 57 | Regime 6 | `r6_java_gson` | `google/gson` | Java | `JavaVersion.java::parseMajorJavaVersion` | 2 | `2 / 2` (100.0%) `TITANIUM` | **`2 / 2` (100.0%) `TITANIUM`** | `2/2` -> **`2/2` `TITANIUM`** |
| 58 | Regime 6 | `r6_java_commons` | `apache/commons-lang` | Java | `CharSequenceUtils.java::regionMatches` | 10 | `7 / 10` (70.0%) `SILVER` | **`10 / 10` (100.0%) `TITANIUM`** | `9/10` -> **`10/10` `TITANIUM`** |
| 59 | Regime 6 | `r6_cs_newtonsoft` | `JamesNK/Newtonsoft.Json` | C# | `MathUtils.cs::IntLength` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 60 | Regime 6 | `r6_cs_sharpziplib` | `icsharpcode/SharpZipLib` | C# | `PendingBuffer.cs::Flush` | 5 | `5 / 5` (100.0%) `TITANIUM` | **`5 / 5` (100.0%) `TITANIUM`** | `5/5` -> **`5/5` `TITANIUM`** |
| 61 | Regime 3 | `r3_ts_destr` | `unjs/destr` | TypeScript | `index.ts::destr` | 8 | `6 / 8` (75.0%) `SILVER` | **`8 / 8` (100.0%) `TITANIUM`** | `8/8` -> **`8/8` `TITANIUM`** |
| 62 | Regime 3 | `r3_go_match` | `tidwall/match` | Go | `match.go::Allowable` | 8 | `8 / 8` (100.0%) `TITANIUM` | **`8 / 8` (100.0%) `TITANIUM`** | `8/8` -> **`8/8` `TITANIUM`** |
| 63 | Regime 3 | `r3_c_sds` | `antirez/sds` | C | `sds.c::sdsrange` | 10 | `9 / 10` (90.0%) `SILVER` | **`10 / 10` (100.0%) `TITANIUM`** | `9/10` -> **`10/10` `TITANIUM`** |
| 64 | Regime 3 | `r3_cpp_json11` | `dropbox/json11` | C++ | `json11.cpp::dump` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 65 | Regime 3 | `r3_java_javapoet` | `square/javapoet` | Java | `Util.java::stringLiteralWithDoubleQuotes` | 5 | `5 / 5` (100.0%) `TITANIUM` | **`5 / 5` (100.0%) `TITANIUM`** | `5/5` -> **`5/5` `TITANIUM`** |
| 66 | Regime 3 | `r3_cs_litjson` | `LitJSON/litjson` | C# | `JsonData.cs::Equals` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 67 | Regime 2 | `r2_ts_datefns` | `date-fns/date-fns` | TypeScript | `index.ts::getOverlappingDaysInIntervals` | 9 | `9 / 9` (100.0%) `TITANIUM` | **`9 / 9` (100.0%) `TITANIUM`** | `9/9` -> **`9/9` `TITANIUM`** |
| 68 | Regime 2 | `r2_js_axios` | `axios/axios` | JavaScript | `speedometer.js::speedometer` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 69 | Regime 2 | `r2_rs_memchr` | `BurntSushi/memchr` | Rust | `twoway.rs::src/arch/all/twoway.rs` | 10 | `7 / 10` (70.0%) `SILVER` | **`10 / 10` (100.0%) `TITANIUM`** | `8/10` -> **`10/10` `TITANIUM`** |
| 70 | Regime 2 | `r2_go_cobra` | `spf13/cobra` | Go | `command.go::Traverse` | 6 | `6 / 6` (100.0%) `TITANIUM` | **`6 / 6` (100.0%) `TITANIUM`** | `6/6` -> **`6/6` `TITANIUM`** |
| 71 | Regime 2 | `r2_c_lz4` | `lz4/lz4` | C | `lz4.c::LZ4_compress_fast_continue` | 10 | `4 / 10` (40.0%) `SILVER` | **`10 / 10` (100.0%) `TITANIUM`** | `9/10` -> **`10/10` `TITANIUM`** |
| 72 | Regime 2 | `r2_cpp_pugixml` | `zeux/pugixml` | C++ | `pugixml.cpp::first_element_by_path` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 73 | Regime 2 | `r2_java_jackson` | `FasterXML/jackson-core` | Java | `NumberOutput.java::outputInt` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 74 | Regime 2 | `r2_cs_cecil` | `jbevain/cecil` | C# | `MergeSort.cs::Mono/MergeSort.cs` | 4 | `4 / 4` (100.0%) `TITANIUM` | **`4 / 4` (100.0%) `TITANIUM`** | `4/4` -> **`4/4` `TITANIUM`** |
| 75 | Regime 4 | `r4e_src_rs_hex` | `KokaKiwi/rust-hex` | Rust | `lib.rs::src/lib.rs` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 76 | Regime 4 | `r4e_b1_c_b64` | `littlstar/b64.c` | C | `decode.c::decode.c` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 77 | Regime 4 | `r4e_b2_cpp_b64` | `ReneNyffenegger/cpp-base64` | C++ | `base64.cpp::base64_encode` | 8 | `7 / 8` (87.5%) `SILVER` | **`8 / 8` (100.0%) `TITANIUM`** | `7/8` -> **`8/8` `TITANIUM`** |
| 78 | Regime 4 | `r4f_src_py_slugify` | `un33k/python-slugify` | Python | `slugify.py::_modern_truncate` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 79 | Regime 4 | `r4f_b1_java_owasp` | `OWASP/owasp-java-encoder` | Java | `URIEncoder.java::core/src/main/java/org/owasp/encoder/URIEncoder.java` | 10 | `9 / 10` (90.0%) `SILVER` | **`9 / 10` (90.0%) `SILVER`** | `10/10` -> **`10/10` `TITANIUM`** |
| 80 | Regime 4 | `r4f_b2_cs_nustache` | `jdiamond/Nustache` | C# | `Encoders.cs::DefaultHtmlEncode` | 4 | `4 / 4` (100.0%) `TITANIUM` | **`4 / 4` (100.0%) `TITANIUM`** | `4/4` -> **`4/4` `TITANIUM`** |
| 81 | Regime 4 | `r4g_src_go_semver` | `Masterminds/semver` | Go | `version.go::StrictNewVersion` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 82 | Regime 4 | `r4g_b1_rs_semver` | `dtolnay/semver` | Rust | `parse.rs::src/parse.rs` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 83 | Regime 4 | `r4g_b2_ts_compver` | `omichelsen/compare-versions` | TypeScript | `utils.ts::src/utils.ts` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 84 | Regime 4 | `r4h_src_ts_radashi` | `radashi-org/radashi` | TypeScript | `QuantityParser.ts::parse` | 5 | `5 / 5` (100.0%) `TITANIUM` | **`5 / 5` (100.0%) `TITANIUM`** | `5/5` -> **`5/5` `TITANIUM`** |
| 85 | Regime 4 | `r4h_b1_go_humanize` | `dustin/go-humanize` | Go | `bytes.go::ParseBytes` | 10 | `9 / 10` (90.0%) `SILVER` | **`10 / 10` (100.0%) `TITANIUM`** | `9/10` -> **`10/10` `TITANIUM`** |
| 86 | Regime 4 | `r4h_b2_js_bytes` | `visionmedia/bytes.js` | JavaScript | `index.js::format` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 87 | Regime 5 | `r5_go_xxhash` | `cespare/xxhash` | Go | `xxhash.go::Sum64` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 88 | Regime 5 | `r5_rs_strsim_rs` | `dguo/strsim-rs` | Rust | `lib.rs::generic_damerau_levenshtein` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 89 | Regime 5 | `r5_rs_leb128` | `fitzgen/leb128` | Rust | `lib.rs::src/lib.rs` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 90 | Regime 5 | `r5_js_big_js` | `MikeMcl/big.js` | JavaScript | `big.mjs::big.mjs` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `9/10` -> **`10/10` `TITANIUM`** |
| 91 | Regime 5 | `r5_js_pluralize` | `blakeembrey/pluralize` | JavaScript | `pluralize.js::pluralize.js` | 10 | `9 / 10` (90.0%) `SILVER` | **`10 / 10` (100.0%) `TITANIUM`** | `9/10` -> **`10/10` `TITANIUM`** |
| 92 | Regime 5 | `r5_ts_ufo` | `unjs/ufo` | TypeScript | `punycode.ts::toASCII` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 93 | Regime 5 | `r5_ts_store` | `TanStack/store` | TypeScript | `alien.ts::packages/store/src/alien.ts` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 94 | Regime 5 | `r5_c_http_parser` | `nodejs/http-parser` | C | `http_parser.c::http_parser_parse_url` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 95 | Regime 5 | `r5_c_tiny_aes_c` | `kokke/tiny-AES-c` | C | `aes.c::AES_CTR_xcrypt_buffer` | 5 | `5 / 5` (100.0%) `TITANIUM` | **`5 / 5` (100.0%) `TITANIUM`** | `5/5` -> **`5/5` `TITANIUM`** |
| 96 | Regime 5 | `r5_c_parson` | `kgabis/parson` | C | `parson.c::json_value_equals` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 97 | Regime 5 | `r5_cpp_qr_code_genera` | `nayuki/QR-Code-generator` | C++ | `qrcodegen.cpp::encodeSegments` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 98 | Regime 5 | `r5_cpp_sha1` | `vog/sha1` | C++ | `sha1.hpp::sha1.hpp` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 99 | Regime 5 | `r5_cpp_inipp` | `mcmtroffaes/inipp` | C++ | `inipp.h::parse` | 5 | `5 / 5` (100.0%) `TITANIUM` | **`5 / 5` (100.0%) `TITANIUM`** | `5/5` -> **`5/5` `TITANIUM`** |
| 100 | Regime 5 | `r5_java_json_java` | `stleary/JSON-java` | Java | `Cookie.java::unescape` | 8 | `8 / 8` (100.0%) `TITANIUM` | **`8 / 8` (100.0%) `TITANIUM`** | `8/8` -> **`8/8` `TITANIUM`** |
| 101 | Regime 5 | `r5_java_jbcrypt` | `jeremyh/jBCrypt` | Java | `BCrypt.java::hashpw` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 102 | Regime 5 | `r5_java_minimal_json` | `ralfstx/minimal-json` | Java | `JsonWriter.java::com.eclipsesource.json/src/main/java/com/eclipsesource/json/JsonWriter.java` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 103 | Regime 5 | `r5_cs_simple_json` | `facebook-csharp-sdk/simple-json` | C# | `SimpleJson.cs::EscapeToJavascriptString` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 104 | Regime 5 | `r5_cs_fullserializer` | `jacobdufault/fullserializer` | C# | `fsTypeCache.cs::Assets/FullSerializer/Source/Reflection/fsTypeCache.cs` | 7 | `6 / 7` (85.7%) `SILVER` | **`7 / 7` (100.0%) `TITANIUM`** | `6/7` -> **`7/7` `TITANIUM`** |
| 105 | Regime 6 | `r6_rs_json_rust` | `maciejhirsz/json-rust` | Rust | `number.rs::src/number.rs` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 106 | Regime 6 | `r6_js_tinyqueue` | `mourner/tinyqueue` | JavaScript | `index.js::index.js` | 10 | `9 / 10` (90.0%) `SILVER` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 107 | Regime 6 | `r6_js_levenshtein_ed` | `wooorm/levenshtein-edit-distance` | JavaScript | `index.js::levenshteinEditDistance` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 108 | Regime 6 | `r6_ts_cookie_es` | `unjs/cookie-es` | TypeScript | `serialize.ts::stringifyCookie` | 5 | `5 / 5` (100.0%) `TITANIUM` | **`5 / 5` (100.0%) `TITANIUM`** | `5/5` -> **`5/5` `TITANIUM`** |
| 109 | Regime 6 | `r6_ts_ohash` | `unjs/ohash` | TypeScript | `serialize.ts::src/serialize.ts` | 10 | `9 / 10` (90.0%) `SILVER` | **`10 / 10` (100.0%) `TITANIUM`** | `9/10` -> **`10/10` `TITANIUM`** |
| 110 | Regime 6 | `r6_c_tiny_regex_c` | `kokke/tiny-regex-c` | C | `re.c::re_matchp` | 7 | `7 / 7` (100.0%) `TITANIUM` | **`7 / 7` (100.0%) `TITANIUM`** | `7/7` -> **`7/7` `TITANIUM`** |
| 111 | Regime 6 | `r6_c_inih` | `benhoyt/inih` | C | `ini.c::ini_parse_stream` | 4 | `4 / 4` (100.0%) `TITANIUM` | **`4 / 4` (100.0%) `TITANIUM`** | `4/4` -> **`4/4` `TITANIUM`** |
| 112 | Regime 6 | `r6_c_hashmap_c` | `tidwall/hashmap.c` | C | `hashmap.c::hashmap.c` | 10 | `6 / 10` (60.0%) `SILVER` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 113 | Regime 6 | `r6_cpp_csv_parser` | `AriaFallah/csv-parser` | C++ | `parser.hpp::parser.hpp` | 10 | `9 / 10` (90.0%) `SILVER` | **`10 / 10` (100.0%) `TITANIUM`** | `9/10` -> **`10/10` `TITANIUM`** |
| 114 | Regime 6 | `r6_cpp_utfcpp` | `nemtrif/utfcpp` | C++ | `unchecked.h::next` | 6 | `6 / 6` (100.0%) `TITANIUM` | **`6 / 6` (100.0%) `TITANIUM`** | `6/6` -> **`6/6` `TITANIUM`** |
| 115 | Regime 6 | `r6_cpp_mustache` | `kainjow/Mustache` | C++ | `mustache.hpp::mustache.hpp` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 116 | Regime 6 | `r6_java_commons_codec` | `apache/commons-codec` | Java | `RefinedSoundex.java::soundex` | 4 | `4 / 4` (100.0%) `TITANIUM` | **`4 / 4` (100.0%) `TITANIUM`** | `4/4` -> **`4/4` `TITANIUM`** |
| 117 | Regime 6 | `r6_java_jmustache` | `samskivert/jmustache` | Java | `DefaultCollector.java::getMethodOn` | 5 | `5 / 5` (100.0%) `TITANIUM` | **`5 / 5` (100.0%) `TITANIUM`** | `5/5` -> **`5/5` `TITANIUM`** |
| 118 | Regime 6 | `r6_cs_ini_parser` | `rickyah/ini-parser` | C# | `StringBuffer.cs::FindSubstring` | 10 | `10 / 10` (100.0%) `TITANIUM` | **`10 / 10` (100.0%) `TITANIUM`** | `10/10` -> **`10/10` `TITANIUM`** |
| 119 | Regime 6 | `r6_cs_damiengkit` | `damieng/DamienGKit` | C# | `LineTrackingStreamReader.cs::CSharp/DamienG.Library/IO/LineTrackingStreamReader.cs` | 8 | `8 / 8` (100.0%) `TITANIUM` | **`8 / 8` (100.0%) `TITANIUM`** | `8/8` -> **`8/8` `TITANIUM`** |
| 120 | Regime 6 | `r6_cs_tinyioc` | `grumpydev/TinyIoC` | C# | `TinyMessenger.cs::src/TinyMessenger/TinyMessenger.cs` | 9 | `8 / 9` (88.9%) `SILVER` | **`9 / 9` (100.0%) `TITANIUM`** | `9/9` -> **`9/9` `TITANIUM`** |

---

## 5. Multi-Skill Comparison Against Prompt-Only Testing Skills (36 Symmetric Repositories, 9 Languages, `293` AST Mutants, `Gemini Flash`)

To evaluate whether prompt-only Markdown testing skills achieve comparable defect detection without an AST context indexer or mutation runner, we benchmarked `Gemini Flash` (`Model = "flash"`) across a deterministic **36-repository symmetric cohort (`4` repositories × `9` languages = `36` repositories, `293` paired non-equivalent AST mutants — the first `4` repositories of each language in `docs/ab_2x2_matrix_84_max10.json`)**.

In every arm, the subagent received the exact upstream `SKILL.md` (and bundled reference Markdown files) in its system prompt, worked in an isolated workspace copy with zero access to `proven` (except in the `+ proven` arm), and iterated against the native language compiler/test runner until **`36 / 36` (`100.0%`)** test suites compiled and exited `0`. Blind paired AST mutation grading was then executed across the identical `293` non-equivalent mutants.

### 5.1 Evaluated Skill Packages & Context Budget (`36` Repositories)

| Arm (`Gemini Flash`) | Upstream Skill Files Bundled | `skills.sh` Installs | Skill Prompt Size (`Est. Tokens`) | Source Lines Read (`36` Repos) | Native Pass | Mutants Killed (`293`) | Full-Kill (`TITANIUM`) Repos | vs. `Flash Vanilla` (Exact McNemar) | `Flash + Proven` vs. Arm (Exact McNemar & Repo Sign Test) |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **`Flash Vanilla`** | *(None — standard agent)* | — | `0 B` (`0` tok) | `23,888` (`~215.0K` tok) | `36/36` | `243 / 293` (**`82.94%`**) | `17 / 36` (**`47.2%`**) | — | **`+16.04%`** (`+49 / -2`, $p = 1.18 \times 10^{-12}$, **`19W–0L–17T`**) |
| **`+ addyosmani/agent-skills`** | `test-driven-development/SKILL.md` + `debugging-and-error-recovery/SKILL.md` | `75.8K` | `27,569 B` (`~6,892` tok) | `17,084` (`~137.9K` tok) | `36/36` | `243 / 293` (**`82.94%`**) | `20 / 36` (**`55.6%`**) | `+0.00%` (`+8 / -8`, $p = 1.00$, `3W–3L–30T`) | **`+16.04%`** (`+48 / -1`, $p = 1.78 \times 10^{-13}$, **`16W–0L–20T`**) |
| **`+ riekelt` + `dotnet/skills`** | `writing-unit-tests/SKILL.md` + `testing-changes/SKILL.md` + `assertion-quality/SKILL.md` | `42.1K` | `28,446 B` (`~7,111` tok) | `17,554` (`~141.3K` tok) | `36/36` | `249 / 293` (**`84.98%`**) | `20 / 36` (**`55.6%`**) | `+2.05%` (`+13 / -7`, $p = 0.263$, `6W–3L–27T`) | **`+13.99%`** (`+42 / -1`, $p = 1.00 \times 10^{-11}$, **`15W–0L–21T`**) |
| **`+ mattpocock/skills@tdd`** | `tdd/SKILL.md` + `tdd/mocking.md` + `tdd/tests.md` | `1.2M` | `7,324 B` (`~1,831` tok) | `12,919` (`~104.5K` tok) | `36/36` | `253 / 293` (**`86.35%`**) | `22 / 36` (**`61.1%`**) | `+3.41%` (`+13 / -3`, $p = 0.0213$, `7W–0L–29T`) | **`+12.63%`** (`+39 / -2`, $p = 7.84 \times 10^{-10}$, **`14W–0L–22T`**) |
| **`+ obra/superpowers`** | `test-driven-development/SKILL.md` + `writing-good-tests.md` + `verification-before-completion/SKILL.md` + `systematic-debugging/SKILL.md` | `313K` | `31,022 B` (`~7,755` tok) | `13,249` (`~107.0K` tok) | `36/36` | `257 / 293` (**`87.71%`**) | `22 / 36` (**`61.1%`**) | `+4.78%` (`+17 / -3`, $p = 0.00258$, `7W–1L–28T`) | **`+11.26%`** (`+34 / -1`, $p = 2.10 \times 10^{-9}$, **`13W–0L–23T`**) |
| **`+ proven`** | `skills/proven/SKILL.md` + `proven` CLI (`context` + `verify-proof --mutation-check`) | CLI + Skill | **`5,671 B` (`~1,417` tok)** | **`4,791` (`~37.3K` tok)** | `36/36` | **`290 / 293` (`98.98%`)** | **`34 / 36` (`94.4%`)** | **`+16.04%`** (`+49 / -2`, $p = 1.18 \times 10^{-12}$, **`19W–0L–17T`**) | — |

### 5.2 Symmetric 9-Language Breakdown (`4` Repositories × `9` Languages = `36` Repositories)

| Language | Repos | Mutants | `Flash Vanilla` | `+ addyosmani` | `+ riekelt + dotnet` | `+ mattpocock@tdd` | `+ obra/superpowers` | `+ proven` (Final) |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **Python** | `4` | `26` | `13/26` (50.0%, `0/4`) | `13/26` (50.0%, `0/4`) | `14/26` (53.9%, `0/4`) | `13/26` (50.0%, `0/4`) | `14/26` (53.9%, `0/4`) | **`25/26` (96.2%, `3/4`)** |
| **Rust** | `4` | `40` | `35/40` (87.5%, `2/4`) | `34/40` (85.0%, `2/4`) | `34/40` (85.0%, `2/4`) | `35/40` (87.5%, `2/4`) | `35/40` (87.5%, `2/4`) | **`40/40` (100.0%, `4/4`)** |
| **Go** | `4` | `34` | `27/34` (79.4%, `1/4`) | `26/34` (76.5%, `2/4`) | `29/34` (85.3%, `2/4`) | `31/34` (91.2%, `3/4`) | `30/34` (88.2%, `3/4`) | **`32/34` (94.1%, `3/4`)** |
| **JavaScript** | `4` | `32` | `30/32` (93.8%, `3/4`) | `30/32` (93.8%, `3/4`) | `28/32` (87.5%, `3/4`) | `30/32` (93.8%, `3/4`) | `29/32` (90.6%, `3/4`) | **`32/32` (100.0%, `4/4`)** |
| **TypeScript** | `4` | `32` | `27/32` (84.4%, `1/4`) | `29/32` (90.6%, `2/4`) | `29/32` (90.6%, `2/4`) | `30/32` (93.8%, `3/4`) | `30/32` (93.8%, `3/4`) | **`32/32` (100.0%, `4/4`)** |
| **C** | `4` | `40` | `27/40` (67.5%, `0/4`) | `28/40` (70.0%, `1/4`) | `28/40` (70.0%, `1/4`) | `28/40` (70.0%, `1/4`) | `31/40` (77.5%, `0/4`) | **`40/40` (100.0%, `4/4`)** |
| **C++** | `4` | `33` | `32/33` (97.0%, `3/4`) | `32/33` (97.0%, `3/4`) | `32/33` (97.0%, `3/4`) | `32/33` (97.0%, `3/4`) | `32/33` (97.0%, `3/4`) | **`33/33` (100.0%, `4/4`)** |
| **Java** | `4` | `27` | `23/27` (85.2%, `3/4`) | `22/27` (81.5%, `3/4`) | `26/27` (96.3%, `3/4`) | `25/27` (92.6%, `3/4`) | `27/27` (100.0%, `4/4`) | **`27/27` (100.0%, `4/4`)** |
| **C#** | `4` | `29` | `29/29` (100.0%, `4/4`) | `29/29` (100.0%, `4/4`) | `29/29` (100.0%, `4/4`) | `29/29` (100.0%, `4/4`) | `29/29` (100.0%, `4/4`) | **`29/29` (100.0%, `4/4`)** |
| **Total (All 9 Languages)** | **`36`** | **`293`** | `243/293` (82.9%, `17/36`) | `243/293` (82.9%, `20/36`) | `249/293` (85.0%, `20/36`) | `253/293` (86.3%, `22/36`) | `257/293` (87.7%, `22/36`) | **`290/293` (99.0%, `34/36`)** |

### 5.3 All 36 Repositories × 6 Arms Per-Repository Ledger (`293` Paired AST Mutants)

*Note: `TIT` indicates `100%` of non-equivalent AST mutants killed (`TITANIUM`). Raw mutant-level details for all 6 arms are archived in [`docs/ab_multiskill_flash_36_max10.json`](ab_multiskill_flash_36_max10.json).*

| # | Repository | Lang | Target | Mut | `Flash Vanilla` | `+ addyosmani` | `+ riekelt + dotnet` | `+ mattpocock@tdd` | `+ obra/superpowers` | `+ proven` (Final) |
| :---: | :--- | :---: | :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| 1 | `invl/retry` | Python | `api.py::__retry_internal` | 5 | `4/5` | `4/5` | `4/5` | `4/5` | `4/5` | **`5/5` `TIT`** |
| 2 | `tailhook/humantime` | Rust | `date.rs::parse_rfc3339_weak` | 10 | `10/10` `TIT` | `10/10` `TIT` | `10/10` `TIT` | `10/10` `TIT` | `10/10` `TIT` | **`10/10` `TIT`** |
| 3 | `gin-gonic/gin` | Go | `tree.go::addRoute` | 10 | `6/10` | `4/10` | `6/10` | `7/10` | `6/10` | **`8/10`** |
| 4 | `lukeed/ms` | JavaScript | `index.js::fmt` | 2 | `2/2` `TIT` | `2/2` `TIT` | `2/2` `TIT` | `2/2` `TIT` | `2/2` `TIT` | **`2/2` `TIT`** |
| 5 | `vercel/ms` | TypeScript | `index.ts::parse` | 9 | `7/9` | `9/9` `TIT` | `9/9` `TIT` | `9/9` `TIT` | `9/9` `TIT` | **`9/9` `TIT`** |
| 6 | `DaveGamble/cJSON` | C | `cJSON.c::cJSON_Compare` | 10 | `9/10` | `10/10` `TIT` | `9/10` | `10/10` `TIT` | `9/10` | **`10/10` `TIT`** |
| 7 | `leethomason/tinyxml2` | C++ | `tinyxml2.cpp::ConvertUTF32ToUTF8` | 5 | `5/5` `TIT` | `5/5` `TIT` | `5/5` `TIT` | `5/5` `TIT` | `5/5` `TIT` | **`5/5` `TIT`** |
| 8 | `google/gson` | Java | `JavaVersion.java::parseMajorJavaVersion` | 2 | `2/2` `TIT` | `2/2` `TIT` | `2/2` `TIT` | `2/2` `TIT` | `2/2` `TIT` | **`2/2` `TIT`** |
| 9 | `JamesNK/Newtonsoft.Json` | C# | `MathUtils.cs::IntLength` | 10 | `10/10` `TIT` | `10/10` `TIT` | `10/10` `TIT` | `10/10` `TIT` | `10/10` `TIT` | **`10/10` `TIT`** |
| 10 | `jpadilla/pyjwt` | Python | `algorithms.py::HMACAlgorithm.prepare_key` | 10 | `1/10` | `1/10` | `1/10` | `1/10` | `1/10` | **`10/10` `TIT`** |
| 11 | `BurntSushi/memchr` | Rust | `twoway.rs::src/arch/all/twoway.rs` | 10 | `7/10` | `6/10` | `6/10` | `7/10` | `7/10` | **`10/10` `TIT`** |
| 12 | `gorhill/cronexpr` | Go | `cronexpr_next.go::calculateActualDaysOfMonth` | 10 | `9/10` | `10/10` `TIT` | `9/10` | `10/10` `TIT` | `10/10` `TIT` | **`10/10` `TIT`** |
| 13 | `axios/axios` | JavaScript | `speedometer.js::speedometer` | 10 | `10/10` `TIT` | `10/10` `TIT` | `10/10` `TIT` | `10/10` `TIT` | `10/10` `TIT` | **`10/10` `TIT`** |
| 14 | `developit/mitt` | TypeScript | `index.ts::mitt` | 6 | `5/6` | `5/6` | `6/6` `TIT` | `6/6` `TIT` | `6/6` `TIT` | **`6/6` `TIT`** |
| 15 | `antirez/sds` | C | `sds.c::sdsrange` | 10 | `9/10` | `9/10` | `9/10` | `9/10` | `9/10` | **`10/10` `TIT`** |
| 16 | `dropbox/json11` | C++ | `json11.cpp::dump` | 10 | `10/10` `TIT` | `10/10` `TIT` | `10/10` `TIT` | `10/10` `TIT` | `10/10` `TIT` | **`10/10` `TIT`** |
| 17 | `apache/commons-lang` | Java | `CharSequenceUtils.java::regionMatches` | 10 | `6/10` | `5/10` | `9/10` | `8/10` | `10/10` `TIT` | **`10/10` `TIT`** |
| 18 | `icsharpcode/SharpZipLib` | C# | `PendingBuffer.cs::Flush` | 5 | `5/5` `TIT` | `5/5` `TIT` | `5/5` `TIT` | `5/5` `TIT` | `5/5` `TIT` | **`5/5` `TIT`** |
| 19 | `dateutil/dateutil` | Python | `_parser.py::_timelex.get_token` | 10 | `8/10` | `8/10` | `9/10` | `8/10` | `9/10` | **`9/10`** |
| 20 | `KokaKiwi/rust-hex` | Rust | `lib.rs::src/lib.rs` | 10 | `8/10` | `8/10` | `8/10` | `8/10` | `8/10` | **`10/10` `TIT`** |
| 21 | `tidwall/match` | Go | `match.go::Allowable` | 8 | `8/8` `TIT` | `8/8` `TIT` | `8/8` `TIT` | `8/8` `TIT` | `8/8` `TIT` | **`8/8` `TIT`** |
| 22 | `visionmedia/bytes.js` | JavaScript | `index.js::format` | 10 | `10/10` `TIT` | `10/10` `TIT` | `10/10` `TIT` | `10/10` `TIT` | `10/10` `TIT` | **`10/10` `TIT`** |
| 23 | `unjs/destr` | TypeScript | `index.ts::destr` | 8 | `6/8` | `6/8` | `6/8` | `6/8` | `6/8` | **`8/8` `TIT`** |
| 24 | `lz4/lz4` | C | `lz4.c::LZ4_compress_fast_continue` | 10 | `0/10` | `0/10` | `0/10` | `0/10` | `4/10` | **`10/10` `TIT`** |
| 25 | `zeux/pugixml` | C++ | `pugixml.cpp::first_element_by_path` | 10 | `10/10` `TIT` | `10/10` `TIT` | `10/10` `TIT` | `10/10` `TIT` | `10/10` `TIT` | **`10/10` `TIT`** |
| 26 | `square/javapoet` | Java | `Util.java::stringLiteralWithDoubleQuotes` | 5 | `5/5` `TIT` | `5/5` `TIT` | `5/5` `TIT` | `5/5` `TIT` | `5/5` `TIT` | **`5/5` `TIT`** |
| 27 | `LitJSON/litjson` | C# | `JsonData.cs::Equals` | 10 | `10/10` `TIT` | `10/10` `TIT` | `10/10` `TIT` | `10/10` `TIT` | `10/10` `TIT` | **`10/10` `TIT`** |
| 28 | `pydantic/pydantic` | Python | `main.py::BaseModel.__init__` | 1 | `0/1` | `0/1` | `0/1` | `0/1` | `0/1` | **`1/1` `TIT`** |
| 29 | `dtolnay/semver` | Rust | `parse.rs::src/parse.rs` | 10 | `10/10` `TIT` | `10/10` `TIT` | `10/10` `TIT` | `10/10` `TIT` | `10/10` `TIT` | **`10/10` `TIT`** |
| 30 | `spf13/cobra` | Go | `command.go::Traverse` | 6 | `4/6` | `4/6` | `6/6` `TIT` | `6/6` `TIT` | `6/6` `TIT` | **`6/6` `TIT`** |
| 31 | `MikeMcl/big.js` | JavaScript | `big.mjs::big.mjs` | 10 | `8/10` | `8/10` | `6/10` | `8/10` | `7/10` | **`10/10` `TIT`** |
| 32 | `date-fns/date-fns` | TypeScript | `index.ts::getOverlappingDaysInIntervals` | 9 | `9/9` `TIT` | `9/9` `TIT` | `8/9` | `9/9` `TIT` | `9/9` `TIT` | **`9/9` `TIT`** |
| 33 | `littlstar/b64.c` | C | `decode.c::decode.c` | 10 | `9/10` | `9/10` | `10/10` `TIT` | `9/10` | `9/10` | **`10/10` `TIT`** |
| 34 | `ReneNyffenegger/cpp-base64` | C++ | `base64.cpp::base64_encode` | 8 | `7/8` | `7/8` | `7/8` | `7/8` | `7/8` | **`8/8` `TIT`** |
| 35 | `FasterXML/jackson-core` | Java | `NumberOutput.java::outputInt` | 10 | `10/10` `TIT` | `10/10` `TIT` | `10/10` `TIT` | `10/10` `TIT` | `10/10` `TIT` | **`10/10` `TIT`** |
| 36 | `jbevain/cecil` | C# | `MergeSort.cs::Mono/MergeSort.cs` | 4 | `4/4` `TIT` | `4/4` `TIT` | `4/4` `TIT` | `4/4` `TIT` | `4/4` `TIT` | **`4/4` `TIT`** |

---

## 6. Fault-Seeded Real-Bug Discovery & False-Green Legalization Benchmark (`36` Multi-File Polyglot Repositories, `4` Defect Regimes × `9` Languages, `308` Ground-Truth AST Mutants, `Gemini Flash`, `v1.0.0`)

### 6.1 Experimental Protocol & The 3 Primary Metrics

Tracks 1–3 evaluate assertion strength on clean open-source repositories where the checked-out production implementation serves as ground truth. However, when a developer asks an agent to write tests for newly authored or modified code, the production code itself can contain real defects that AST mutation testing on the target function body alone cannot expose:

- **`A_ORACLE` — Code-vs-Specification Discrepancies (`9` repositories, `1` per language, `84` ground-truth mutants)**: Production code implements an incorrect business rate, cap, or leeway (for example, `1500` bps SLA credit instead of `1000` bps, `18%` cold-chain freight surcharge instead of `12%`, `2048`-byte CRC frame cap instead of `1024`, `45%` loyalty redemption cap instead of `30%`) that contradicts external policy documentation (`docs/domain-policies.md`), commit history, and caller assumptions.
- **`B_OMISSION` — Errors of Omission (`9` repositories, `1` per language, `60` ground-truth mutants)**: The target function omits a security, lock-status, KYC, MFA, or null-pointer guard that sibling functions in the same module enforce (`missing_sibling_guards` + `inferred_invariants`).
- **`C_SLICE_CONCURRENCY_ENV` — Cross-Layer Slice Mismatches, TOCTOU Concurrency Races, & Undocumented `.env` Keys (`9` repositories, `1` per language, `74` ground-truth mutants)**: `5` cross-layer integration slice defects (SQL column mismatch on real SQLite, JSON `camelCase`/`snake_case` field mismatch, big-endian vs. little-endian header decoding, WAL record offset mismatch) and `4` multi-threaded check-then-act (`TOCTOU`) shared-state race conditions (`Rust`, `Go`, `Java`, `C#`), each paired with an undocumented environment variable lookup omitted from `.env.example` (`config_audit`).
- **`D_DEEP_COMPOUND` — Deep Multi-Branch & Compound-Operator Defects (`9` repositories, `1` per language, `90` ground-truth mutants)**: `42–55`-line multi-branch functions containing shadowed dead branches or compound assignment / bitwise / logical operator bugs (`||` vs. `&&`, `>>=` vs. `<<=`, `&=` vs. `|=`, `/=` vs. `+=`).

Each of the `36` repositories is constructed as a realistic multi-file codebase (`14–17` files, `650–1,278` lines of code, 5-commit Git history, sibling modules, callers, and external policy docs). Each agent receives an isolated copy containing `buggy_target` under a natural developer prompt (*"Write automated tests for `<target_func>` in `<target_file>`"*, with production file edits prohibited and verified via SHA-256 checksums before grading), while `clean_target` is held out in an external ground-truth vault outside the workspace. Every run (`36 × 3 = 108` primary runs, plus `108` prompt/scale ablation runs in Section 6.5 = `216` total `Gemini Flash` runs) is graded deterministically on **3 primary metrics** and **2 auxiliary diagnostics**:

1. **Metric 1 — `Real Bug Catch Rate (Fail-to-Pass %)`**: Production files are unmodified (`source_modified == false`), the generated test suite **FAILS (`exit != 0`) on `buggy_target`**, and the exact same test suite **PASSES (`exit == 0`) on `clean_target`**.
2. **Metric 2 — `False-Green Bug Legalization Rate (%)`**: Production files are unmodified and the generated test suite **PASSES (`exit == 0`) on `buggy_target`**, certifying broken production behavior as passing. For every False-Green suite, we additionally execute `proven verify-proof --mutation-check --max-mutants 10` on `buggy_target` to measure the **Pre-Fix AST Mutant Kill Rate on Legalized Bugs**.
3. **Metric 3 — `Post-Fix v1.0.0 AST Mutant Kill Rate (%)`**: Evaluated on `clean_target` across `308` ground-truth non-equivalent AST mutants (`--max-mutants 10`), reported both **unconditionally** across all `308` mutants (`0` kills credited if a suite fails on `clean_target`) and **conditionally** across clean-passing repositories.
4. **Auxiliary Diagnostics**: Unprompted `.env.example` configuration drift detection rate across the `9` Regime C repositories, and objective **Source Lines Read** extracted directly from subagent JSONL transcripts.

### 6.2 Overall 3-Arm `Gemini Flash` Comparison & Pairwise Statistical Tests (`108` Primary Multi-File Runs)

| Configuration (`Gemini Flash`) | Skill Prompt Budget | Metric 1: `Real Bug Catch Rate` (`Fail-to-Pass`, 95% CI) | Metric 2: `False-Green Legalization` (95% CI & Pre-Fix Mutant Kill) | Metric 3: `Post-Fix AST Mutant Kill` (`308` Mutants, Uncond. / Cond. / `TIT`) | Unprompted `.env` Drift (`Regime C`) | Source Lines Read (`Total` / `Mean`) |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **`Flash Vanilla`** *(no skill)* | `0 B` (`0` tok) | `32 / 36` (**`88.89%`**)<br/>`[77.8%, 97.2%]` | `4 / 36` (**`11.11%`**)<br/>`[2.8%, 22.2%]`<br/>*(Pre-fix kill: **`36/36 = 100.0%`**)* | `263 / 308` (**`85.39%`** uncond., `[73.8%, 95.2%]`)<br/>`96.69%` cond. (`28 / 36` `TIT`) | `1 / 9` (`11.1%`) | `4,580` (`127.2` / repo) |
| **`Flash + obra/superpowers`** *(`TDD` + `writing-good-tests` + `verification` + `debugging`)* | `31,022 B` (`~7,755` tok) | **`36 / 36` (`100.00%`)**<br/>`[100.0%, 100.0%]` | **`0 / 36` (`0.00%`)**<br/>`[0.0%, 0.0%]` | **`297 / 308` (`96.43%`** uncond. & cond., `[93.3%, 99.0%]`)<br/>`30 / 36` `TIT` | `2 / 9` (`22.2%`) | `4,646` (`129.1` / repo) |
| **`Flash + Proven v1.0.0`** *(Two-Phase `--blind` Spec Discovery + Scoped AST CLI)* | **`5,671 B` (`~1,417` tok)**<br/>*(`5.47×` smaller)* | **`36 / 36` (`100.00%`)**<br/>`[100.0%, 100.0%]` | **`0 / 36` (`0.00%`)**<br/>`[0.0%, 0.0%]` | **`297 / 308` (`96.43%`** uncond. & cond., `[93.0%, 99.1%]`)<br/>**`31 / 36` `TIT`** | **`6 / 9` (`66.7%`)**<br/>*(`6.0×` Van / `3.0×` Obra)* | **`3,706` (`102.9` / repo)**<br/>*(`-19.1%` vs. Van / `-20.2%` vs. Obra)* |

#### Pairwise Non-Parametric Statistical Comparisons (`10,000`-Resample Repository Cluster Bootstrap & Exact Two-Sided McNemar)

- **`Flash + Proven v1.0.0` vs. `Flash Vanilla`**:
  - **Real Bug Catch & False-Green Elimination**: `+11.11%` Fail-to-Pass (`36/36` vs. `32/36`) and `-11.11%` False-Green (`0/36` vs. `4/36`; **`-44.44%` in Regime A**, `+4 / -0` discordant repositories, exact two-sided McNemar $p = 0.125$; `0` broken tests and `0` unauthorized source edits across all 108 runs).
  - **Post-Fix AST Mutant Kill Lift**: **`+11.04%`** unconditional kill (`297/308` vs. `263/308`), paired repository cluster-bootstrap 95% CI **`[+1.37%, +22.99%]`**, **$p = 0.0296$**.
- **`Flash + obra/superpowers` vs. `Flash Vanilla`**:
  - `+11.11%` Fail-to-Pass (`+4 / -0` discordant repositories, exact McNemar $p = 0.125$), `-11.11%` False-Green, and **`+11.04%`** unconditional Post-Fix AST Mutant Kill (paired cluster-bootstrap 95% CI **`[+1.31%, +23.00%]`**, **$p = 0.0290$**).
- **`Flash + Proven v1.0.0` vs. `Flash + obra/superpowers` (Why Turn-1 Pre-Fix Ties at `96.43%` Complement Post-Fix `98.98%` in Section 5)**:
  - Both arms achieve `36/36` (`100.00%`) Fail-to-Pass, `0/36` (`0.00%`) False-Green, and `297/308` (`96.43%`) Post-Fix AST Mutant Kill (`31/36` vs. `30/36` full-kill repositories, paired bootstrap difference 95% CI `[-1.30%, +1.31%]`, $p = 0.798$), while `Proven v1.0.0` uses **`5.47×` fewer skill prompt tokens** (`~1,417` vs. `~7,755`), reads **`20.2%` fewer source lines** (`3,706` vs. `4,646`), and catches **`3.0×` more undocumented `.env.example` keys** (`6/9` vs. `2/9`).
  - **Structural Reason for Turn-1 Pre-Fix Parity**: Because every fault-seeded test suite intentionally fails on `buggy_target` (`exit != 0`) and modifying production code is prohibited, `proven verify-proof --mutation-check` stops at the failing baseline gate (`baseline_test_passed == false`) without executing post-green AST mutation-feedback iterations. Thus `297 / 308` (`96.43%`) measures **pure Turn-1 pre-fix assertion quality**. Once production code passes (evaluated across the `36` clean open-source repositories in Section 5), activating the post-green AST mutation gate separates `Flash + Proven` (**`98.98%`, `290/293`, `34/36` Titanium**) from `Flash + obra/superpowers` (**`87.71%`, `257/293`, `22/36` Titanium**, exact McNemar $p = 2.10 \times 10^{-9}$).

### 6.3 Breakdown Across the 4 Defect Regimes (`9` Repositories / `9` Languages per Regime)

| Defect Regime | Repos (`Mutants`) | `Flash Vanilla` (`F2P` / `False-Green` / `Post-Fix Kill` / `Lines`) | `Flash + obra/superpowers` (`F2P` / `False-Green` / `Post-Fix Kill` / `Lines`) | `Flash + Proven v1.0.0` (`F2P` / `False-Green` / `Post-Fix Kill` / `Lines`) |
| :--- | :---: | :---: | :---: | :---: |
| **`A_ORACLE`** *(Code-vs-Spec Discrepancy)* | `9` (`84`) | `5/9` (`55.6%`) F2P \| **`4/9` (`44.4%`) FG**<br/>`48/84` (`57.1%` uncond. / `100%` cond.)<br/>`429` lines (`47.7`/repo) | **`9/9` (`100.0%`) F2P** \| `0/9` FG<br/>**`84/84` (`100.0%`)**<br/>`572` lines (`63.6`/repo) | **`9/9` (`100.0%`) F2P** \| **`0/9` FG**<br/>**`84/84` (`100.0%`)**<br/>`508` lines (`56.4`/repo) |
| **`B_OMISSION`** *(Missing Sibling Guard)* | `9` (`60`) | **`9/9` (`100.0%`) F2P** \| `0/9` FG<br/>`58/60` (`96.7%`)<br/>`1,751` lines (`194.6`/repo) | **`9/9` (`100.0%`) F2P** \| `0/9` FG<br/>`57/60` (`95.0%`)<br/>`1,711` lines (`190.1`/repo) | **`9/9` (`100.0%`) F2P** \| **`0/9` FG**<br/>`57/60` (`95.0%`)<br/>**`1,512` lines (`168.0`/repo)** |
| **`C_SLICE_CONCURRENCY_ENV`** *(Slice + Race + `.env`)* | `9` (`74`) | **`9/9` (`100.0%`) F2P** \| `0/9` FG<br/>`67/74` (`90.5%`) \| Env `1/9`<br/>`799` lines (`88.8`/repo) | **`9/9` (`100.0%`) F2P** \| `0/9` FG<br/>`68/74` (`91.9%`) \| Env `2/9`<br/>`759` lines (`84.3`/repo) | **`9/9` (`100.0%`) F2P** \| **`0/9` FG**<br/>`67/74` (`90.5%`) \| **Env `6/9`**<br/>**`618` lines (`68.7`/repo)** |
| **`D_DEEP_COMPOUND`** *(Deep `42–55`-LOC Bugs)* | `9` (`90`) | **`9/9` (`100.0%`) F2P** \| `0/9` FG<br/>`90/90` (`100.0%`)<br/>`1,601` lines (`177.9`/repo) | **`9/9` (`100.0%`) F2P** \| `0/9` FG<br/>`88/90` (`97.8%`)<br/>`1,604` lines (`178.2`/repo) | **`9/9` (`100.0%`) F2P** \| **`0/9` FG**<br/>**`89/90` (`98.9%`)**<br/>**`1,068` lines (`118.7`/repo, `-33.4%`)** |

#### Three Structural Findings from Regime Analysis

1. **The Oracle Paradox (`36 / 36 = 100.0%` Pre-Fix AST Mutant Kill on Legalized Bugs)**: In `A_ORACLE`, `Flash Vanilla` read the target source file directly without searching `docs/domain-policies.md` on `4 / 9` (`44.4%`) repositories (`02_rs_oracle_sla` asserting buggy `1500` bps instead of `1000` bps; `04_js_oracle_shipping` asserting buggy `18%` surcharge instead of `12%`; `06_c_oracle_crc_frame` asserting buggy `2048`-byte cap instead of `1024`; `08_java_oracle_loyalty` asserting buggy `45%` redemption cap instead of `30%`). When evaluated with `proven verify-proof --mutation-check` on `buggy_target`, those 4 False-Green suites killed **`36 / 36` (`100.0%`)** of pre-fix AST mutants (`10/10`, `10/10`, `6/6`, `10/10`) while killing `0 / 36` (`0.0%`) on ground-truth `clean_target`. This confirms empirically that AST mutation testing alone rewards bug-legalizing tests with `100%` kill scores unless preceded by Phase A Blind Spec Discovery (`proven context --blind`).
2. **Barrier Synchronization vs. Uncoordinated Thread Spawning (`27_cs_conc_quota`)**: On `27_cs_conc_quota` (`QuotaBucket.cs::TryConsumeQuota`, TOCTOU double-spend race), `Flash + obra/superpowers` wrote a 2-thread test without a synchronization barrier (`t1.Start(); t2.Start();`). Under multi-worker CPU load, `t1` finishes before `t2` enters the critical section, causing the test to pass falsely on broken code unless executed on an uncontended CPU. Guided by `concurrency_risk.recommended_harness` from `proven context`, `Flash + Proven v1.0.0` emitted an 8-worker `System.Threading.Barrier` (`barrier.SignalAndWait()`) harness that reproduces the race deterministically even under heavy CPU contention.
3. **Unprompted `.env.example` Configuration Drift (`config_audit`)**: Because `proven context` embeds `config_audit` (`undocumented_in_env_example`) directly in its Phase B JSON payload, `Flash + Proven v1.0.0` flagged **`6 / 9` (`66.7%`)** undocumented environment variable keys across Regime C (`ORDER_SIGNING_SECRET`, `LEDGER_HMAC_SECRET`, `INVOICE_KMS_KEY_ID`, `TRANSPORT_PSK_HEX`, `KV_WAL_SYNC_MODE`, `QUOTA_REDIS_PREFIX`) without any user prompt mentioning `.env` files, compared to `2 / 9` (`22.2%`) for `Flash + obra/superpowers` and `1 / 9` (`11.1%`) for `Flash Vanilla`.

### 6.4 Symmetric 9-Language Breakdown (`4` Repositories × `9` Languages = `36` Repositories, `308` Ground-Truth Mutants)

| Language | Repos | Ground-Truth Mutants | `Flash Vanilla` (`F2P` / `FG` / `Post-Fix Kill`) | `Flash + obra/superpowers` (`F2P` / `FG` / `Post-Fix Kill`) | `Flash + Proven v1.0.0` (`F2P` / `FG` / `Post-Fix Kill`) |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **Python** | `4` | `32` | F2P `4/4`, FG `0/4`, Kill `32/32` (`100.0%`) | F2P `4/4`, FG `0/4`, Kill `32/32` (`100.0%`) | **F2P `4/4`, FG `0/4`, Kill `32/32` (`100.0%`)** |
| **Rust** | `4` | `31` | F2P `3/4`, **FG `1/4`**, Kill `21/31` (`67.7%`) | F2P `4/4`, FG `0/4`, Kill `30/31` (`96.8%`) | **F2P `4/4`, FG `0/4`, Kill `31/31` (`100.0%`)** |
| **Go** | `4` | `37` | F2P `4/4`, FG `0/4`, Kill `35/37` (`94.6%`) | F2P `4/4`, FG `0/4`, Kill `34/37` (`91.9%`) | **F2P `4/4`, FG `0/4`, Kill `34/37` (`91.9%`)** |
| **JavaScript** | `4` | `35` | F2P `3/4`, **FG `1/4`**, Kill `24/35` (`68.6%`) | F2P `4/4`, FG `0/4`, Kill `34/35` (`97.1%`) | **F2P `4/4`, FG `0/4`, Kill `34/35` (`97.1%`)** |
| **TypeScript** | `4` | `36` | F2P `4/4`, FG `0/4`, Kill `36/36` (`100.0%`) | F2P `4/4`, FG `0/4`, Kill `36/36` (`100.0%`) | **F2P `4/4`, FG `0/4`, Kill `35/36` (`97.2%`)** |
| **C** | `4` | `29` | F2P `3/4`, **FG `1/4`**, Kill `23/29` (`79.3%`) | F2P `4/4`, FG `0/4`, Kill `28/29` (`96.5%`) | **F2P `4/4`, FG `0/4`, Kill `29/29` (`100.0%`)** |
| **C++** | `4` | `37` | F2P `4/4`, FG `0/4`, Kill `37/37` (`100.0%`) | F2P `4/4`, FG `0/4`, Kill `37/37` (`100.0%`) | **F2P `4/4`, FG `0/4`, Kill `37/37` (`100.0%`)** |
| **Java** | `4` | `35` | F2P `3/4`, **FG `1/4`**, Kill `22/35` (`62.9%`) | F2P `4/4`, FG `0/4`, Kill `32/35` (`91.4%`) | **F2P `4/4`, FG `0/4`, Kill `32/35` (`91.4%`)** |
| **C#** | `4` | `36` | F2P `4/4`, FG `0/4`, Kill `33/36` (`91.7%`) | F2P `4/4`, FG `0/4`, Kill `34/36` (`94.4%`) | **F2P `4/4`, FG `0/4`, Kill `33/36` (`91.7%`)** |
| **Total (All 9 Languages)** | **`36`** | **`308`** | **F2P `32/36` (`88.9%`), FG `4/36` (`11.1%`), Kill `263/308` (`85.4%`)** | **F2P `36/36` (`100.0%`), FG `0/36`, Kill `297/308` (`96.4%`)** | **F2P `36/36` (`100.0%`), FG `0/36`, Kill `297/308` (`96.4%`)** |

### 6.5 Prompt-Framing & Repository-Scale Ablation (`Cohort 1` Explicit-Audit Micro-Repos vs. `Cohort 2` Natural-Prompt Multi-File Repos, `216` Total Runs)

To isolate why False-Green Bug Legalization occurs in real developer workflows, we also ran all `3` `Gemini Flash` configurations across a `36`-repository upper-bound control condition (**`Cohort 1`**, `108` runs archived under `ablation_explicit_audit_micro36` in [`docs/ab_fault_seeded_flash_36.json`](ab_fault_seeded_flash_36.json)) where each repository was shrunk to `~49` LOC with specifications placed in a root `README.md` and the user prompt explicitly listed a 6-category defect-audit rubric (*spec mismatch, omitted sibling guards, slice contracts, concurrency races, `.env.example` drift*):

| Cohort Condition (`108` Runs Each, `216` Total) | `Flash Vanilla` (`F2P` / `FG` / `Post-Fix Kill` / `Env` / `Mean Lines`) | `Flash + obra/superpowers` (`F2P` / `FG` / `Post-Fix Kill` / `Env` / `Mean Lines`) | `Flash + Proven v1.0.0` (`F2P` / `FG` / `Post-Fix Kill` / `Env` / `Mean Lines`) |
| :--- | :---: | :---: | :---: |
| **`Cohort 1` — Explicit Defect-Audit Prompt + `~49`-LOC Micro-Repos** *(Root `README.md` Spec)* | `36/36` (`100.0%`) \| `0/36` FG<br/>`297/308` (`96.43%`) \| Env `9/9`<br/>`49.2` lines/repo | `35/36` (`97.22%`, `1` broken Java test) \| `0/36` FG<br/>`284/308` (`92.21%`) \| Env `9/9`<br/>`45.0` lines/repo | **`36/36` (`100.0%`)** \| **`0/36` FG**<br/>**`294/308` (`95.45%`)** \| **Env `9/9`**<br/>`48.5` lines/repo |
| **`Cohort 2` (Primary) — Natural Prompt (*"Write tests for X"*) + `650–1,278`-LOC Multi-File Repos** | `32/36` (`88.89%`) \| **`4/36` (`11.11%`) FG** (`44.4%` in `A_ORACLE`)<br/>`263/308` (`85.39%`) \| Env `1/9`<br/>`127.2` lines/repo | **`36/36` (`100.0%`)** \| `0/36` FG<br/>**`297/308` (`96.43%`)** \| Env `2/9`<br/>`129.1` lines/repo | **`36/36` (`100.0%`)** \| **`0/36` FG**<br/>**`297/308` (`96.43%`)** \| **Env `6/9`**<br/>**`102.9` lines/repo (`-19.1%`)** |

Contrasting `Cohort 1` (`0%` Vanilla False-Green, `9/9` Vanilla `.env` drift) with `Cohort 2` (`44.4%` Vanilla Regime A False-Green, `1/9` Vanilla `.env` drift) shows that `Gemini Flash` does not lack reasoning ability when told what defect class to inspect in a 49-line repo; rather, **False-Green Bug Legalization and missed `.env` drift emerge specifically under natural developer prompts on multi-file codebases where specifications and `.env` contracts live outside the target file**.

### 6.6 All 36 Multi-File Fault-Seeded Repositories × 3 Arms Per-Repository Ledger (`308` Ground-Truth AST Mutants)

*Note: `F2P` denotes `Fail-to-Pass` (test suite fails on `buggy_target` and passes on `clean_target` with `0` production file edits); `FALSE-GREEN` denotes a suite that passed (`exit == 0`) on `buggy_target` (with pre-fix AST mutant kill on `buggy_target` shown in parentheses); `TIT` denotes `100%` of ground-truth AST mutants killed on `clean_target`; `SIL` denotes `≥ 70%` killed (`SILVER`); `Env Drift (V/O/P)` records unprompted `.env.example` config drift detection (`Y` = reported, `N` = missed) for `Vanilla / Obra / Proven`.*

| # | Repo ID | Regime | Lang | Files (`LOC`) | Target | Mut | `Flash Vanilla` | `Flash + obra/superpowers` | `Flash + Proven v1.0.0` | Env Drift (`V/O/P`) |
| :---: | :--- | :--- | :---: | :---: | :--- | :---: | :---: | :---: | :---: | :---: |
| 1 | `01_py_oracle_rebate` | `A_ORACLE` | Python | `16` (`725`) | `rebate.py::compute_partner_rebate` | 10 | `F2P` `10/10` `TIT` | `F2P` `10/10` `TIT` | **`F2P` `10/10` `TIT`** | — |
| 2 | `02_rs_oracle_sla` | `A_ORACLE` | Rust | `17` (`890`) | `sla.rs::compute_sla_credit_bps` | 10 | `FALSE-GREEN` (`0/10`; pre-fix `10/10`) | `F2P` `10/10` `TIT` | **`F2P` `10/10` `TIT`** | — |
| 3 | `03_go_oracle_ratelimit` | `A_ORACLE` | Go | `16` (`1119`) | `quota.go::ComputeBurstTokens` | 9 | `F2P` `9/9` `TIT` | `F2P` `9/9` `TIT` | **`F2P` `9/9` `TIT`** | — |
| 4 | `04_js_oracle_shipping` | `A_ORACLE` | JavaScript | `16` (`736`) | `shipping.js::calculateFreightCents` | 10 | `FALSE-GREEN` (`0/10`; pre-fix `10/10`) | `F2P` `10/10` `TIT` | **`F2P` `10/10` `TIT`** | — |
| 5 | `05_ts_oracle_jwt_leeway` | `A_ORACLE` | TypeScript | `16` (`721`) | `token.ts::validateClockSkew` | 10 | `F2P` `10/10` `TIT` | `F2P` `10/10` `TIT` | **`F2P` `10/10` `TIT`** | — |
| 6 | `06_c_oracle_crc_frame` | `A_ORACLE` | C | `16` (`724`) | `frame.c::validate_packet_length` | 6 | `FALSE-GREEN` (`0/6`; pre-fix `6/6`) | `F2P` `6/6` `TIT` | **`F2P` `6/6` `TIT`** | — |
| 7 | `07_cpp_oracle_margin` | `A_ORACLE` | C++ | `16` (`725`) | `margin.cpp::compute_maintenance_margin` | 9 | `F2P` `9/9` `TIT` | `F2P` `9/9` `TIT` | **`F2P` `9/9` `TIT`** | — |
| 8 | `08_java_oracle_loyalty` | `A_ORACLE` | Java | `15` (`650`) | `LoyaltyEngine.java::calculateRedemptionCents` | 10 | `FALSE-GREEN` (`0/10`; pre-fix `10/10`) | `F2P` `10/10` `TIT` | **`F2P` `10/10` `TIT`** | — |
| 9 | `09_cs_oracle_payroll` | `A_ORACLE` | C# | `15` (`665`) | `OvertimeCalc.cs::CalculateWeeklyOvertimePay` | 10 | `F2P` `10/10` `TIT` | `F2P` `10/10` `TIT` | **`F2P` `10/10` `TIT`** | — |
| 10 | `10_py_omission_wallet` | `B_OMISSION` | Python | `15` (`833`) | `wallet.py::issue_instant_refund` | 7 | `F2P` `7/7` `TIT` | `F2P` `7/7` `TIT` | **`F2P` `7/7` `TIT`** | — |
| 11 | `11_rs_omission_vault` | `B_OMISSION` | Rust | `16` (`1002`) | `vault.rs::export_wrapped_key` | 6 | `F2P` `6/6` `TIT` | `F2P` `6/6` `TIT` | **`F2P` `6/6` `TIT`** | — |
| 12 | `12_go_omission_tenant` | `B_OMISSION` | Go | `15` (`1278`) | `ops.go::PurgeTenantCache` | 10 | `F2P` `8/10` `SIL` | `F2P` `7/10` `SIL` | **`F2P` `7/10` `SIL`** | — |
| 13 | `13_js_omission_coupon` | `B_OMISSION` | JavaScript | `15` (`848`) | `coupons.js::redeemGiftVoucher` | 5 | `F2P` `5/5` `TIT` | `F2P` `5/5` `TIT` | **`F2P` `5/5` `TIT`** | — |
| 14 | `14_ts_omission_webhook` | `B_OMISSION` | TypeScript | `15` (`841`) | `webhooks.ts::dispatchPayoutWebhook` | 6 | `F2P` `6/6` `TIT` | `F2P` `6/6` `TIT` | **`F2P` `6/6` `TIT`** | — |
| 15 | `15_c_omission_dma` | `B_OMISSION` | C | `15` (`853`) | `dma_buf.c::dma_copy_slice` | 7 | `F2P` `7/7` `TIT` | `F2P` `7/7` `TIT` | **`F2P` `7/7` `TIT`** | — |
| 16 | `16_cpp_omission_orderbook` | `B_OMISSION` | C++ | `15` (`845`) | `orderbook.cpp::amend_limit_order` | 8 | `F2P` `8/8` `TIT` | `F2P` `8/8` `TIT` | **`F2P` `8/8` `TIT`** | — |
| 17 | `17_java_omission_prescription` | `B_OMISSION` | Java | `14` (`771`) | `PharmacyDispense.java::dispenseRefill` | 5 | `F2P` `5/5` `TIT` | `F2P` `5/5` `TIT` | **`F2P` `5/5` `TIT`** | — |
| 18 | `18_cs_omission_escrow` | `B_OMISSION` | C# | `14` (`788`) | `EscrowService.cs::ReleaseMilestone` | 6 | `F2P` `6/6` `TIT` | `F2P` `6/6` `TIT` | **`F2P` `6/6` `TIT`** | — |
| 19 | `19_py_slice_sqlite_order` | `C_SLICE_CONCURRENCY_ENV` | Python | `17` (`751`) | `order_repo.py::insert_settled_order` | 5 | `F2P` `5/5` `TIT` | `F2P` `5/5` `TIT` | **`F2P` `5/5` `TIT`** | `ORDER_SIGNING_SECRET` (`Y/Y/Y`) |
| 20 | `20_rs_conc_inventory` | `C_SLICE_CONCURRENCY_ENV` | Rust | `17` (`922`) | `inventory.rs::reserve_stock` | 5 | `F2P` `5/5` `TIT` | `F2P` `5/5` `TIT` | **`F2P` `5/5` `TIT`** | `INVENTORY_AUDIT_SALT` (`N/N/N`) |
| 21 | `21_go_conc_wallet` | `C_SLICE_CONCURRENCY_ENV` | Go | `16` (`1159`) | `ledger.go::TryDebitBalance` | 8 | `F2P` `8/8` `TIT` | `F2P` `8/8` `TIT` | **`F2P` `8/8` `TIT`** | `WALLET_AUDIT_TOPIC` (`N/N/N`) |
| 22 | `22_js_slice_json_ledger` | `C_SLICE_CONCURRENCY_ENV` | JavaScript | `17` (`761`) | `ledger_store.js::applySettlementJson` | 10 | `F2P` `9/10` `SIL` | `F2P` `9/10` `SIL` | **`F2P` `9/10` `SIL`** | `LEDGER_HMAC_SECRET` (`N/N/Y`) |
| 23 | `23_ts_slice_invoice_serde` | `C_SLICE_CONCURRENCY_ENV` | TypeScript | `17` (`745`) | `invoice_repo.ts::deserializeInvoiceTotal` | 10 | `F2P` `10/10` `TIT` | `F2P` `10/10` `TIT` | **`F2P` `10/10` `TIT`** | `INVOICE_KMS_KEY_ID` (`N/N/Y`) |
| 24 | `24_c_slice_packet_codec` | `C_SLICE_CONCURRENCY_ENV` | C | `17` (`756`) | `codec.c::decode_frame_payload_len` | 6 | `F2P` `6/6` `TIT` | `F2P` `6/6` `TIT` | **`F2P` `6/6` `TIT`** | `TRANSPORT_PSK_HEX` (`N/N/Y`) |
| 25 | `25_cpp_slice_kv_wal` | `C_SLICE_CONCURRENCY_ENV` | C++ | `17` (`757`) | `wal_reader.cpp::parse_wal_record_seq` | 10 | `F2P` `10/10` `TIT` | `F2P` `10/10` `TIT` | **`F2P` `10/10` `TIT`** | `KV_WAL_SYNC_MODE` (`N/N/Y`) |
| 26 | `26_java_conc_seatmap` | `C_SLICE_CONCURRENCY_ENV` | Java | `15` (`688`) | `SeatInventory.java::tryAllocateSeats` | 10 | `F2P` `7/10` `SIL` | `F2P` `7/10` `SIL` | **`F2P` `7/10` `SIL`** | `BOOKING_CLUSTER_REGION` (`N/N/N`) |
| 27 | `27_cs_conc_quota` | `C_SLICE_CONCURRENCY_ENV` | C# | `15` (`703`) | `QuotaBucket.cs::TryConsumeQuota` | 10 | `F2P` `7/10` `SIL` | `F2P` `8/10` `SIL` | **`F2P` `7/10` `SIL`** | `QUOTA_REDIS_PREFIX` (`N/Y/Y`) |
| 28 | `28_py_deep_access_matrix` | `D_DEEP_COMPOUND` | Python | `15` (`832`) | `access_matrix.py::evaluate_access_policy` | 10 | `F2P` `10/10` `TIT` | `F2P` `10/10` `TIT` | **`F2P` `10/10` `TIT`** | — |
| 29 | `29_rs_deep_packet_flags` | `D_DEEP_COMPOUND` | Rust | `16` (`990`) | `packet_flags.rs::encode_control_word` | 10 | `F2P` `10/10` `TIT` | `F2P` `9/10` `SIL` | **`F2P` `10/10` `TIT`** | — |
| 30 | `30_go_deep_retry_backoff` | `D_DEEP_COMPOUND` | Go | `15` (`1249`) | `policy.go::ComputeRetryDelayMs` | 10 | `F2P` `10/10` `TIT` | `F2P` `10/10` `TIT` | **`F2P` `10/10` `TIT`** | — |
| 31 | `31_js_deep_risk_scorer` | `D_DEEP_COMPOUND` | JavaScript | `15` (`843`) | `risk_scorer.js::computeTransactionRisk` | 10 | `F2P` `10/10` `TIT` | `F2P` `10/10` `TIT` | **`F2P` `10/10` `TIT`** | — |
| 32 | `32_ts_deep_semver_gate` | `D_DEEP_COMPOUND` | TypeScript | `15` (`833`) | `semver_gate.ts::classifyUpgradeImpact` | 10 | `F2P` `10/10` `TIT` | `F2P` `10/10` `TIT` | **`F2P` `9/10` `SIL`** | — |
| 33 | `33_c_deep_bitstream` | `D_DEEP_COMPOUND` | C | `15` (`830`) | `bitstream.c::pack_sensor_sample` | 10 | `F2P` `10/10` `TIT` | `F2P` `9/10` `SIL` | **`F2P` `10/10` `TIT`** | — |
| 34 | `34_cpp_deep_fee_schedule` | `D_DEEP_COMPOUND` | C++ | `15` (`837`) | `fee_schedule.cpp::compute_clearing_fee_cents` | 10 | `F2P` `10/10` `TIT` | `F2P` `10/10` `TIT` | **`F2P` `10/10` `TIT`** | — |
| 35 | `35_java_deep_audit_mask` | `D_DEEP_COMPOUND` | Java | `14` (`761`) | `AuditMask.java::computeEffectivePermissions` | 10 | `F2P` `10/10` `TIT` | `F2P` `10/10` `TIT` | **`F2P` `10/10` `TIT`** | — |
| 36 | `36_cs_deep_invoice_state` | `D_DEEP_COMPOUND` | C# | `14` (`777`) | `InvoiceWorkflow.cs::TransitionInvoice` | 10 | `F2P` `10/10` `TIT` | `F2P` `10/10` `TIT` | **`F2P` `10/10` `TIT`** | — |
