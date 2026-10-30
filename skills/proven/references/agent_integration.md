# Agent Integration Guide (v1.0.0)

This guide explains how autonomous coding agents (Claude Code, Cursor, Codex, Windsurf, OpenClaw) integrate with **Proven (`proven`)** for AST context extraction and TITANIUM runtime proof verification.

---

## 1. Installing the Skill

Run the cross-agent installer script once from the repository root:

```bash
python skills/proven/scripts/install_skill.py --target all
```

Supported `--target` options:
- `claude`: Installs to `~/.claude/skills/proven`
- `openclaw`: Installs to `~/.openclaw/skills/proven`
- `cursor` / `codex`: Installs to `.agents/skills/proven`
- `windsurf`: Installs to `.windsurf/skills/proven`
- `all`: Installs to all targets above (skipping self-copy if already running from `.agents/skills/proven`)

---

## 2. Binary vs. Python Skill Wrappers

All algorithmic logic is compiled into the single binary (`proven` / `proven.exe`). Agents can invoke either:

1. **Direct Rust Binary** (fastest, ~0.2ms startup):
   ```bash
   proven scan --path . --json
   proven context --target "FUNC:calculate_order_total" --file "src/pricing.py" --json
   proven verify-proof --target "FUNC:calculate_order_total" --test "tests/test_pricing.py" --mutation-check --json
   ```
2. **Skill Wrapper Scripts** (`skills/proven/scripts/*.py`, auto-locating `skills/proven/bin/proven(.exe)`):
   ```bash
   python skills/proven/scripts/scan_impact.py --path . --json
   python skills/proven/scripts/get_target_context.py --target "FUNC:calculate_order_total" --file "src/pricing.py" --json
   python skills/proven/scripts/verify_runtime_proof.py --target "FUNC:calculate_order_total" --test "tests/test_pricing.py" --mutation-check --json
   ```

Every wrapper script supports `--schema` for machine-readable tool introspection.

---

## 3. Subagent Fan-Out Mode (`--fan-out`)

When scanning large pull requests with multiple untested functions, pass `--fan-out`:

```bash
proven scan --path . --fan-out --json
```

The JSON output includes a `fan_out_tasks` array where each entry contains:
- `task_id`: Unique task identifier (`subagent_1`, ...)
- `target_id`: Target symbol (`FUNC:...`)
- `file`: Relative source file path
- `suggested_test_file`: Recommended test file path
- `prompt`: Ready-to-run prompt instructing a subagent how to extract context and verify TITANIUM proof for that specific target.
