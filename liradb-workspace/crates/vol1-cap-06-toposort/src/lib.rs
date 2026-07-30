//! Vol.I — Capítulo 6: Topological sort + detección de ciclos.
//!
//! Migrado desde `vol1-grafos-de-cero-a-experto-rust.md` §6.7.
//!
//! - [`kahn`] — algoritmo BFS por in-degree.
//! - [`dfs_topsort`] — reverse postorder iterativo.
//! - [`es_dag`] — wrapper que devuelve `true` si el grafo es un DAG.
//! - [`longest_path_dag`] — DP sobre orden topológico (camino más largo en DAG ponderado).

use std::collections::VecDeque;

/// Resultado de un topological sort.
#[derive(Debug, PartialEq, Eq)]
pub enum Topsort {
    /// Orden topológico válido.
    Order(Vec<usize>),
    /// El grafo tiene un ciclo; contiene los vértices atrapados.
    Cycle(Vec<usize>),
}

/// Kahn: BFS por in-degree. Devuelve `Order` si es DAG, `Cycle` si no.
pub fn kahn(n: usize, adj: &[Vec<usize>]) -> Topsort {
    let mut in_deg = vec![0usize; n];
    for adj_u in adj {
        for &v in adj_u {
            in_deg[v] += 1;
        }
    }

    let mut queue: VecDeque<usize> = (0..n).filter(|&u| in_deg[u] == 0).collect();
    let mut order = Vec::with_capacity(n);

    while let Some(u) = queue.pop_front() {
        order.push(u);
        for &v in &adj[u] {
            in_deg[v] -= 1;
            if in_deg[v] == 0 {
                queue.push_back(v);
            }
        }
    }

    if order.len() == n {
        Topsort::Order(order)
    } else {
        Topsort::Cycle((0..n).filter(|&u| in_deg[u] > 0).collect())
    }
}

/// DFS-based: reverse postorder, iterativo para no reventar la pila en grafos grandes.
pub fn dfs_topsort(n: usize, adj: &[Vec<usize>]) -> Topsort {
    // Colores: 0 = blanco (no visto), 1 = gris (en stack), 2 = negro (terminado).
    let mut color = vec![0u8; n];
    let mut stack: Vec<(usize, usize)> = Vec::new(); // (vértice, idx del vecino a explorar)
    let mut order = Vec::with_capacity(n);

    for start in 0..n {
        if color[start] != 0 {
            continue;
        }
        color[start] = 1;
        stack.push((start, 0));

        while let Some((u, i)) = stack.last().copied() {
            if i < adj[u].len() {
                let v = adj[u][i];
                stack.last_mut().unwrap().1 += 1;
                match color[v] {
                    0 => {
                        color[v] = 1;
                        stack.push((v, 0));
                    }
                    1 => {
                        // Back-edge: ciclo.
                        return Topsort::Cycle((0..n).filter(|&x| color[x] == 1).collect());
                    }
                    _ => {} // negro, ya procesado
                }
            } else {
                color[u] = 2;
                order.push(u);
                stack.pop();
            }
        }
    }

    order.reverse();
    Topsort::Order(order)
}

/// Detección rápida de ciclo: ¿el grafo es DAG?
pub fn es_dag(n: usize, adj: &[Vec<usize>]) -> bool {
    matches!(kahn(n, adj), Topsort::Order(_))
}

/// Longest path en DAG: DP sobre orden topológico.
///
/// `weights[u]` es el peso del vértice `u` (o `i64::MIN` para no incluir).  
/// Devuelve el peso máximo de cualquier path desde `source`.
pub fn longest_path_dag(n: usize, adj: &[Vec<usize>], weights: &[i64], source: usize) -> i64 {
    let order = match kahn(n, adj) {
        Topsort::Order(o) => o,
        Topsort::Cycle(_) => return i64::MIN, // no hay longest path en grafos cíclicos
    };
    let mut dist = vec![i64::MIN; n];
    dist[source] = weights[source];
    for &u in &order {
        if dist[u] == i64::MIN {
            continue;
        }
        for &v in &adj[u] {
            dist[v] = dist[v].max(dist[u] + weights[v]);
        }
    }
    *dist.iter().max().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dag_ejemplo() -> (usize, Vec<Vec<usize>>) {
        // 6 módulos con dependencias A->C, A->D, B->D, B->E, C->F, D->F
        let n = 6;
        let mut adj = vec![vec![]; n];
        // A=0, B=1, C=2, D=3, E=4, F=5
        adj[0].extend([2, 3]);
        adj[1].extend([3, 4]);
        adj[2].push(5);
        adj[3].push(5);
        (n, adj)
    }

    #[test]
    fn kahn_orden_valido() {
        let (n, adj) = dag_ejemplo();
        if let Topsort::Order(o) = kahn(n, &adj) {
            assert_eq!(o.len(), 6);
            let pos_f = o.iter().position(|&x| x == 5).unwrap();
            assert!(o.iter().take(pos_f).any(|&x| x == 0));
            assert!(o.iter().take(pos_f).any(|&x| x == 1));
        } else {
            panic!("debería ser un DAG");
        }
    }

    #[test]
    fn dfs_orden_valido() {
        let (n, adj) = dag_ejemplo();
        if let Topsort::Order(o) = dfs_topsort(n, &adj) {
            assert_eq!(o.len(), 6);
        } else {
            panic!("debería ser un DAG");
        }
    }

    #[test]
    fn detecta_ciclo() {
        let adj = vec![vec![1], vec![2], vec![0]];
        assert!(matches!(kahn(3, &adj), Topsort::Cycle(_)));
        assert!(matches!(dfs_topsort(3, &adj), Topsort::Cycle(_)));
        assert!(!es_dag(3, &adj));
    }

    #[test]
    fn longest_path_ejemplo() {
        // Cadena 0 -> 1 -> 2 -> 3 con pesos 1, 2, 3, 4.
        // Longest path = 1+2+3+4 = 10.
        let n = 4;
        let adj = vec![vec![1], vec![2], vec![3], vec![]];
        let weights = vec![1, 2, 3, 4];
        assert_eq!(longest_path_dag(n, &adj, &weights, 0), 10);
    }
}
