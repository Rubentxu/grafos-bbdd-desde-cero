use crate::cap07_modelo::{Edge, EdgeId, Node, NodeId};

// ─────────────────── Cap 8: trait GraphStore + MemoryStore ───────────────────

/// API principal de un grafo de propiedades (en memoria o en disco).
///
/// El diseño hexagonal (ports & adapters): cualquier backend (memoria,
/// disco, red) implementa este trait. La aplicación que usa el grafo
/// permanece agnóstica al backend.
pub trait GraphStore {
    /// Inserta o reemplaza un nodo.
    fn put_node(&mut self, node: Node) -> Result<(), StoreError>;

    /// Inserta o reemplaza una arista.
    fn put_edge(&mut self, edge: Edge) -> Result<(), StoreError>;

    /// Recupera un nodo por ID.
    fn get_node(&self, id: NodeId) -> Option<&Node>;

    /// Recupera una arista por ID.
    fn get_edge(&self, id: EdgeId) -> Option<&Edge>;

    /// IDs de aristas salientes del nodo `u`.
    fn out_edges(&self, u: NodeId) -> Vec<EdgeId>;

    /// IDs de aristas entrantes al nodo `u`.
    fn in_edges(&self, u: NodeId) -> Vec<EdgeId>;

    /// Número total de nodos.
    fn node_count(&self) -> usize;

    /// Número total de aristas.
    fn edge_count(&self) -> usize;

    /// Elimina un nodo (y todas sus aristas). Devuelve true si existía.
    fn delete_node(&mut self, id: NodeId) -> bool;

    /// Elimina una arista.
    fn delete_edge(&mut self, id: EdgeId) -> bool;

    /// Itera sobre todos los nodos (orden no garantizado).
    fn iter_nodes(&self) -> Box<dyn Iterator<Item = &Node> + '_>;

    /// Itera sobre todas las aristas (orden no garantizado).
    fn iter_edges(&self) -> Box<dyn Iterator<Item = &Edge> + '_>;
}

/// Errores del store.
#[derive(Debug, Clone, PartialEq)]
pub enum StoreError {
    DuplicateNode(NodeId),
    DuplicateEdge(EdgeId),
    UnknownNode(NodeId),
    UnknownEdge(EdgeId),
    InvalidEdgeEndpoints { source: NodeId, target: NodeId },
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::DuplicateNode(id) => write!(f, "duplicate node id {id}"),
            StoreError::DuplicateEdge(id) => write!(f, "duplicate edge id {id}"),
            StoreError::UnknownNode(id) => write!(f, "unknown node id {id}"),
            StoreError::UnknownEdge(id) => write!(f, "unknown edge id {id}"),
            StoreError::InvalidEdgeEndpoints { source, target } => {
                write!(f, "edge endpoints {source} -> {target} not both present")
            }
        }
    }
}

impl std::error::Error for StoreError {}

/// Implementación del trait en memoria.
#[derive(Debug, Clone, Default)]
pub struct MemoryStore {
    pub nodes: Vec<Option<Node>>,
    pub edges: Vec<Option<Edge>>,
    pub adj_out: Vec<Vec<EdgeId>>,
    pub adj_in: Vec<Vec<EdgeId>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn ensure_node_capacity(&mut self, id: NodeId) {
        while self.nodes.len() <= id {
            self.nodes.push(None);
            self.adj_out.push(Vec::new());
            self.adj_in.push(Vec::new());
        }
    }

    fn ensure_edge_capacity(&mut self, id: EdgeId) {
        while self.edges.len() <= id {
            self.edges.push(None);
        }
    }

    fn node_exists(&self, id: NodeId) -> bool {
        id < self.nodes.len() && self.nodes[id].is_some()
    }
}

impl GraphStore for MemoryStore {
    fn put_node(&mut self, node: Node) -> Result<(), StoreError> {
        let id = node.id;
        if self.node_exists(id) {
            return Err(StoreError::DuplicateNode(id));
        }
        self.ensure_node_capacity(id);
        self.nodes[id] = Some(node);
        Ok(())
    }

