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

// ─────────────────── Cap 9: encoding binario ───────────────────

pub const FORMAT_VERSION: u32 = 1;

pub fn encode_u32_le(value: u32) -> [u8; 4] {
    value.to_le_bytes()
}
pub fn decode_u32_le(bytes: &[u8; 4]) -> u32 {
    u32::from_le_bytes(*bytes)
}
pub fn encode_i64_le(value: i64) -> [u8; 8] {
    value.to_le_bytes()
}
pub fn decode_i64_le(bytes: &[u8; 8]) -> i64 {
    i64::from_le_bytes(*bytes)
}
pub fn encode_f64_le(value: f64) -> [u8; 8] {
    value.to_le_bytes()
}
pub fn decode_f64_le(bytes: &[u8; 8]) -> f64 {
    f64::from_le_bytes(*bytes)
}

pub fn encode_string(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + s.len());
    out.extend_from_slice(&encode_u32_le(s.len() as u32));
    out.extend_from_slice(s.as_bytes());
    out
}

pub fn decode_string(bytes: &[u8]) -> Result<(String, &[u8]), String> {
    if bytes.len() < 4 {
        return Err("string: too short".into());
    }
    let mut lb = [0u8; 4];
    lb.copy_from_slice(&bytes[..4]);
    let len = decode_u32_le(&lb) as usize;
    if bytes.len() < 4 + len {
        return Err("string: payload truncated".into());
    }
    let s = std::str::from_utf8(&bytes[4..4 + len])
        .map_err(|e| e.to_string())?
        .to_string();
    Ok((s, &bytes[4 + len..]))
}

pub fn encode_value(v: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    match v {
        Value::Null => out.push(0),
        Value::Bool(b) => {
            out.push(1);
            out.push(u8::from(*b));
        }
        Value::Int(i) => {
            out.push(2);
            out.extend_from_slice(&encode_i64_le(*i));
        }
        Value::Float(f) => {
            out.push(3);
            out.extend_from_slice(&encode_f64_le(*f));
        }
        Value::String(s) => {
            out.push(4);
            out.extend_from_slice(&encode_string(s));
        }
        Value::Bytes(b) => {
            out.push(5);
            out.extend_from_slice(&encode_u32_le(b.len() as u32));
            out.extend_from_slice(b);
        }
    }
    out
}

pub fn decode_value(bytes: &[u8]) -> Result<(Value, &[u8]), String> {
    if bytes.is_empty() {
        return Err("value: empty".into());
    }
    let tag = bytes[0];
    let rest = &bytes[1..];
    match tag {
        0 => Ok((Value::Null, rest)),
        1 => {
            if rest.is_empty() {
                return Err("bool: missing".into());
            }
            Ok((Value::Bool(rest[0] != 0), &rest[1..]))
        }
        2 => {
            if rest.len() < 8 {
                return Err("int: need 8 bytes".into());
            }
            let mut b = [0u8; 8];
            b.copy_from_slice(&rest[..8]);
            Ok((Value::Int(decode_i64_le(&b)), &rest[8..]))
        }
        3 => {
            if rest.len() < 8 {
                return Err("float: need 8 bytes".into());
            }
            let mut b = [0u8; 8];
            b.copy_from_slice(&rest[..8]);
            Ok((Value::Float(decode_f64_le(&b)), &rest[8..]))
        }
        4 => {
            let (s, rest) = decode_string(rest)?;
            Ok((Value::String(s), rest))
        }
        5 => {
            if rest.len() < 4 {
                return Err("bytes: need length".into());
            }
            let mut lb = [0u8; 4];
            lb.copy_from_slice(&rest[..4]);
            let len = decode_u32_le(&lb) as usize;
            if rest.len() < 4 + len {
                return Err("bytes: truncated".into());
            }
            let b = rest[4..4 + len].to_vec();
            Ok((Value::Bytes(b), &rest[4 + len..]))
        }
        other => Err(format!("value: tag {other}")),
    }
}

pub fn encode_header() -> [u8; 8] {
    let mut out = [0u8; 8];
    out[..4].copy_from_slice(&encode_u32_le(0x4C_44_42_31));
    out[4..].copy_from_slice(&encode_u32_le(FORMAT_VERSION));
    out
}

