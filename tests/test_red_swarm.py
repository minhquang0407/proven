"""Tests for Parallel Multi-Persona Red Team (Adversarial Swarm)."""

import ast
import pytest
from proven.core.swarm.red_team import (
    RedBoundaryAgent,
    RedChaosAgent,
    RedStateAgent,
    RedTeamSwarm,
)

SAMPLE_CODE = """
def process_order(price, quantity, is_vip):
    if price <= 0:
        raise ValueError("Invalid price")
    
    total = price * quantity
    if is_vip and total > 100:
        total -= 10
        audit_log("VIP discount applied")
        
    return total
"""


def test_red_boundary_agent():
    tree = ast.parse(SAMPLE_CODE)
    fn = next(n for n in ast.walk(tree) if isinstance(n, ast.FunctionDef) and n.name == "process_order")
    mutants = RedBoundaryAgent.attack(tree, fn, max_mutants=2)
    assert len(mutants) >= 1
    assert any(m.mutation_type in ("BOUNDARY_INVERSION", "BOOLEAN_FLIP") for m in mutants)


def test_red_state_agent():
    tree = ast.parse(SAMPLE_CODE)
    fn = next(n for n in ast.walk(tree) if isinstance(n, ast.FunctionDef) and n.name == "process_order")
    mutants = RedStateAgent.attack(tree, fn, max_mutants=2)
    assert len(mutants) >= 1
    types = [m.mutation_type for m in mutants]
    assert "SIDE_EFFECT_NULLIFICATION" in types or "STATE_MUTATION_OMISSION" in types


def test_red_chaos_agent():
    tree = ast.parse(SAMPLE_CODE)
    fn = next(n for n in ast.walk(tree) if isinstance(n, ast.FunctionDef) and n.name == "process_order")
    mutants = RedChaosAgent.attack(tree, fn, max_mutants=2)
    assert len(mutants) >= 1
    assert mutants[0].mutation_type == "CHAOS_RETURN_NULL"
    assert "return None" in mutants[0].mutated_snippet


def test_red_team_swarm_aggregation_and_deduplication():
    mutants = RedTeamSwarm.generate_swarm_mutants(SAMPLE_CODE, "process_order", full_swarm=True)
    assert len(mutants) >= 3
    personas = {m.persona for m in mutants}
    assert "RedBoundaryAgent" in personas
    assert "RedStateAgent" in personas or "RedChaosAgent" in personas


def test_dynamic_scaling():
    simple_code = "def add(a, b):\n    return a + b\n"
    assert RedTeamSwarm.should_scale_to_full_swarm(simple_code, blast_radius=0.1) is False
    assert RedTeamSwarm.should_scale_to_full_swarm(SAMPLE_CODE, blast_radius=0.1) is True
    assert RedTeamSwarm.should_scale_to_full_swarm(simple_code, blast_radius=0.6) is True
