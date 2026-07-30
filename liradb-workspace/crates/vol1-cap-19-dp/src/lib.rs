//! Vol.I — Capítulo 19: Programación dinámica en grafos.
//!
//! Migrado desde `vol1-grafos-de-cero-a-experto-rust.md` §19.1-§19.4.
//!
//! - [`longest_path_dag`] — DP sobre orden topológico (camino más largo en DAG ponderado).
//! - [`subtree_sizes`] — Tree DP: tamaño del subárbol desde una raíz.
//! - [`held_karp`] — DP O(2^n · n²) para TSP.
//!
//! Held-Karp también vive en `vol1-cap-18-np` (es un duplicado histórico
//! del Vol.I, donde el autor lo introdujo primero en cap. 18 y lo
//! desarrolló en cap. 19).

use std::collections::HashMap;

use petgraph::Direction;
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};

/// Longest path en DAG: DP sobre orden topológico.
pub fn longest_path_dag(g: &DiGraph<i32, i32>, src: NodeIndex) -> HashMap<NodeIndex, i64> {
    let topo = toposort(g, None).expect("es un DAG");
    let mut dp: HashMap<NodeIndex, i64> = HashMap::new();
    dp.insert(src, 0);
    for &v in &topo {
        if !dp.contains_key(&v) {
            continue;
        }
        for w in g.neighbors_directed(v, Direction::Outgoing) {
            let weight = *g.edge_weight(g.find_edge(v, w).unwrap()).unwrap() as i64;
            let new = dp[&v] + weight;
            dp.entry(w)
                .and_modify(|e| *e = (*e).max(new))
                .or_insert(new);
        }
    }
    dp
}

/// Tree DP: tamaño del subárbol desde `root`.
pub fn subtree_sizes(g: &DiGraph<(), ()>, root: NodeIndex) -> HashMap<NodeIndex, usize> {
    let mut sizes = HashMap::new();

    fn dfs(
        g: &DiGraph<(), ()>,
        v: NodeIndex,
        parent: Option<NodeIndex>,
        sizes: &mut HashMap<NodeIndex, usize>,
    ) -> usize {
        let mut s = 1;
        for w in g.neighbors_directed(v, Direction::Outgoing) {
            if Some(w) == parent {
                continue;
            }
            s += dfs(g, w, Some(v), sizes);
        }
        sizes.insert(v, s);
        s
    }

    dfs(g, root, None, &mut sizes);
    sizes
}

/// Held-Karp TSP: programación dinámica O(2^n · n²).
pub fn held_karp(dist: &[Vec<f64>], start: usize) -> f64 {
    let n = dist.len();
    debug_assert!(n <= 20, "Held-Karp es O(2^n · n²); no abuses.");
    let full = (1usize << n) - 1;
    let mut dp = vec![f64::INFINITY; (1usize << n) * n];
    let idx = |mask: usize, v: usize| mask * n + v;

    dp[idx(1 << start, start)] = 0.0;

    for size in 2..=n {
        for mask in 0..(1usize << n) {
            if mask.count_ones() as usize != size {
                continue;
            }
            if (mask & (1 << start)) == 0 {
                continue;
            }
            for v in 0..n {
                if (mask & (1 << v)) == 0 {
                    continue;
                }
                if v == start {
                    continue;
                }
                let prev_mask = mask ^ (1 << v);
                let mut best = f64::INFINITY;
                for u in 0..n {
                    if (prev_mask & (1 << u)) == 0 {
                        continue;
                    }
                    let prev = dp[idx(prev_mask, u)];
                    if prev == f64::INFINITY {
                        continue;
                    }
                    best = best.min(prev + dist[u][v]);
                }
                dp[idx(mask, v)] = best;
            }
        }
    }

    (0..n)
        .filter(|&v| v != start)
        .map(|v| dp[idx(full, v)] + dist[v][start])
        .fold(f64::INFINITY, f64::min)
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::graph::DiGraph;

    #[test]
    fn longest_path_dag_ejemplo() {
        // DAG: a -> b (1), a -> c (2), b -> d (3), c -> d (1).
        // Longest a->d = max(1+3, 2+1) = 4.
        let mut g: DiGraph<i32, i32> = DiGraph::new();
        let a = g.add_node(0);
        let b = g.add_node(0);
        let c = g.add_node(0);
        let d = g.add_node(0);
        g.add_edge(a, b, 1);
        g.add_edge(a, c, 2);
        g.add_edge(b, d, 3);
        g.add_edge(c, d, 1);
        let dp = longest_path_dag(&g, a);
        assert_eq!(dp[&d], 4);
    }

    #[test]
    fn subtree_sizes_arbol() {
        // Árbol: 0 -> 1, 0 -> 2, 1 -> 3, 1 -> 4.
        // Tamaños desde 0: {0: 5, 1: 3, 2: 1, 3: 1, 4: 1}.
        let mut g: DiGraph<(), ()> = DiGraph::new();
        let n0 = g.add_node(());
        let n1 = g.add_node(());
        let n2 = g.add_node(());
        let n3 = g.add_node(());
        let n4 = g.add_node(());
        g.add_edge(n0, n1, ());
        g.add_edge(n0, n2, ());
        g.add_edge(n1, n3, ());
        g.add_edge(n1, n4, ());
        let sizes = subtree_sizes(&g, n0);
        assert_eq!(sizes[&n0], 5);
        assert_eq!(sizes[&n1], 3);
        assert_eq!(sizes[&n2], 1);
        assert_eq!(sizes[&n3], 1);
        assert_eq!(sizes[&n4], 1);
    }

    #[test]
    fn held_karp_cuadrado() {
        let d = vec![
            vec![0.0, 1.0, 2.0, 1.0],
            vec![1.0, 0.0, 1.0, 2.0],
            vec![2.0, 1.0, 0.0, 1.0],
            vec![1.0, 2.0, 1.0, 0.0],
        ];
        assert!((held_karp(&d, 0) - 4.0).abs() < 1e-9);
    }

    #[test]
    fn held_karp_triangulo() {
        let d = vec![
            vec![0.0, 1.0, 1.0],
            vec![1.0, 0.0, 1.0],
            vec![1.0, 1.0, 0.0],
        ];
        assert!((held_karp(&d, 0) - 3.0).abs() < 1e-9);
    }
}