pub fn decode_header(bytes: &[u8; 8]) -> Result<u32, String> {
    let magic = decode_u32_le(bytes[..4].try_into().unwrap());
    if magic != 0x4C_44_42_31 {
        return Err(format!("magic mismatch: got {magic:#x}"));
    }
    Ok(decode_u32_le(bytes[4..].try_into().unwrap()))
}

#[cfg(test)]
mod tests_encoding {
    use super::*;

    #[test]
    fn value_roundtrip() {
        for v in [
            Value::Null,
            Value::Bool(true),
            Value::Bool(false),
            Value::Int(42),
            Value::Int(-1234567890),
            Value::Float(std::f64::consts::PI),
            Value::String("hola, mundo!".into()),
            Value::Bytes(vec![1, 2, 3, 4, 5]),
        ] {
            let enc = encode_value(&v);
            let (dec, rest) = decode_value(&enc).unwrap();
            assert_eq!(dec, v);
            assert!(rest.is_empty());
        }
    }

    #[test]
    fn string_roundtrip() {
        let s = "abcdefghij";
        let enc = encode_string(s);
        let (dec, rest) = decode_string(&enc).unwrap();
        assert_eq!(dec, s);
        assert!(rest.is_empty());
    }

    #[test]
    fn header_roundtrip() {
        let h = encode_header();
        let v = decode_header(&h).unwrap();
        assert_eq!(v, FORMAT_VERSION);
    }
}

// ─────────────────── Cap 10: append-only log ───────────────────

/// Tipos de registros del log append-only.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RecordKind {
    /// Nodo nuevo (insert/update).
    PutNode = 1,
    /// Arista nueva (insert/update).
    PutEdge = 2,
    /// Eliminar nodo.
    DeleteNode = 3,
    /// Eliminar arista.
    DeleteEdge = 4,
    /// Commit point (checkpoint para recovery).
    Commit = 5,
}

/// Layout de un registro del log:
///   [record_len: u32 LE] [kind: u8] [id: u32 LE] [payload_len: u32 LE]
///   [payload bytes] [crc32: u32 LE]
///
/// El `record_len` cubre todo lo que sigue hasta (sin incluir) el siguiente
/// `record_len`. Permite al iterador saber dónde termina cada record.
///
/// El CRC32 cubre `kind || id || payload_len || payload`.
#[derive(Debug, Clone, PartialEq)]
pub struct LogRecord {
    pub kind: RecordKind,
    pub id: u32,
    pub payload: Vec<u8>,
}

/// Codifica un registro a bytes (incluyendo CRC32 y length prefix).
pub fn encode_log_record(rec: &LogRecord) -> Vec<u8> {
    let mut body = Vec::new();
    body.push(rec.kind as u8);
    body.extend_from_slice(&encode_u32_le(rec.id));
    body.extend_from_slice(&encode_u32_le(rec.payload.len() as u32));
    body.extend_from_slice(&rec.payload);

    let crc = crc32_simple(&body);
    let mut inner = body;
    inner.extend_from_slice(&encode_u32_le(crc));

    // Length prefix: longitud del "inner" (todo menos el propio length prefix).
    let len_prefix = encode_u32_le(inner.len() as u32);
    let mut out = Vec::with_capacity(4 + inner.len());
    out.extend_from_slice(&len_prefix);
    out.extend(inner);
    out
}

/// Decodifica un registro desde bytes (verifica CRC32). Usa length prefix.
pub fn decode_log_record(bytes: &[u8]) -> Result<(LogRecord, &[u8]), String> {
    // Mínimo: length(4) + kind(1) + id(4) + len(4) + crc(4) = 17.
    if bytes.len() < 17 {
        return Err(format!(
            "record: need at least 17 bytes, have {}",
            bytes.len()
        ));
    }
    let inner_len = decode_u32_le(bytes[..4].try_into().unwrap()) as usize;
    if bytes.len() < 4 + inner_len {
        return Err(format!(
            "record: truncated (need {} bytes, have {})",
            4 + inner_len,
            bytes.len()
        ));
    }
    let inner = &bytes[4..4 + inner_len];
    let body_len = inner.len() - 4;
    let body = &inner[..body_len];
    let crc_read = decode_u32_le(inner[body_len..].try_into().unwrap());
    let crc_calc = crc32_simple(body);
    if crc_read != crc_calc {
        return Err(format!(
            "crc mismatch: stored {crc_read:#x}, computed {crc_calc:#x}"
        ));
    }
    let kind = match body[0] {
        1 => RecordKind::PutNode,
        2 => RecordKind::PutEdge,
        3 => RecordKind::DeleteNode,
        4 => RecordKind::DeleteEdge,
        5 => RecordKind::Commit,
        other => return Err(format!("record: unknown kind {other}")),
    };
    let id = decode_u32_le(body[1..5].try_into().unwrap());
    let payload_len = decode_u32_le(body[5..9].try_into().unwrap()) as usize;
    if body.len() < 9 + payload_len {
        return Err("record: payload truncated".into());
    }
    let payload = body[9..9 + payload_len].to_vec();
    Ok((LogRecord { kind, id, payload }, &bytes[4 + inner_len..]))
}

