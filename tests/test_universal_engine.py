"""Automated tests for Universal Engine (Track 2: LanguageRouter, UniversalASTParser, UniversalCoverageMapper)."""

import os
from pathlib import Path
import pytest

from proven.core.agent_service import AgentService
from proven.infrastructure.pipelines.language_router import detect_project_language
from proven.infrastructure.pipelines.universal_ast_parser import UniversalASTParser
from proven.infrastructure.pipelines.universal_coverage_mapper import UniversalCoverageMapper


# --- 1. LanguageRouter Tests ---

def test_language_router_detects_python(tmp_path):
    (tmp_path / "pyproject.toml").write_text("[project]\nname='test'\n", encoding="utf-8")
    assert detect_project_language(str(tmp_path)) == "python"


def test_language_router_detects_typescript(tmp_path):
    (tmp_path / "package.json").write_text('{"devDependencies": {"typescript": "^5.0.0"}}', encoding="utf-8")
    (tmp_path / "tsconfig.json").write_text('{}', encoding="utf-8")
    assert detect_project_language(str(tmp_path)) == "typescript"


def test_language_router_detects_go(tmp_path):
    (tmp_path / "go.mod").write_text("module example.com/test\ngo 1.22\n", encoding="utf-8")
    assert detect_project_language(str(tmp_path)) == "go"


def test_language_router_detects_rust(tmp_path):
    (tmp_path / "Cargo.toml").write_text('[package]\nname = "test"\n', encoding="utf-8")
    assert detect_project_language(str(tmp_path)) == "rust"


# --- 2. UniversalASTParser Tests ---

def test_universal_ast_parser_typescript(tmp_path):
    ts_code = """
// Sample TypeScript file
export function calculateTax(amount: number, rate: number): number {
    if (amount <= 0) {
        return 0;
    }
    return amount * rate;
}

export const formatPrice = (price: number): string => {
    return `$${price.toFixed(2)}`;
};

class OrderService {
    processOrder(id: string) {
        return true;
    }
}
"""
    ts_file = tmp_path / "orders.ts"
    ts_file.write_text(ts_code, encoding="utf-8")

    parser = UniversalASTParser(str(tmp_path))
    results = parser.parse_file(str(ts_file))

    names = {r["name"] for r in results}
    assert "calculateTax" in names
    assert "formatPrice" in names
    assert "processOrder" in names

    calc_tax = next(r for r in results if r["name"] == "calculateTax")
    assert calc_tax["function_id"] == "FUNC:calculateTax"
    assert calc_tax["start"] == 3
    assert calc_tax["end"] >= 7
    assert calc_tax["language"] == "typescript"


def test_universal_ast_parser_go(tmp_path):
    go_code = """
package main

func Add(a int, b int) int {
    return a + b
}

func (s *Server) Start() error {
    return nil
}
"""
    go_file = tmp_path / "math.go"
    go_file.write_text(go_code, encoding="utf-8")

    parser = UniversalASTParser(str(tmp_path))
    results = parser.parse_file(str(go_file))

    names = {r["name"] for r in results}
    assert "Add" in names
    assert "Start" in names

    add_fn = next(r for r in results if r["name"] == "Add")
    assert add_fn["function_id"] == "FUNC:Add"
    assert add_fn["start"] == 4
    assert add_fn["end"] >= 5


# --- 3. UniversalCoverageMapper Tests ---

def test_universal_coverage_mapper_lcov_parsing(tmp_path):
    lcov_content = """
TN:
SF:src/orders.ts
FN:3,calculateTax
FNDA:1,calculateTax
DA:3,1
DA:4,1
DA:5,0
DA:7,1
end_of_record
"""
    cov_data = UniversalCoverageMapper.parse_lcov(lcov_content)
    assert "src/orders.ts" in cov_data
    assert cov_data["src/orders.ts"][3] == 1
    assert cov_data["src/orders.ts"][5] == 0

    function_ranges = {
        "src/orders.ts": [
            {"function_id": "FUNC:calculateTax", "name": "calculateTax", "start": 3, "end": 7},
            {"function_id": "FUNC:uncalledFunc", "name": "uncalledFunc", "start": 20, "end": 25},
        ]
    }

    # Test passing proof
    pass_res = UniversalCoverageMapper.verify_proof("FUNC:calculateTax", function_ranges, cov_data)
    assert pass_res["proof_status"] == "pass"
    assert pass_res["covered_fraction"] > 0.0
    assert 3 in pass_res["covered_lines"]
    assert "Universal Runtime Proof CONFIRMED" in pass_res["message"]

    # Test failing proof (0 lines executed)
    fail_res = UniversalCoverageMapper.verify_proof("FUNC:uncalledFunc", function_ranges, cov_data)
    assert fail_res["proof_status"] == "fail"
    assert fail_res["covered_fraction"] == 0.0
    assert "Universal Runtime Proof FAILED" in fail_res["message"]


def test_universal_coverage_mapper_go_coverprofile():
    go_cov_content = """
mode: set
example.com/math/calc.go:10.15,14.2 3 1
example.com/math/calc.go:20.15,25.2 2 0
"""
    cov_data = UniversalCoverageMapper.parse_go_coverprofile(go_cov_content)
    assert "example.com/math/calc.go" in cov_data
    assert cov_data["example.com/math/calc.go"][10] == 1
    assert cov_data["example.com/math/calc.go"][14] == 1
    assert cov_data["example.com/math/calc.go"][20] == 0


# --- 4. AgentService End-to-End Dual-Track Tests ---

def test_agent_service_dual_track_routing_and_lcov_proof(tmp_path):
    ts_code = """
export function addNumbers(a: number, b: number): number {
    return a + b;
}
"""
    ts_file = tmp_path / "calc.ts"
    ts_file.write_text(ts_code, encoding="utf-8")

    lcov_file = tmp_path / "lcov.info"
    lcov_file.write_text(f"""
TN:
SF:{ts_file.as_posix()}
DA:2,1
DA:3,1
end_of_record
""", encoding="utf-8")

    svc = AgentService(project="test-universal", repo_path=str(tmp_path), language="typescript")
    assert svc.language == "typescript"

    # Context test for TypeScript
    ctx = svc.get_context(target_id="FUNC:addNumbers", source_file="calc.ts")
    assert ctx["status"] == "success"
    assert ctx["track"] == "universal_engine"
    assert "addNumbers" in ctx["source_code"]

    # Proof verification test using LCOV
    res = svc.verify_proof(
        target_id="FUNC:addNumbers",
        lcov_path=str(lcov_file),
    )
    assert res["status"] == "success"
    assert res["track"] == "universal_engine"
    assert res["proof_status"] == "pass"
    assert res["covered_fraction"] > 0.0
