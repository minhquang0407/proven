import pickle

import networkx as nx
import pandas as pd

from proven.core.graph_exporter import export_graph


def test_export_graph_returns_required_shape(tmp_path, monkeypatch):
    project_dir = tmp_path / 'demo'
    graph_dir = project_dir / 'graph'
    graph_dir.mkdir(parents=True)
    paths = {
        'GRAPH_PATH': graph_dir / 'relationship_graph.pkl',
        'NODES_DATA_PATH': project_dir / 'nodes_data.csv',
        'TEST_COVERAGE_EDGES_PATH': project_dir / 'test_coverage_edges.json',
        'RUNTIME_TEST_COVERAGE_EDGES_PATH': project_dir / 'runtime_test_coverage_edges.json',
    }
    for p in paths.values():
        p.parent.mkdir(parents=True, exist_ok=True)
    pd.DataFrame([
        {'id': 'FILE:src/foo.py', 'name': 'src/foo.py', 'type': 'File'},
        {'id': 'FUNC:foo', 'name': 'foo', 'type': 'Function', 'source_file': 'src/foo.py'},
        {'id': 'FUNC:bar', 'name': 'bar', 'type': 'Function', 'source_file': 'src/bar.py'},
    ]).to_csv(paths['NODES_DATA_PATH'], index=False)
    graph = nx.DiGraph()
    graph.add_edge('FILE:src/foo.py', 'FUNC:foo', type='defines')
    graph.add_edge('FUNC:foo', 'FUNC:bar', type='calls')
    with open(paths['GRAPH_PATH'], 'wb') as f:
        pickle.dump(graph, f)
    monkeypatch.setattr('proven.core.graph_exporter.get_project_paths', lambda project: paths)

    data = export_graph('demo', focus='src/foo.py', depth=1, max_nodes=10)
    assert set(data) >= {'project', 'summary', 'nodes', 'edges'}
    assert data['summary']['nodes'] >= 1
    assert any(n['id'] == 'FUNC:foo' for n in data['nodes'])
    assert len(data['nodes']) <= 10