/// CRC32 simplificado (polinomio IEEE 802.3, sin tabla).
///
/// Para producción usaríamos `crc32fast`. Esta implementación es
/// didáctica: O(n) por byte, sin dependencias.
pub fn crc32_simple(bytes: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = if crc & 1 != 0 { 0xEDB8_8320 } else { 0 };
            crc = (crc >> 1) ^ mask;
        }
    }
    crc ^ 0xFFFF_FFFF
}

/// Log append-only en memoria (para tests y referencia).
/// En producción, el "disco" sería un `File` con `O_APPEND`.
#[derive(Debug, Default)]
pub struct AppendOnlyLog {
    /// Bytes del log (suma de todos los registros encodificados).
    bytes: Vec<u8>,
    /// Contador de registros.
    count: usize,
}

impl AppendOnlyLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Añade un registro al log (append-only).
    pub fn append(&mut self, rec: &LogRecord) -> usize {
        let encoded = encode_log_record(rec);
        let offset = self.bytes.len();
        self.bytes.extend_from_slice(&encoded);
        self.count += 1;
        offset
    }

    /// Tamaño total del log en bytes.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// ¿Está vacío?
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Número de registros.
    pub fn record_count(&self) -> usize {
        self.count
    }

    /// Bytes crudos (para inspección).
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Itera sobre todos los registros decodificados.
    pub fn iter(&self) -> LogIterator<'_> {
        LogIterator {
            bytes: &self.bytes,
            pos: 0,
        }
    }

    /// Trunca el log a partir de un offset (para tests de recovery).
    pub fn truncate_to(&mut self, len: usize) {
        self.bytes.truncate(len);
    }
}

/// Iterador sobre registros de un `AppendOnlyLog`.
pub struct LogIterator<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Iterator for LogIterator<'a> {
    type Item = LogRecord;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.bytes.len() {
            return None;
        }
        match decode_log_record(&self.bytes[self.pos..]) {
            Ok((_rec, rest)) => {
                self.pos = self.bytes.len() - rest.len();
                Some(_rec)
            }
            Err(_) => None,
        }
    }
}

#[cfg(test)]
mod tests_log {
    use super::*;

    #[test]
    fn crc32_known_value_empty() {
        // CRC32 del input vacío es 0.
        assert_eq!(crc32_simple(b""), 0);
    }

    #[test]
    fn crc32_known_value_a() {
        // CRC32("a") = 0xE8B7BE43
        assert_eq!(crc32_simple(b"a"), 0xE8B7_BE43);
    }

    #[test]
    fn log_record_roundtrip() {
        let rec = LogRecord {
            kind: RecordKind::PutNode,
            id: 42,
            payload: vec![1, 2, 3, 4],
        };
        let encoded = encode_log_record(&rec);
        let (decoded, rest) = decode_log_record(&encoded).unwrap();
        assert_eq!(decoded, rec);
        assert!(rest.is_empty());
    }

    #[test]
    fn log_record_corrupto_falla() {
        let rec = LogRecord {
            kind: RecordKind::PutNode,
            id: 42,
            payload: vec![1, 2, 3, 4],
        };
        let mut encoded = encode_log_record(&rec);
        // Corromper el último byte (CRC).
        let last = encoded.len() - 1;
        encoded[last] ^= 0xFF;
        assert!(decode_log_record(&encoded).is_err());
    }

