//! Vol.I — Capítulo 4: Dijkstra y Bellman-Ford (versiones a mano, sin petgraph).
//!
//! Migrado desde `vol1-grafos-de-cero-a-experto-rust.md` §4.4 y §4.5.
//!
//! - §4.4 Dijkstra con `BinaryHeap<Reverse<(u32, usize)>>` (truco min-heap).
//! - §4.5 Bellman-Ford con pesos negativos (`i64`) y detección de ciclos.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Lista de adyacencia ponderada: `adj[u] = [(v_1, w_1), (v_2, w_2), ...]`.
pub type AristasPonderadas = Vec<Vec<(u32, u32)>>;

/// Devuelve un vector `dist` donde `dist[v]` es la distancia mínima desde
/// `origen` hasta `v`. Si `v` es inalcanzable, `dist[v] = u32::MAX`.
///
/// Asume pesos **no negativos**. Para pesos negativos, usa [`bellman_ford`].
pub fn dijkstra(adj: &AristasPonderadas, origen: usize) -> Vec<u32> {
    let n = adj.len();
    let mut dist: Vec<u32> = vec![u32::MAX; n];
    let mut heap: BinaryHeap<Reverse<(u32, usize)>> = BinaryHeap::new();

    dist[origen] = 0;
    heap.push(Reverse((0, origen)));

    while let Some(Reverse((d, u))) = heap.pop() {
        // Si ya hay una distancia mejor registrada, saltamos.
        if d > dist[u] {
            continue;
        }
        for &(v, peso) in &adj[u] {
            let v = v as usize;
            // Importante: controlar overflow antes de sumar.
            let nueva = match d.checked_add(peso) {
                Some(x) => x,
                None => continue,
            };
            if nueva < dist[v] {
                dist[v] = nueva;
                heap.push(Reverse((nueva, v)));
            }
        }
    }
    dist
}

/// Bellman-Ford para grafos con posibles pesos negativos.
///
/// `aristas`: lista de tuplas `(u, v, peso)` para un grafo dirigido.
/// `n`: número de vértices. `origen`: vértice de partida.
///
/// Devuelve `Ok(dist)` si no hay ciclo negativo alcanzable; `Err(msg)` si lo hay.
pub fn bellman_ford(
    aristas: &[(u32, u32, i64)],
    n: usize,
    origen: usize,
) -> Result<Vec<i64>, &'static str> {
    let mut dist: Vec<i64> = vec![i64::MAX; n];
    dist[origen] = 0;

    for _ in 0..n - 1 {
        let mut cambio = false;
        for &(u, v, w) in aristas {
            if dist[u as usize] != i64::MAX && dist[u as usize] + w < dist[v as usize] {
                dist[v as usize] = dist[u as usize] + w;
                cambio = true;
            }
        }
        if !cambio {
            break; // optimización: si nada cambió, terminamos
        }
    }

    // Detección de ciclo negativo.
    for &(u, v, w) in aristas {
        if dist[u as usize] != i64::MAX && dist[u as usize] + w < dist[v as usize] {
            return Err("¡Hay un ciclo negativo alcanzable desde el origen!");
        }
    }
    Ok(dist)
}

/// Grafo de ejemplo del capítulo 4 (4 vértices, 4 aristas, ponderado).
///
/// ```text
///     0 --1-- 1
///     |       |
///     4       2
///     |       |
///     2 --3-- 3
/// ```
pub fn grafo_ejemplo() -> AristasPonderadas {
    vec![
        vec![(1, 1), (2, 4)],
        vec![(0, 1), (3, 2)],
        vec![(0, 4), (3, 3)],
        vec![(1, 2), (2, 3)],
    ]
}

#[cfg(test)]
mod tests_dijkstra {
    use super::*;

    #[test]
    fn distancias_desde_0() {
        let g = grafo_ejemplo();
        let dist = dijkstra(&g, 0);
        // dist[0] = 0
        // dist[1] = 1 (0 -> 1)
        // dist[2] = 4 (0 -> 2 directo) mejor que 0 -> 1 -> 3 -> 2 = 1+2+3 = 6
        // dist[3] = 3 (0 -> 1 -> 3) mejor que 0 -> 2 -> 3 = 4+3 = 7
        assert_eq!(dist, vec![0, 1, 4, 3]);
    }

    #[test]
    fn destino_inalcanzable_es_max() {
        // Grafo con 3 vértices pero sólo 0 y 1 conectados; 2 es aislado.
        let g = vec![
            vec![(1, 5)], // 0 -> 1
            vec![(0, 5)], // 1 -> 0
            vec![],       // 2 aislado (sin aristas)
        ];
        let dist = dijkstra(&g, 0);
        assert_eq!(dist[0], 0);
        assert_eq!(dist[1], 5);
        assert_eq!(dist[2], u32::MAX);
    }
}

#[cfg(test)]
mod tests_bellman_ford {
    use super::*;

    #[test]
    fn grafo_sin_negativos_coincide_con_dijkstra() {
        let adj = grafo_ejemplo();
        let aristas = vec![
            (0, 1, 1_i64),
            (0, 2, 4),
            (1, 0, 1),
            (1, 3, 2),
            (2, 0, 4),
            (2, 3, 3),
            (3, 1, 2),
            (3, 2, 3),
        ];
        let dist_d = dijkstra(&adj, 0);
        let dist_bf = bellman_ford(&aristas, 4, 0).unwrap();
        let dist_d_i64: Vec<i64> = dist_d.iter().map(|&x| x as i64).collect();
        assert_eq!(dist_bf, dist_d_i64);
    }

    #[test]
    fn detecta_ciclo_negativo() {
        // 0 -> 1 (peso 1), 1 -> 2 (peso -3), 2 -> 0 (peso 1) suma -1 por ciclo.
        let aristas = vec![(0, 1, 1_i64), (1, 2, -3), (2, 0, 1)];
        let res = bellman_ford(&aristas, 3, 0);
        assert!(res.is_err());
    }

    #[test]
    fn permite_pesos_negativos_sin_ciclo() {
        // 0 -> 1 (peso 1), 1 -> 2 (peso -1). Sin ciclo.
        let aristas = vec![(0, 1, 1_i64), (1, 2, -1)];
        let res = bellman_ford(&aristas, 3, 0).unwrap();
        assert_eq!(res[0], 0);
        assert_eq!(res[1], 1);
        assert_eq!(res[2], 0);
    }
}
