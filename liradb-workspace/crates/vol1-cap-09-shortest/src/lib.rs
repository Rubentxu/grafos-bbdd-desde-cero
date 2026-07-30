//! Vol.I — Capítulo 9: Shortest paths avanzado.
//!
//! Migrado desde `vol1-grafos-de-cero-a-experto-rust.md` §9.2-§9.5.
//!
//! Allow clippy lints globales: este crate es una traducción directa del Vol.I,
//! que usa `for i in 0..n { ...vec[i]... }` y similares. Reescribir a iteradores
//! no aporta claridad pedagógica.
//!
//! - [`floyd_warshall`] — todos los pares, O(V³), admite pesos negativos.
//! - [`johnson`] — todos los pares, O(V·E + V² log V) en grafos dispersos.
//! - [`shortest_path_dag`] — shortest path en DAG, O(V+E).
//! - [`astar`] — A* con heurística Manhattan sobre cuadrícula.
//!
//! Allow clippy lints globales: este crate es una traducción directa del Vol.I,
//! que usa `for i in 0..n { ...vec[i]... }` y similares. Reescribir a iteradores
//! no aporta claridad pedagógica.

#![allow(clippy::needless_range_loop, clippy::type_complexity, unused_variables)]

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, VecDeque};

/// Floyd-Warshall: distancias mínimas para todos los pares.
/// Si hay un ciclo negativo, alguna diagonal quedará < 0.
pub fn floyd_warshall(graph: &[Vec<Option<i64>>]) -> Vec<Vec<i64>> {
    let n = graph.len();
    let inf = i64::MAX / 4;
    let mut dist = vec![vec![inf; n]; n];

    for i in 0..n {
        dist[i][i] = 0;
        for j in 0..n {
            if let Some(w) = graph[i][j] {
                dist[i][j] = w;
            }
        }
    }

    for k in 0..n {
        for i in 0..n {
            for j in 0..n {
                let via_k = dist[i][k].saturating_add(dist[k][j]);
                if via_k < dist[i][j] {
                    dist[i][j] = via_k;
                }
            }
        }
    }

    dist
}

/// Dijkstra estándar desde `src` con pesos no negativos.
pub fn dijkstra(graph: &[Vec<(usize, i64)>], src: usize) -> (Vec<i64>, Vec<Option<usize>>) {
    let n = graph.len();
    let inf = i64::MAX / 4;
    let mut dist = vec![inf; n];
    let mut prev: Vec<Option<usize>> = vec![None; n];
    dist[src] = 0;

    let mut heap: BinaryHeap<Reverse<(i64, usize)>> = BinaryHeap::new();
    heap.push(Reverse((0, src)));

    while let Some(Reverse((d, u))) = heap.pop() {
        if d > dist[u] {
            continue;
        }
        for &(v, w) in &graph[u] {
            let nd = d.saturating_add(w);
            if nd < dist[v] {
                dist[v] = nd;
                prev[v] = Some(u);
                heap.push(Reverse((nd, v)));
            }
        }
    }
    (dist, prev)
}

