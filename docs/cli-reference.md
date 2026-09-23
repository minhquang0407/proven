# SoftGNN Advisor — CLI Command Reference

Comprehensive documentation for all `softgnn` command-line interface (CLI) commands, arguments, and options.

---

## Table of Contents
1. [Global Options](#global-options)
2. [🤖 Agent Commands (`softgnn agent`)](#1--agent-commands-softgnn-agent)
3. [🖥️ Interactive Web Dashboard](#2-interactive-web-dashboard)
4. [📊 PR Impact & Analysis](#3--pr-impact--analysis)
5. [🛠️ Automated Test Planning & Generation](#4-automated-test-planning--generation)
6. [🩺 Diagnostics, Health Check & Triage](#5-diagnostics-health-check--triage)
7. [🧠 Knowledge Graph & AI Engine Pipeline](#6-knowledge-graph--ai-engine-pipeline)

---

## Global Options

```bash
softgnn --help
```

Displays available top-level commands and version information.

---

## 1. 🤖 Agent Commands (`softgnn agent`)

These commands are tailored for Coding Agents (Antigravity, Claude Code, Cursor, Codex) to inspect changes, query code context, verify runtime proof, and check mutations without requiring third-party LLM API keys.

### `softgnn agent scan`
Scans Git changes or uncommitted modifications to detect functions that were altered but lack runtime test verification.

```bash
softgnn agent scan [OPTIONS]
```

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `--repo` | Path | `.` | Target repository root path. |
| `--base` | String | `main` | Base Git branch/commit for comparison. |
| `--head` | String | `HEAD` | Target Git branch/commit for comparison. |
| `--lang` | Choice | `auto` | Language mode (`auto`, `python`, `typescript`, `go`, etc.). |
| `--fan-out` | Flag | `False` | Emits structured JSON task payloads for parallel Sub-Agent Swarms. |

**Example:**
```bash
softgnn agent scan --base main --head HEAD
softgnn agent scan --fan-out
```

---

### `softgnn agent context`
Fetches exact AST context, line ranges, callers, and import dependencies for a specific target symbol.

```bash
softgnn agent context --target TARGET [OPTIONS]
```

| Option | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `--target` | String | **Yes** | Target symbol identifier (e.g., `FUNC:my_module.calculate_price`). |
| `--repo` | Path | No | Repository root path (default: `.`). |

**Example:**
```bash
softgnn agent context --target "FUNC:softgnn_advisor.core.agent_service.AgentService.verify_proof"
```

---

### `softgnn agent verify`
Verifies whether a test file actually executes the target function at runtime, diagnoses early branches (Self-Healing), and optionally tests against AST micro-mutations.

```bash
softgnn agent verify --target TARGET --test TEST_FILE [OPTIONS]
```

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `--target` | String | (Required) | Target function identifier. |
| `--test` | Path | (Required) | Path to test file (e.g., `tests/test_service.py`). |
| `--repo` | Path | `.` | Repository root path. |
| `--coverage-file` | Path | `None` | Optional external LCOV (`lcov.info`) or Go coverage file. |
| `--mutation-check` | Flag | `False` | **PRO**: Injects AST mutations to kill weak assertions. |
| `--max-mutants` | Int | `5` | Maximum number of AST mutants to generate. |

**Example:**
```bash
# Basic Runtime Proof verification:
softgnn agent verify --target "FUNC:checkout" --test "tests/test_checkout.py"

# PRO Titanium Proof verification:
softgnn agent verify --target "FUNC:checkout" --test "tests/test_checkout.py" --mutation-check
```

---

### `softgnn agent refresh`
Refreshes the AST index and runtime coverage map across the codebase.

```bash
softgnn agent refresh [--repo PATH]
```

---

## 2. 🖥️ Interactive Web Dashboard

Launch the Cytoscape.js web-based knowledge graph visualizer and control center.

```bash
softgnn dashboard --project PROJECT [OPTIONS]
```

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `--project` | String | **Required** | Project identifier name. |
| `--port` | Int | `8765` | Web server port (binds to `127.0.0.1` for security). |
| `--open` | Flag | `False` | Automatically opens default browser upon start. |

**Example:**
```bash
softgnn dashboard --project my-app --open
softgnn dashboard --project my-app --port 9000
```

---

## 3. 📊 PR Impact & Analysis

### `softgnn scan`
Analyzes repo changes against git history or filesystem snapshots and identifies coverage gaps.

```bash
softgnn scan --project PROJECT [OPTIONS]
```

| Option | Description |
| :--- | :--- |
| `--project` | Project name. |
| `--source` | Change source mode: `auto` (default), `git`, `reflog`, or `filesystem`. |
| `--base` / `--head` | Git refs for diff comparison. |

---

### `softgnn pr-scan`
Deep PR evaluation: computes impacted functions, blast radius, developer reviewer recommendations, and risk scores.

```bash
softgnn pr-scan --project PROJECT [OPTIONS]
```

| Option | Description |
| :--- | :--- |
| `--project` | Project name. |
| `--report` | Generates a standalone HTML impact report. |
| `--open-report` | Automatically opens the HTML report in your browser. |

---

### `softgnn impact`
Queries the blast radius of a specific function or file in the dependency graph.

```bash
softgnn impact --project PROJECT TARGET
```

**Example:**
```bash
softgnn impact --project my-app FUNC:process_payment
```

---

### `softgnn report`
Prints or opens the latest generated HTML summary report.

```bash
softgnn report --project PROJECT [--open/--no-open]
```

---

## 4. 🛠️ Automated Test Planning & Generation

SoftGNN provides a deterministic, safe pipeline for generating test files with boundary markers and automatic rollback if tests fail.

### `softgnn setup`
Builds the initial code knowledge graph and records a filesystem baseline snapshot.

```bash
softgnn setup /path/to/repo --project PROJECT
```

---

### `softgnn plan`
Proposes new pytest test blocks for impacted functions without modifying source code.

```bash
softgnn plan --project PROJECT [OPTIONS]
```

| Option | Description |
| :--- | :--- |
| `--only-file` | Scopes plan generation to targets within a specific file. |
| `--max-targets` | Maximum number of missing-coverage targets to plan for (default: 3). |

---

### `softgnn apply`
Applies a previously saved plan: patches test files transactionally, runs pytest verification, attempts repair on failure, rolls back if unresolvable, and updates the runtime map.

```bash
softgnn apply --project PROJECT [OPTIONS]
```

---

### `softgnn generate`
Convenience shortcut that runs `scan -> plan -> apply` in a single execution.

```bash
softgnn generate --project PROJECT [OPTIONS]
```

| Option | Description |
| :--- | :--- |
| `--only-file` | Generate tests only for a specific source file. |
| `--target` | Generate tests for a specific function symbol. |
| `--yes` | Automatically accept safe diff fallbacks without prompt. |

---

## 5. 🩺 Diagnostics, Health Check & Triage

### `softgnn doctor`
Validates environment dependencies, project metadata, graph data integrity, and model configurations.

```bash
softgnn doctor --project PROJECT
```

---

### `softgnn map`
Runs full pytest test suite and builds the runtime execution map (`Test -> executes -> Function`).

```bash
softgnn map --project PROJECT
```

---

### `softgnn triage`
Recommends the best-suited engineers for a bug based on commit history and code graph ownership.

```bash
softgnn triage --project PROJECT "Brief description of the bug or error"
```

---

## 6. 🧠 Knowledge Graph & AI Engine Pipeline

Commands for low-level graph extraction and Heterogeneous Graph Transformer (HGT) model training.

### `softgnn etl`
Parses codebase AST and Git commits into NetworkX graph representations.

```bash
softgnn etl --project PROJECT [--path PATH]
```

---

### `softgnn train`
Trains the local PyTorch Geometric HGT model for link prediction and impact scoring.

```bash
softgnn train --project PROJECT
```

---

### `softgnn prepare`
Runs the complete onboarding pipeline: `ETL -> Snapshot -> Train`.

```bash
softgnn prepare --project PROJECT [--path PATH] [--skip-train]
```

---

### `softgnn inspect`
Prints graph statistics (node counts, edge types, density, isolated components).

```bash
softgnn inspect --project PROJECT
```
