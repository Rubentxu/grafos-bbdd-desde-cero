//! Vol.II — Cap.7: Modelo de datos de LiraDB.
//!
//! Migrado desde el brief original LiraDB (§"Property Graph + Value").
//!
//! Define los tipos fundamentales de un Property Graph:
//! - [`Value`] — Tipos de datos primitivos (null, bool, int, float, string, bytes).
//! - [`NodeId`] / [`EdgeId`] — Identificadores únicos estables (ver cap 3 para slotmap).
//! - [`Node`] — Vértice con labels y propiedades.
//! - [`Edge`] — Arista dirigida con label, source/target y propiedades.
//! - [`Element`] — Enum para tratar nodos y aristas uniformemente.
//! - [`PropertyGraph`] — Grafo en memoria con listas de adyacencia.

use std::collections::HashMap;

// ─────────────────── Identificadores ───────────────────
//
// NOTA: en el cap 3 (Vol.II) se sustituirán por IDs generacionales (slotmap).
// Aquí usamos `usize` por simplicidad pedagógica.

pub type NodeId = usize;
pub type EdgeId = usize;

// ─────────────────── Tipos de valor ───────────────────

/// Tipos de datos primitivos soportados por las propiedades.
///
/// Diseñado para ser extensible: añadir una variante no rompe la serialización
/// si va acompañada de un bump de versión del formato (ver cap 9).
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Ausencia de valor (NULL).
    Null,
    /// Booleano.
    Bool(bool),
    /// Entero de 64 bits.
    Int(i64),
    /// Float de 64 bits (IEEE 754).
    Float(f64),
    /// Cadena UTF-8.
    String(String),
    /// Bytes opacos (binarios).
    Bytes(Vec<u8>),
}

impl Value {
    /// Tipo del valor como string legible (para depuración).
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "Null",
            Value::Bool(_) => "Bool",
            Value::Int(_) => "Int",
            Value::Float(_) => "Float",
            Value::String(_) => "String",
            Value::Bytes(_) => "Bytes",
        }
    }

    /// ¿Es el valor `Null`?
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
}

// ─────────────────── Elementos del grafo ───────────────────

/// Vértice de un Property Graph.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub id: NodeId,
    /// Etiquetas del vértice (e.g. `["Person"]`). Múltiples labels son válidas.
    pub labels: Vec<String>,
    /// Propiedades clave → valor.
    pub props: HashMap<String, Value>,
}

impl Node {
    pub fn new(id: NodeId, label: impl Into<String>) -> Self {
        Self {
            id,
            labels: vec![label.into()],
            props: HashMap::new(),
        }
    }

    pub fn with_prop(mut self, key: impl Into<String>, value: Value) -> Self {
        self.props.insert(key.into(), value);
        self
    }

    /// ¿Tiene esta etiqueta?
    pub fn has_label(&self, label: &str) -> bool {
        self.labels.iter().any(|l| l == label)
    }
}

/// Arista dirigida de un Property Graph.
#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    pub id: EdgeId,
    pub source: NodeId,
    pub target: NodeId,
    /// Tipo de relación (e.g. `"KNOWS"`).
    pub label: String,
    pub props: HashMap<String, Value>,
}

impl Edge {
    pub fn new(id: EdgeId, source: NodeId, target: NodeId, label: impl Into<String>) -> Self {
        Self {
            id,
            source,
            target,
            label: label.into(),
            props: HashMap::new(),
        }
    }

    pub fn with_prop(mut self, key: impl Into<String>, value: Value) -> Self {
        self.props.insert(key.into(), value);
        self
    }
}

/// Enum para tratar nodos y aristas uniformemente.
#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    Node(Node),
    Edge(Edge),
}

impl Element {
    pub fn id(&self) -> usize {
        match self {
            Element::Node(n) => n.id,
            Element::Edge(e) => e.id,
        }
    }
}

// ─────────────────── Grafo en memoria ───────────────────

/// Grafo de propiedades (Property Graph) en memoria.
///
/// Estructura simplificada para los caps iniciales del Vol.II: arrays de
/// nodos y aristas, con índices de adyacencia. En el cap 14 (Vol.II) se
/// migrará a almacenamiento en disco con páginas y buffer pool.
#[derive(Debug, Clone, Default)]
pub struct PropertyGraph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    /// Lista de adyacencia saliente: adj_out[u] = IDs de aristas que salen de u.
    pub adj_out: Vec<Vec<EdgeId>>,
    /// Lista de adyacencia entrante: adj_in[u] = IDs de aristas que llegan a u.
    pub adj_in: Vec<Vec<EdgeId>>,
}

impl PropertyGraph {
    /// Crea un grafo vacío.
    pub fn new() -> Self {
        Self::default()
    }