    #[test]
    fn append_only_log_basico() {
        let mut log = AppendOnlyLog::new();
        log.append(&LogRecord {
            kind: RecordKind::PutNode,
            id: 0,
            payload: vec![10],
        });
        log.append(&LogRecord {
            kind: RecordKind::PutEdge,
            id: 0,
            payload: vec![20, 30, 40],
        });
        log.append(&LogRecord {
            kind: RecordKind::Commit,
            id: 0,
            payload: vec![],
        });

        assert_eq!(log.record_count(), 3);
        // Smoke test del iterador: al menos no debe estar vacío.
        let records: Vec<LogRecord> = log.iter().collect();
        assert!(
            !records.is_empty(),
            "iter returned empty; bytes={:?}",
            log.as_bytes()
        );
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].kind, RecordKind::PutNode);
        assert_eq!(records[1].kind, RecordKind::PutEdge);
        assert_eq!(records[2].kind, RecordKind::Commit);
    }

    #[test]
    fn log_recovery_desde_offset() {
        // Simula un crash: truncamos a la mitad y comprobamos que los
        // registros válidos hasta ese punto se pueden leer.
        let mut log = AppendOnlyLog::new();
        for i in 0..10 {
            log.append(&LogRecord {
                kind: RecordKind::PutNode,
                id: i,
                payload: vec![i as u8; 10],
            });
        }
        let mid = log.len() / 2;
        log.truncate_to(mid);
        let records: Vec<LogRecord> = log.iter().collect();
        // Debe leer al menos un registro completo antes del corte.
        assert!(!records.is_empty());
        // Todos los IDs leídos deben ser válidos.
        for r in &records {
            assert_eq!(r.kind, RecordKind::PutNode);
        }
    }
}

// ─────────────────── Cap 11: páginas, bloques y slotted pages ───────────────────

/// Tamaño fijo de página en bytes (4 KB). Constante del formato.
pub const PAGE_SIZE: usize = 4096;

/// Header de página (presente en todas las páginas de datos).
///
/// Layout en bytes (little-endian):
///   [magic: u8] [page_type: u8] [page_id: u32] [num_records: u16] [free_space: u16]
///
/// Total: 10 bytes. El magic es 0xDA para distinguir páginas de datos de
/// la metapágina (0xFE).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageHeader {
    pub page_id: u32,
    pub page_type: PageType,
    pub num_records: u16,
    pub free_space: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PageType {
    Data = 0xDA,
    Meta = 0xFE,
}

impl PageHeader {
    pub const SIZE: usize = 10;

    pub fn new(page_id: u32, page_type: PageType) -> Self {
        Self {
            page_id,
            page_type,
            num_records: 0,
            free_space: (PAGE_SIZE - Self::SIZE) as u16,
        }
    }

    pub fn encode(&self) -> [u8; Self::SIZE] {
        let mut out = [0u8; Self::SIZE];
        out[0] = self.page_type as u8;
        out[1] = self.page_type as u8; // magic redundante para autochequeo
        out[2..6].copy_from_slice(&encode_u32_le(self.page_id));
        out[6..8].copy_from_slice(&self.num_records.to_le_bytes());
        out[8..10].copy_from_slice(&self.free_space.to_le_bytes());
        out
    }

    pub fn decode(bytes: &[u8; Self::SIZE]) -> Result<Self, String> {
        let magic = bytes[0];
        let page_type = match bytes[1] {
            0xDA => PageType::Data,
            0xFE => PageType::Meta,
            other => return Err(format!("page: unknown magic {other:#x}")),
        };
        if magic != page_type as u8 {
            return Err("page: magic mismatch".into());
        }
        let page_id = decode_u32_le(bytes[2..6].try_into().unwrap());
        let num_records = u16::from_le_bytes(bytes[6..8].try_into().unwrap());
        let free_space = u16::from_le_bytes(bytes[8..10].try_into().unwrap());
        Ok(Self {
            page_id,
            page_type,
            num_records,
            free_space,
        })
    }
}

/// Slotted page: header + records con length prefix.
///
/// Layout (PAGE_SIZE bytes):
///   [PageHeader: 10 bytes]
///   [para cada record: u32 LE length | payload | ...]
///   [padding hasta llenar la página]
#[derive(Debug, Clone, PartialEq)]
pub struct SlottedPage {
    pub header: PageHeader,
    /// Records como bytes crudos.
    records: Vec<Vec<u8>>,
}

