//! Vol.I — Capítulo 3: BFS y DFS (versiones recursiva e iterativa, sin petgraph).
//!
//! Migrado desde `vol1-grafos-de-cero-a-experto-rust.md` §3.2 y §3.4.
//!
//! - §3.2 BFS iterativo con `VecDeque`
//! - §3.4 DFS recursivo + DFS iterativo con pila explícita
//!
//! Las versiones con `petgraph` se enseñan en §3.5 del libro; este crate se
//! queda con las versiones "a mano" para mostrar el algoritmo sin abstracciones.

use std::collections::{HashSet, VecDeque};

/// Realiza un BFS desde `inicio` en un grafo dado por su lista de adyacencia.
/// Devuelve el orden en que se visitan los vértices.
///
/// # Ejemplo
///
/// ```
/// use vol1_cap_03_bfs_dfs::bfs;
/// let adj = vec![vec![1, 3], vec![0, 2, 4], vec![1], vec![0, 4], vec![1, 3]];
/// let orden = bfs(&adj, 0);
/// assert_eq!(orden, vec![0, 1, 3, 2, 4]);
/// ```
pub fn bfs(adj: &[Vec<u32>], inicio: usize) -> Vec<u32> {
    let mut visitados: HashSet<u32> = HashSet::new();
    let mut cola: VecDeque<u32> = VecDeque::new();
    let mut orden: Vec<u32> = Vec::new();

    cola.push_back(inicio as u32);
    visitados.insert(inicio as u32);

    while let Some(v) = cola.pop_front() {
        orden.push(v);
        for &w in &adj[v as usize] {
            if !visitados.contains(&w) {
                visitados.insert(w);
                cola.push_back(w);
            }
        }
    }
    orden
}

/// DFS recursivo. ¡Cuidado con grafos profundos: desborda la pila!
pub fn dfs_recursivo(adj: &[Vec<u32>], inicio: usize) -> Vec<u32> {
    fn visitar(adj: &[Vec<u32>], v: u32, visitados: &mut HashSet<u32>, orden: &mut Vec<u32>) {
        visitados.insert(v);
        orden.push(v);
        for &w in &adj[v as usize] {
            if !visitados.contains(&w) {
                visitar(adj, w, visitados, orden);
            }
        }
    }
    let mut visitados = HashSet::new();
    let mut orden = Vec::new();
    visitar(adj, inicio as u32, &mut visitados, &mut orden);
    orden
}

/// DFS iterativo, con pila explícita. No desborda (salvo que la pila sea enorme).
pub fn dfs_iterativo(adj: &[Vec<u32>], inicio: usize) -> Vec<u32> {
    let mut visitados: HashSet<u32> = HashSet::new();
    let mut pila: Vec<u32> = vec![inicio as u32];
    let mut orden: Vec<u32> = Vec::new();

    while let Some(v) = pila.pop() {
        if visitados.contains(&v) {
            continue;
        }
        visitados.insert(v);
        orden.push(v);
        // Metemos los vecinos en orden inverso para que el comportamiento
        // sea equivalente a la versión recursiva.
        for &w in adj[v as usize].iter().rev() {
            if !visitados.contains(&w) {
                pila.push(w);
            }
        }
    }
    orden
}

/// Grafo de ejemplo del capítulo 3 (5 vértices, no dirigido).
///
/// ```text
///     0 - 1 - 2
///     |   |
///     3 - 4
/// ```
pub fn grafo_ejemplo() -> Vec<Vec<u32>> {
    vec![
        vec![1, 3],    // 0
        vec![0, 2, 4], // 1
        vec![1],       // 2
        vec![0, 4],    // 3
        vec![1, 3],    // 4
    ]
}

#[cfg(test)]
mod tests_bfs {
    use super::*;

    #[test]
    fn bfs_desde_0() {
        let g = grafo_ejemplo();
        let orden = bfs(&g, 0);
        // Nivel 0: {0}; nivel 1: {1, 3}; nivel 2: {2, 4}.
        assert_eq!(orden, vec![0, 1, 3, 2, 4]);
    }

    #[test]
    fn bfs_visita_todos() {
        let g = grafo_ejemplo();
        let orden = bfs(&g, 0);
        assert_eq!(orden.len(), 5);
    }
}

#[cfg(test)]
mod tests_dfs {
    use super::*;

    #[test]
    fn dfs_recursivo_visita_todos() {
        let g = grafo_ejemplo();
        let orden = dfs_recursivo(&g, 0);
        assert_eq!(orden.len(), 5);
        // Una de las posibles: 0, 1, 2, 4, 3
    }

    #[test]
    fn dfs_iterativo_visita_todos() {
        let g = grafo_ejemplo();
        let orden = dfs_iterativo(&g, 0);
        assert_eq!(orden.len(), 5);
    }

    #[test]
    fn dfs_iterativo_y_recursivo_coinciden_en_longitud() {
        let g = grafo_ejemplo();
        assert_eq!(dfs_recursivo(&g, 0).len(), dfs_iterativo(&g, 0).len());
    }
}
