//! Vol.I — Capítulo 2: Representaciones de grafos (versión manual, sin petgraph).
//!
//! Migrado desde `vol1-grafos-de-cero-a-experto-rust.md` §2.5.
//!
//! Grafo no dirigido implementado con lista de adyacencia (`HashMap<u32, Vec<u32>>`).
//! Es la versión "a mano" que se compara con `petgraph` en §2.6.

use std::collections::HashMap;

/// Grafo no dirigido implementado con lista de adyacencia (HashMap).
#[derive(Debug, Clone)]
pub struct MiGrafo {
    /// Clave: vértice. Valor: lista de vecinos.
    adj: HashMap<u32, Vec<u32>>,
}

impl MiGrafo {
    /// Crea un grafo vacío.
    pub fn nuevo() -> Self {
        Self {
            adj: HashMap::new(),
        }
    }

    /// Añade un vértice si no existía.
    pub fn agrega_vertice(&mut self, v: u32) {
        self.adj.entry(v).or_default();
    }

    /// Añade una arista no dirigida entre `u` y `v`.
    pub fn agrega_arista(&mut self, u: u32, v: u32) {
        // Aseguramos que ambos vértices existen
        self.agrega_vertice(u);
        self.agrega_vertice(v);
        // No añadimos duplicados
        if !self.adj[&u].contains(&v) {
            self.adj.get_mut(&u).unwrap().push(v);
        }
        if !self.adj[&v].contains(&u) {
            self.adj.get_mut(&v).unwrap().push(u);
        }
    }

    /// Devuelve los vecinos de `v` (¡orden no garantizado!).
    pub fn vecinos(&self, v: u32) -> &[u32] {
        self.adj.get(&v).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Número de vértices.
    pub fn n(&self) -> usize {
        self.adj.len()
    }

    /// Número de aristas (en no dirigido, cada arista cuenta 1).
    pub fn m(&self) -> usize {
        self.adj.values().map(|v| v.len()).sum::<usize>() / 2
    }
}

impl Default for MiGrafo {
    fn default() -> Self {
        Self::nuevo()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grafo_vacio() {
        let g = MiGrafo::nuevo();
        assert_eq!(g.n(), 0);
        assert_eq!(g.m(), 0);
    }

    #[test]
    fn agrega_aristas_basicas() {
        let mut g = MiGrafo::nuevo();
        g.agrega_arista(1, 2);
        g.agrega_arista(2, 3);
        g.agrega_arista(1, 3);
        assert_eq!(g.n(), 3);
        assert_eq!(g.m(), 3);
        assert_eq!(g.vecinos(1), &[2, 3]);
        assert_eq!(g.vecinos(2), &[1, 3]);
    }

    #[test]
    fn no_duplicar_aristas() {
        let mut g = MiGrafo::nuevo();
        g.agrega_arista(1, 2);
        g.agrega_arista(2, 1); // misma arista
        assert_eq!(g.m(), 1);
    }

    #[test]
    fn vertice_aislado() {
        let mut g = MiGrafo::nuevo();
        g.agrega_vertice(42);
        assert_eq!(g.n(), 1);
        assert_eq!(g.m(), 0);
        assert_eq!(g.vecinos(42), &[] as &[u32]);
    }
}
