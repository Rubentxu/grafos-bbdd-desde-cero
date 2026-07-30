//! Vol.I — Capítulo 8: Grafos bipartitos y matching.
//!
//! Migrado desde `vol1-grafos-de-cero-a-experto-rust.md` §8.
//!
//! - [`es_bipartito`] — 2-coloring BFS; `Some([L, R])` si bipartito, `None` si no.
//! - [`kuhn_matching`] — augmenting paths DFS (O(V·E)).
//! - [`hopcroft_karp`] — BFS + DFS en capas (O(√V · E)).
//!
//! No se incluye `lapjv` (Hungarian) en esta migración — ver Apéndice C.

/// Test de bipartitud por 2-coloring BFS.
/// Devuelve `Some([lado_L, lado_R])` si es bipartito, `None` si hay ciclo impar.
pub fn es_bipartito(n: usize, adj: &[Vec<usize>]) -> Option<(Vec<usize>, Vec<usize>)> {
    let mut color = vec![i8::MIN; n]; // -1 o +1
    let mut l = Vec::new();
    let mut r = Vec::new();

    for start in 0..n {
        if color[start] != i8::MIN {
            continue;
        }
        color[start] = 1;
        l.push(start);
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(start);
        while let Some(u) = queue.pop_front() {
            for &v in &adj[u] {
                if color[v] == i8::MIN {
                    color[v] = -color[u];
                    if color[v] == 1 {
                        l.push(v);
                    } else {
                        r.push(v);
                    }
                    queue.push_back(v);
                } else if color[v] == color[u] {
                    return None;
                }
            }
        }
    }
    Some((l, r))
}

/// Kuhn: matching bipartito por augmenting paths DFS.
///
/// Devuelve el número de matches y, opcionalmente, el vector `match_r[v]` para v∈R.
pub fn kuhn_matching(n_l: usize, n_r: usize, edges: &[(usize, usize)]) -> (usize, Vec<i32>) {
    // edges: pares (u, v) con u ∈ [0, n_l), v ∈ [0, n_r).
    // Construimos adj[u] = lista de v's conectados a u.
    let mut adj_l: Vec<Vec<usize>> = vec![vec![]; n_l];
    for &(u, v) in edges {
        if u < n_l && v < n_r {
            adj_l[u].push(v);
        }
    }
    let mut match_r: Vec<i32> = vec![-1; n_r];

    fn try_kuhn(u: usize, adj_l: &[Vec<usize>], match_r: &mut [i32], seen: &mut [bool]) -> bool {
        for &v in &adj_l[u] {
            if seen[v] {
                continue;
            }
            seen[v] = true;
            if match_r[v] == -1 || try_kuhn(match_r[v] as usize, adj_l, match_r, seen) {
                match_r[v] = u as i32;
                return true;
            }
        }
        false
    }

    let mut matches = 0;
    for u in 0..n_l {
        let mut seen = vec![false; n_r];
        if try_kuhn(u, &adj_l, &mut match_r, &mut seen) {
            matches += 1;
        }
    }
    (matches, match_r)
}

/// Hopcroft-Karp: matching bipartito en O(√V · E).
///
/// Estructura: BFS construye capas por distancia mínima al nodo libre;
/// DFS intenta encontrar augmenting paths sólo a través de esas capas.
pub fn hopcroft_karp(n_l: usize, n_r: usize, edges: &[(usize, usize)]) -> (usize, Vec<i32>) {
    let mut adj_l: Vec<Vec<usize>> = vec![vec![]; n_l];
    for &(u, v) in edges {
        if u < n_l && v < n_r {
            adj_l[u].push(v);
        }
    }
    let mut pair_u: Vec<i32> = vec![-1; n_l];
    let mut pair_v: Vec<i32> = vec![-1; n_r];
    let mut dist: Vec<usize> = vec![0; n_l];

    fn bfs(
        adj_l: &[Vec<usize>],
        pair_u: &[i32],
        pair_v: &[i32],
        dist: &mut [usize],
        n_l: usize,
    ) -> bool {
        let mut queue = std::collections::VecDeque::new();
        for u in 0..n_l {
            if pair_u[u] == -1 {
                dist[u] = 0;
                queue.push_back(u);
            } else {
                dist[u] = usize::MAX;
            }
        }
        let mut found = false;
        while let Some(u) = queue.pop_front() {
            for &v in &adj_l[u] {
                let pu = pair_v[v];
                if pu != -1 && dist[pu as usize] == usize::MAX {
                    dist[pu as usize] = dist[u] + 1;
                    queue.push_back(pu as usize);
                } else if pu == -1 {
                    found = true;
                }
            }
        }
        found
    }

    fn dfs(
        u: usize,
        adj_l: &[Vec<usize>],
        pair_u: &mut [i32],
        pair_v: &mut [i32],
        dist: &[usize],
    ) -> bool {
        for &v in &adj_l[u] {
            let pu = pair_v[v];
            if pu == -1
                || (dist[pu as usize] == dist[u] + 1
                    && dfs(pu as usize, adj_l, pair_u, pair_v, dist))
            {
                pair_u[u] = v as i32;
                pair_v[v] = u as i32;
                return true;
            }
        }
        false
    }

    let mut matching = 0;
    while bfs(&adj_l, &pair_u, &pair_v, &mut dist, n_l) {
        for u in 0..n_l {
            if pair_u[u] == -1 && dfs(u, &adj_l, &mut pair_u, &mut pair_v, &dist) {
                matching += 1;
            }
        }
    }
    (matching, pair_v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grafo_bipartito_ejemplo() -> (usize, usize, Vec<(usize, usize)>) {
        // Grafo bipartito simple: L = {0, 1, 2}, R = {0, 1, 2}.
        // Aristas: 0-0, 0-1, 1-1, 2-2, 2-0.
        // Matching máximo = 3.
        let n_l = 3;
        let n_r = 3;
        let edges = vec![(0, 0), (0, 1), (1, 1), (2, 2), (2, 0)];
        (n_l, n_r, edges)
    }

    #[test]
    fn bipartito_true() {
        let (n_l, n_r, edges) = grafo_bipartito_ejemplo();
        let mut adj = vec![vec![]; n_l + n_r];
        for &(u, v) in &edges {
            adj[u].push(n_l + v);
            adj[n_l + v].push(u);
        }
        let (l, r) = es_bipartito(n_l + n_r, &adj).expect("debería ser bipartito");
        assert_eq!(l.len() + r.len(), n_l + n_r);
    }

    #[test]
    fn no_bipartito_con_ciclo_impar() {
        // Triángulo 0-1-2-0 no es bipartito.
        let adj = vec![vec![1, 2], vec![0, 2], vec![0, 1]];
        assert!(es_bipartito(3, &adj).is_none());
    }

    #[test]
    fn kuhn_encuentra_matching_maximo() {
        let (n_l, n_r, edges) = grafo_bipartito_ejemplo();
        let (m, _) = kuhn_matching(n_l, n_r, &edges);
        assert_eq!(m, 3);
    }

    #[test]
    fn hopcroft_karp_encuentra_matching_maximo() {
        let (n_l, n_r, edges) = grafo_bipartito_ejemplo();
        let (m, _) = hopcroft_karp(n_l, n_r, &edges);
        assert_eq!(m, 3);
    }

    #[test]
    fn kuhn_y_hopcroft_karp_coinciden() {
        let (n_l, n_r, edges) = grafo_bipartito_ejemplo();
        let (m1, _) = kuhn_matching(n_l, n_r, &edges);
        let (m2, _) = hopcroft_karp(n_l, n_r, &edges);
        assert_eq!(m1, m2);
    }
}
