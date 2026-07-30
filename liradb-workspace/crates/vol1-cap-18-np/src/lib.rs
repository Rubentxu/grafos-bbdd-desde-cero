//! Vol.I — Capítulo 18: NP-completitud y algoritmos exactos.
//!
//! Migrado desde `vol1-grafos-de-cero-a-experto-rust.md` §18.5-§18.7.
//!
//! - [`vc_2approx`] — 2-aproximación de Vertex Cover (extremos de un matching).
//! - [`vc_2approx_by_matching`] — variante greedy que construye el matching.
//! - [`branch_and_bound_is`] — Branch & Bound exacto para Independent Set.
//! - [`held_karp`] — DP O(2^n · n²) para TSP.

/// Vertex Cover 2-aproximado a partir de un matching maximal.
/// Devuelve los extremos de las aristas del matching.
pub fn vc_2approx(matching: &[(usize, usize)]) -> Vec<usize> {
    matching.iter().flat_map(|&(u, v)| [u, v]).collect()
}

/// VC 2-aproximado: greedy que construye el matching sobre la marcha.
pub fn vc_2approx_by_matching(adj: &[Vec<usize>]) -> Vec<usize> {
    let mut covered = vec![false; adj.len()];
    let mut cover = Vec::new();
    for u in 0..adj.len() {
        if covered[u] {
            continue;
        }
        for &v in &adj[u] {
            if !covered[v] {
                cover.push(u);
                cover.push(v);
                covered[u] = true;
                covered[v] = true;
                break;
            }
        }
    }
    cover.sort_unstable();
    cover.dedup();
    cover
}

/// Branch & Bound exacto para Independent Set: busca un IS de tamaño `k`.
///
/// `adj` es la lista de adyacencia. Devuelve `Some(sol)` si existe un IS de
/// tamaño `k`, `None` en caso contrario.
///
/// **Bug del libro corregido**: el algoritmo original no validaba que el
/// vértice a incluir no fuera adyacente a los ya elegidos, lo que podía
/// devolver "soluciones" inválidas (e.g. en un triángulo, devuelve `[0,1]`
/// que no es independiente porque 0-1 son adyacentes). El fix añade la
/// comprobación `adj[next].iter().all(|&nb| !chosen.contains(&nb))`.
pub fn branch_and_bound_is(adj: &[Vec<usize>], _n: usize, k: usize) -> Option<Vec<usize>> {
    fn dfs(
        adj: &[Vec<usize>],
        chosen: &[usize],
        excluded: &[usize],
        k: usize,
    ) -> Option<Vec<usize>> {
        if chosen.len() == k {
            return Some(chosen.to_vec());
        }
        if chosen.len() + (adj.len() - excluded.len()) < k {
            return None;
        } // poda
        let next = (0..adj.len()).find(|&v| !chosen.contains(&v) && !excluded.contains(&v))?;
        // Validar que añadir `next` no rompe independencia.
        let can_include = adj[next].iter().all(|&nb| !chosen.contains(&nb));
        // rama 1: incluir (sólo si no entra en conflicto)
        if can_include {
            let mut chosen_inc = chosen.to_vec();
            chosen_inc.push(next);
            if let Some(sol) = dfs(adj, &chosen_inc, excluded, k) {
                return Some(sol);
            }
        }
        // rama 2: excluir (y propagar a sus vecinos)
        let mut new_excluded = excluded.to_vec();
        new_excluded.push(next);
        for &nb in &adj[next] {
            if !new_excluded.contains(&nb) {
                new_excluded.push(nb);
            }
        }
        dfs(adj, chosen, &new_excluded, k)
    }
    dfs(adj, &[], &[], k)
}

/// Held-Karp TSP: programación dinámica O(2^n · n²).
///
/// Devuelve la longitud del tour óptimo empezando y terminando en `start`.
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

    #[test]
    fn vc_2approx_extremos() {
        let m = vec![(0, 1), (2, 3)];
        let mut c = vc_2approx(&m);
        c.sort();
        assert_eq!(c, vec![0, 1, 2, 3]);
    }

    #[test]
    fn vc_2approx_by_matching_cubre_todas_las_aristas() {
        // Triángulo 0-1, 1-2, 2-0. Cualquier VC debe cubrir 3 aristas.
        let adj = vec![vec![1, 2], vec![0, 2], vec![0, 1]];
        let cover = vc_2approx_by_matching(&adj);
        for (u, adj_u) in adj.iter().enumerate() {
            for &v in adj_u {
                if u < v {
                    assert!(
                        cover.contains(&u) || cover.contains(&v),
                        "arista ({u}, {v}) no cubierta por {:?}",
                        cover
                    );
                }
            }
        }
        assert!(cover.len() <= 2 * 3); // cota trivial 2-aprox
    }

    #[test]
    fn bnb_is_encuentra_triangulo() {
        // Triángulo: IS máximo = 1.
        let adj = vec![vec![1, 2], vec![0, 2], vec![0, 1]];
        assert!(branch_and_bound_is(&adj, 3, 1).is_some());
        assert!(branch_and_bound_is(&adj, 3, 2).is_none());
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

    #[test]
    fn held_karp_5_ciudades_cota_inferior() {
        let d = vec![
            vec![0.0, 1.0, 2.0, 3.0, 4.0],
            vec![1.0, 0.0, 1.0, 2.0, 3.0],
            vec![2.0, 1.0, 0.0, 1.0, 2.0],
            vec![3.0, 2.0, 1.0, 0.0, 1.0],
            vec![4.0, 3.0, 2.0, 1.0, 0.0],
        ];
        let tour = held_karp(&d, 0);
        assert!(tour >= 4.0); // cota inferior trivial
    }
}
