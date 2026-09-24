# Proven — Complete CLI Command Reference

This is the comprehensive, definitive reference guide for all commands, arguments, options, and execution modes in **Proven v1.0.0 (PRO Edition)**.

> **Note:** The primary CLI command is `proven` (e.g., `proven agent scan`, `proven setup`). The `softgnn` command is retained as a fully compatible alias throughout all subcommands.

---

## Table of Contents
1. [Execution Modes: Agent Mode vs. Standalone Mode](#execution-modes)
2. [Configuring LLM Providers & API Keys (For Standalone Mode)](#configuring-llm-providers--api-keys)
3. [🤖 Agent Commands (`softgnn agent`)](#1--agent-commands-softgnn-agent)
4. [🚀 Standalone Test Generation (Without Agent)](#2--standalone-test-generation-without-agent)
5. [📊 PR Impact & Blast Radius Analysis](#3--pr-impact--blast-radius-analysis)
6. [🖥️ Interactive Web Dashboard](#4-interactive-web-dashboard)
7. [🩺 Diagnostics, Mapping & Bug Triage](#5-diagnostics-mapping--bug-triage)
8. [🧠 Graph AI Core & HGT Training Pipeline](#6-graph-ai-core--hgt-training-pipeline)

---

## Execution Modes

SoftGNN Advisor is engineered to operate seamlessly in two distinct environments:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 🤖 MODE A: AI CODING AGENT SKILL (Recommended for IDEs)                     │
│ - Supported Agents: Antigravity, Claude Code, Cursor, Codex                │
│ - API Key Requirement: ZERO API KEYS (Agent is the LLM author)             │
│ - Commands: `softgnn agent scan`, `context`, `verify-proof`, `impact`, etc. │
│ - Core Role: SoftGNN provides AST Ground Truth, Runtime Trace & Mutation    │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│ 💻 MODE B: STANDALONE DEVELOPER CLI (Run directly in terminal)              │
│ - Supported LLMs: Google Gemini, OpenAI-compatible APIs, Local Ollama / vLLM│
│ - API Key Requirement: Required if using LLM generation (or use templates)  │
│ - Commands: `softgnn setup`, `scan`, `plan`, `apply`, `generate`            │
│ - Core Role: SoftGNN calls LLM itself, validates JSON, patches test files,  │
│              runs pytest, auto-repairs on failure, or rolls back.           │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Configuring LLM Providers & API Keys

*(Required **only** for Standalone Mode `softgnn plan` / `softgnn generate`. **Not** required when using Agent Mode).*

SoftGNN supports three generation strategies:
- `auto` (default): Attempts LLM semantic generation first; gracefully falls back to deterministic AST templates if LLM is unavailable.
- `llm`: Strictly requires a configured LLM provider; fails if unreachable.
- `template`: Uses deterministic, offline AST templates (zero network calls, zero API keys).

### Option 1: Google Gemini (Recommended)

Set the following environment variables:

**Linux / macOS (Bash / Zsh):**
```bash
export SOFTGNN_LLM_PROVIDER="gemini"
export SOFTGNN_LLM_MODEL="gemini-2.5-flash"
export SOFTGNN_LLM_API_KEY="AIzaSyYourGeminiApiKeyHere..."
```

**Windows (PowerShell):**
```powershell
$env:SOFTGNN_LLM_PROVIDER="gemini"
$env:SOFTGNN_LLM_MODEL="gemini-2.5-flash"
$env:SOFTGNN_LLM_API_KEY="AIzaSyYourGeminiApiKeyHere..."
```

### Option 2: OpenAI / Local Ollama / vLLM / DeepSeek

Compatible with any standard OpenAI `/v1/chat/completions` endpoint:

**Linux / macOS:**
```bash
export SOFTGNN_LLM_PROVIDER="openai-compatible"
export SOFTGNN_LLM_BASE_URL="http://localhost:11434/v1"   # For Ollama
export SOFTGNN_LLM_MODEL="qwen2.5-coder:7b"               # Or gpt-4o, deepseek-coder
export SOFTGNN_LLM_API_KEY="optional-or-sk-..."
```

**Windows (PowerShell):**
```powershell
$env:SOFTGNN_LLM_PROVIDER="openai-compatible"
$env:SOFTGNN_LLM_BASE_URL="http://localhost:11434/v1"
$env:SOFTGNN_LLM_MODEL="qwen2.5-coder:7b"
$env:SOFTGNN_LLM_API_KEY="optional-or-sk-..."
```

---

## 1. 🤖 Agent Commands (`softgnn agent`)

Tailored for Coding Agents (Antigravity, Claude Code, Cursor) returning clean, structured JSON payloads.

### `softgnn agent scan`
Detects changed functions lacking runtime test execution proof.

```bash
softgnn agent scan [OPTIONS]
```

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `--repo` | Path | `.` | Target repository path. |
| `--base` | String | `main` | Base Git branch/commit for diff. |
| `--head` | String | `HEAD` | Target Git branch/commit for diff. |
| `--source` | Choice | `auto` | Diff source: `auto`, `git`, `filesystem`, `full-scan`. |
| `--lang` | String | `auto` | Language override (`python`, `typescript`, `go`, etc.). |
| `--fan-out` | Flag | `False` | Emits structured JSON payloads for parallel Sub-Agent Swarms. |
| `--json / --no-json`| Flag | `True` | Output formatted JSON for agent parsing. |

**Example:**
```bash
softgnn agent scan --base main --head HEAD
softgnn agent scan --fan-out
```

---

### `softgnn agent context`
Fetches exact AST implementation, signature, imports, callers, callees, and test fixture conventions.

```bash
softgnn agent context --target TARGET [OPTIONS]
```

| Option | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `--target` | String | **Yes** | Target symbol identifier (e.g. `FUNC:billing.calc_fee`). |
| `--file` | Path | No | Source file path if known. |
| `--repo` | Path | No | Repository root path (default: `.`). |

**Example:**
```bash
softgnn agent context --target "FUNC:softgnn_advisor.core.agent_service.AgentService.verify_proof"
```

---

### `softgnn agent verify-proof`
Validates that a written test actually executes target bytecode at runtime, diagnoses early exits, and checks micro-mutations.

```bash
softgnn agent verify-proof --target TARGET --test TEST_FILE [OPTIONS]
```

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `--target` | String | **Required**| Target symbol identifier. |
| `--test` | Path | `None` | Path to test file (e.g. `tests/test_fee.py`). |
| `--mutation-check` | Flag | `False` | **PRO**: Injects AST mutations to kill weak assertions (**Titanium Proof**). |
| `--max-mutants` | Int | `5` | Maximum number of AST mutants to generate. |
| `--lcov` | Path | `None` | Path to `lcov.info` or `coverage.out` for non-Python repos. |
| `--test-cmd` | String | `None` | Custom execution command (e.g. `npm test -- --coverage`). |

**Example:**
```bash
# Basic Runtime Proof:
softgnn agent verify-proof --target "FUNC:checkout" --test "tests/test_checkout.py"

# PRO Titanium Grade Proof (Mutation Verified):
softgnn agent verify-proof --target "FUNC:checkout" --test "tests/test_checkout.py" --mutation-check
```

---

### `softgnn agent refresh`
Refreshes the AST code graph and pytest runtime coverage edges.

```bash
softgnn agent refresh [--pytest-args "tests"]
```

---

### `softgnn agent impact`
Queries direct downstream callers and latent HGT blast radius risks for a symbol.

```bash
softgnn agent impact --target TARGET [OPTIONS]
```

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `--target` | String | **Required**| Target symbol ID (e.g. `FUNC:process_payment`). |
| `--mode` | Choice | `hybrid` | Analysis mode (`hybrid`, `graph`, `gnn`). |
| `--threshold` | Float | `0.1` | Score cutoff threshold. |

**Example:**
```bash
softgnn agent impact --target "FUNC:checkout" --mode hybrid
```

---

### `softgnn agent triage`
Recommends expert developers and related source files for a bug description or PR change.

```bash
softgnn agent triage "QUERY_DESCRIPTION" [OPTIONS]
```

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `QUERY` | String | **Required**| Bug description or PR summary. |
| `--max-devs` | Int | `3` | Maximum recommended developers. |
| `--max-files` | Int | `5` | Maximum related files. |

**Example:**
```bash
softgnn agent triage "Database connection timeout under heavy traffic"
```

---

### `softgnn agent train`
Triggers offline HGT Graph AI training for the current project.

```bash
softgnn agent train [--project PROJECT]
```

---

## 2. 🚀 Standalone Test Generation (Without Agent)

In Standalone Mode, developers can run SoftGNN directly from the terminal. SoftGNN acts as the orchestrator: scanning changes, consulting LLMs, patching test files, running pytest, and rolling back failed tests automatically.

### `softgnn setup`
Builds the initial code knowledge graph and records a filesystem baseline snapshot. Run this once when introducing a repo to SoftGNN.

```bash
softgnn setup /path/to/repo --project PROJECT [OPTIONS]
```

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `--project` | String | Folder name | Project identifier. |
| `--train` | Flag | `False` | Also train HGT Graph AI model during setup. |

**Example:**
```bash
softgnn setup . --project my-app
```

---

### `softgnn generate`
The all-in-one command: runs `scan -> plan -> apply -> verify -> refresh` in a single command.

```bash
softgnn generate --project PROJECT [OPTIONS]
```

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `--project` | String | **Required**| Project identifier. |
| `--strategy` | Choice | `auto` | Generation strategy: `auto`, `llm`, `template`. |
| `--only-file` | Path | `None` | Scopes test generation to a single source file. |
| `--target` | String | `None` | Generate tests for a specific function symbol only. |
| `--max-targets`| Int | `3` | Maximum missing-coverage targets to process. |
| `--source` | Choice | `auto` | Diff source: `auto`, `git`, `reflog`, `filesystem`. |
| `--yes` | Flag | `False` | Automatically accept safe diff fallbacks without interactive prompt. |
| `--repair-iters`| Int | `2` | Number of automatic repair loops if generated tests fail pytest. |

**Example:**
```bash
# Generate missing tests for current PR changes:
softgnn generate --project my-app

# Generate tests only for a specific file:
softgnn generate --project my-app --only-file src/pricing.py

# Force pure LLM generation using configured API:
softgnn generate --project my-app --strategy llm
```

---

### `softgnn plan`
Generates a structured test plan and proposed test code without modifying any files on disk.

```bash
softgnn plan --project PROJECT [OPTIONS]
```

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `--project` | String | **Required**| Project identifier. |
| `--strategy` | Choice | `auto` | `auto`, `llm`, or `template`. |
| `--only-file` | Path | `None` | Filter targets by source file. |
| `--max-targets`| Int | `3` | Maximum targets to plan for. |

**Example:**
```bash
softgnn plan --project my-app
```

---

### `softgnn apply`
Applies a previously reviewed plan: transactionally writes test blocks into `tests/`, runs `pytest`, auto-repairs errors, and rolls back if tests cannot pass.

```bash
softgnn apply --project PROJECT [OPTIONS]
```

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `--project` | String | **Required**| Project identifier. |
| `--only-file` | Path | `None` | Apply plan for targets in a specific file only. |
| `--repair-iters`| Int | `2` | Bounded repair loops on pytest failure. |

**Example:**
```bash
softgnn apply --project my-app
```

---

## 3. 📊 PR Impact & Blast Radius Analysis

### `softgnn scan`
Inspects git diff or filesystem changes and identifies coverage gaps without writing tests.

```bash
softgnn scan --project PROJECT [OPTIONS]
```

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `--project` | String | **Required**| Project identifier. |
| `--source` | Choice | `auto` | `auto`, `git`, `reflog`, `filesystem`, `full-scan`. |
| `--base` | String | `main` | Base Git ref. |
| `--head` | String | `HEAD` | Head Git ref. |

---

### `softgnn pr-scan`
Deep PR analysis: calculates blast radius, suggests developer reviewers, and exports an HTML report.

```bash
softgnn pr-scan --project PROJECT [OPTIONS]
```

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `--project` | String | **Required**| Project identifier. |
| `--report` | Flag | `False` | Generate a standalone HTML report. |
| `--open-report`| Flag | `False` | Automatically open HTML report in default browser. |

**Example:**
```bash
softgnn pr-scan --project my-app --report --open-report
```

---

### `softgnn impact`
Queries the downstream dependencies and blast radius of a specific symbol.

```bash
softgnn impact --project PROJECT TARGET [OPTIONS]
```

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `--project` | String | **Required**| Project identifier. |
| `TARGET` | String | **Required**| Symbol ID (e.g. `FUNC:process_payment`). |
| `--mode` | Choice | `hybrid` | `deterministic`, `hybrid`, `gnn`. |

**Example:**
```bash
softgnn impact --project my-app FUNC:process_payment --mode hybrid
```

---

### `softgnn report`
Prints or opens the latest generated HTML summary report.

```bash
softgnn report --project PROJECT [--open / --no-open]
```

---

## 4. 🖥️ Interactive Web Dashboard

Launch the local Cytoscape.js web-based knowledge graph visualizer and control center.

```bash
softgnn dashboard --project PROJECT [OPTIONS]
```

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `--project` | String | **Required**| Project identifier. |
| `--port` | Int | `8765` | Server port (binds to `127.0.0.1` for safety). |
| `--open` | Flag | `False` | Open default web browser upon launch. |

**Example:**
```bash
softgnn dashboard --project my-app --open
```

**Dashboard Capabilities:**
- **Interactive Graph**: Pan, zoom, inspect nodes (`FUNC`, `CLASS`, `FILE`, `TEST`), and view callers/callees.
- **Node Filtering**: Filter by symbol type or search by keyword.
- **One-Click Actions**: Trigger scans, runtime mapping, and test generation directly from buttons in the browser.

---

## 5. 🩺 Diagnostics, Mapping & Bug Triage

### `softgnn doctor`
Checks Python environment, dependencies, metadata, and knowledge graph validity.

```bash
softgnn doctor --project PROJECT
```

---

### `softgnn map`
Executes pytest with tracing to build the runtime mapping between tests and source functions.

```bash
softgnn map --project PROJECT [--pytest-args "tests"]
```

---

### `softgnn triage`
Recommends the best-suited engineers for a bug based on commit history, code graph ownership, and HGT embeddings.

```bash
softgnn triage --project PROJECT "BUG_DESCRIPTION"
```

**Example:**
```bash
softgnn triage --project my-app "Memory leak during large file streaming"
```

---

### `softgnn refresh`
Rebuilds the SoftGNN code graph and snapshot after pulling shared changes (`git pull`).

```bash
softgnn refresh --project PROJECT [OPTIONS]
```

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `--project` | String | **Required**| Project identifier. |
| `--runtime` | Flag | `False` | Also refresh pytest runtime coverage mapping. |
| `--train` | Flag | `False` | Also retrain HGT Graph AI embeddings. |

**Example:**
```bash
softgnn refresh --project my-app --runtime
```

---

## 6. 🧠 Graph AI Core & HGT Training Pipeline

Low-level commands for graph extraction and Heterogeneous Graph Transformer (HGT) training.

### `softgnn etl`
Parses codebase AST and Git commit history into NetworkX graph representations.

```bash
softgnn etl --project PROJECT [--path PATH]
```

---

### `softgnn train`
Trains the local PyTorch Geometric HGT link prediction model.

```bash
softgnn train --project PROJECT
```

---

### `softgnn prepare`
Runs the onboarding pipeline: `ETL -> Snapshot -> Train`.

```bash
softgnn prepare --project PROJECT [--path PATH] [--skip-train]
```

---

### `softgnn inspect`
Prints graph statistics (node counts, edge types, density, isolated components).

```bash
softgnn inspect --project PROJECT
```

---

### `softgnn explain`
Explains why a developer was recommended for a specific file or symbol.

```bash
softgnn explain --project PROJECT --developer "Alice"
```
