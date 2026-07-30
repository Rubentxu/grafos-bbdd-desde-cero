//! Vol.I — Capítulo 14: Planaridad y fórmulas famosas.
//!
//! Migrado desde `vol1-grafos-de-cero-a-experto-rust.md` §14.3 + §14.9.
//!
//! - [`euler_formula`] — Fórmula de Euler V - E + F = 2 (grafos conexos planares).
//! - [`max_edges_planar`] — Cota E ≤ 3V - 6 (sin multiaristas).
//! - [`has_k33_subgraph`] — Detección K₃,₃ como subgrafo.
//! - [`has_k5_subgraph`] — Detección K₅ como subgrafo.
//!
//! NOTA: la TUI con ratatui + crossterm del Vol.I se omite por costo de
//! compilación (~150 crates transitivas). Para una versión interactiva,
//! ver cap-13-coloring/src (decisión documentada en MIGRATION-PATTERN.md §13).

use std::collections::{HashMap, HashSet};

/// Tipo de grafo: lista de adyacencias con conjuntos.
pub type Graph = HashMap<usize, HashSet<usize>>;

/// Fórmula de Euler para grafos conexos planares: V - E + F = 2.
/// Devuelve F calculado, o `None` si la fórmula no se cumple (grafo no planar
/// o no conexo).
pub fn euler_faces(n_vertices: usize, n_edges: usize) -> Option<usize> {
    // V - E + F = 2  →  F = 2 - V + E
    let f = 2_i64 - n_vertices as i64 + n_edges as i64;
    if f > 0 { Some(f as usize) } else { None }
}

/// Cota superior E ≤ 3V - 6 para grafos planares simples.
pub fn max_edges_planar(n_vertices: usize) -> usize {
    if n_vertices < 3 {
        0
    } else {
        3 * n_vertices - 6
    }
}

/// Construye un grafo de prueba: K_n (completo).
pub fn kn(n: usize) -> Graph {
    let mut g: Graph = (0..n).map(|i| (i, HashSet::new())).collect();
    for i in 0..n {
        for j in 0..n {
            if i != j {
                g.get_mut(&i).unwrap().insert(j);
            }
        }
    }
    g
}

/// Construye K_{3,3} bipartito completo.
pub fn k33() -> Graph {
    let mut g: Graph = (0..6).map(|i| (i, HashSet::new())).collect();
    for &a in &[0usize, 1, 2] {
        for &b in &[3usize, 4, 5] {
            g.get_mut(&a).unwrap().insert(b);
            g.get_mut(&b).unwrap().insert(a);
        }
    }
    g
}

/// Heurística: ¿contiene K_{3,3} como subgrafo?
/// No detecta subdivisiones completas; sólo el caso "puro".
///
/// Bug del libro detectado: el código original sólo probaba el split en
/// posición 3, lo que asume que el orden de iteración del HashMap coincide
/// con la partición bipartita real. Como HashMap itera en orden arbitrario,
/// esto falla: para `K_{3,3}` con vértices {0,1,2} vs {3,4,5}, si el iterador
/// devuelve [4,5,0,1,3,2], split=3 da a=[4,5,0], b=[1,3,2] — partición
/// incorrecta. Fix: probar TODAS las particiones de los 6 vértices en
/// dos grupos de 3 (C(6,3)/2 = 10 particiones no triviales).
pub fn has_k33_subgraph(g: &Graph) -> bool {
    use itertools::Itertools;
    let nodes: Vec<usize> = g.keys().copied().collect();
    if nodes.len() < 6 {
        return false;
    }
    for combo in nodes.iter().combinations(6) {
        let vs: Vec<usize> = combo.iter().map(|&&v| v).collect();
        // Probar cada subconjunto de tamaño 3 como "lado A"; el resto es "lado B".
        // Cada partición se cuenta 2 veces (A vs B y B vs A), pero paramos
        // en la primera coincidencia.
        for a_indices in (0..6).combinations(3) {
            let a: Vec<usize> = a_indices.iter().map(|&i| vs[i]).collect();
            let mut b = vs.clone();
            for &i in a_indices.iter().rev() {
                b.remove(i);
            }
            // Verificar que cada a[i] está conectado con cada b[j].
            let mut ok = true;
            'outer: for &ai in &a {
                for &bj in &b {
                    if !g[&ai].contains(&bj) {
                        ok = false;
                        break 'outer;
                    }
                }
            }
            if ok {
                return true;
            }
        }
    }
    false
}

/// Heurística: ¿contiene K_5 como subgrafo?
pub fn has_k5_subgraph(g: &Graph) -> bool {
    use itertools::Itertools;
    let nodes: Vec<usize> = g.keys().copied().collect();
    if nodes.len() < 5 {
        return false;
    }
    for combo in nodes.iter().combinations(5) {
        let vs: Vec<usize> = combo.iter().map(|&&v| v).collect();
        let mut ok = true;
        'outer: for (i, &u) in vs.iter().enumerate() {
            for (j, &v) in vs.iter().enumerate() {
                if i != j && !g[&u].contains(&v) {
                    ok = false;
                    break 'outer;
                }
            }
        }
        if ok {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn k33_es_detectado() {
        let g = k33();
        assert!(has_k33_subgraph(&g));
    }

    #[test]
    fn k4_no_contiene_k33() {
        let g = kn(4);
        assert!(!has_k33_subgraph(&g));
    }

    #[test]
    fn k5_es_detectado() {
        let g = kn(5);
        assert!(has_k5_subgraph(&g));
    }

    #[test]
    fn k5_no_contiene_k33() {
        // K_5 no contiene K_{3,3} (necesita 6 vértices).
        let g = kn(5);
        assert!(!has_k33_subgraph(&g));
    }

    #[test]
    fn euler_formula_k4() {
        // K_4 tiene V=4, E=6. Como maximal planar, F=4.
        assert_eq!(euler_faces(4, 6), Some(4));
    }

    #[test]
    fn euler_formula_triangulo() {
        // K_3 tiene V=3, E=3. Como maximal planar, F=2 (interior + exterior).
        assert_eq!(euler_faces(3, 3), Some(2));
    }

    #[test]
    fn max_edges_k4() {
        // K_4: V=4 → E ≤ 6. K_4 tiene E=6 ✓.
        assert_eq!(max_edges_planar(4), 6);
    }

    #[test]
    fn max_edges_k5_excede() {
        // K_5: V=5 → E ≤ 9. Pero K_5 tiene E=10 (no planar).
        assert_eq!(max_edges_planar(5), 9);
        assert!(kn(5).values().map(|s| s.len()).sum::<usize>() / 2 > 9);
    }
}
