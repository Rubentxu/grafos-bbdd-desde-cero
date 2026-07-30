//! Vol.I — Capítulo 5: Minimum Spanning Tree (MST).
//!
//! Migrado desde `vol1-grafos-de-cero-a-experto-rust.md` §5.7.
//!
//! Implementa tres piezas:
//! - [`UnionFind`] — DSU con path compression + union by rank.
//! - [`mst_kruskal`] — algoritmo greedy sobre aristas ordenadas.
//! - [`mst_prim`] — algoritmo "lazy" con `BinaryHeap<Reverse<...>>`.
//!
//! Todas son implementaciones "a mano" (sin `petgraph`); el §5.8 del libro
//! muestra la versión con la crate industrial.

// ─────────────────────────── Union-Find ───────────────────────────

/// Estructura para mantener conjuntos disjuntos (DSU).
#[derive(Debug)]
pub struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<u8>,
    n_components: usize,
}

impl UnionFind {
    /// Crea un DSU con `n` elementos, cada uno en su propio conjunto.
    pub fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            rank: vec![0; n],
            n_components: n,
        }
    }

    /// Devuelve el representante (raíz) del conjunto de `x`.
    /// Aplica *path compression* en dos pasos para aplanar el camino.
    pub fn find(&mut self, x: usize) -> usize {
        // Primer pase: encuentra la raíz.
        let mut root = x;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        // Segundo pase: cuélga cada nodo visitado directamente de la raíz.
        let mut cur = x;
        while self.parent[cur] != root {
            let next = self.parent[cur];
            self.parent[cur] = root;
            cur = next;
        }
        root
    }

    /// Fusiona los conjuntos de `a` y `b`. Devuelve `true` si estaban separados.
    /// Aplica *union by rank* para mantener la altura acotada.
    pub fn union(&mut self, a: usize, b: usize) -> bool {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra == rb {
            return false;
        }
        // El árbol más bajo se cuelga del más alto.
        let (big, small) = if self.rank[ra] < self.rank[rb] {
            (rb, ra)
        } else {
            (ra, rb)
        };
        self.parent[small] = big;
        if self.rank[big] == self.rank[small] {
            self.rank[big] += 1;
        }
        self.n_components -= 1;
        true
    }

    /// ¿Están `a` y `b` en el mismo conjunto?
    pub fn connected(&mut self, a: usize, b: usize) -> bool {
        self.find(a) == self.find(b)
    }

    /// Número de conjuntos disjuntos actuales.
    pub fn components(&self) -> usize {
        self.n_components
    }
}

// ─────────────────────────── Kruskal ───────────────────────────

/// Arista no-dirigida con peso. Implementa `Ord` por peso para ordenar/heap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edge {
    pub u: usize,
    pub v: usize,
    pub w: f64,
}

impl Eq for Edge {}

impl PartialOrd for Edge {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Edge {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // `f64` no implementa `Ord` por la presencia de NaN; en MST los pesos
        // son finitos, así que la comparación por `total_cmp` es segura.
        self.w.total_cmp(&other.w)
    }
}

/// MST de Kruskal. Devuelve (aristas, peso total).
/// Si el grafo no es conexo, devuelve un *Minimum Spanning Forest* (MSF).
pub fn mst_kruskal(n: usize, mut edges: Vec<Edge>) -> (Vec<Edge>, f64) {
    edges.sort(); // por peso, gracias al `Ord` que definimos
    let mut dsu = UnionFind::new(n);
    let mut mst: Vec<Edge> = Vec::with_capacity(n.saturating_sub(1));
    let mut total = 0.0;

    for e in edges {
        if dsu.union(e.u, e.v) {
            total += e.w;
            mst.push(e);
            if mst.len() == n.saturating_sub(1) {
                break;
            }
        }
    }
    (mst, total)
}

// ─────────────────────────── Prim ───────────────────────────

/// Wrapper de `f64` para usarlo en `BinaryHeap` (que requiere `Ord`).
#[derive(Debug, Clone, Copy, PartialEq)]
struct OrderedFloat(f64);

