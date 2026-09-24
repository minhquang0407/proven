"""Tests for Code GraphRAG, Multi-Hop Attention Extraction, and Explainability Tree."""

import pytest
from click.testing import CliRunner
from proven.cli import cli
from proven.core.agent_service import AgentService
from proven.core.graph_exporter import export_graph
from proven.core.graph_rag import CodeGraphRAG


def test_extract_subgraph_attention_structure(tmp_path):
    repo_path = str(tmp_path)
    # Create sample Python source
    src_file = tmp_path / "orders.py"
    src_file.write_text(
        "def helper(x):\n"
        "    return x * 2\n\n"
        "def checkout(amount):\n"
        "    h = helper(amount)\n"
        "    return h > 0\n",
        encoding="utf-8",
    )

    data = CodeGraphRAG.extract_subgraph_attention(
        target_id="FUNC:orders.checkout",
        hops=3,
        top_k=5,
        mode="topological",
        repo_path=repo_path,
    )

    assert data["target_id"] == "FUNC:orders.checkout"
    assert data["hops"] == 3
    assert "attended_nodes" in data
    assert "prompt_block" in data


def test_build_graph_rag_prompt_bounded_tokens(tmp_path):
    repo_path = str(tmp_path)
    src_file = tmp_path / "billing.py"
    src_file.write_text(
        "def charge(user, amount):\n    return True\n",
        encoding="utf-8",
    )

    prompt = CodeGraphRAG.build_graph_rag_prompt(
        target_id="FUNC:billing.charge",
        max_tokens=120,
        repo_path=repo_path,
    )

    # Tokens safeguard: 120 tokens ~ 480 chars
    assert len(prompt) <= 120 * 4
    if prompt:
        assert "### CODE GRAPHRAG" in prompt


def test_format_attention_tree():
    sample_data = {
        "target_id": "FUNC:orders.checkout",
        "target_label": "orders.checkout",
        "engine": "hgt_hybrid",
        "hops": 3,
        "attended_nodes": [
            {
                "symbol": "FUNC:billing.charge_card",
                "node_type": "Function",
                "attention_score": 0.94,
                "hop_distance": 1,
                "relation": "calls",
                "semantic_rationale": "Direct callee; high execution failure blast radius",
            },
            {
                "symbol": "CLASS:UserSession",
                "node_type": "Class",
                "attention_score": 0.72,
                "hop_distance": 2,
                "relation": "uses",
                "semantic_rationale": "Shared state container; concurrency sensitive",
            },
        ],
    }

    tree_str = CodeGraphRAG.format_attention_tree(sample_data)
    assert "Target: orders.checkout (Root)" in tree_str
    assert "[Attn: 94%]" in tree_str
    assert "calls" in tree_str
    assert "[Attn: 72%]" in tree_str
    assert "UserSession" in tree_str


def test_agent_service_explain_attention(tmp_path):
    repo_path = str(tmp_path)
    src_file = tmp_path / "auth.py"
    src_file.write_text(
        "def verify_token(tok):\n    return bool(tok)\n",
        encoding="utf-8",
    )

    svc = AgentService(repo_path=repo_path)
    data = svc.explain_attention(
        target_symbol="FUNC:auth.verify_token",
        hops=3,
        top_k=3,
        mode="hybrid",
    )

    assert "target_id" in data
    assert "attended_nodes" in data
    assert "ascii_tree" in data


def test_cli_explain_attention_command(tmp_path):
    repo_path = str(tmp_path)
    src_file = tmp_path / "payment.py"
    src_file.write_text(
        "def refund(id):\n    return True\n",
        encoding="utf-8",
    )

    runner = CliRunner()
    # Test top-level command
    res1 = runner.invoke(cli, [
        "explain-attention",
        "--target", "FUNC:payment.refund",
        "--path", repo_path,
        "--json",
    ])
    assert res1.exit_code == 0
    assert "attended_nodes" in res1.output

    # Test under agent group
    res2 = runner.invoke(cli, [
        "agent", "explain-attention",
        "--target", "FUNC:payment.refund",
        "--path", repo_path,
        "--json",
    ])
    assert res2.exit_code == 0
    assert "attended_nodes" in res2.output


def test_cytoscape_export_graph_attention(tmp_path):
    repo_path = str(tmp_path)
    project_name = "test_rag_project"

    # Export graph with target parameter
    res = export_graph(project=project_name, target="FUNC:payment.refund")
    assert "summary" in res
    assert "nodes" in res
    assert "edges" in res
    assert res["target"] == "FUNC:payment.refund"