/// Bellman-Ford desde súper-origen (h inicial = 0).
/// Sirve para encontrar potenciales válidos (reweighting de Johnson).
pub fn bellman_ford_from_super(edges: &[(usize, usize, i64)], n: usize) -> Option<Vec<i64>> {
    let inf = i64::MAX / 4;
    let mut h = vec![0i64; n];
    for _ in 0..n.saturating_sub(1) {
        let mut changed = false;
        for &(u, v, w) in edges {
            if h[u].saturating_add(w) < h[v] {
                h[v] = h[u].saturating_add(w);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    for &(u, v, w) in edges {
        if h[u].saturating_add(w) < h[v] {
            return None;
        }
    }
    Some(h)
}

/// Johnson's algorithm: todos los pares, O(V·E + V²·log V) en grafos dispersos.
pub fn johnson(graph: &[Vec<(usize, i64)>]) -> Option<Vec<Vec<i64>>> {
    let n = graph.len();
    let mut edges: Vec<(usize, usize, i64)> =
        Vec::with_capacity(graph.iter().map(|v| v.len()).sum());
    for (u, vs) in graph.iter().enumerate() {
        for &(v, w) in vs {
            edges.push((u, v, w));
        }
    }

    let h = bellman_ford_from_super(&edges, n)?;

    let reweighted: Vec<Vec<(usize, i64)>> = (0..n)
        .map(|u| {
            graph[u]
                .iter()
                .map(|&(v, w)| (v, w + h[u] - h[v]))
                .collect()
        })
        .collect();

    let mut all_dist = vec![vec![0i64; n]; n];
    for src in 0..n {
        let (d, _) = dijkstra(&reweighted, src);
        for v in 0..n {
            all_dist[src][v] = d[v] - h[src] + h[v];
        }
    }
    Some(all_dist)
}

/// Shortest path desde `src` en un DAG.
/// Devuelve distancias y predecesores. Si el grafo tiene ciclos, no garantizamos nada.
pub fn shortest_path_dag(
    graph: &[Vec<(usize, i64)>],
    indeg: &[usize],
    src: usize,
) -> (Vec<i64>, Vec<Option<usize>>) {
    let n = graph.len();
    let inf = i64::MAX / 4;
    let mut dist = vec![inf; n];
    let mut prev: Vec<Option<usize>> = vec![None; n];
    dist[src] = 0;

    // Orden topológico: Kahn clásico.
    let mut queue: VecDeque<usize> = VecDeque::new();
    let mut indeg = indeg.to_vec();
    for v in 0..n {
        if indeg[v] == 0 {
            queue.push_back(v);
        }
    }
    let mut order = Vec::with_capacity(n);
    while let Some(u) = queue.pop_front() {
        order.push(u);
        for &(v, _) in &graph[u] {
            indeg[v] -= 1;
            if indeg[v] == 0 {
                queue.push_back(v);
            }
        }
    }

    for &u in &order {
        if dist[u] == inf {
            continue;
        }
        for &(v, w) in &graph[u] {
            let nd = dist[u].saturating_add(w);
            if nd < dist[v] {
                dist[v] = nd;
                prev[v] = Some(u);
            }
        }
    }
    (dist, prev)
}

/// Heurística admisible: distancia Manhattan.
pub fn manhattan(a: (i32, i32), b: (i32, i32)) -> i64 {
    ((a.0 - b.0).abs() + (a.1 - b.1).abs()) as i64
}

/// A* sobre una cuadrícula 2D. Movimientos en 4 direcciones, coste 1.
pub fn astar(
    start: (i32, i32),
    goal: (i32, i32),
    blocked: &[(i32, i32)],
) -> Option<Vec<(i32, i32)>> {
    let mut open: BinaryHeap<Reverse<(i64, i64, (i32, i32))>> = BinaryHeap::new();
    let mut g: HashMap<(i32, i32), i64> = HashMap::new();
    let mut came_from: HashMap<(i32, i32), (i32, i32)> = HashMap::new();

    let h0 = manhattan(start, goal);
    g.insert(start, 0);
    open.push(Reverse((h0, 0, start)));

    while let Some(Reverse((_, cost, current))) = open.pop() {
        if current == goal {
            let mut path = vec![current];
            let mut c = current;
            while let Some(&p) = came_from.get(&c) {
                path.push(p);
                c = p;
            }
            path.reverse();
            return Some(path);
        }
        if cost > *g.get(&current).unwrap_or(&i64::MAX) {
            continue;
        }

        for (dx, dy) in [(0, 1), (1, 0), (0, -1), (-1, 0)] {
            let next = (current.0 + dx, current.1 + dy);
            if blocked.contains(&next) {
                continue;
            }
            let tentative = cost + 1;
            if tentative < *g.get(&next).unwrap_or(&i64::MAX) {
                came_from.insert(next, current);
                g.insert(next, tentative);
                let f = tentative + manhattan(next, goal);
                open.push(Reverse((f, tentative, next)));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests_floyd {
    use super::*;

    #[test]
    fn grafo_simple_3_nodos() {
        // Cuadrado 4 nodos: 0--1--3
        //                    2--/
        // 0-1:1, 0-2:4, 1-3:2, 2-3:1.
        let g = vec![
            vec![None, Some(1), Some(4), None],
            vec![Some(1), None, None, Some(2)],
            vec![Some(4), None, None, Some(1)],
            vec![None, Some(2), Some(1), None],
        ];
        let d = floyd_warshall(&g);
        assert_eq!(d[0][3], 3); // 0 -> 1 -> 3 = 1+2
        // d[0][2] = min(0->2 directo = 4, 0->1->3->2 = 1+2+1 = 4) = 4.
        // Bug del libro: assert_eq!(d[0][2], 3) — no hay forma de llegar en 3.
        assert_eq!(d[0][2], 4);
        assert_eq!(d[3][0], 3); // 3 -> 2 -> 0 = 1+4 = 5? No, 3 -> 1 -> 0 = 2+1 = 3.
        for i in 0..4 {
            assert_eq!(d[i][i], 0);
        }
    }

    #[test]
    fn detecta_ciclo_negativo() {
        // 0 -> 1 (1), 1 -> 2 (-3), 2 -> 0 (1) → ciclo de suma -1.
        let g = vec![
            vec![None, Some(1), None],
            vec![None, None, Some(-3)],
            vec![Some(1), None, None],
        ];
        let d = floyd_warshall(&g);
        assert!(d[0][0] < 0);
    }
}

#[cfg(test)]
mod tests_johnson {
    use super::*;

    #[test]
    fn johnson_coincide_con_floyd() {
        let g = vec![
            vec![(1, 1), (2, 4)],
            vec![(0, 1), (3, 2)],
            vec![(0, 4), (3, 3)],
            vec![(1, 2), (2, 3)],
        ];
        let j = johnson(&g).expect("sin ciclos negativos");

        // Comparar con Floyd.
        let f_adj: Vec<Vec<Option<i64>>> = (0..4)
            .map(|i| {
                (0..4)
                    .map(|j| g[i].iter().find(|&&(v, _)| v == j).map(|&(_, w)| w))
                    .collect()
            })
            .collect();
        let f = floyd_warshall(&f_adj);

        for i in 0..4 {
            for k in 0..4 {
                assert_eq!(j[i][k], f[i][k], "diff en ({}, {})", i, k);
            }
        }
    }
}

#[cfg(test)]
mod tests_dag {
    use super::*;

    #[test]
    fn dag_basico() {
        // 0 -> 1 (5), 0 -> 2 (3), 2 -> 1 (1), 1 -> 3 (2).
        // Camino óptimo 0->2->1->3 = 3+1+2 = 6.
        let g = vec![vec![(1, 5), (2, 3)], vec![(3, 2)], vec![(1, 1)], vec![]];
        let indeg = vec![0, 2, 1, 1];
        let (d, _) = shortest_path_dag(&g, &indeg, 0);
        assert_eq!(d[0], 0);
        assert_eq!(d[3], 6);
    }
}

#[cfg(test)]
mod tests_astar {
    use super::*;

    #[test]
    fn astar_llega_a_meta() {
        let path = astar((0, 0), (3, 3), &[]).unwrap();
        assert_eq!(path.len() - 1, 6); // 6 movimientos
        assert_eq!(path.first(), Some(&(0, 0)));
        assert_eq!(path.last(), Some(&(3, 3)));
    }

    #[test]
    fn astar_evita_obstaculos() {
        let wall: Vec<(i32, i32)> = (0..3).map(|y| (1, y)).collect();
        let path = astar((0, 0), (2, 2), &wall).unwrap();
        assert!(path.contains(&(1, 3)) || path.len() - 1 == 6);
    }
}