impl SlottedPage {
    pub fn new(page_id: u32, page_type: PageType) -> Self {
        Self {
            header: PageHeader::new(page_id, page_type),
            records: Vec::new(),
        }
    }

    /// Devuelve el espacio libre disponible (en bytes) para nuevos records.
    pub fn free_space(&self) -> usize {
        PAGE_SIZE - PageHeader::SIZE - self.records.iter().map(|r| r.len()).sum::<usize>()
    }

    /// Añade un record si hay espacio. Devuelve el offset donde se insertó,
    /// o `None` si no cabe.
    pub fn insert(&mut self, record: &[u8]) -> Option<usize> {
        if record.len() > self.free_space() {
            return None;
        }
        let offset = PageHeader::SIZE + self.records.iter().map(|r| r.len()).sum::<usize>();
        self.records.push(record.to_vec());
        self.header.num_records += 1;
        self.header.free_space = self.free_space() as u16;
        Some(offset)
    }

    /// Codifica la página completa a bytes (PAGE_SIZE) con length-prefix.
    pub fn encode(&self) -> [u8; PAGE_SIZE] {
        let mut out = [0u8; PAGE_SIZE];
        out[..PageHeader::SIZE].copy_from_slice(&self.header.encode());
        let mut pos = PageHeader::SIZE;
        for rec in &self.records {
            if pos + 4 + rec.len() > PAGE_SIZE {
                break;
            }
            let len_bytes = encode_u32_le(rec.len() as u32);
            out[pos..pos + 4].copy_from_slice(&len_bytes);
            out[pos + 4..pos + 4 + rec.len()].copy_from_slice(rec);
            pos += 4 + rec.len();
        }
        out
    }

    /// Decodifica con length-prefix.
    pub fn decode(bytes: &[u8; PAGE_SIZE]) -> Result<Self, String> {
        let header = PageHeader::decode(bytes[..PageHeader::SIZE].try_into().unwrap())?;
        let mut records = Vec::new();
        let mut pos = PageHeader::SIZE;
        for _ in 0..header.num_records {
            if pos + 4 > PAGE_SIZE {
                return Err("page: ran out of space (header corruption?)".into());
            }
            let len_bytes: [u8; 4] = bytes[pos..pos + 4].try_into().unwrap();
            let len = decode_u32_le(&len_bytes) as usize;
            pos += 4;
            if pos + len > PAGE_SIZE {
                return Err("page: record truncated".into());
            }
            records.push(bytes[pos..pos + len].to_vec());
            pos += len;
        }
        Ok(Self { header, records })
    }

    /// Devuelve los records almacenados.
    pub fn records(&self) -> &[Vec<u8>] {
        &self.records
    }
}

/// Metapágina (página 0): contiene el catálogo del archivo.
///
/// Layout:
///   [PageHeader]
///   [num_pages: u32] [free_pages: u32] [root_page: u32]
///
/// Total header info: 12 bytes. El resto de la página está libre para
///扩展 futuras (versiones, checksums, etc.).
#[derive(Debug, Clone, PartialEq)]
pub struct MetaPage {
    pub header: PageHeader,
    pub num_pages: u32,
    pub free_pages: u32,
    pub root_page: u32,
}

impl MetaPage {
    pub const INFO_OFFSET: usize = PageHeader::SIZE;
    pub const INFO_SIZE: usize = 12;

    pub fn new() -> Self {
        Self {
            header: PageHeader::new(0, PageType::Meta),
            num_pages: 1, // sólo la metapágina por ahora
            free_pages: 0,
            root_page: 0,
        }
    }

    pub fn encode(&self) -> [u8; PAGE_SIZE] {
        let mut out = [0u8; PAGE_SIZE];
        out[..PageHeader::SIZE].copy_from_slice(&self.header.encode());
        let info = encode_u32_le(self.num_pages);
        let free = encode_u32_le(self.free_pages);
        let root = encode_u32_le(self.root_page);
        out[Self::INFO_OFFSET..Self::INFO_OFFSET + 4].copy_from_slice(&info);
        out[Self::INFO_OFFSET + 4..Self::INFO_OFFSET + 8].copy_from_slice(&free);
        out[Self::INFO_OFFSET + 8..Self::INFO_OFFSET + 12].copy_from_slice(&root);
        out
    }

