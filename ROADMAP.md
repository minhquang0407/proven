# Roadmap

SoftGNN Advisor's long-term direction is:

```text
Graph-guided, runtime-proven, LLM-assisted PR testing.
Exposed as an MCP server so any AI agent can reason about code risk and coverage.
```

---

## v0.1 — Single-Agent LLM-Assisted Test Generation

Status: **✅ complete**

```text
code graph extraction
runtime test mapping
PR impact scan
semantic pytest generation
Gemini provider
OpenAI-compatible provider
structured JSON validation
transactional patching
pytest verification
LLM repair hook
runtime refresh
PR scan confirmation
```

---

## M4 — Runtime-Proven Test Generation

Status: **✅ complete**

```text
target-level runtime proof gate
reject passing tests that do not hit target
coverage delta before/after
proof report per generated test
quality gate against smoke-only tests
```

---

## M5 — Smart Scan + Dashboard + File-Scoped Generate

Status: **✅ complete**

```text
smart scan fallback for empty diffs (git pull / same-branch)
safe interactive generate fallback
softgnn refresh command after git pull
HTML PR scan report
interactive local graph dashboard (http://127.0.0.1:8765)
action buttons: scan, pr-scan, impact, generate, map runtime
live command logs in dashboard
graph auto-reload after generate / map
file-scoped generation: --only-file src/foo.py
```

---

## M6 — MCP Server + Graph Intelligence

Goal:

```text
expose SoftGNN as a Model Context Protocol (MCP) server
so AI agents (Claude, Antigravity, Cursor, etc.) can call it as a tool
and use graph + runtime data to guide their own test generation
```

### M6a — SoftGNN MCP Server

MCP tools to expose:

```text
softgnn.scan_impact(base, head)
  → changed files, changed nodes, risk scores

softgnn.get_missing_coverage(file?)
  → functions missing runtime test proof

softgnn.get_impact(target)
  → call graph / propagation graph for one function

softgnn.get_coverage_status(target)
  → runtime_proof: bool, edge_count: int

softgnn.export_graph(focus, depth, max_nodes)
  → subgraph JSON for context injection

softgnn.generate_tests(only_file?, target?, source?)
  → trigger generation + return result summary

softgnn.refresh()
  → rebuild graph and snapshot
```

Agent workflow this enables:

```text
Dev: "Review PR and generate tests for what is missing"

Agent:
  1. softgnn.scan_impact()     → know what changed and what is at risk
  2. softgnn.get_missing_coverage() → know exactly where tests are missing
  3. generate tests for those targets
  4. softgnn.verify_coverage() → confirm runtime proof
  5. report back to dev
```

### M6b — GNN Risk Scoring

```text
train GNN on graph topology + git history + runtime edges
predict risk score per node (not rule-based priority)
expose risk scores via MCP: softgnn.get_risk_scores()
dashboard highlights nodes by GNN-predicted risk
agent uses risk scores to prioritize generation targets
```

---

## M7 — Multi-Agent Quality Swarm

Goal:

```text
improve generated test quality using role-specialized agents
```

Planned roles:

```text
ContextAgent    → queries SoftGNN MCP for graph/coverage context
WriterAgent     → generates test code
ReviewerAgent   → critiques test quality and coverage
RepairAgent     → fixes failing tests
CoverageAgent   → confirms runtime proof via SoftGNN MCP
Deterministic QualityGate
```

Principle:

```text
agents propose
SoftGNN MCP provides ground truth about coverage and risk
validators enforce safety
pytest verifies correctness
runtime graph proves target execution
```

---

## M8 — Large-Scale Repo Automation

Goal:

```text
scale from one target to many targets across a repo
```

Planned features:

```text
batch target selection guided by GNN risk scores
LLM rate limiting
cost budget
checkpoint/resume
batch rollback
parallel pytest shards
```

---

## M9 — Controlled Production-Code Fixes

Goal:

```text
optionally fix production bugs revealed by generated tests
```

Safety model:

```text
disabled by default
requires explicit flag
requires user approval
shows diff before applying
rollback on failure
```

---

## M10 — Provider/Auth/Enterprise Hardening

Planned features:

```text
Vertex AI auth
service accounts
Azure OpenAI
Anthropic
per-agent model routing
secret redaction
audit logs
retry/rate-limit handling
```

---

## M11 — Local Model Management / Fine-Tuning

Planned features:

```text
Ollama/vLLM setup helper
local model health checks
model quality benchmark
successful generation/repair dataset
fine-tuning pipeline
```
