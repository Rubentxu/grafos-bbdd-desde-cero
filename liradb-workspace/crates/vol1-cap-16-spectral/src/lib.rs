//! Vol.I — Capítulo 16: Teoría espectral de grafos.
//!
//! Migrado desde `vol1-grafos-de-cero-a-experto-rust.md` §16.5-§16.6.
//!
//! - [`laplacian`] — Construye la matriz Laplaciana L = D - A.
//! - [`sorted_eigenvalues`] — Autovalores de la Laplaciana ordenados.
//! - [`pagerank`] — PageRank por power iteration.
//!
//! Requiere `nalgebra = "0.32"`.

use nalgebra::{DMatrix, DVector, SymmetricEigen};

/// Construye la Laplaciana de un grafo a partir de su lista de aristas.
pub fn laplacian(n: usize, edges: &[(usize, usize)]) -> DMatrix<f64> {
    let mut l = DMatrix::<f64>::zeros(n, n);
    let mut deg = vec![0i32; n];
    for &(u, v) in edges {
        l[(u, v)] = -1.0;
        l[(v, u)] = -1.0;
        deg[u] += 1;
        deg[v] += 1;
    }
    for i in 0..n {
        l[(i, i)] = deg[i] as f64;
    }
    l
}

/// Autovalores de la Laplaciana ordenados de menor a mayor.
pub fn sorted_eigenvalues(l: &DMatrix<f64>) -> Vec<f64> {
    let sym = SymmetricEigen::new(l.clone());
    let mut evs: Vec<f64> = sym.eigenvalues.iter().copied().collect();
    evs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    evs
}

/// PageRank por power iteration.
///
/// `m` es la matriz estocástica por columnas (cada columna suma 1).
/// `dangling` indica qué nodos son "sumideros" (sin salida).
pub fn pagerank(
    m: &DMatrix<f64>,
    alpha: f64,
    dangling: &[bool],
    tol: f64,
    max_iter: usize,
) -> DVector<f64> {
    let n = m.nrows();
    let mut v = DVector::from_element(n, 1.0 / n as f64);
    let teleport = DVector::from_element(n, (1.0 - alpha) / n as f64);

    for _ in 0..max_iter {
        let dangling_sum: f64 = v
            .iter()
            .zip(dangling.iter())
            .filter_map(|(vi, d)| d.then_some(*vi))
            .sum();

        let mut v_new = alpha * (m.transpose() * &v);
        for i in 0..n {
            v_new[i] += alpha * dangling_sum / n as f64;
        }
        v_new += &teleport;

        if (&v_new - &v).norm() < tol {
            return v_new;
        }
        v = v_new;
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn laplacian_y_autovalores_camino() {
        // Camino 0-1-2: aristas [(0,1), (1,2)].
        // L = [[1, -1, 0], [-1, 2, -1], [0, -1, 1]].
        let l = laplacian(3, &[(0, 1), (1, 2)]);
        assert_eq!(l[(0, 0)], 1.0);
        assert_eq!(l[(1, 1)], 2.0);
        assert_eq!(l[(2, 2)], 1.0);
        assert_eq!(l[(0, 1)], -1.0);
        assert_eq!(l[(1, 0)], -1.0);
        assert_eq!(l[(1, 2)], -1.0);
        assert_eq!(l[(2, 1)], -1.0);
        assert_eq!(l[(0, 2)], 0.0);

        let evs = sorted_eigenvalues(&l);
        // λ_1 = 0 (autovector constante), λ_2 > 0 (conectividad algebraica).
        assert!(evs[0].abs() < 1e-9, "λ_1 debe ser 0, es {}", evs[0]);
        assert!(evs[1] > 0.0, "λ_2 debe ser positivo (grafo conexo)");
    }

    #[test]
    fn laplacian_puente_lambda2_pequena() {
        // Dos triángulos conectados por un puente: aristas (0,1),(1,2),(0,2),(2,3),(3,4),(4,5),(3,5).
        // λ_2 debe ser pequeño porque el puente crea un cuello de botella.
        let edges = vec![(0, 1), (1, 2), (0, 2), (2, 3), (3, 4), (4, 5), (3, 5)];
        let l = laplacian(6, &edges);
        let evs = sorted_eigenvalues(&l);
        assert!(evs[0].abs() < 1e-9);
        assert!(
            evs[1] < 1.0,
            "λ_2 con puente debe ser pequeño, es {}",
            evs[1]
        );
    }

    #[test]
    fn pagerank_suma_uno() {
        // 3 nodos en un ciclo dirigido: 0 -> 1, 1 -> 2, 2 -> 0.
        // Matriz estocástica por COLUMNAS M[i][j] = P(j -> i):
        //   col 0 (from 0): 0.5 a 1, 0.5 a 2 (0 -> 1, 0 -> 2 → no, 0 -> 1)
        // Para que cada columna sume 1 con exactamente 1 emisor por columna,
        // redefinimos: 0 → 1, 1 → 2, 2 → 0. Cada uno tiene 1 emisor.
        // Entonces M[:,0] = (0,1,0)^T, M[:,1] = (0,0,1)^T, M[:,2] = (1,0,0)^T.
        // Sin nodos dangling.
        let mut m = DMatrix::<f64>::zeros(3, 3);
        // j=0 -> i=1
        m[(1, 0)] = 1.0;
        // j=1 -> i=2
        m[(2, 1)] = 1.0;
        // j=2 -> i=0
        m[(0, 2)] = 1.0;
        let dangling = vec![false, false, false];
        let pr = pagerank(&m, 0.85, &dangling, 1e-6, 200);
        let s: f64 = pr.iter().sum();
        assert!(
            (s - 1.0).abs() < 1e-4,
            "PageRank debe sumar 1, suma = {}",
            s
        );
    }
}