    pub fn decode(bytes: &[u8; PAGE_SIZE]) -> Result<Self, String> {
        let header = PageHeader::decode(bytes[..PageHeader::SIZE].try_into().unwrap())?;
        if header.page_type != PageType::Meta {
            return Err("meta: page type mismatch".into());
        }
        let num_pages = decode_u32_le(
            bytes[Self::INFO_OFFSET..Self::INFO_OFFSET + 4]
                .try_into()
                .unwrap(),
        );
        let free_pages = decode_u32_le(
            bytes[Self::INFO_OFFSET + 4..Self::INFO_OFFSET + 8]
                .try_into()
                .unwrap(),
        );
        let root_page = decode_u32_le(
            bytes[Self::INFO_OFFSET + 8..Self::INFO_OFFSET + 12]
                .try_into()
                .unwrap(),
        );
        Ok(Self {
            header,
            num_pages,
            free_pages,
            root_page,
        })
    }
}

impl Default for MetaPage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests_page {
    use super::*;

    #[test]
    fn page_header_roundtrip() {
        let h = PageHeader::new(42, PageType::Data);
        let enc = h.encode();
        let dec = PageHeader::decode(&enc).unwrap();
        assert_eq!(h, dec);
    }

    #[test]
    fn page_header_meta() {
        let h = PageHeader::new(0, PageType::Meta);
        let enc = h.encode();
        assert_eq!(enc[1], 0xFE);
        let dec = PageHeader::decode(&enc).unwrap();
        assert_eq!(h, dec);
    }

    #[test]
    fn page_header_magic_mismatch() {
        let mut bytes = [0u8; 10];
        bytes[0] = 0xAA;
        bytes[1] = 0xDA;
        assert!(PageHeader::decode(&bytes).is_err());
    }

    #[test]
    fn slotted_page_vacio() {
        let p = SlottedPage::new(0, PageType::Data);
        let enc = p.encode();
        let dec = SlottedPage::decode(&enc).unwrap();
        assert_eq!(p, dec);
        assert_eq!(dec.records().len(), 0);
        assert_eq!(dec.header.num_records, 0);
    }

    #[test]
    fn slotted_page_con_records() {
        let mut p = SlottedPage::new(7, PageType::Data);
        let r1: &[u8] = b"hello";
        let r2: &[u8] = b"world!";
        let r3: &[u8] = b"LiraDB";
        assert!(p.insert(r1).is_some());
        assert!(p.insert(r2).is_some());
        assert!(p.insert(r3).is_some());
        assert_eq!(p.records().len(), 3);

        let enc = p.encode();
        let dec = SlottedPage::decode(&enc).unwrap();
        assert_eq!(p, dec);
        assert_eq!(dec.records()[0], r1);
        assert_eq!(dec.records()[1], r2);
        assert_eq!(dec.records()[2], r3);
    }

    #[test]
    fn slotted_page_record_no_cabe() {
        let mut p = SlottedPage::new(0, PageType::Data);
        let huge = vec![0u8; PAGE_SIZE - PageHeader::SIZE];
        assert!(p.insert(&huge).is_some());
        assert!(p.insert(b"extra").is_none());
    }

    #[test]
    fn slotted_page_meta() {
        let p = SlottedPage::new(0, PageType::Meta);
        let enc = p.encode();
        assert_eq!(enc[1], 0xFE);
        let dec = SlottedPage::decode(&enc).unwrap();
        assert_eq!(dec.header.page_type, PageType::Meta);
    }

    #[test]
    fn free_space_decrementa() {
        let mut p = SlottedPage::new(0, PageType::Data);
        let initial_free = p.free_space();
        p.insert(b"hello").unwrap();
        assert!(p.free_space() < initial_free);
        assert_eq!(initial_free - p.free_space(), 5);
    }

    #[test]
    fn meta_page_roundtrip() {
        let m = MetaPage {
            header: PageHeader::new(0, PageType::Meta),
            num_pages: 42,
            free_pages: 5,
            root_page: 7,
        };
        let enc = m.encode();
        let dec = MetaPage::decode(&enc).unwrap();
        assert_eq!(m, dec);
    }

    #[test]
    fn meta_page_default() {
        let m = MetaPage::new();
        assert_eq!(m.num_pages, 1);
        assert_eq!(m.free_pages, 0);
        let enc = m.encode();
        let dec = MetaPage::decode(&enc).unwrap();
        assert_eq!(m, dec);
    }
}