    fn put_edge(&mut self, edge: Edge) -> Result<(), StoreError> {
        let id = edge.id;
        if id < self.edges.len() && self.edges[id].is_some() {
            return Err(StoreError::DuplicateEdge(id));
        }
        if !self.node_exists(edge.source) || !self.node_exists(edge.target) {
            return Err(StoreError::InvalidEdgeEndpoints {
                source: edge.source,
                target: edge.target,
            });
        }
        let max_id = edge.source.max(edge.target);
        self.ensure_node_capacity(max_id);
        self.ensure_edge_capacity(id);
        self.adj_out[edge.source].push(id);
        self.adj_in[edge.target].push(id);
        self.edges[id] = Some(edge);
        Ok(())
    }

    fn get_node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id).and_then(|n| n.as_ref())
    }

    fn get_edge(&self, id: EdgeId) -> Option<&Edge> {
        self.edges.get(id).and_then(|e| e.as_ref())
    }

    fn out_edges(&self, u: NodeId) -> Vec<EdgeId> {
        self.adj_out.get(u).cloned().unwrap_or_default()
    }

    fn in_edges(&self, u: NodeId) -> Vec<EdgeId> {
        self.adj_in.get(u).cloned().unwrap_or_default()
    }

    fn node_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.is_some()).count()
    }

    fn edge_count(&self) -> usize {
        self.edges.iter().filter(|e| e.is_some()).count()
    }

    fn delete_node(&mut self, id: NodeId) -> bool {
        if !self.node_exists(id) {
            return false;
        }
        let edges_to_remove: Vec<EdgeId> = self
            .adj_out
            .get(id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .chain(self.adj_in.get(id).cloned().unwrap_or_default())
            .collect();
        for eid in edges_to_remove {
            self.delete_edge(eid);
        }
        if id < self.nodes.len() {
            self.nodes[id] = None;
            if id < self.adj_out.len() {
                self.adj_out[id].clear();
                self.adj_in[id].clear();
            }
        }
        true
    }

    fn delete_edge(&mut self, id: EdgeId) -> bool {
        if let Some(Some(edge)) = self.edges.get(id) {
            let src = edge.source;
            let tgt = edge.target;
            if id < self.edges.len() {
                self.edges[id] = None;
            }
            if src < self.adj_out.len() {
                self.adj_out[src].retain(|&e| e != id);
            }
            if tgt < self.adj_in.len() {
                self.adj_in[tgt].retain(|&e| e != id);
            }
            true
        } else {
            false
        }
    }

    fn iter_nodes(&self) -> Box<dyn Iterator<Item = &Node> + '_> {
        Box::new(self.nodes.iter().filter_map(|n| n.as_ref()))
    }

    fn iter_edges(&self) -> Box<dyn Iterator<Item = &Edge> + '_> {
        Box::new(self.edges.iter().filter_map(|e| e.as_ref()))
    }
}

#[cfg(test)]
mod tests_store {
    use super::*;

    #[test]
    fn memory_store_basico() {
        let mut s = MemoryStore::new();
        s.put_node(Node::new(0, "Person")).unwrap();
        s.put_node(Node::new(1, "Person")).unwrap();
        s.put_edge(Edge::new(0, 0, 1, "KNOWS")).unwrap();
        assert_eq!(s.node_count(), 2);
        assert_eq!(s.edge_count(), 1);
        assert_eq!(s.out_edges(0), vec![0]);
        assert_eq!(s.in_edges(1), vec![0]);
    }

    #[test]
    fn rechaza_duplicado() {
        let mut s = MemoryStore::new();
        s.put_node(Node::new(0, "A")).unwrap();
        assert_eq!(
            s.put_node(Node::new(0, "B")),
            Err(StoreError::DuplicateNode(0))
        );
    }

    #[test]
    fn delete_node_elimina_aristas() {
        let mut s = MemoryStore::new();
        s.put_node(Node::new(0, "A")).unwrap();
        s.put_node(Node::new(1, "B")).unwrap();
        s.put_edge(Edge::new(0, 0, 1, "X")).unwrap();
        assert!(s.delete_node(0));
        assert_eq!(s.node_count(), 1);
        assert_eq!(s.edge_count(), 0);
    }
}