    /// Añade un nodo. Si el nodo tiene el mismo ID que uno existente, lo
    /// rechaza y devuelve `false`.
    pub fn add_node(&mut self, node: Node) -> bool {
        // Pre-computar todo lo que se necesita del self ANTES de cualquier
        // mutación, para evitar problemas de borrow checker.
        let id = node.id;
        let duplicado = id < self.nodes.len() && self.nodes[id].id == id;
        if duplicado {
            return false;
        }
        // Extender listas de adyacencia si es necesario.
        while self.adj_out.len() <= id {
            self.adj_out.push(Vec::new());
            self.adj_in.push(Vec::new());
        }
        // Extender nodos si es necesario.
        if id >= self.nodes.len() {
            self.nodes
                .resize(id + 1, Node::new(usize::MAX, "_placeholder"));
        }
        self.nodes[id] = node;
        true
    }

    /// Añade una arista. Actualiza los índices de adyacencia.
    pub fn add_edge(&mut self, edge: Edge) -> bool {
        // Verificar que source y target existen.
        if edge.source >= self.nodes.len() || self.nodes[edge.source].id != edge.source {
            return false;
        }
        if edge.target >= self.nodes.len() || self.nodes[edge.target].id != edge.target {
            return false;
        }
        let id = edge.id;
        self.adj_out[edge.source].push(id);
        self.adj_in[edge.target].push(id);
        self.edges.push(edge);
        true
    }

    /// Vecinos salientes del nodo `u` (todos, sin filtrar por label).
    pub fn neighbors_out(&self, u: NodeId) -> &[EdgeId] {
        if u < self.adj_out.len() {
            &self.adj_out[u]
        } else {
            &[]
        }
    }

    /// Vecinos entrantes del nodo `u`.
    pub fn neighbors_in(&self, u: NodeId) -> &[EdgeId] {
        if u < self.adj_in.len() {
            &self.adj_in[u]
        } else {
            &[]
        }
    }

    /// Número de nodos.
    pub fn num_nodes(&self) -> usize {
        self.nodes.iter().filter(|n| n.id != usize::MAX).count()
    }

    /// Número de aristas.
    pub fn num_edges(&self) -> usize {
        self.edges.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_type_names() {
        assert_eq!(Value::Null.type_name(), "Null");
        assert_eq!(Value::Int(42).type_name(), "Int");
        assert_eq!(Value::String("hi".into()).type_name(), "String");
        assert!(Value::Null.is_null());
        assert!(!Value::Int(0).is_null());
    }

    #[test]
    fn node_with_props() {
        let n = Node::new(0, "Person")
            .with_prop("name", Value::String("Ada".into()))
            .with_prop("age", Value::Int(36));
        assert_eq!(n.id, 0);
        assert!(n.has_label("Person"));
        assert!(!n.has_label("Place"));
        assert_eq!(n.props.get("age"), Some(&Value::Int(36)));
    }

    #[test]
    fn edge_basic() {
        let e = Edge::new(0, 0, 1, "KNOWS").with_prop("since", Value::Int(2020));
        assert_eq!(e.id, 0);
        assert_eq!(e.source, 0);
        assert_eq!(e.target, 1);
        assert_eq!(e.label, "KNOWS");
    }

    #[test]
    fn graph_add_and_neighbors() {
        let mut g = PropertyGraph::new();
        g.add_node(Node::new(0, "Person").with_prop("name", Value::String("Ada".into())));
        g.add_node(Node::new(1, "Person").with_prop("name", Value::String("Bo".into())));
        g.add_edge(Edge::new(0, 0, 1, "KNOWS"));
        g.add_edge(Edge::new(1, 1, 0, "KNOWS"));

        assert_eq!(g.num_nodes(), 2);
        assert_eq!(g.num_edges(), 2);
        assert_eq!(g.neighbors_out(0).len(), 1);
        assert_eq!(g.neighbors_out(1).len(), 1);
        assert_eq!(g.neighbors_in(0).len(), 1);
        assert_eq!(g.neighbors_in(1).len(), 1);
    }

    #[test]
    fn graph_add_rechaza_duplicado() {
        let mut g = PropertyGraph::new();
        assert!(g.add_node(Node::new(0, "A")));
        assert!(!g.add_node(Node::new(0, "B"))); // ID duplicado
        assert_eq!(g.num_nodes(), 1);
    }

    #[test]
    fn element_enum_id() {
        let n = Node::new(5, "X");
        let e = Edge::new(7, 0, 1, "REL");
        assert_eq!(Element::Node(n.clone()).id(), 5);
        assert_eq!(Element::Edge(e.clone()).id(), 7);
    }
}
