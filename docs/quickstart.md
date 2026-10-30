# Quickstart Guide (`proven` v1.0.0)

## Option A: Install as an Agent Skill via `npx`
```bash
npx skills add minhquang0407/proven
# Or globally:
npx skills add minhquang0407/proven -g
```
On first invocation, the skill wrapper downloads the prebuilt `proven` release binary for your OS/CPU (Windows x64, Linux x64, macOS arm64/x64) or builds via `cargo` if available.

---

## Option B: Build or Install the Standalone CLI
- **Rust Toolchain** (`cargo` / `rustc` 1.85+)

```bash
cargo install --git https://github.com/minhquang0407/proven
# Or from a local clone:
cargo build --release
./target/release/proven --version
```

---

## Step 2: Install the Agent Skill (Optional)
Copy the `proven` skill into your coding agent configuration directories (`.claude/skills/proven`, `.agents/skills/proven`, `.windsurf/skills/proven`, `.openclaw/skills/proven`):
```bash
python skills/proven/scripts/install_skill.py --target all
```

---

## Step 3: Run the Verification Workflow

### 1. Scan for Untested Functions
```bash
proven scan --path . --json
```

### 2. Extract Scoped AST & Call-Graph Context
```bash
proven context --target "FUNC:calculate_order_total" --file "src/pricing.py" --path . --json
```
`proven context` returns the target function body, direct callers/callees, 3-hop call-graph neighbors, and `<80` tokens of stored failure rules.

### 3. Run Tests & AST Mutation Verification
```bash
proven verify-proof \
  --target "FUNC:calculate_order_total" \
  --test "tests/test_pricing.py" \
  --mutation-check \
  --json
```
If `proof_grade` is `"SILVER"`, inspect `mutation_proof.details` to see which mutated operators survived and add assertions until `proof_grade` reaches `"TITANIUM"` (`0` surviving mutants).
