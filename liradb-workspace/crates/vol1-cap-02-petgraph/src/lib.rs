//! Vol.I — Capítulo 2: Representaciones de grafos con `petgraph`.
//!
//! Migrado desde `vol1-grafos-de-cero-a-experto-rust.md` §2.6.
//!
//! Mismo grafo que `vol1-cap-02-vec-adj` (4 vértices, 4 aristas, no dirigido)
//! pero implementado con `petgraph::Graph<(), (), Undirected>`. Compara la API
//! "a mano" con la API industrial.

use petgraph::graph::UnGraph;

/// Construye el grafo de ejemplo (4 vértices, 4 aristas).
///
/// Estructura:
///
/// ```text
///        b
///       / \
///      a   d
///       \ /
///        c
/// ```
pub fn ejemplo_petgraph() -> UnGraph<(), ()> {
    let mut g = UnGraph::<(), ()>::new_undirected();

    let a = g.add_node(());
    let b = g.add_node(());
    let c = g.add_node(());
    let d = g.add_node(());

    g.add_edge(a, b, ());
    g.add_edge(a, c, ());
    g.add_edge(b, d, ());
    g.add_edge(c, d, ());

    g
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::graph::NodeIndex;

    #[test]
    fn cuenta_vertices_y_aristas() {
        let g = ejemplo_petgraph();
        assert_eq!(g.node_count(), 4);
        assert_eq!(g.edge_count(), 4);
    }

    #[test]
    fn vecinos_del_primer_vertice() {
        let g = ejemplo_petgraph();
        // El primer vértice añadido (`a`) tiene índice 0.
        let a = NodeIndex::new(0);
        let vecinos: Vec<_> = g.neighbors(a).collect();
        assert_eq!(vecinos.len(), 2);
    }

    #[test]
    fn vecinos_del_segundo_vertice() {
        let g = ejemplo_petgraph();
        // El segundo vértice añadido (`b`) tiene índice 1.
        let b = NodeIndex::new(1);
        let vecinos: Vec<_> = g.neighbors(b).collect();
        assert_eq!(vecinos.len(), 2);
    }
}
