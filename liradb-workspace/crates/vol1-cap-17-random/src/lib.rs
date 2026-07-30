//! Vol.I — Capítulo 17: Algoritmos randomizados.
//!
//! Migrado desde `vol1-grafos-de-cero-a-experto-rust.md` §17.2 y §17.4.
//!
//! - [`karger_min_cut`] — algoritmo de contracción aleatoria (Karger 1993).
//! - [`karger_min_cut_repeated`] — corre Karger N veces y devuelve el mínimo.
//! - [`find_independent_set`] — método probabilístico (alteration method).

use rand::Rng;
use rand::seq::IteratorRandom;

/// Representamos el grafo como lista de adyacencia.
pub type Adj = Vec<Vec<usize>>;

/// Construye un grafo simple a partir de aristas (no dirigido).
pub fn from_edges(n: usize, edges: &[(usize, usize)]) -> Adj {
    let mut adj = vec![Vec::new(); n];
    for &(u, v) in edges {
        debug_assert!(u < n && v < n && u != v);
        adj[u].push(v);
        adj[v].push(u);
    }
    adj
}

/// Karger min-cut: contracción aleatoria hasta que queden 2 vértices.
///
/// **Nota**: implementación correcta y completa — usa máscara de "active"
/// en vez de compactar el grafo, lo que evita el bug del cap-11-mincut
/// (que re-implementaba Karger con closure `&mut` y daba resultados
/// incorrectos).
pub fn karger_min_cut(mut adj: Adj, rng: &mut impl Rng) -> usize {
    let n = adj.len();
    if n < 2 {
        return 0;
    }

    let mut active: Vec<bool> = vec![true; n];
    let mut num_active = n;

    while num_active > 2 {
        let edges = collect_active_edges(&adj, &active);
        if edges.is_empty() {
            return 0;
        }
        let (u, v) = *edges.iter().choose(rng).expect("arista");

        // Fusiona `v` dentro de `u`: todo vecino de `v` se vuelve vecino de `u`.
        let v_neighbors = adj[v].clone();
        for w in v_neighbors {
            if w == u {
                continue;
            }
            adj[u].push(w);
            for x in adj[w].iter_mut() {
                if *x == v {
                    *x = u;
                }
            }
        }
        adj[v].clear();
        active[v] = false;
        num_active -= 1;

        // Limpia auto-loops en `u`.
        adj[u].retain(|&x| x != u);
    }

    let remaining: usize = (0..n).filter(|&i| active[i]).map(|i| adj[i].len()).sum();
    remaining / 2 // cada arista se cuenta dos veces
}

fn collect_active_edges(adj: &Adj, active: &[bool]) -> Vec<(usize, usize)> {
    let mut edges = Vec::new();
    for (u, ns) in adj.iter().enumerate() {
        if !active[u] {
            continue;
        }
        for &v in ns {
            if active[v] && u < v {
                edges.push((u, v));
            }
        }
    }
    edges
}

/// Wrapper: corre Karger `trials` veces y devuelve el mínimo.
pub fn karger_min_cut_repeated(adj: Adj, trials: usize, rng: &mut impl Rng) -> usize {
    (0..trials)
        .map(|_| karger_min_cut(adj.clone(), rng))
        .min()
        .unwrap_or(0)
}

/// Método probabilístico para encontrar un independent set aproximado.
///
/// Cada vértice se incluye con probabilidad `p`. Luego se eliminan conflictos
/// (si `i` y `j` están ambos y `j > i`, se quita `j`).
pub fn find_independent_set(g: &[Vec<usize>], rng: &mut impl Rng) -> Vec<usize> {
    let n = g.len();
    let p = 0.5;
    let mut chosen: Vec<bool> = (0..n).map(|_| rng.random::<f64>() < p).collect();
    for i in 0..n {
        if !chosen[i] {
            continue;
        }
        for &j in &g[i] {
            if chosen[j] && j > i {
                chosen[j] = false;
            }
        }
    }
    (0..n).filter(|&i| chosen[i]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    /// Brute force global min-cut: enumera todas las particiones no triviales
    /// representadas como máscaras de bits. Devuelve el coste mínimo.
    /// Solo válido para grafos pequeños (<= 16 nodos en la práctica).
    fn brute_force_min_cut(n: usize, adj: &Adj) -> usize {
        let mut best = usize::MAX;
        let aristas: Vec<(usize, usize)> = collect_all_edges(adj);
        for mask in 1..(1usize << n) - 1 {
            let mut cut = 0;
            for &(u, v) in &aristas {
                let in_s_u = (mask >> u) & 1 == 1;
                let in_s_v = (mask >> v) & 1 == 1;
                if in_s_u != in_s_v {
                    cut += 1;
                }
            }
            if cut < best {
                best = cut;
            }
        }
        best
    }

    fn collect_all_edges(adj: &Adj) -> Vec<(usize, usize)> {
        let mut edges = Vec::new();
        for (u, ns) in adj.iter().enumerate() {
            for &v in ns {
                if u < v {
                    edges.push((u, v));
                }
            }
        }
        edges
    }

    /// Triángulo: brute force = 2 (par de aristas). Karger debe dar >= 2.
    #[test]
    fn triangulo() {
        let adj = from_edges(3, &[(0, 1), (1, 2), (0, 2)]);
        let mut rng = StdRng::seed_from_u64(42);
        let brute = brute_force_min_cut(3, &adj);
        assert_eq!(brute, 2);
        let cut = karger_min_cut_repeated(adj, 50, &mut rng);
        assert!(cut >= brute, "Karger no debe dar corte menor que el óptimo");
        assert!(cut <= 3, "trivial upper bound: total de aristas");
    }

    /// Ciclo de 4: brute force = 2. Karger debe dar >= 2.
    #[test]
    fn ciclo_cuatro() {
        let adj = from_edges(4, &[(0, 1), (1, 2), (2, 3), (3, 0)]);
        let mut rng = StdRng::seed_from_u64(7);
        let brute = brute_force_min_cut(4, &adj);
        assert_eq!(brute, 2);
        let cut = karger_min_cut_repeated(adj, 100, &mut rng);
        assert!(cut >= brute);
        assert!(cut <= 4);
    }

    /// K4: brute force = 3. Karger debe dar >= 3.
    #[test]
    fn k4() {
        let adj = from_edges(4, &[(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]);
        let mut rng = StdRng::seed_from_u64(2024);
        let brute = brute_force_min_cut(4, &adj);
        assert_eq!(brute, 3);
        let cut = karger_min_cut_repeated(adj, 200, &mut rng);
        assert!(cut >= brute);
        assert!(cut <= 6);
    }

    /// Independent set probabilístico: la salida debe ser efectivamente independiente.
    #[test]
    fn independent_set_valido() {
        let g = vec![vec![1, 2], vec![0, 2], vec![0, 1]]; // triángulo
        let mut rng = StdRng::seed_from_u64(1);
        let is = find_independent_set(&g, &mut rng);
        for &u in &is {
            for &v in &g[u] {
                if u < v {
                    assert!(
                        !is.contains(&v),
                        "IS no válido: contiene {u} y {v} adyacentes"
                    );
                }
            }
        }
    }
}