impl Eq for OrderedFloat {}

impl PartialOrd for OrderedFloat {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedFloat {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// MST de Prim "lazy". Recibe lista de adyacencia `adj[u] = [(v, w), ...]`.
///
/// Complejidad: O(E · log V) en la práctica (binary heap).
pub fn mst_prim(n: usize, adj: &[Vec<(usize, f64)>]) -> (Vec<Edge>, f64) {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;

    debug_assert!(!adj.is_empty());
    let mut visited = vec![false; n];
    let mut heap: BinaryHeap<Reverse<(OrderedFloat, usize, usize)>> = BinaryHeap::new();
    let mut mst: Vec<Edge> = Vec::with_capacity(n.saturating_sub(1));
    let mut total = 0.0;

    // Empezamos por el vértice 0 (arbitrario).
    visited[0] = true;
    for &(v, w) in &adj[0] {
        heap.push(Reverse((OrderedFloat(w), 0, v)));
    }

    while let Some(Reverse((OrderedFloat(w), u, v))) = heap.pop() {
        if visited[v] {
            continue; // arista obsoleta
        }
        visited[v] = true;
        mst.push(Edge { u, v, w });
        total += w;
        if mst.len() == n.saturating_sub(1) {
            break;
        }
        for &(x, wx) in &adj[v] {
            if !visited[x] {
                heap.push(Reverse((OrderedFloat(wx), v, x)));
            }
        }
    }
    (mst, total)
}

// ─────────────────────────── Tests ───────────────────────────

#[cfg(test)]
mod tests_union_find {
    use super::*;

    #[test]
    fn union_basico() {
        let mut dsu = UnionFind::new(5);
        assert!(dsu.union(0, 1));
        assert!(dsu.union(2, 3));
        assert!(dsu.union(3, 4));
        assert!(dsu.union(0, 4));
        assert!(dsu.connected(0, 4));
        assert_eq!(dsu.components(), 1);
    }

    #[test]
    fn union_rechaza_repetidos() {
        let mut dsu = UnionFind::new(3);
        assert!(dsu.union(0, 1));
        assert!(!dsu.union(0, 1)); // ya estaban juntos
        assert_eq!(dsu.components(), 2);
    }
}

#[cfg(test)]
mod tests_kruskal {
    use super::*;

    fn grafo_ejemplo() -> Vec<Edge> {
        vec![
            Edge { u: 0, v: 1, w: 1.0 },
            Edge { u: 0, v: 2, w: 4.0 },
            Edge { u: 0, v: 3, w: 3.0 },
            Edge { u: 1, v: 3, w: 2.0 },
            Edge { u: 2, v: 3, w: 5.0 },
        ]
    }

    #[test]
    fn mst_peso_7() {
        let (mst, total) = mst_kruskal(4, grafo_ejemplo());
        assert_eq!(mst.len(), 3);
        assert!((total - 7.0).abs() < 1e-9);
    }

    #[test]
    fn mst_completo_triangulo() {
        // Triángulo equilátero: MST = 2 aristas más baratas.
        let edges = vec![
            Edge { u: 0, v: 1, w: 1.0 },
            Edge { u: 1, v: 2, w: 1.0 },
            Edge { u: 0, v: 2, w: 1.0 },
        ];
        let (mst, total) = mst_kruskal(3, edges);
        assert_eq!(mst.len(), 2);
        assert!((total - 2.0).abs() < 1e-9);
    }
}

#[cfg(test)]
mod tests_prim {
    use super::*;

    #[test]
    fn prim_coincide_con_kruskal() {
        let adj = vec![
            vec![(1, 1.0), (2, 4.0), (3, 3.0)], // 0
            vec![(0, 1.0), (3, 2.0)],           // 1
            vec![(0, 4.0), (3, 5.0)],           // 2
            vec![(0, 3.0), (1, 2.0), (2, 5.0)], // 3
        ];
        let (mst, total) = mst_prim(4, &adj);
        assert_eq!(mst.len(), 3);
        assert!((total - 7.0).abs() < 1e-9);
    }
}
