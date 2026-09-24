<div align="center">

# Proven (v1.0.0 PRO)

### The Neuro-Symbolic & Runtime-Proven Test Advisor for AI Coding Agents & CI/CD

**Adversarial Tri-Agent Co-Evolution · Causal Reflexion Engine · Topological Memory · Titanium Mutation Gate**

[![Release](https://img.shields.io/github/v/tag/minhquang0407/softgnn-advisor?label=release&color=blue)](https://github.com/minhquang0407/softgnn-advisor/releases)
[![Tests](https://img.shields.io/badge/tests-92%2F92%20passed-brightgreen.svg)](#test-suite)
[![Python](https://img.shields.io/badge/Python-3.11%2B-3776AB?logo=python&logoColor=white)](https://www.python.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![GitHub Action](https://img.shields.io/badge/GitHub%20Action-Marketplace-2088FF?logo=github-actions&logoColor=white)](#mode-c-github-action-cicd-quality-gate)
[![Agent Skill](https://img.shields.io/badge/Agent%20Skill-Antigravity%20%7C%20Claude%20%7C%20Cursor-purple)](#mode-a-ai-coding-agent-skill-zero-api-key)

**[Watch Demo Video](https://www.youtube.com/watch?v=3d071eUmfq0)** · **[Quickstart](#quickstart)** · **[CLI Manual](docs/cli-reference.md)**

<br/>

<img width="650" alt="Proven Interactive Graph Dashboard" src="https://github.com/user-attachments/assets/b14c4da0-850b-4537-918c-ab6e0957dcb9" />

</div>

---

## Core Philosophy: "LLM = Author, Proven = Ground Truth"

AI test generators write code fast, but **two massive blindspots remain**:
1. **The Fake Coverage Trap**: A test passes `pytest`, but due to early returns or over-mocking, **0% of the modified target function code is actually executed**.
2. **The Weak Assertion Trap**: A test executes the function, but only asserts `assert res is not None`. If a breaking logic bug is introduced, the test still passes.

```mermaid
flowchart LR
    Scan[1. SCAN<br/>Blast Radius] --> Context[2. CONTEXT<br/>AST & Callers]
    Context --> Author[3. AUTHOR<br/>Agent writes test]
    Author --> RunProof{4. RUNTIME<br/>Real execution?}
    RunProof -->|Pass| MutProof{5. MUTATION<br/>Kill mutants?}
    RunProof -->|Fail: Early Exit| Repair[6. REPAIR LOOP<br/>Branch diagnosis]
    MutProof -->|Fail: Weak Assert| Repair
    Repair --> RunProof
    MutProof -->|Pass: TITANIUM| Audit[7. FINAL AUDIT<br/>Refresh graph]
```

> **Proven PRO is your infallible Ground Truth Gatekeeper.** It proves real execution, diagnoses early exits, and performs surgical micro-mutations to guarantee **Titanium-Grade tests**.

---

## 🧠 Neuro-Symbolic AI: Tri-Agent Adversarial Co-Evolution

Beyond standard test runners, Proven is engineered as a **Neuro-Symbolic Multi-Agent System**. Testing is framed as a non-zero-sum game between specialized agents mediated by an infallible symbolic referee:

```
               ┌──────────────────────────────────────────────────┐
               │       🧠 CRITIC AGENT (Supervisor / Referee)     │
               │   - Dynamic Bytecode Tracing (Coverage Gate)     │
               │   - HGT Heterogeneous Graph Topology (Causal AST)│
               │   - Micro-Mutation Verification & Mutant Vault   │
               │   - Causal Reflexion Diagnostic Synthesis        │
               └─────────────────────────┬────────────────────────┘
                                         │
                 ┌───────────────────────┴───────────────────────┐
                 ▼                                               ▼
   ┌───────────────────────────┐                   ┌───────────────────────────┐
   │   🛡️ AUTHOR AGENT (Blue)  │                   │  ⚔️ ADVERSARY AGENT (Red) │
   │ - Synthesizes test suite  │◀──White-box Review── - Injects subtle mutants │
   │ - Reads Graph Axioms      │                   │ - Aims to bypass Author   │
   │ - Bounded memory (<80 tok)│                   │   with semantic anomalies │
   └───────────────────────────┘                   └───────────────────────────┘
```

### Key AI Components
1. **Causal Reflexion Engine (`reflexion_engine.py`)**:
   When tests fail to kill mutants or miss code branches, the engine fuses physical bytecode traces with upstream/downstream callers in the Heterogeneous Graph Transformer (HGT) network. It classifies failures into `UNCOVERED_BRANCH` vs `WEAK_ASSERTION` and generates structured causal reflexion prompts for the Author Agent.
2. **Zero-Token Mutant Vault (`mutant_vault.py`)**:
   Surviving mutants generated during adversarial exploration are automatically archived into `.proven/mutant_vault/{target}/`. Future regression verification executes in **< 0.05s** with **0 LLM tokens**.
3. **Topological Graph-Pinned Memory & Sleep Consolidation (`memory_manager.py`)**:
   Instead of blowing up LLM context windows, lessons are pinned directly to AST graph nodes. Queries inject only the top-2 relevant lessons (**< 80 tokens** overhead). Periodic **Sleep Consolidation** synthesizes episodic lessons into repo-wide architecture rules (`.proven/axioms.md`).

---

## Execution Modes: Choose Your Setup

Proven operates in **two primary ways** depending on whether you are using an AI Coding Agent or running standalone in your terminal:

```mermaid
flowchart TD
    Repo[Your Codebase / PR] --> Choice{How do you run Proven?}
    Choice -->|With Coding Agent| AgentMode[Mode A: AI Agent Skill]
    Choice -->|Without Coding Agent| StandaloneMode[Mode B: Standalone CLI]
    Choice -->|On Pull Requests| ActionBot[Mode C: GitHub Action Bot]
    Choice -->|In Web Browser| Dashboard[Mode D: Interactive Web Dashboard]

    AgentMode --> ZeroKey[Zero API Key Required: Agent writes, Proven verifies]
    StandaloneMode --> BuiltInLLM[Built-in Gemini / OpenAI / Ollama generator]
    ActionBot --> PRComment[Automated PR Comment & Merge Blocker]
    Dashboard --> GraphView[Cytoscape.js Code Graph Visualization]
```

---

### Mode A: AI Coding Agent Skill (Zero API Key)
**Recommended for Antigravity, Claude Code, Cursor, and Codex.**

In Agent Mode, the Coding Agent acts as the author while Proven provides the AST ground truth, runtime proof gate, and mutation testing. **Zero third-party API keys are required for Proven**:

```bash
# 1. Install skill for Antigravity:
python skills/proven/scripts/install_skill.py --target antigravity

# Or install via Open Skills standard (Claude Code / Codex):
npx skills add minhquang0407/softgnn-advisor
```

Ask your agent:
> *"Use proven to scan my changes and write runtime-proven tests with mutation checks."*

---

### Mode B: Standalone Developer CLI (Built-in LLM Generator)
**Run directly from your terminal without any external AI coding agent.**

SoftGNN connects to your configured LLM (Gemini, OpenAI, or local Ollama), generates structured pytest tests, patches them transactionally, runs pytest, auto-repairs failures, and rolls back cleanly if tests cannot pass.

#### 1. Configure LLM API Keys (For Standalone Mode)

**Google Gemini (Recommended):**
```bash
# Linux / macOS:
export SOFTGNN_LLM_PROVIDER="gemini"
export SOFTGNN_LLM_MODEL="gemini-2.5-flash"
export SOFTGNN_LLM_API_KEY="AIzaSyYourGeminiApiKeyHere..."

# Windows PowerShell:
$env:SOFTGNN_LLM_PROVIDER="gemini"
$env:SOFTGNN_LLM_MODEL="gemini-2.5-flash"
$env:SOFTGNN_LLM_API_KEY="AIzaSyYourGeminiApiKeyHere..."
```

**OpenAI / Local Ollama / vLLM / DeepSeek:**
```bash
# Linux / macOS:
export SOFTGNN_LLM_PROVIDER="openai-compatible"
export SOFTGNN_LLM_BASE_URL="http://localhost:11434/v1"   # For Ollama
export SOFTGNN_LLM_MODEL="qwen2.5-coder:7b"               # Or gpt-4o
export SOFTGNN_LLM_API_KEY="optional-or-sk-..."

# Windows PowerShell:
$env:SOFTGNN_LLM_PROVIDER="openai-compatible"
$env:SOFTGNN_LLM_BASE_URL="http://localhost:11434/v1"
$env:SOFTGNN_LLM_MODEL="qwen2.5-coder:7b"
$env:SOFTGNN_LLM_API_KEY="optional-or-sk-..."
```

*(Note: If no API key is provided, Proven automatically falls back to offline deterministic AST templates).*

#### 2. Standalone Workflow Commands

```bash
# Step 1: Onboard repository (build graph & snapshot)
proven setup . --project my-app

# Step 2: One-shot scan -> plan -> generate -> verify
proven generate --project my-app

# Or review proposed test code before patching:
proven plan --project my-app
proven apply --project my-app
```

---

### Mode C: GitHub Action (CI/CD Quality Gate)
Block untested code changes directly on your GitHub Pull Requests.
Add `.github/workflows/proven-audit.yml` to **any repo**:

```yaml
name: Proven PR Quality Gate
on: [pull_request]

jobs:
  audit:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      pull-requests: write
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - uses: minhquang0407/softgnn-advisor@v1.0.0
        with:
          github-token: ${{ secrets.GITHUB_TOKEN }}
          fail-on-missing: "false" # Set 'true' to block merge if tests are missing
```

---

### Mode D: Interactive Web Dashboard
Visualize your entire codebase knowledge graph and manage test impact from an interactive web UI:

```bash
proven dashboard --project my-app --open
```

Opens at `http://127.0.0.1:8765`:
- **Interactive Cytoscape Graph**: Visually navigate functions, classes, and runtime test-to-code edges.
- **Node Filtering & Search**: Filter by symbol type (`FUNC`, `CLASS`, `FILE`, `TEST`) or search by name.
- **One-Click Actions**: Run Scans, trigger Runtime Mapping, and generate tests directly from the browser UI.

---

## What Makes Proven PRO Unique?

| Feature | Standard AI / CI | Proven PRO | Why It Matters |
| :--- | :---: | :---: | :--- |
| **Tri-Agent Arena** | Single loop | **Adversarial Co-Evolution** | Critic, Blue Author, and Red Adversary co-evolve to eliminate blindspots. |
| **Causal Reflexion** | Syntax error | **Topological Diagnosis** | Combines physical bytecode traces with HGT graph callers (`UNCOVERED_BRANCH` vs `WEAK_ASSERTION`). |
| **Zero-Token Vault** | Re-prompt LLM | **Deterministic Vault** | Killer mutants archived in `.proven/mutant_vault/` and replayed in <0.05s with 0 tokens. |
| **Topological Memory** | Context overflow | **AST-Pinned Axioms** | Scoped memory injection (<80 tokens) with Sleep Consolidation into repo axioms. |
| **Runtime Proof Gate** | None | **Enforced** | Verifies tests hit exact bytecode/AST line ranges, not just smoke tests. |
| **Self-Healing Diagnoser** | Raw trace | **AST Deep Scan** | Identifies early-exit `if` guards & mock mismatches to guide the Agent. |
| **Micro-Mutation Gate** | Slow / Heavy | **Targeted AST** | Inverts operators (`>`, `==`, `+`, `True`) inside the target function to kill weak asserts (**TITANIUM PROOF**). |
| **Latent Blast Radius** | Direct only | **HGT Graph AI** | Predicts remote components at risk of breaking via graph embeddings and co-change history. |
| **Bug & Reviewer Triage**| Manual | **Semantic Matching**| Suggests the best-suited code owner to review PRs or fix bugs based on Git authorship and GNN embeddings. |
| **Swarm Concurrency** | Conflicts | **Process-Isolated** | Isolated temp coverage sessions allow sub-agents to test concurrently with zero race conditions. |
| **Polyglot Track** | Python only | **Universal LCOV** | Dual-track support for Python, TypeScript, JavaScript, Go, Rust, Java, and C++. |

---

## Quickstart

### Daily Commands (Agent Mode)

```bash
# 1. Scan changed functions needing tests
proven agent scan

# 2. Query latent blast radius & downstream dependencies
proven agent impact --target "FUNC:checkout" --mode hybrid

# 3. Recommend expert reviewers or triage a bug
proven agent triage "Database connection pool timeout"

# 4. PRO: Verify test execution with Micro-Mutation Gate
python skills/proven/scripts/verify_runtime_proof.py \
  --target "FUNC:calculate_total" \
  --test "tests/test_pricing.py" \
  --mutation-check

# 5. Arena: Launch Tri-Agent Adversarial Round
python skills/proven/scripts/run_arena.py \
  --target "FUNC:calculate_total" \
  --test "tests/test_pricing.py" \
  --mutation-check
```

### Self-Healing Feedback Example
When a test fails runtime proof, Proven provides immediate actionable hints:

```json
{
  "proof_status": "fail",
  "covered_fraction": 0.0,
  "self_healing": {
    "diagnosis": "EARLY_BRANCH",
    "failing_branch_line": 42,
    "condition_code": "if user is None:",
    "actionable_suggestion": "Function exited early at line 42 ('if user is None:'). Adjust mock/input data so condition evaluates to proceed into body."
  }
}
```

### Micro-Mutation Feedback Example (Proof Grade)

```json
{
  "proof_status": "pass",
  "proof_grade": "TITANIUM",
  "mutation_proof": {
    "mutation_gate_status": "passed",
    "mutants_total": 3,
    "mutants_killed": 3,
    "mutants_survived": 0,
    "message": "TITANIUM PROOF: All mutants killed. Behavioral assertions verified."
  }
}
```

---

## CLI Quick Reference

> Note: `proven` and `softgnn` commands are interchangeable aliases.

| Command | Action |
| :--- | :--- |
| `proven agent scan` | Detect altered functions lacking runtime test proof |
| `proven agent context --target <id>` | Query AST line ranges, callers, and import dependencies |
| `proven agent verify-proof --target <id> --test <p>` | Verify runtime execution proof |
| `proven agent verify-proof ... --mutation-check` | **PRO**: Invert AST operators to kill weak assertions (**Titanium Proof**) |
| `proven arena --target <id> --test <p>` | **AI Swarm**: Coordinate Tri-Agent Adversarial Arena co-evolution round |
| `proven memory list` | **Memory**: Inspect graph-pinned episodic lessons |
| `proven memory consolidate` | **Memory**: Trigger Sleep Consolidation of lessons into repository axioms |
| `proven agent impact --target <id>` | **Tier 2**: Query direct dependents & latent HGT blast radius |
| `proven agent triage "bug description"` | **Tier 2**: Recommend expert code reviewers for a bug or PR |
| `proven agent train` | **Tier 2**: Trigger offline HGT Graph AI training |
| `proven dashboard --project <app> --open` | Launch interactive Cytoscape.js web knowledge graph |
| `proven setup <repo> --project <app>` | Build initial code graph & filesystem baseline snapshot |
| `proven generate --project <app>` | Standalone one-shot test generation with automatic rollback |
| `proven pr-scan --project <app> --report` | Scan PR diff, compute blast radius & export HTML report |
| `proven doctor --project <app>` | Validate dependencies, metadata, and graph integrity |

For the complete reference of all commands, options, and advanced pipelines, see [docs/cli-reference.md](docs/cli-reference.md).

---

## Test Suite

Proven is verified across a comprehensive test suite covering AST parsing, runtime mapping, self-healing diagnostics, mutation gates, HGT triage, Tri-Agent Arena, Causal Reflexion, and GitHub Action workflows:

```bash
pytest
# ======================= 92 passed in 105s =======================
```

---

## 📄 License

Distributed under the [MIT License](LICENSE).
