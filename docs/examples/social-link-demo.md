# Walkthrough: Catching Weak Assertions with AST Mutation Testing

This example shows how `proven` detects weak assertions that pass standard test runners and guides an agent to cover all boundary conditions.

---

## 1. Target Function (`src/pricing.py`)

```python
def calculate_order_total(amount, discount_code=None, tax_rate=0.1):
    if amount is None or amount < 0:
        raise ValueError("Invalid amount")
    if amount == 0:
        return 0.0
    discounted = amount
    if discount_code == "VIP":
        discounted = amount * 0.8
    elif discount_code == "SAVE10" and amount >= 100:
        discounted = amount - 10
    total = discounted * (1.0 + tax_rate)
    return round(total, 2)
```

---

## 2. Scoped Context Extraction (`proven context`)

```bash
proven context --target "FUNC:calculate_order_total" --file "src/pricing.py" --json
```
`proven` returns the function body, 3-hop call-graph neighbors, and matching failure rules (`NULL_GUARD`, `DIV_ZERO_OR_BOUNDARY`, `EXCEPTION_CONTRACT`).

---

## 3. Detecting Weak Assertions

Suppose an agent initially writes a smoke test:
```python
from src.pricing import calculate_order_total

def test_smoke():
    res = calculate_order_total(50)
    assert res > 0
```
`pytest` passes (`exit code 0`). Running `proven verify-proof --mutation-check`:
```bash
proven verify-proof --target "FUNC:calculate_order_total" --test "tests/test_naive.py" --mutation-check --json
```
- **Output**: `status: "PARTIAL"`, `proof_grade: "SILVER"` (`mutation_score: 0.33`).
- Mutating `amount < 0` to `amount >= 0` or `amount >= 100` to `amount < 100` still passes `assert res > 0`, so those mutants survive and are recorded in `.proven/memory.json`.

---

## 4. Strengthening Assertions

Using the surviving-mutant report, the agent adds exact value, boundary, and exception checks:
```python
import pytest
from src.pricing import calculate_order_total

def test_calculate_order_total():
    with pytest.raises(ValueError):
        calculate_order_total(None)
    with pytest.raises(ValueError):
        calculate_order_total(-1)
    assert calculate_order_total(0) == 0.0
    assert calculate_order_total(100, "VIP", 0.1) == 88.0
    assert calculate_order_total(100, "SAVE10", 0.1) == 99.0
    assert calculate_order_total(99, "SAVE10", 0.1) == 108.9
```
Re-running `proven verify-proof --mutation-check` reports `proof_grade: "TITANIUM"` (`mutation_score: 1.0`, `0` surviving mutants).
