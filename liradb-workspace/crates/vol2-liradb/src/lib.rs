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
//! - [`GraphStore`] / [`MemoryStore`] — Trait hexagonal (cap 8) + impl en memoria.
//! - [`Pager`] / [`FilePager`] — Gestor de páginas en disco (cap 12): asignación,
//!   lectura, escritura y sincronización de páginas de tamaño fijo.
//! - [`BufferPool`] + [`PolicyKind`] — Caché de páginas en memoria (cap 13):
//!   pinning, dirty tracking, política de reemplazo Clock (por defecto) o LRU,
//!   y métricas (hits/misses/reads/writes/evictions).
//! - [`Csr`] / [`PersistentCsr`] — Representación CSR (Compressed Sparse Row)
//!   persistente (cap 14): forward + backward indexes, `replace()` y `load()`
//!   sobre `BufferPool<Pager>`, con `CsrHeader` (24 bytes) en la página 1.
//! - [`HashIndex`] / [`BPlusTree`] — Índices secundarios (cap 15): hash estático
//!   con FNV-1a + overflow encadenado, y B+ tree de un solo nivel con
//!   búsqueda binaria y `range_scan`. Ambos viven sobre `BufferPool<Pager>`,
//!   con catálogos persistidos (magic + counts) y errores tipados
//!   (`IndexError`). Sin crates externas — implementados a mano conforme a
//!   la regla "primero a mano, luego con crates" del Vol.II.
//! - [`inspect`] / [`check`] / [`compact`] / [`repack_page`] — Mantenimiento
//!   (cap 16): estadísticas de almacenamiento ([`StorageStats`]), verificación
//!   de integridad ([`CheckReport`] con [`IssueKind`]: bad magic, page_id
//!   mismatch, free_space desactualizado, records truncados) y compactación
//!   in-place ([`CompactReport`]). Errores tipados [`MaintenanceError`].
//!   Cierra la Parte III (motor de almacenamiento) del Vol.II.
//! - [`Span`] / [`TokenKind`] / [`Expression`] / [`PathPattern`] /
//!   [`MatchClause`] / [`WhereClause`] / [`ReturnClause`] / [`Query`] —
//!   Diseño del lenguaje de consulta **LiraQL** (cap 17, abre la Parte IV):
//!   tokens, gramática (EBNF), AST, expresiones, patrones de camino
//!   (MATCH-WHERE-RETURN mini), validación semántica y pretty-printer.
//!   Errores tipados [`QueryError`] con posición ([`Span`]). El lexer y el
//!   parser llegan en el cap.18.
//! - [`Lexer`] / [`Parser`] / [`parse`] — Texto → tokens → AST (cap 18):
//!   escáner manual con maximal-munch y parser descendente recursivo con
//!   precedencia por cadena de funciones. Errores tipados [`LexError`] y
//!   [`ParseError`] con `Span` exacto.
//! - [`LogicalPlan`] / [`ScalarExpr`] / [`Bindings`] / [`LogicalType`] /
//!   [`lower`] — Del AST al **plan lógico** (cap 19): el binder liga las
//!   variables del MATCH ([`Bindings`]), resuelve las expresiones de
//!   WHERE/RETURN a [`ScalarExpr`] (sin spans, ya ligadas) y construye el
//!   árbol de operadores `NodeScan` / `Expand` / `Filter` / `Project` /
//!   `CartesianProduct` con pretty-printer (base de `liradb explain`, cap 21)
//!   e inferencia de tipos básica ([`LogicalType`]). Errores tipados
//!   [`PlanError`].
//! - [`PhysicalOperator`] / [`Row`] / [`Cell`] / [`Executor`] / [`run`] — El
//!   **motor de ejecución Volcano** (cap 20, cierra el hito "ejecutar
//!   consultas completas desde texto"): cada operador del plan es un iterador
//!   pull-based (`open`/`next`/`close`) que produce filas ([`Row`]) de
//!   variables ligadas a celdas ([`Cell`]: escalar, nodo o arista).
//!   Operadores `NodeScanOp` / `IndexSeekOp` / `ExpandOp` (direcciones
//!   out/in/undirected) / `FilterOp` (evaluación trivalente con NULL y
//!   cortocircuito) / `ProjectOp` / `CartesianProductOp` (materializa su lado
//!   derecho: Volcano no rebobina) / `LimitOp` / `DistinctOp`, sobre el trait
//!   `GraphStore` del cap 8. [`Executor`] compila el plan y expone métricas
//!   por operador ([`ExecMetrics`], semilla del explain del cap 21);
//!   [`ResultSet`] devuelve columnas + filas; [`run`] y [`Query::execute`]
//!   completan el pipeline parse → lower → execute. Errores tipados
//!   [`ExecError`].
//!
//! Organización del crate: cada capítulo vive en su propio módulo de origen —
//! `cap07_modelo`, `cap08_graph_store`, `cap09_encoding`, `cap10_append_only`,
//! `cap11_slotted_pages`, `cap12_pager`, `cap13_buffer_pool`, `cap14_csr`,
//! `cap15_indices`, `cap16_mantenimiento`, `cap17_liraql_ast`,
//! `cap18_lexer_parser`, `cap19_plan_logico` y `cap20_volcano` — y este
//! `lib.rs` es sólo el punto de entrada: declara los módulos y los re-exporta
//! con `pub use capNN::*` para mantener una API pública plana
//! (`vol2_liradb::Node`, `vol2_liradb::run`, ...). Cada módulo viaja con sus
//! tests (`mod tests_*`).

mod cap07_modelo;
mod cap08_graph_store;
mod cap09_encoding;
mod cap10_append_only;
mod cap11_slotted_pages;
mod cap12_pager;
mod cap13_buffer_pool;
mod cap14_csr;
mod cap15_indices;
mod cap16_mantenimiento;
mod cap17_liraql_ast;
mod cap18_lexer_parser;
mod cap19_plan_logico;
mod cap20_volcano;

pub use cap07_modelo::*;
pub use cap08_graph_store::*;
pub use cap09_encoding::*;
pub use cap10_append_only::*;
pub use cap11_slotted_pages::*;
pub use cap12_pager::*;
pub use cap13_buffer_pool::*;
pub use cap14_csr::*;
pub use cap15_indices::*;
pub use cap16_mantenimiento::*;
pub use cap17_liraql_ast::*;
pub use cap18_lexer_parser::*;
pub use cap19_plan_logico::*;
pub use cap20_volcano::*;
