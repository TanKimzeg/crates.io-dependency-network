import argparse
import csv
import json
import math
import os
import random
import sys
from pathlib import Path

def configure_tcl_tk() -> None:
    interpreter_root = Path(os.__file__).resolve().parent.parent
    tcl_root = interpreter_root / "tcl"
    if not tcl_root.exists():
        return

    tcl_library = next((path for path in [tcl_root / "tcl8.6", tcl_root / "tcl8"] if path.exists()), None)
    tk_library = tcl_root / "tk8.6"

    if tcl_library is not None:
        os.environ.setdefault("TCL_LIBRARY", str(tcl_library))
    if tk_library.exists():
        os.environ.setdefault("TK_LIBRARY", str(tk_library))


configure_tcl_tk()

import matplotlib.pyplot as plt
import networkx as nx
import numpy as np
import powerlaw
from tqdm import tqdm
from dataclasses import dataclass

@dataclass
class AnalysisConfig:
    edges: Path
    nodes: Path
    out_dir: Path = Path("outputs/analysis")
    path_samples: int = 50
    top_k: int = 30
    seed: int = 7
    skip_powerlaw: bool = False
    skip_community: bool = False
    skip_pagerank: bool = False


def parse_args() -> AnalysisConfig:
    parser = argparse.ArgumentParser(
        description="Analyze crates.io dependency core graph exported by the Rust parser."
    )
    parser.add_argument("--edges", required=True, help="Path to core_edges.csv")
    parser.add_argument("--nodes", required=True, help="Path to core_nodes.csv")
    parser.add_argument("--out-dir", default="outputs/analysis", help="Output directory")
    parser.add_argument("--path-samples", type=int, default=50, help="BFS samples for path length")
    parser.add_argument("--top-k", type=int, default=30, help="Top K nodes to export")
    parser.add_argument("--seed", type=int, default=7, help="Random seed")
    parser.add_argument("--skip-powerlaw", action="store_true", help="Skip powerlaw fitting")
    parser.add_argument("--skip-community", action="store_true", help="Skip community detection")
    parser.add_argument("--skip-pagerank", action="store_true", help="Skip pagerank export")
    args = parser.parse_args()
    return AnalysisConfig(
        edges=Path(args.edges),
        nodes=Path(args.nodes),
        out_dir=Path(args.out_dir),
        path_samples=args.path_samples,
        top_k=args.top_k,
        seed=args.seed,
        skip_powerlaw=args.skip_powerlaw,
        skip_community=args.skip_community,
        skip_pagerank=args.skip_pagerank
    )


def progress(iterable, **kwargs):
    return tqdm(iterable, disable=not sys.stderr.isatty(), **kwargs)


def stage_bar(total: int):
    return tqdm(total=total, desc="Analysis", unit="step", disable=not sys.stderr.isatty())


def read_edges(path: Path) -> list[tuple[str, str]]:
    edges: list[tuple[str, str]] = []
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        for row in progress(reader, desc="Loading edges", unit="edges"):
            src = row.get("src")
            dst = row.get("dst")
            if not src or not dst:
                continue
            edges.append((src, dst))
    return edges


def read_nodes(path: Path) -> list[str]:
    nodes: list[str] = []
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        for row in progress(reader, desc="Loading nodes", unit="nodes"):
            name = row.get("name")
            if name:
                nodes.append(name)
    return nodes


def write_json(path: Path, payload: dict) -> None:
    path.write_text(json.dumps(payload, indent=2), encoding="utf-8")


def degree_stats(degrees: np.ndarray) -> dict[str, float]:
    if degrees.size == 0:
        return {}
    return {
        "min": int(degrees.min()),
        "max": int(degrees.max()),
        "mean": float(degrees.mean()),
        "median": float(np.median(degrees)),
    }


def plot_degree_distribution(degrees: np.ndarray, out_path: Path, title: str) -> None:
    values = degrees[degrees > 0]
    if values.size == 0:
        return
    max_value = values.max()
    if max_value <= 0:
        return

    if max_value <= 1:
        bins = np.array([1, 2])
    else:
        bins = np.logspace(0, math.log10(max_value), num=30)
    hist, bin_edges = np.histogram(values, bins=bins)
    x = bin_edges[:-1]
    y = hist / hist.sum()

    plt.figure(figsize=(6, 4))
    plt.loglog(x, y, marker="o", linestyle="none")
    plt.title(title)
    plt.xlabel("degree")
    plt.ylabel("P(k)")
    plt.tight_layout()
    plt.savefig(out_path, dpi=200)
    plt.show()
    plt.close()


def fit_powerlaw(degrees: np.ndarray) -> dict | None:
    values = degrees[degrees > 0]
    if values.size < 50:
        return None
    fit = powerlaw.Fit(values, discrete=True, verbose=False)
    return {
        "alpha": float(fit.alpha),
        "xmin": float(fit.xmin),
        "D": float(fit.D),
        "sigma": float(fit.sigma),
    }


def largest_component_graph(graph: nx.Graph) -> nx.Graph:
    if graph.number_of_nodes() == 0:
        return graph
    if nx.is_connected(graph):
        return graph
    largest = max(nx.connected_components(graph), key=len)
    return graph.subgraph(largest).copy()


def estimate_average_shortest_path_length(
    graph: nx.Graph, sample_sources: int, rng: random.Random
) -> float | None:
    if graph.number_of_nodes() <= 1:
        return 0.0
    nodes = list(graph.nodes())
    sample_sources = min(sample_sources, len(nodes))
    sources = rng.sample(nodes, sample_sources)

    total_distance = 0
    total_pairs = 0
    # Approximate by averaging BFS distances from sampled sources.
    for source in progress(sources, desc="Sampling paths", unit="source"):
        lengths = nx.single_source_shortest_path_length(graph, source)
        for target, distance in lengths.items():
            if target == source:
                continue
            total_distance += distance
            total_pairs += 1

    if total_pairs == 0:
        return None
    return total_distance / total_pairs


def export_top_centrality(path: Path, graph: nx.DiGraph, top_k: int) -> None:
    pagerank = nx.pagerank(graph, alpha=0.85, max_iter=100)
    ranked = sorted(pagerank.items(), key=lambda item: item[1], reverse=True)[:top_k]

    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(["rank", "name", "pagerank", "in_degree", "out_degree"])
        for index, (name, score) in enumerate(ranked, start=1):
            writer.writerow([
                index,
                name,
                f"{score:.6f}",
                graph.in_degree(name),
                graph.out_degree(name),
            ])

