use crate::cap07_modelo::{Edge, EdgeId, Node, NodeId, Value};
use crate::cap08_graph_store::{GraphStore, MemoryStore};
use crate::cap17_liraql_ast::{CompareOp, Query, RelDirection, display_value};
use crate::cap18_lexer_parser::{ParseError, parse};
use crate::cap19_plan_logico::{LogicalPlan, PlanError, Projection, ScalarExpr};
use crate::cap21_optimizador::{Catalog, optimize};

// ─────────────────── Cap 20: El motor de ejecución (modelo Volcano) ───────────────────
//
// Los caps. 17-19 construyeron la cadena `texto → tokens → AST → plan lógico`.
// Este capítulo la cierra: EJECUTAR el plan sobre un grafo real y devolver
// filas. El hito del brief (§cap 20) es "ejecutar consultas completas desde
// texto":
//
// ```text
//   "MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE p.name = \"Ana\" RETURN f.name"
//        │ parse (cap 18) │ lower (cap 19) │ compile + Volcano (este cap)
//        └──► Query ──────┴──► LogicalPlan ┴──► ResultSet (columnas + filas)
// ```
//
// El modelo **Volcano** (Graefe, 1994): cada operador del plan es un ITERADOR
// con la tríada `open()` / `next()` / `close()`. La consulta se evalúa en
// *pull* (el consumidor pide la siguiente fila; nadie calcula nada que no se
// haya pedido), anidando bucles: el `Filter` pide filas a su entrada, que
// pide a su `Expand`, que pide a su `NodeScan`. Así un `Limit(n)` detiene el
// pipeline tras n filas sin trabajo extra — la gracia del modelo frente a
// materializar resultados intermedios.
//
// ```text
//   Project(f.name)          ──► ProjectOp   (evalúa RETURN → celdas de salida)
//     Filter(f:Person …)     ──► FilterOp    (evalúa el predicado, 3VL)
//       Expand(p,KNOWS,→,f)  ──► ExpandOp    (adyacencia por dirección)
//         NodeScan(Person)   ──► NodeScanOp  (itera nodos del store)
// ```
//
// Responsabilidades del capítulo (brief §cap 20):
//   1. **Operadores**: `NodeScanOp`, `IndexSeekOp`, `ExpandOp`, `FilterOp`,
//      `ProjectOp`, `LimitOp`, `DistinctOp` (los siete del brief; el
//      `CartesianProduct` del cap 19 llega como `CartesianProductOp`). La
//      interfaz es el trait `PhysicalOperator` (open/next/close), tal cual
//      lo esboza el brief.
//   2. **Filas**: el operador interno produce `Row`s — variables ligadas a
//      `Cell`s (nodo, arista o escalar). Es la materialización en memoria de
//      los `Bindings` del cap 19: el scanner liga, el expand extiende, el
//      filter descarta y el project convierte a columnas de salida.
//   3. **Evaluación**: `eval_scalar` evalúa una `ScalarExpr` sobre una fila:
//      literales (`Value` del cap 7), `p.name` (propiedad ausente → NULL),
//      `f:Person`, comparaciones con semántica NULL (SQL/Cypher: cualquier
//      comparación con NULL da NULL) y AND/OR/NOT en lógica trivalente con
//      CORTOCIRCUITO (prometido en los caps. 17 y 19).
//   4. **Métricas**: cada operador cuenta sus filas producidas; el
//      `Executor` las expone (`ExecMetrics`) — la semilla del `explain`
//      estimado del cap. 21.
//
// Decisiones (documentadas también en MIGRATION-PATTERN §24):
//   - El executor consume el trait `GraphStore` del cap 8 (puerto hexagonal):
//     funciona con `MemoryStore` hoy y con el store en disco de la Parte III
//     sin cambios. `PropertyGraph` (cap 7) es la estructura didáctica; el
//     store real de las consultas es `MemoryStore`.
//   - `IndexSeekOp` existe y es testeable, pero recibe los IDs del nodo "desde
//     fuera": la SELECCIÓN del índice la hace el optimizador del cap 21
//     (regla `index_seek`, cuyo ejemplo transforma `Filter(name="Ana")+
//     NodeScan` en `IndexSeek`) y los deja resueltos en el propio plan.
//   - `LimitOp`/`DistinctOp` son operadores de pleno derecho, pero la
//     gramática LiraQL (caps. 17-18) aún no expone `LIMIT`/`DISTINCT`: se
//     usan programáticamente (ver tests) hasta que el lenguaje los admita.
//   - El pipeline es de una sola pasada (Volcano es monotónico: no rebobina).
//     Por eso `CartesianProductOp` MATERIALIZA su lado derecho en `open()` —
//     el precio del producto cartesiano, y la motivación del join reordenado
//     del cap. 21.

// ─── Cell y Row: la fila del modelo Volcano ───

/// Valor de una celda: escalar, nodo completo o arista completa.
///
/// Las filas INTERNAS (NodeScan/Expand/…) ligan variables a nodos/aristas;
/// las filas de SALIDA (Project) llevan el resultado de cada expresión del
/// RETURN, que puede ser un escalar (`f.name`) o un elemento entero (`RETURN p`).
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    /// Valor escalar (reutiliza `Value` del cap 7).
    Scalar(Value),
    /// Un nodo completo (variable de nodo ligada).
    Node(Node),
    /// Una arista completa (variable de relación ligada).
    Edge(Edge),
}

impl Cell {
    /// Nombre del tipo para mensajes de error (mayúsculas, como `LogicalType`).
    pub fn type_name(&self) -> &'static str {
        match self {
            Cell::Scalar(v) => match v {
                Value::Null => "NULL",
                Value::Bool(_) => "BOOL",
                Value::Int(_) => "INT",
                Value::Float(_) => "FLOAT",
                Value::String(_) => "STRING",
                Value::Bytes(_) => "BYTES",
            },
            Cell::Node(_) => "NODE",
            Cell::Edge(_) => "EDGE",
        }
    }

    /// Referencia al escalar si la celda es escalar.
    pub fn as_scalar(&self) -> Option<&Value> {
        match self {
            Cell::Scalar(v) => Some(v),
            _ => None,
        }
    }
}

impl std::fmt::Display for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cell::Scalar(v) => display_value(f, v),
            Cell::Node(n) => write!(f, "Node#{}:{}", n.id, n.labels.join(":")),
            Cell::Edge(e) => write!(f, "Edge#{}:{}", e.id, e.label),
        }
    }
}

/// Una fila: variables ligadas a celdas, en orden de ligadura.
///
/// Es la materialización en ejecución de los `Bindings` del cap 19: `NodeScan`
/// crea la fila con su variable, `Expand` la extiende con la relación y el
/// destino, `CartesianProduct` concatena dos filas de patrones disjuntos y
/// `Project` produce una fila nueva con las columnas del RETURN (una entrada
/// por `Projection`, nombrada por su `output_name`).
///
/// `bind` añade (push): el planner garantiza variables únicas dentro de la
/// fila; en la salida, `RETURN p, p` produce dos columnas con el mismo nombre
/// y ambas se conservan.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Row {
    entries: Vec<(String, Cell)>,
}

impl Row {
    /// Fila vacía.
    pub fn new() -> Self {
        Self::default()
    }

    /// Liga (añade) una variable a una celda.
    pub fn bind(&mut self, variable: impl Into<String>, cell: Cell) {
        self.entries.push((variable.into(), cell));
    }

    /// Celda ligada a una variable (la primera si hay nombres repetidos).
    pub fn get(&self, variable: &str) -> Option<&Cell> {
        self.entries
            .iter()
            .find(|(name, _)| name == variable)
            .map(|(_, cell)| cell)
    }

    /// Número de celdas (variables ligadas o columnas de salida).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// ¿Fila vacía?
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterar `(variable, celda)` en orden de ligadura.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Cell)> {
        self.entries.iter().map(|(n, c)| (n.as_str(), c))
    }

    /// Concatena dos filas (patrones disjuntos del CartesianProduct).
    pub fn merge(mut self, other: Row) -> Row {
        self.entries.extend(other.entries);
        self
    }

    /// Consume la fila y devuelve sólo las celdas, en orden.
    pub fn cells(self) -> Vec<Cell> {
        self.entries.into_iter().map(|(_, c)| c).collect()
    }
}

impl std::fmt::Display for Row {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("{")?;
        for (i, (name, cell)) in self.entries.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{name}: {cell}")?;
        }
        f.write_str("}")
    }
}

// ─── ExecError: fallos de ejecución ───

/// Error del motor de ejecución (cap 20).
///
/// A diferencia de `QueryError`/`ParseError`/`PlanError`, aquí no hay `Span`:
/// la ejecución opera sobre el plan ya resuelto. Los fallos de sintaxis y de
/// planificación llegan envueltos (`Parse`/`Plan`) a través de [`run`] y
/// [`Query::execute`], que completan el pipeline entero.
#[derive(Debug, Clone, PartialEq)]
pub enum ExecError {
    /// El texto no parsea (sólo desde [`run`]).
    Parse(ParseError),
    /// El AST no baja a plan (sólo desde [`run`] y [`Query::execute`]).
    Plan(PlanError),
    /// Variable que el plan debería tener ligada y no está (defensivo: el
    /// binder del cap 19 ya rechazó los nombres sin ligar).
    UnboundVariable { variable: String },
    /// `Expand` sobre una variable que no está ligada a un nodo.
    NotANode { variable: String },
    /// `p.name` donde `p` ligó a un escalar (defensivo: imposible desde el
    /// binder, posible con planes construidos a mano).
    PropertyOnScalar { variable: String },
    /// Un índice devolvió un id de nodo inexistente (índice desactualizado).
    UnknownNode(NodeId),
    /// Operando no booleano en un contexto lógico (WHERE, AND/OR/NOT). Es la
    /// versión en ejecución del `TypeMismatch` del cap 19: lo que el plan sólo
    /// podía tipar como `Any` (propiedades schemaless) se concreta aquí.
    TypeMismatch {
        context: &'static str,
        expected: &'static str,
        got: String,
    },
    /// El plan raíz no es `Project`: el `Executor` necesita sus columnas.
    NotAProjection,
}

impl std::fmt::Display for ExecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecError::Parse(e) => write!(f, "error de sintaxis: {e}"),
            ExecError::Plan(e) => write!(f, "error de planificación: {e}"),
            ExecError::UnboundVariable { variable } => {
                write!(f, "variable '{variable}' sin ligar en la fila actual")
            }
            ExecError::NotANode { variable } => {
                write!(f, "la variable '{variable}' no está ligada a un nodo")
            }
            ExecError::PropertyOnScalar { variable } => {
                write!(f, "'{variable}' es un escalar: no tiene propiedades")
            }
            ExecError::UnknownNode(id) => {
                write!(
                    f,
                    "el índice apunta al nodo {id}, que no existe (¿desactualizado?)"
                )
            }
            ExecError::TypeMismatch {
                context,
                expected,
                got,
            } => write!(
                f,
                "tipos incompatibles en {context}: se esperaba {expected}, se obtuvo {got}"
            ),
            ExecError::NotAProjection => {
                f.write_str("el plan raíz no es un Project: no hay columnas de salida")
            }
        }
    }
}

impl std::error::Error for ExecError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecError::Parse(e) => Some(e),
            ExecError::Plan(e) => Some(e),
            _ => None,
        }
    }
}

impl From<ParseError> for ExecError {
    fn from(e: ParseError) -> Self {
        ExecError::Parse(e)
    }
}

impl From<PlanError> for ExecError {
    fn from(e: PlanError) -> Self {
        ExecError::Plan(e)
    }
}

// ─── Evaluación de ScalarExpr sobre una fila ───

/// Igualdad de dos celdas para `=` / `<>`.
///
/// `None` = algún operando es NULL → el resultado de la comparación es NULL
/// (semántica SQL/Cypher). Nodos y aristas se comparan por ID (identidad):
/// `WHERE a = b` sobre dos variables de nodo es el predicado "mismo nodo",
/// útil para self-loops.
fn eq_cells(a: &Cell, b: &Cell) -> Option<bool> {
    match (a, b) {
        (Cell::Scalar(Value::Null), _) | (_, Cell::Scalar(Value::Null)) => None,
        (Cell::Scalar(x), Cell::Scalar(y)) => Some(scalars_eq(x, y)),
        (Cell::Node(x), Cell::Node(y)) => Some(x.id == y.id),
        (Cell::Edge(x), Cell::Edge(y)) => Some(x.id == y.id),
        // Tipos distintos (nodo vs escalar, nodo vs arista…): no son iguales.
        _ => Some(false),
    }
}

/// Igualdad de escalares: los numéricos cruzados (Int/Float) se promocionan;
/// tipos distintos (`1 = "1"`) son simplemente distintos, estilo Cypher.
fn scalars_eq(x: &Value, y: &Value) -> bool {
    match (x, y) {
        (Value::Int(i), Value::Float(f)) | (Value::Float(f), Value::Int(i)) => *i as f64 == *f,
        _ => x == y,
    }
}

/// Orden de dos celdas para `<`, `<=`, `>`, `>=`.
///
/// `None` = NULL involucrado o par no ordenable (el resultado es NULL): sólo
/// se ordenan números entre sí (con promoción) y cadenas entre sí — coherente
/// con `LogicalType::order_compatible` del cap 19.
fn cmp_cells(a: &Cell, b: &Cell) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Cell::Scalar(x), Cell::Scalar(y)) => match (x, y) {
            (Value::Null, _) | (_, Value::Null) => None,
            (Value::Int(i), Value::Int(j)) => Some(i.cmp(j)),
            (Value::Float(p), Value::Float(q)) => p.partial_cmp(q),
            (Value::Int(i), Value::Float(f)) => (*i as f64).partial_cmp(f),
            (Value::Float(f), Value::Int(i)) => f.partial_cmp(&(*i as f64)),
            (Value::String(s), Value::String(t)) => Some(s.cmp(t)),
            // Bool/Bytes o tipos cruzados: sin orden definido.
            _ => None,
        },
        // Nodos y aristas tienen identidad, no orden.
        _ => None,
    }
}

/// Resultado de una comparación binaria como `Value` (Bool o Null).
fn compare_op(op: CompareOp, l: &Cell, r: &Cell) -> Value {
    use std::cmp::Ordering;
    match op {
        CompareOp::Eq => match eq_cells(l, r) {
            Some(eq) => Value::Bool(eq),
            None => Value::Null,
        },
        CompareOp::NotEq => match eq_cells(l, r) {
            Some(eq) => Value::Bool(!eq),
            None => Value::Null,
        },
        CompareOp::Lt => cmp_cells(l, r)
            .map(|o| Value::Bool(o == Ordering::Less))
            .unwrap_or(Value::Null),
        CompareOp::Lte => cmp_cells(l, r)
            .map(|o| Value::Bool(o != Ordering::Greater))
            .unwrap_or(Value::Null),
        CompareOp::Gt => cmp_cells(l, r)
            .map(|o| Value::Bool(o == Ordering::Greater))
            .unwrap_or(Value::Null),
        CompareOp::Gte => cmp_cells(l, r)
            .map(|o| Value::Bool(o != Ordering::Less))
            .unwrap_or(Value::Null),
    }
}

/// Operando lógico: `Some(bool)`, `None` si NULL (lógica trivalente) o error
/// si la celda no es booleana (p.ej. una propiedad `age` usada como condición).
fn as_bool(cell: &Cell, context: &'static str) -> Result<Option<bool>, ExecError> {
    match cell {
        Cell::Scalar(Value::Bool(b)) => Ok(Some(*b)),
        Cell::Scalar(Value::Null) => Ok(None),
        other => Err(ExecError::TypeMismatch {
            context,
            expected: "BOOL",
            got: other.type_name().to_string(),
        }),
    }
}

/// Evalúa una [`ScalarExpr`] sobre una fila y devuelve la celda resultante.
///
/// Semántica (cap 7 + caps 17/19):
/// - Literales y variables: la celda correspondiente (una variable de nodo
///   evalúa al NODO entero — así funciona `RETURN p`).
/// - `p.name`: propiedad del elemento ligado; ausente → `Null` (schemaless).
/// - `f:Person`: predicado de etiqueta; sobre una ARISTA da `Null` (las
///   aristas no tienen labels → desconocido).
/// - Comparaciones: NULL domina (`Null = x` → `Null`); igualdad numérica
///   cruzada con promoción; tipos distintos no son iguales; sólo números y
///   cadenas se ordenan.
/// - AND/OR/NOT en lógica trivalente (TRUE/FALSE/NULL) con cortocircuito:
///   `FALSE AND x` no evalúa `x`, `TRUE OR x` tampoco (prometido en cap 17).
pub fn eval_scalar(expr: &ScalarExpr, row: &Row) -> Result<Cell, ExecError> {
    match expr {
        ScalarExpr::Literal(v) => Ok(Cell::Scalar(v.clone())),
        ScalarExpr::Var { name, .. } => {
            row.get(name)
                .cloned()
                .ok_or_else(|| ExecError::UnboundVariable {
                    variable: name.clone(),
                })
        }
        ScalarExpr::Property { variable, property } => match row.get(variable) {
            Some(Cell::Node(n)) => Ok(Cell::Scalar(
                n.props.get(property).cloned().unwrap_or(Value::Null),
            )),
            Some(Cell::Edge(e)) => Ok(Cell::Scalar(
                e.props.get(property).cloned().unwrap_or(Value::Null),
            )),
            Some(_) => Err(ExecError::PropertyOnScalar {
                variable: variable.clone(),
            }),
            None => Err(ExecError::UnboundVariable {
                variable: variable.clone(),
            }),
        },
        ScalarExpr::HasLabel { variable, label } => match row.get(variable) {
            Some(Cell::Node(n)) => Ok(Cell::Scalar(Value::Bool(n.has_label(label)))),
            // Una arista no tiene labels: desconocido (NULL), no FALSE —
            // coherente con la lógica trivalente del resto de predicados.
            Some(Cell::Edge(_)) => Ok(Cell::Scalar(Value::Null)),
            Some(other) => Err(ExecError::TypeMismatch {
                context: "predicado de etiqueta",
                expected: "NODE",
                got: other.type_name().to_string(),
            }),
            None => Err(ExecError::UnboundVariable {
                variable: variable.clone(),
            }),
        },
        ScalarExpr::Compare { op, left, right } => {
            let l = eval_scalar(left, row)?;
            let r = eval_scalar(right, row)?;
            Ok(Cell::Scalar(compare_op(*op, &l, &r)))
        }
        ScalarExpr::And { left, right } => {
            let l = as_bool(&eval_scalar(left, row)?, "operando de AND")?;
            // CORTOCIRCUITO: FALSE AND x = FALSE sin evaluar x.
            if l == Some(false) {
                return Ok(Cell::Scalar(Value::Bool(false)));
            }
            let r = as_bool(&eval_scalar(right, row)?, "operando de AND")?;
            let v = match (l, r) {
                (_, Some(false)) => Value::Bool(false),
                (Some(true), Some(true)) => Value::Bool(true),
                // NULL con TRUE/FALSE/NULL: se propaga el desconocido.
                _ => Value::Null,
            };
            Ok(Cell::Scalar(v))
        }
        ScalarExpr::Or { left, right } => {
            let l = as_bool(&eval_scalar(left, row)?, "operando de OR")?;
            // CORTOCIRCUITO: TRUE OR x = TRUE sin evaluar x.
            if l == Some(true) {
                return Ok(Cell::Scalar(Value::Bool(true)));
            }
            let r = as_bool(&eval_scalar(right, row)?, "operando de OR")?;
            let v = match (l, r) {
                (_, Some(true)) => Value::Bool(true),
                (Some(false), Some(false)) => Value::Bool(false),
                _ => Value::Null,
            };
            Ok(Cell::Scalar(v))
        }
        ScalarExpr::Not { expr } => match as_bool(&eval_scalar(expr, row)?, "operando de NOT")? {
            Some(b) => Ok(Cell::Scalar(Value::Bool(!b))),
            None => Ok(Cell::Scalar(Value::Null)),
        },
    }
}

// ─── El trait PhysicalOperator (interfaz Volcano del brief) ───

/// Interfaz de todo operador físico: la tríada Volcano `open`/`next`/`close`.
///
/// - `open` prepara el operador (posiciona cursores, abre hijos, materializa
///   lo imprescindible). Debe poder llamarse tras un `close` para re-ejecutar.
/// - `next` devuelve la siguiente fila (`Row`) o `None` al agotarse. El
///   modelo es *pull*: nadie calcula filas que el consumidor no pida.
/// - `close` libera el estado y se propaga a los hijos (idempotente).
///
/// Además, cada operador sabe su `name` y cuántas filas ha producido — las
/// métricas que el cap 21 convertirá en `explain` con cardinalidades reales.
pub trait PhysicalOperator {
    /// Prepara el operador y sus hijos (resetea el estado de una ejecución).
    fn open(&mut self) -> Result<(), ExecError>;
    /// Siguiente fila, o `None` si está agotado.
    fn next(&mut self) -> Result<Option<Row>, ExecError>;
    /// Libera recursos y resetea cursores (idempotente).
    fn close(&mut self) -> Result<(), ExecError>;
    /// Nombre canónico del operador (métricas / explain).
    fn name(&self) -> &'static str;
    /// Filas emitidas por este operador desde el último `open`.
    fn rows_produced(&self) -> u64;
    /// Métricas en pre-orden: este operador seguido de sus hijos.
    fn collect_metrics(&self) -> Vec<(&'static str, u64)> {
        vec![(self.name(), self.rows_produced())]
    }
}

// ─── NodeScanOp: iterar nodos del store ───

/// Escaneo de nodos: liga `variable` a cada nodo (con `label`, si la hay).
///
/// Es la hoja del pipeline: su cursor es un iterador perezoso sobre
/// `GraphStore::iter_nodes` (cap 8) que se posiciona en `open()`. El orden es
/// el del store (en `MemoryStore`, orden de inserción por id) — determinista,
/// requisito para tests y para el `explain` del cap 21.
pub struct NodeScanOp<'a> {
    store: &'a dyn GraphStore,
    variable: String,
    label: Option<String>,
    /// Cursor perezoso: `None` = sin abrir o ya cerrado.
    cursor: Option<Box<dyn Iterator<Item = NodeId> + 'a>>,
    produced: u64,
}

impl<'a> NodeScanOp<'a> {
    pub fn new(store: &'a dyn GraphStore, variable: String, label: Option<String>) -> Self {
        Self {
            store,
            variable,
            label,
            cursor: None,
            produced: 0,
        }
    }
}

impl PhysicalOperator for NodeScanOp<'_> {
    fn name(&self) -> &'static str {
        "NodeScan"
    }

    fn rows_produced(&self) -> u64 {
        self.produced
    }

    fn open(&mut self) -> Result<(), ExecError> {
        // Copiar la referencia al store ANTES de guardar el iterador: así el
        // préstamo del iterador vive tanto como el store ('a), no como &mut self.
        let store = self.store;
        self.cursor = Some(Box::new(store.iter_nodes().map(|n| n.id)));
        self.produced = 0;
        Ok(())
    }

    fn next(&mut self) -> Result<Option<Row>, ExecError> {
        let Some(cursor) = self.cursor.as_mut() else {
            return Ok(None); // sin abrir o cerrado
        };
        for id in cursor.by_ref() {
            let Some(node) = self.store.get_node(id) else {
                continue;
            };
            if self.label.as_ref().is_some_and(|l| !node.has_label(l)) {
                continue;
            }
            let mut row = Row::new();
            row.bind(&self.variable, Cell::Node(node.clone()));
            self.produced += 1;
            return Ok(Some(row));
        }
        Ok(None)
    }

    fn close(&mut self) -> Result<(), ExecError> {
        self.cursor = None;
        Ok(())
    }
}

// ─── IndexSeekOp: empezar por IDs conocidos ───

/// Búsqueda por índice: liga `variable` exactamente a los nodos cuyos IDs
/// recibe por parámetro.
///
/// La gracia del operador es NO escanear: quien lo construye ya resolvió la
/// búsqueda (p.ej. `Person.name = "Ana"`) con un índice del cap 15. Hoy los
/// IDs se aportan manualmente; ELEGIR este operador en vez de `NodeScanOp` es
/// trabajo del optimizador del cap 21 (su ejemplo canónico transforma
/// `Filter(name="Ana") + NodeScan` en `IndexSeek`). Si un ID no existe, el
/// índice está desactualizado y se reporta como error.
pub struct IndexSeekOp<'a> {
    store: &'a dyn GraphStore,
    variable: String,
    /// IDs de nodos, orden de llegada (orden del índice).
    ids: Vec<NodeId>,
    pos: usize,
    produced: u64,
}

impl<'a> IndexSeekOp<'a> {
    pub fn new(store: &'a dyn GraphStore, variable: String, ids: Vec<NodeId>) -> Self {
        Self {
            store,
            variable,
            ids,
            pos: 0,
            produced: 0,
        }
    }
}

impl PhysicalOperator for IndexSeekOp<'_> {
    fn name(&self) -> &'static str {
        "IndexSeek"
    }

    fn rows_produced(&self) -> u64 {
        self.produced
    }

    fn open(&mut self) -> Result<(), ExecError> {
        self.pos = 0;
        self.produced = 0;
        Ok(())
    }

    fn next(&mut self) -> Result<Option<Row>, ExecError> {
        if self.pos >= self.ids.len() {
            return Ok(None);
        }
        let id = self.ids[self.pos];
        self.pos += 1;
        let node = self.store.get_node(id).ok_or(ExecError::UnknownNode(id))?;
        let mut row = Row::new();
        row.bind(&self.variable, Cell::Node(node.clone()));
        self.produced += 1;
        Ok(Some(row))
    }

    fn close(&mut self) -> Result<(), ExecError> {
        Ok(())
    }
}

// ─── ExpandOp: recorrer la adyacencia ───

/// Expansión: por cada fila del input, liga la relación y el nodo del otro
/// extremo siguiendo `direction` (OUTGOING/INCOMING/UNDIRECTED).
///
/// Bucle anidado clásico: por cada fila "externa" (pedida al input) se
/// recorren sus aristas candidatas una a una (bucle "interno"). Es un
/// *index nested loop join* contra la adyacencia del store (cap 8):
/// `out_edges`/`in_edges` hacen de índice.
///
/// - OUTGOING `(a)-[r]->(b)`: aristas que SAEN de `a`; `b` = target.
/// - INCOMING `(a)<-[r]-(b)`: aristas que LLEGAN a `a`; `b` = source.
/// - UNDIRECTED `(a)-[r]-(b)`: ambas; un self-loop cuenta UNA vez (ya
///   apareció en la pasada saliente).
/// - `rel_type` filtra por etiqueta de la arista; `rel_variable` liga la
///   arista entera si el patrón la nombra (`-[r:KNOWS]->`).
pub struct ExpandOp<'a> {
    store: &'a dyn GraphStore,
    input: Box<dyn PhysicalOperator + 'a>,
    from: String,
    rel_variable: Option<String>,
    rel_type: Option<String>,
    direction: RelDirection,
    to: String,
    /// Fila actual del input cuyas aristas se están recorriendo.
    current: Option<Row>,
    /// Candidatos (arista, nodo destino) para la fila actual.
    candidates: Vec<(EdgeId, NodeId)>,
    pos: usize,
    produced: u64,
}

impl<'a> ExpandOp<'a> {
    pub fn new(
        store: &'a dyn GraphStore,
        input: Box<dyn PhysicalOperator + 'a>,
        from: String,
        rel_variable: Option<String>,
        rel_type: Option<String>,
        direction: RelDirection,
        to: String,
    ) -> Self {
        Self {
            store,
            input,
            from,
            rel_variable,
            rel_type,
            direction,
            to,
            current: None,
            candidates: Vec::new(),
            pos: 0,
            produced: 0,
        }
    }

    /// Carga los candidatos de adyacencia para el nodo ligado a `from`.
    fn load_candidates(&mut self, row: &Row) -> Result<(), ExecError> {
        let Some(cell) = row.get(&self.from) else {
            return Err(ExecError::UnboundVariable {
                variable: self.from.clone(),
            });
        };
        let Cell::Node(node) = cell else {
            return Err(ExecError::NotANode {
                variable: self.from.clone(),
            });
        };
        // Copiar la referencia para no cruzar préstamos de self.
        let store = self.store;
        self.candidates.clear();
        self.pos = 0;
        match self.direction {
            RelDirection::Outgoing => {
                for eid in store.out_edges(node.id) {
                    if let Some(e) = store.get_edge(eid) {
                        self.candidates.push((eid, e.target));
                    }
                }
            }
            RelDirection::Incoming => {
                for eid in store.in_edges(node.id) {
                    if let Some(e) = store.get_edge(eid) {
                        self.candidates.push((eid, e.source));
                    }
                }
            }
            RelDirection::Undirected => {
                for eid in store.out_edges(node.id) {
                    if let Some(e) = store.get_edge(eid) {
                        self.candidates.push((eid, e.target));
                    }
                }
                for eid in store.in_edges(node.id) {
                    if let Some(e) = store.get_edge(eid) {
                        // Un self-loop ya salió en la pasada saliente: una vez.
                        if e.source != e.target {
                            self.candidates.push((eid, e.source));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl PhysicalOperator for ExpandOp<'_> {
    fn name(&self) -> &'static str {
        "Expand"
    }

    fn rows_produced(&self) -> u64 {
        self.produced
    }

    fn collect_metrics(&self) -> Vec<(&'static str, u64)> {
        let mut out = vec![(self.name(), self.produced)];
        out.extend(self.input.collect_metrics());
        out
    }

    fn open(&mut self) -> Result<(), ExecError> {
        self.input.open()?;
        self.current = None;
        self.candidates.clear();
        self.pos = 0;
        self.produced = 0;
        Ok(())
    }

    fn next(&mut self) -> Result<Option<Row>, ExecError> {
        loop {
            // 1. Consumir los candidatos de la fila actual (bucle interno).
            while self.pos < self.candidates.len() {
                let (eid, to) = self.candidates[self.pos];
                self.pos += 1;
                let Some(edge) = self.store.get_edge(eid) else {
                    continue;
                };
                if self.rel_type.as_ref().is_some_and(|t| &edge.label != t) {
                    continue;
                }
                let Some(node) = self.store.get_node(to) else {
                    continue;
                };
                // Materializar la fila extendida: clonar la actual y ligar
                // relación (si el patrón la nombra) y destino.
                let Some(mut extended) = self.current.clone() else {
                    continue; // defensivo: hay candidatos ⟹ hay fila actual
                };
                if let Some(rv) = &self.rel_variable {
                    extended.bind(rv, Cell::Edge(edge.clone()));
                }
                extended.bind(&self.to, Cell::Node(node.clone()));
                self.produced += 1;
                return Ok(Some(extended));
            }
            // 2. Candidatos agotados: pedir la siguiente fila (bucle externo).
            match self.input.next()? {
                Some(row) => {
                    self.load_candidates(&row)?;
                    self.current = Some(row);
                }
                None => return Ok(None),
            }
        }
    }

    fn close(&mut self) -> Result<(), ExecError> {
        self.input.close()?;
        self.current = None;
        self.candidates.clear();
        self.pos = 0;
        Ok(())
    }
}

// ─── FilterOp: evaluar el predicado sobre cada fila ───

/// Filtro: deja pasar las filas cuyo predicado evalúa a TRUE.
///
/// FALSE y NULL se descartan (lógica trivalente: `p.missing > 30` es
/// desconocido, no cierto). Un predicado no booleano (`WHERE p.age`) es un
/// `ExecError::TypeMismatch` EN EJECUCIÓN — el plan del cap 19 sólo pudo
/// tiparlo como `Any` (schemaless).
pub struct FilterOp<'a> {
    input: Box<dyn PhysicalOperator + 'a>,
    predicate: ScalarExpr,
    produced: u64,
}

impl<'a> FilterOp<'a> {
    pub fn new(input: Box<dyn PhysicalOperator + 'a>, predicate: ScalarExpr) -> Self {
        Self {
            input,
            predicate,
            produced: 0,
        }
    }
}

impl PhysicalOperator for FilterOp<'_> {
    fn name(&self) -> &'static str {
        "Filter"
    }

    fn rows_produced(&self) -> u64 {
        self.produced
    }

    fn collect_metrics(&self) -> Vec<(&'static str, u64)> {
        let mut out = vec![(self.name(), self.produced)];
        out.extend(self.input.collect_metrics());
        out
    }

    fn open(&mut self) -> Result<(), ExecError> {
        self.input.open()?;
        self.produced = 0;
        Ok(())
    }

    fn next(&mut self) -> Result<Option<Row>, ExecError> {
        loop {
            let Some(row) = self.input.next()? else {
                return Ok(None);
            };
            let verdict = eval_scalar(&self.predicate, &row)?;
            match as_bool(&verdict, "Filter (WHERE)")? {
                Some(true) => {
                    self.produced += 1;
                    return Ok(Some(row));
                }
                // FALSE o NULL: la fila queda fuera.
                _ => continue,
            }
        }
    }

    fn close(&mut self) -> Result<(), ExecError> {
        self.input.close()
    }
}

// ─── ProjectOp: la proyección del RETURN ───

/// Proyección: convierte cada fila interna (variables ligadas) en la fila de
/// salida (una celda por item del RETURN, nombrada por su `output_name`).
///
/// Es la única operación que "cambia de forma": debajo los operadores pasan
/// bindings; a partir de aquí las filas son columnas de resultado.
pub struct ProjectOp<'a> {
    input: Box<dyn PhysicalOperator + 'a>,
    items: Vec<Projection>,
    produced: u64,
}

impl<'a> ProjectOp<'a> {
    pub fn new(input: Box<dyn PhysicalOperator + 'a>, items: Vec<Projection>) -> Self {
        Self {
            input,
            items,
            produced: 0,
        }
    }
}

impl PhysicalOperator for ProjectOp<'_> {
    fn name(&self) -> &'static str {
        "Project"
    }

    fn rows_produced(&self) -> u64 {
        self.produced
    }

    fn collect_metrics(&self) -> Vec<(&'static str, u64)> {
        let mut out = vec![(self.name(), self.produced)];
        out.extend(self.input.collect_metrics());
        out
    }

    fn open(&mut self) -> Result<(), ExecError> {
        self.input.open()?;
        self.produced = 0;
        Ok(())
    }

    fn next(&mut self) -> Result<Option<Row>, ExecError> {
        let Some(row) = self.input.next()? else {
            return Ok(None);
        };
        let mut out = Row::new();
        for item in &self.items {
            let cell = eval_scalar(&item.expr, &row)?;
            out.bind(item.output_name(), cell);
        }
        self.produced += 1;
        Ok(Some(out))
    }

    fn close(&mut self) -> Result<(), ExecError> {
        self.input.close()
    }
}

// ─── CartesianProductOp: patrones disjuntos del MATCH ───

/// Producto cartesiano anidado: cada fila de la izquierda × cada fila de la
/// derecha (`Row::merge` concatena las variables de ambos patrones).
///
/// Volcano es de una sola pasada: un operador no puede "rebobinar" a su
/// input. El producto necesita re-leer el lado derecho por cada fila de la
/// izquierda, así que lo MATERIALIZA una vez en `open()`. Ese coste extra
/// (memoria + filas de más antes de cualquier filtro) es exactamente lo que
/// el optimizador del cap 21 evitará reordenando el punto de partida.
pub struct CartesianProductOp<'a> {
    left: Box<dyn PhysicalOperator + 'a>,
    right: Box<dyn PhysicalOperator + 'a>,
    /// Lado derecho materializado en `open()`.
    right_rows: Vec<Row>,
    current: Option<Row>,
    pos: usize,
    produced: u64,
}

impl<'a> CartesianProductOp<'a> {
    pub fn new(
        left: Box<dyn PhysicalOperator + 'a>,
        right: Box<dyn PhysicalOperator + 'a>,
    ) -> Self {
        Self {
            left,
            right,
            right_rows: Vec::new(),
            current: None,
            pos: 0,
            produced: 0,
        }
    }
}

impl PhysicalOperator for CartesianProductOp<'_> {
    fn name(&self) -> &'static str {
        "CartesianProduct"
    }

    fn rows_produced(&self) -> u64 {
        self.produced
    }

    fn collect_metrics(&self) -> Vec<(&'static str, u64)> {
        let mut out = vec![(self.name(), self.produced)];
        out.extend(self.left.collect_metrics());
        out.extend(self.right.collect_metrics());
        out
    }

    fn open(&mut self) -> Result<(), ExecError> {
        self.left.open()?;
        self.right.open()?;
        // Materializar el lado derecho completo (una sola pasada).
        let mut rows = Vec::new();
        while let Some(row) = self.right.next()? {
            rows.push(row);
        }
        self.right.close()?;
        self.right_rows = rows;
        self.current = None;
        self.pos = 0;
        self.produced = 0;
        Ok(())
    }

    fn next(&mut self) -> Result<Option<Row>, ExecError> {
        loop {
            // 1. ¿Queda un cruce pendiente con la fila izquierda actual?
            if self.pos < self.right_rows.len()
                && let Some(left_row) = self.current.clone()
            {
                let right_row = self.right_rows[self.pos].clone();
                self.pos += 1;
                let merged = left_row.merge(right_row);
                self.produced += 1;
                return Ok(Some(merged));
            }
            // 2. Siguiente fila de la izquierda: reiniciar el puntero derecho.
            match self.left.next()? {
                Some(row) => {
                    self.current = Some(row);
                    self.pos = 0;
                }
                None => return Ok(None),
            }
        }
    }

    fn close(&mut self) -> Result<(), ExecError> {
        self.left.close()?;
        // El derecho ya se cerró tras materializar; cerrar de nuevo es idempotente.
        self.right.close()?;
        self.right_rows.clear();
        self.current = None;
        self.pos = 0;
        Ok(())
    }
}

// ─── LimitOp y DistinctOp ───

/// Límite: emite como máximo `max` filas y se agota.
///
/// En un pipeline pull-based esto corta la ejecución de verdad: si es la
/// raíz, nadie pide más filas al árbol de abajo. La gramática LiraQL aún no
/// expone `LIMIT` (caps 17-18); el operador queda listo para la CLI (cap 31)
/// y el optimizador (cap 21).
pub struct LimitOp<'a> {
    input: Box<dyn PhysicalOperator + 'a>,
    max: usize,
    emitted: usize,
    produced: u64,
}

impl<'a> LimitOp<'a> {
    pub fn new(input: Box<dyn PhysicalOperator + 'a>, max: usize) -> Self {
        Self {
            input,
            max,
            emitted: 0,
            produced: 0,
        }
    }
}

impl PhysicalOperator for LimitOp<'_> {
    fn name(&self) -> &'static str {
        "Limit"
    }

    fn rows_produced(&self) -> u64 {
        self.produced
    }

    fn collect_metrics(&self) -> Vec<(&'static str, u64)> {
        let mut out = vec![(self.name(), self.produced)];
        out.extend(self.input.collect_metrics());
        out
    }

    fn open(&mut self) -> Result<(), ExecError> {
        self.input.open()?;
        self.emitted = 0;
        self.produced = 0;
        Ok(())
    }

    fn next(&mut self) -> Result<Option<Row>, ExecError> {
        if self.emitted >= self.max {
            return Ok(None);
        }
        match self.input.next()? {
            Some(row) => {
                self.emitted += 1;
                self.produced += 1;
                Ok(Some(row))
            }
            None => Ok(None),
        }
    }

    fn close(&mut self) -> Result<(), ExecError> {
        self.input.close()
    }
}

/// Distinct: descarta filas repetidas (comparación exacta de celdas).
///
/// El conjunto de vistos se guarda en un `Vec` con búsqueda lineal —
/// deliberadamente simple: las celdas contienen `f64` (no hasheables) y el
/// tamaño didáctico lo permite; una versión real usaría una firma hasheable
/// por fila.
pub struct DistinctOp<'a> {
    input: Box<dyn PhysicalOperator + 'a>,
    seen: Vec<Row>,
    produced: u64,
}

impl<'a> DistinctOp<'a> {
    pub fn new(input: Box<dyn PhysicalOperator + 'a>) -> Self {
        Self {
            input,
            seen: Vec::new(),
            produced: 0,
        }
    }
}

impl PhysicalOperator for DistinctOp<'_> {
    fn name(&self) -> &'static str {
        "Distinct"
    }

    fn rows_produced(&self) -> u64 {
        self.produced
    }

    fn collect_metrics(&self) -> Vec<(&'static str, u64)> {
        let mut out = vec![(self.name(), self.produced)];
        out.extend(self.input.collect_metrics());
        out
    }

    fn open(&mut self) -> Result<(), ExecError> {
        self.input.open()?;
        self.seen.clear();
        self.produced = 0;
        Ok(())
    }

    fn next(&mut self) -> Result<Option<Row>, ExecError> {
        loop {
            let Some(row) = self.input.next()? else {
                return Ok(None);
            };
            if self.seen.iter().any(|r| r == &row) {
                continue;
            }
            self.seen.push(row.clone());
            self.produced += 1;
            return Ok(Some(row));
        }
    }

    fn close(&mut self) -> Result<(), ExecError> {
        self.input.close()?;
        self.seen.clear();
        Ok(())
    }
}

// ─── Compilación: LogicalPlan → árbol de operadores físicos ───

/// Compila un [`LogicalPlan`] en su árbol de operadores físicos.
///
/// Traducción 1:1: el optimizador del cap. 21 reescribe el plan ANTES de
/// llegar aquí (push-down de filtros, `NodeScan` → `IndexSeek`, reordenación
/// de expansiones vía [`crate::cap21_optimizador::optimize`]); la compilación
/// sólo elige el operador físico de cada nodo lógico.
pub fn compile<'a>(
    plan: &LogicalPlan,
    store: &'a dyn GraphStore,
) -> Result<Box<dyn PhysicalOperator + 'a>, ExecError> {
    Ok(match plan {
        LogicalPlan::NodeScan { variable, label } => {
            Box::new(NodeScanOp::new(store, variable.clone(), label.clone()))
        }
        LogicalPlan::IndexSeek { variable, ids, .. } => {
            Box::new(IndexSeekOp::new(store, variable.clone(), ids.clone()))
        }
        LogicalPlan::Expand {
            input,
            from,
            rel_variable,
            rel_type,
            direction,
            to,
        } => Box::new(ExpandOp::new(
            store,
            compile(input, store)?,
            from.clone(),
            rel_variable.clone(),
            rel_type.clone(),
            *direction,
            to.clone(),
        )),
        LogicalPlan::Filter { input, predicate } => {
            Box::new(FilterOp::new(compile(input, store)?, predicate.clone()))
        }
        LogicalPlan::Project { input, items } => {
            Box::new(ProjectOp::new(compile(input, store)?, items.clone()))
        }
        LogicalPlan::CartesianProduct { left, right } => Box::new(CartesianProductOp::new(
            compile(left, store)?,
            compile(right, store)?,
        )),
    })
}

// ─── ResultSet y métricas ───

/// Resultado de una consulta: columnas con nombre y filas de celdas.
///
/// `rows[i][j]` es la celda de la fila `i` bajo la columna `j` (el orden de
/// los items del RETURN). El `Display` dibuja una tabla — la semilla de la
/// salida de la CLI (cap 31).
#[derive(Debug, Clone, PartialEq)]
pub struct ResultSet {
    /// Nombres de columna (`alias` o derivado: `f.name`).
    pub columns: Vec<String>,
    /// Filas de resultado, en el orden en que el pipeline las produjo.
    pub rows: Vec<Vec<Cell>>,
}

impl ResultSet {
    /// Número de filas devueltas.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// ¿Sin filas? (las columnas pueden seguir presentes).
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Índice de la primera columna con ese nombre.
    pub fn column(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c == name)
    }
}

impl std::fmt::Display for ResultSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Ancho de cada columna = máximo entre cabecera y celdas (en chars).
        let widths: Vec<usize> = self
            .columns
            .iter()
            .enumerate()
            .map(|(j, col)| {
                let cells_w = self
                    .rows
                    .iter()
                    .map(|r| r.get(j).map(|c| c.to_string().chars().count()).unwrap_or(0))
                    .max()
                    .unwrap_or(0);
                col.chars().count().max(cells_w)
            })
            .collect();
        let line = |cells: Vec<String>| {
            cells
                .into_iter()
                .zip(&widths)
                .map(|(c, w)| format!("{c:<w$}"))
                .collect::<Vec<_>>()
                .join(" | ")
        };
        writeln!(f, "{}", line(self.columns.clone()))?;
        for row in &self.rows {
            let cells: Vec<String> = row.iter().map(|c| c.to_string()).collect();
            writeln!(f, "{}", line(cells))?;
        }
        Ok(())
    }
}

/// Métricas de una ejecución: filas producidas por operador (pre-orden) y
/// filas devueltas al cliente.
///
/// Es la semilla del `liradb explain` con cardinalidades REALES del cap 21:
/// `NodeScan: 4 | Expand: 4 | Filter: 1 | Project: 1` cuenta lo que de verdad
/// fluyó por el pipeline, no lo estimado.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExecMetrics {
    /// `(operador, filas producidas)` en pre-orden desde la raíz.
    pub per_operator: Vec<(&'static str, u64)>,
    /// Filas que salieron por la raíz.
    pub rows_returned: u64,
}

impl std::fmt::Display for ExecMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (name, rows) in &self.per_operator {
            writeln!(f, "{name}: {rows} filas")?;
        }
        write!(f, "filas devueltas: {}", self.rows_returned)
    }
}

// ─── Executor: el ciclo open → next* → close ───

/// Ejecuta un plan completo sobre un store y devuelve el `ResultSet`.
///
/// Ciclo Volcano clásico: `open` → pedir `next` hasta agotar → `close`
/// (siempre, también en error). Las columnas salen del `Project` raíz —
/// requisito del `lower()` del cap 19. Tras `execute`, [`Executor::metrics`]
/// expone las métricas del árbol.
pub struct Executor<'a> {
    root: Box<dyn PhysicalOperator + 'a>,
    columns: Vec<String>,
    rows_returned: u64,
}

impl<'a> Executor<'a> {
    /// Compila `plan` sobre `store`. El plan debe tener un `Project` raíz
    /// (como todo plan generado por [`lower`]).
    pub fn new(plan: &LogicalPlan, store: &'a dyn GraphStore) -> Result<Self, ExecError> {
        let columns = match plan {
            LogicalPlan::Project { items, .. } => items.iter().map(|p| p.output_name()).collect(),
            _ => return Err(ExecError::NotAProjection),
        };
        Ok(Self {
            root: compile(plan, store)?,
            columns,
            rows_returned: 0,
        })
    }

    /// Ejecuta el pipeline entero y materializa el resultado.
    pub fn execute(&mut self) -> Result<ResultSet, ExecError> {
        self.root.open()?;
        let mut rows: Vec<Vec<Cell>> = Vec::new();
        let drained = loop {
            match self.root.next() {
                Ok(Some(row)) => rows.push(row.cells()),
                Ok(None) => break Ok(()),
                Err(e) => break Err(e),
            }
        };
        // close SIEMPRE (incluso tras error): como un defer.
        self.root.close()?;
        drained?;
        self.rows_returned = rows.len() as u64;
        Ok(ResultSet {
            columns: self.columns.clone(),
            rows,
        })
    }

    /// Métricas de la última `execute` (0s si aún no se ejecutó).
    pub fn metrics(&self) -> ExecMetrics {
        ExecMetrics {
            per_operator: self.root.collect_metrics(),
            rows_returned: self.rows_returned,
        }
    }
}

// ─── API de alto nivel: el hito del capítulo ───

impl Query {
    /// Ejecuta esta consulta sobre un store: `lower` + `optimize` + `Executor`.
    ///
    /// Desde el cap. 21 el plan pasa por el optimizador (estadísticas +
    /// reglas) antes de ejecutarse; los resultados son los mismos que los
    /// del plan ingenuo (verificados por los tests de equivalencia del
    /// cap. 21) — sin `ORDER BY`, el ORDEN de las filas no está garantizado.
    ///
    /// ```text
    /// let rs = parse("MATCH (p:Person) RETURN p.name")?.execute(&store)?;
    /// ```
    pub fn execute(&self, store: &dyn GraphStore) -> Result<ResultSet, ExecError> {
        let plan = self.lower()?;
        let catalog = Catalog::collect(store);
        let plan = optimize(&plan, &catalog);
        Executor::new(&plan, store)?.execute()
    }
}

/// Hito del cap 20: **ejecutar consultas completas desde texto**.
///
/// Pipeline entero: `parse` (cap 18) → `lower` (cap 19) → `optimize`
/// (cap 21: estadísticas + reglas) → `compile` + Volcano (este cap) → filas.
///
/// ```text
/// let rs = run("MATCH (p:Person)-[:KNOWS]->(f:Person) \
///               WHERE p.name = \"Ana\" RETURN f.name", &store)?;
/// // rs.columns == ["f.name"], rs.rows == [["Bo"]]
/// ```
pub fn run(src: &str, store: &dyn GraphStore) -> Result<ResultSet, ExecError> {
    let query = parse(src)?;
    query.execute(store)
}

/// El **grafo demo** del libro: personas/conocidos + ciudades, en un
/// `MemoryStore` listo para consultar.
///
/// Es el MISMO fixture que usan los tests del cap. 20 (`tests_executor`),
/// expuesto como API pública para no duplicarlo: el hito de CLI mínima
/// (tras el cap. 20, ADR-005) y las demos de capítulos futuros comparten
/// este único punto de verdad. Si el grafo crece (p.ej. con el dataset
/// KB-Lira del Vol. III), crece aquí y todas las demos lo ven.
///
/// Contenido:
/// - Nodos `Person`: `0=Ana(36)`, `1=Bo(41)`, `2=Carla(29)`, `3=Dani(36)`.
/// - Nodos `City`: `4=Madrid`, `5=Lisboa`.
/// - Aristas `KNOWS`: `0:Ana→Bo(since 2020)`, `1:Bo→Carla(2021)`,
///   `2:Carla→Ana(2022)`, `3:Dani→Dani` (self-loop, sin props).
/// - Aristas `LIVES_IN`: `4:Ana→Madrid`, `5:Bo→Lisboa`.
///
/// ```
/// use vol2_liradb::{demo_graph, run, GraphStore};
///
/// let store = demo_graph();
/// assert_eq!(store.node_count(), 6);
/// assert_eq!(store.edge_count(), 6);
///
/// // El pipeline entero del cap 20 sobre el grafo demo:
/// let rs = run(
///     "MATCH (p:Person) WHERE p.age < 40 RETURN p.name",
///     &store,
/// )
/// .unwrap();
/// assert_eq!(rs.len(), 3); // Ana (36), Carla (29) y Dani (36)
/// ```
pub fn demo_graph() -> MemoryStore {
    let mut s = MemoryStore::new();
    let persona = |id: NodeId, name: &str, age: i64| {
        Node::new(id, "Person")
            .with_prop("name", Value::String(name.into()))
            .with_prop("age", Value::Int(age))
    };
    s.put_node(persona(0, "Ana", 36)).unwrap();
    s.put_node(persona(1, "Bo", 41)).unwrap();
    s.put_node(persona(2, "Carla", 29)).unwrap();
    s.put_node(persona(3, "Dani", 36)).unwrap();
    s.put_node(Node::new(4, "City").with_prop("name", Value::String("Madrid".into())))
        .unwrap();
    s.put_node(Node::new(5, "City").with_prop("name", Value::String("Lisboa".into())))
        .unwrap();
    s.put_edge(Edge::new(0, 0, 1, "KNOWS").with_prop("since", Value::Int(2020)))
        .unwrap();
    s.put_edge(Edge::new(1, 1, 2, "KNOWS").with_prop("since", Value::Int(2021)))
        .unwrap();
    s.put_edge(Edge::new(2, 2, 0, "KNOWS").with_prop("since", Value::Int(2022)))
        .unwrap();
    s.put_edge(Edge::new(3, 3, 3, "KNOWS")).unwrap();
    s.put_edge(Edge::new(4, 0, 4, "LIVES_IN")).unwrap();
    s.put_edge(Edge::new(5, 1, 5, "LIVES_IN")).unwrap();
    s
}

#[cfg(test)]
mod tests_executor {
    use super::*;
    use crate::cap19_plan_logico::{BindingKind, PlanErrorKind};

    // ════════════════════════════════════════════════════════════════
    //  Fixture: grafo de personas/conocidos (tipo brief) + ciudades
    // ════════════════════════════════════════════════════════════════

    /// El fixture es el helper público [`demo_graph`] (único punto de
    /// verdad): Personas 0=Ana(36), 1=Bo(41), 2=Carla(29), 3=Dani(36);
    /// Ciudades 4=Madrid, 5=Lisboa; KNOWS 0-3 (Dani→Dani self-loop) y
    /// LIVES_IN 4-5. Descrito en detalle en su doc-comment.
    fn grafo() -> MemoryStore {
        demo_graph()
    }

    /// Drena un operador con el ciclo Volcano completo (open/next*/close).
    fn drenar(op: &mut dyn PhysicalOperator) -> Result<Vec<Row>, ExecError> {
        op.open()?;
        let mut rows = Vec::new();
        while let Some(row) = op.next()? {
            rows.push(row);
        }
        op.close()?;
        Ok(rows)
    }

    /// Ejecuta `src` sobre `store` y devuelve las filas como Strings
    /// (comodidad para comparar resultados de una sola columna).
    fn texto(rs: &ResultSet, col: usize) -> Vec<String> {
        rs.rows.iter().map(|r| r[col].to_string()).collect()
    }

    // ════════════════════════════════════════════════════════════════
    //  Row y Cell — la fila del modelo Volcano
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn row_bind_get_merge_y_display() {
        let mut row = Row::new();
        assert!(row.is_empty());
        row.bind("p", Cell::Node(Node::new(0, "Person")));
        row.bind("r", Cell::Edge(Edge::new(7, 0, 1, "KNOWS")));
        assert_eq!(row.len(), 2);
        assert!(row.get("p").is_some());
        assert!(row.get("x").is_none());
        assert_eq!(row.to_string(), "{p: Node#0:Person, r: Edge#7:KNOWS}");

        // Merge concatena (patrones disjuntos del cartesiano).
        let mut other = Row::new();
        other.bind("c", Cell::Node(Node::new(4, "City")));
        let merged = row.merge(other);
        assert_eq!(merged.len(), 3);
        assert!(merged.get("c").is_some());

        // Iter en orden de ligadura.
        let nombres: Vec<&str> = merged.iter().map(|(n, _)| n).collect();
        assert_eq!(nombres, vec!["p", "r", "c"]);
    }

    #[test]
    fn cell_display_y_type_name() {
        assert_eq!(Cell::Scalar(Value::Null).to_string(), "NULL");
        assert_eq!(Cell::Scalar(Value::Int(36)).to_string(), "36");
        assert_eq!(
            Cell::Scalar(Value::String("Ana".into())).to_string(),
            "\"Ana\""
        );
        let multi = Node::new(1, "Person");
        let multi = Node {
            labels: vec!["Person".into(), "Admin".into()],
            ..multi
        };
        assert_eq!(Cell::Node(multi).to_string(), "Node#1:Person:Admin");
        assert_eq!(
            Cell::Edge(Edge::new(2, 0, 1, "KNOWS")).to_string(),
            "Edge#2:KNOWS"
        );
        assert_eq!(Cell::Scalar(Value::Bool(true)).type_name(), "BOOL");
        assert_eq!(Cell::Scalar(Value::Null).type_name(), "NULL");
        assert_eq!(Cell::Node(Node::new(0, "X")).type_name(), "NODE");
        assert_eq!(Cell::Edge(Edge::new(0, 0, 1, "X")).type_name(), "EDGE");
        assert_eq!(
            Cell::Scalar(Value::Int(1)).as_scalar(),
            Some(&Value::Int(1))
        );
        assert_eq!(Cell::Node(Node::new(0, "X")).as_scalar(), None);
    }

    // ════════════════════════════════════════════════════════════════
    //  Evaluación de ScalarExpr (semántica NULL / trivalente)
    // ════════════════════════════════════════════════════════════════

    /// Fila de prueba: p = Ana (con name/age), q = Bo.
    fn fila_prueba() -> Row {
        let mut row = Row::new();
        row.bind(
            "p",
            Cell::Node(
                Node::new(0, "Person")
                    .with_prop("name", Value::String("Ana".into()))
                    .with_prop("age", Value::Int(36)),
            ),
        );
        row.bind("q", Cell::Node(Node::new(1, "Person")));
        row.bind(
            "r",
            Cell::Edge(Edge::new(3, 0, 1, "KNOWS").with_prop("since", Value::Int(2020))),
        );
        row
    }

    #[test]
    fn eval_literales_variables_y_propiedades() {
        let row = fila_prueba();
        assert_eq!(
            eval_scalar(&ScalarExpr::lit(Value::Int(7)), &row).unwrap(),
            Cell::Scalar(Value::Int(7))
        );
        // Variable entera → el nodo/arista, no un escalar.
        assert!(matches!(
            eval_scalar(&ScalarExpr::var("p", BindingKind::Node), &row).unwrap(),
            Cell::Node(n) if n.id == 0
        ));
        // Propiedad presente / ausente (→ NULL) en nodo y en arista.
        assert_eq!(
            eval_scalar(&ScalarExpr::prop("p", "name"), &row).unwrap(),
            Cell::Scalar(Value::String("Ana".into()))
        );
        assert_eq!(
            eval_scalar(&ScalarExpr::prop("p", "missing"), &row).unwrap(),
            Cell::Scalar(Value::Null)
        );
        assert_eq!(
            eval_scalar(&ScalarExpr::prop("r", "since"), &row).unwrap(),
            Cell::Scalar(Value::Int(2020))
        );
        // HasLabel sobre nodo y sobre arista (→ NULL: las aristas no tienen).
        assert_eq!(
            eval_scalar(&ScalarExpr::has_label("p", "Person"), &row).unwrap(),
            Cell::Scalar(Value::Bool(true))
        );
        assert_eq!(
            eval_scalar(&ScalarExpr::has_label("p", "City"), &row).unwrap(),
            Cell::Scalar(Value::Bool(false))
        );
        assert_eq!(
            eval_scalar(&ScalarExpr::has_label("r", "KNOWS"), &row).unwrap(),
            Cell::Scalar(Value::Null)
        );
    }

    #[test]
    fn eval_comparaciones_null_promocion_y_tipos() {
        let row = fila_prueba();
        let eval = |e: ScalarExpr| eval_scalar(&e, &row).unwrap();
        let cmp = |op, l, r| ScalarExpr::Compare {
            op,
            left: Box::new(l),
            right: Box::new(r),
        };
        // Promoción numérica Int/Float.
        assert_eq!(
            eval(cmp(
                CompareOp::Eq,
                ScalarExpr::lit(Value::Int(1)),
                ScalarExpr::lit(Value::Float(1.0))
            )),
            Cell::Scalar(Value::Bool(true))
        );
        assert_eq!(
            eval(cmp(
                CompareOp::Lt,
                ScalarExpr::lit(Value::Int(1)),
                ScalarExpr::lit(Value::Float(1.5))
            )),
            Cell::Scalar(Value::Bool(true))
        );
        // Orden de cadenas (lexicográfico).
        assert_eq!(
            eval(cmp(
                CompareOp::Lt,
                ScalarExpr::lit(Value::String("Ana".into())),
                ScalarExpr::lit(Value::String("Bo".into()))
            )),
            Cell::Scalar(Value::Bool(true))
        );
        // NULL domina: comparar con NULL da NULL (no false).
        assert_eq!(
            eval(cmp(
                CompareOp::Eq,
                ScalarExpr::prop("p", "missing"),
                ScalarExpr::lit(Value::Int(1))
            )),
            Cell::Scalar(Value::Null)
        );
        assert_eq!(
            eval(cmp(
                CompareOp::Gt,
                ScalarExpr::prop("p", "missing"),
                ScalarExpr::lit(Value::Int(1))
            )),
            Cell::Scalar(Value::Null)
        );
        // Tipos distintos: no son iguales (= false), pero SIN orden (→ NULL).
        assert_eq!(
            eval(cmp(
                CompareOp::Eq,
                ScalarExpr::lit(Value::Int(1)),
                ScalarExpr::lit(Value::String("1".into()))
            )),
            Cell::Scalar(Value::Bool(false))
        );
        assert_eq!(
            eval(cmp(
                CompareOp::NotEq,
                ScalarExpr::lit(Value::Int(1)),
                ScalarExpr::lit(Value::String("1".into()))
            )),
            Cell::Scalar(Value::Bool(true))
        );
        assert_eq!(
            eval(cmp(
                CompareOp::Lt,
                ScalarExpr::lit(Value::Int(1)),
                ScalarExpr::lit(Value::String("a".into()))
            )),
            Cell::Scalar(Value::Null)
        );
        // Bool no se ordena.
        assert_eq!(
            eval(cmp(
                CompareOp::Lt,
                ScalarExpr::lit(Value::Bool(true)),
                ScalarExpr::lit(Value::Bool(false))
            )),
            Cell::Scalar(Value::Null)
        );
    }

    #[test]
    fn eval_igualdad_de_nodos_por_identidad() {
        let mut row = Row::new();
        row.bind("a", Cell::Node(Node::new(2, "Person")));
        row.bind("b", Cell::Node(Node::new(2, "Person")));
        row.bind("c", Cell::Node(Node::new(9, "Person")));
        // Mismo id ⇒ iguales aunque las copias sean objetos distintos.
        let a_b = ScalarExpr::eq(
            ScalarExpr::var("a", BindingKind::Node),
            ScalarExpr::var("b", BindingKind::Node),
        );
        assert_eq!(
            eval_scalar(&a_b, &row).unwrap(),
            Cell::Scalar(Value::Bool(true))
        );
        let a_c = ScalarExpr::eq(
            ScalarExpr::var("a", BindingKind::Node),
            ScalarExpr::var("c", BindingKind::Node),
        );
        assert_eq!(
            eval_scalar(&a_c, &row).unwrap(),
            Cell::Scalar(Value::Bool(false))
        );
    }

    #[test]
    fn eval_logica_trivalente_completa() {
        let row = fila_prueba();
        let t = || ScalarExpr::lit(Value::Bool(true));
        let f = || ScalarExpr::lit(Value::Bool(false));
        let n = || ScalarExpr::lit(Value::Null);
        let and = |l, r| ScalarExpr::And {
            left: Box::new(l),
            right: Box::new(r),
        };
        let or = |l, r| ScalarExpr::Or {
            left: Box::new(l),
            right: Box::new(r),
        };
        let eval = |e| match eval_scalar(&e, &row).unwrap() {
            Cell::Scalar(Value::Bool(b)) => Some(b),
            Cell::Scalar(Value::Null) => None,
            other => panic!("no booleano: {other:?}"),
        };
        // Tabla AND: F∧x=F, T∧T=T, N domina salvo con F.
        assert_eq!(eval(and(f(), t())), Some(false));
        assert_eq!(eval(and(t(), f())), Some(false));
        assert_eq!(eval(and(n(), f())), Some(false)); // N AND F = F
        assert_eq!(eval(and(f(), n())), Some(false)); // cortocircuito
        assert_eq!(eval(and(t(), t())), Some(true));
        assert_eq!(eval(and(n(), t())), None);
        assert_eq!(eval(and(t(), n())), None);
        assert_eq!(eval(and(n(), n())), None);
        // Tabla OR: T∨x=T, F∨F=F, N domina salvo con T.
        assert_eq!(eval(or(t(), f())), Some(true));
        assert_eq!(eval(or(f(), t())), Some(true));
        assert_eq!(eval(or(n(), t())), Some(true)); // N OR T = T
        assert_eq!(eval(or(t(), n())), Some(true)); // cortocircuito
        assert_eq!(eval(or(f(), f())), Some(false));
        assert_eq!(eval(or(n(), f())), None);
        assert_eq!(eval(or(f(), n())), None);
        // NOT: invierte; NOT NULL = NULL.
        assert_eq!(
            eval(ScalarExpr::Not {
                expr: Box::new(t())
            }),
            Some(false)
        );
        assert_eq!(
            eval(ScalarExpr::Not {
                expr: Box::new(n())
            }),
            None
        );
    }

    #[test]
    fn eval_cortocircuito_real() {
        // El cortocircuito es OBSERVABLE: el operando de la derecha sería un
        // error de tipos en ejecución (propiedad Int usada como booleano).
        // Con la izquierda resuelta a FALSE (AND) / TRUE (OR), la derecha
        // jamás se evalúa.
        let mut row = Row::new();
        row.bind(
            "p",
            Cell::Node(Node::new(0, "Person").with_prop("age", Value::Int(36))),
        );
        let edad = || ScalarExpr::prop("p", "age"); // INT: no booleano
        let falso = || ScalarExpr::lit(Value::Bool(false));
        let cierto = || ScalarExpr::lit(Value::Bool(true));
        let and = ScalarExpr::And {
            left: Box::new(falso()),
            right: Box::new(edad()),
        };
        assert_eq!(
            eval_scalar(&and, &row).unwrap(),
            Cell::Scalar(Value::Bool(false))
        );
        let or = ScalarExpr::Or {
            left: Box::new(cierto()),
            right: Box::new(edad()),
        };
        assert_eq!(
            eval_scalar(&or, &row).unwrap(),
            Cell::Scalar(Value::Bool(true))
        );
        // Sin cortocircuito el error salta: TRUE AND p.age → TypeMismatch.
        let and_malo = ScalarExpr::And {
            left: Box::new(cierto()),
            right: Box::new(edad()),
        };
        assert!(matches!(
            eval_scalar(&and_malo, &row),
            Err(ExecError::TypeMismatch { context, expected, got })
                if context == "operando de AND" && expected == "BOOL" && got == "INT"
        ));
    }

    #[test]
    fn eval_errores_defensivos() {
        let row = fila_prueba();
        // Variable sin ligar en la fila.
        assert!(matches!(
            eval_scalar(&ScalarExpr::var("x", BindingKind::Node), &row),
            Err(ExecError::UnboundVariable { variable }) if variable == "x"
        ));
        // Propiedad sobre un escalar: ligamos "s" a un Int a mano.
        let mut row2 = Row::new();
        row2.bind("s", Cell::Scalar(Value::Int(3)));
        assert!(matches!(
            eval_scalar(&ScalarExpr::prop("s", "k"), &row2),
            Err(ExecError::PropertyOnScalar { variable }) if variable == "s"
        ));
    }

    // ════════════════════════════════════════════════════════════════
    //  Operadores — NodeScan, IndexSeek
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn nodescan_todos_con_label_y_orden() {
        let store = grafo();
        let mut scan = NodeScanOp::new(&store, "p".into(), None);
        let rows = drenar(&mut scan).unwrap();
        assert_eq!(rows.len(), 6); // 4 personas + 2 ciudades
        assert_eq!(scan.name(), "NodeScan");
        assert_eq!(scan.rows_produced(), 6);

        let mut personas = NodeScanOp::new(&store, "p".into(), Some("Person".into()));
        let rows = drenar(&mut personas).unwrap();
        assert_eq!(rows.len(), 4);
        // Orden del store (inserción por id): la primera es Ana (id 0).
        assert!(matches!(&rows[0].get("p"), Some(Cell::Node(n)) if n.id == 0));

        let mut ciudades = NodeScanOp::new(&store, "c".into(), Some("City".into()));
        assert_eq!(drenar(&mut ciudades).unwrap().len(), 2);

        // Label inexistente: pipeline vacío (no error).
        let mut zombies = NodeScanOp::new(&store, "z".into(), Some("Zombie".into()));
        assert_eq!(drenar(&mut zombies).unwrap().len(), 0);
    }

    #[test]
    fn nodescan_ciclo_open_close_reopen() {
        let store = grafo();
        let mut scan = NodeScanOp::new(&store, "p".into(), Some("Person".into()));
        // Primera pasada.
        assert_eq!(drenar(&mut scan).unwrap().len(), 4);
        // close ya se llamó dentro de drenar; re-open reinicia limpio.
        assert_eq!(drenar(&mut scan).unwrap().len(), 4);
        assert_eq!(scan.rows_produced(), 4);
        // next tras close sin open: agotado, sin pánico.
        assert!(scan.next().unwrap().is_none());
    }

    #[test]
    fn indexseek_ids_exactos_y_error_stale() {
        let store = grafo();
        let mut seek = IndexSeekOp::new(&store, "p".into(), vec![0, 2]);
        let rows = drenar(&mut seek).unwrap();
        assert_eq!(rows.len(), 2);
        let ids: Vec<usize> = rows
            .iter()
            .map(|r| match r.get("p") {
                Some(Cell::Node(n)) => n.id,
                other => panic!("no nodo: {other:?}"),
            })
            .collect();
        assert_eq!(ids, vec![0, 2]); // Ana y Carla, sin escanear nada más
        assert_eq!(seek.name(), "IndexSeek");

        // Un índice desactualizado que apunta a un id inexistente: error.
        let mut stale = IndexSeekOp::new(&store, "p".into(), vec![99]);
        assert!(matches!(
            drenar(&mut stale),
            Err(ExecError::UnknownNode(99))
        ));
    }

    // ════════════════════════════════════════════════════════════════
    //  Operadores — Expand (direcciones, tipos, self-loops)
    // ════════════════════════════════════════════════════════════════

    /// Compila el SUBPLAN bajo el Project raíz (los operadores internos).
    fn compilar_interno<'a>(src: &str, store: &'a MemoryStore) -> Box<dyn PhysicalOperator + 'a> {
        let plan = parse(src).unwrap().lower().unwrap();
        let LogicalPlan::Project { input, .. } = plan else {
            panic!("la raíz siempre es Project");
        };
        compile(&input, store).unwrap()
    }

    #[test]
    fn expand_outgoing_filtra_por_tipo() {
        let store = grafo();
        // (p)-[:KNOWS]->(f): KNOWS salientes (LIVES_IN queda fuera).
        let mut op = compilar_interno("MATCH (p:Person)-[:KNOWS]->(f) RETURN f", &store);
        let rows = drenar(op.as_mut()).unwrap();
        // Ana→Bo, Bo→Carla, Carla→Ana, Dani→Dani = 4 filas.
        assert_eq!(rows.len(), 4);
        assert!(
            rows.iter()
                .all(|r| r.get("p").is_some() && r.get("f").is_some())
        );

        // Sin tipo de relación (el wildcard es `-[]->`, no `:ANY`): las
        // salientes de TODOS los nodos: KNOWS(4) + LIVES_IN(2) = 6.
        let mut op = compilar_interno("MATCH (p)-[]->(f) RETURN f", &store);
        assert_eq!(drenar(op.as_mut()).unwrap().len(), 6);
    }

    #[test]
    fn expand_direcciones_incoming_y_undirected() {
        let store = grafo();
        // INCOMING: (a)<-[:KNOWS]-(b) → b = source de las aristas que llegan a a.
        let mut op = compilar_interno("MATCH (a:Person)<-[:KNOWS]-(b:Person) RETURN b", &store);
        let rows = drenar(op.as_mut()).unwrap();
        // Ana recibe de Carla; Bo recibe de Ana; Carla recibe de Bo; Dani de sí.
        assert_eq!(rows.len(), 4);
        let b_de_ana: Vec<String> = rows
            .iter()
            .filter(|r| matches!(r.get("a"), Some(Cell::Node(n)) if n.id == 0))
            .map(|r| match r.get("b") {
                Some(Cell::Node(n)) => n.id.to_string(),
                _ => "?".into(),
            })
            .collect();
        assert_eq!(b_de_ana, vec!["2"]); // Carla

        // UNDIRECTED: out + in por cada persona; el self-loop de Dani UNA vez.
        // Ana: out[Bo] + in[Carla] = 2; Bo: 2; Carla: 2; Dani: self-loop = 1.
        let mut op = compilar_interno("MATCH (p:Person)-[:KNOWS]-(f:Person) RETURN p", &store);
        let rows = drenar(op.as_mut()).unwrap();
        assert_eq!(rows.len(), 7);
        let dani: Vec<&Row> = rows
            .iter()
            .filter(|r| matches!(r.get("p"), Some(Cell::Node(n)) if n.id == 3))
            .collect();
        assert_eq!(dani.len(), 1, "self-loop undirected cuenta una vez");
    }

    #[test]
    fn expand_liga_variable_de_relacion() {
        let store = grafo();
        let mut op = compilar_interno("MATCH (p:Person)-[r:KNOWS]->(f:Person) RETURN r", &store);
        let rows = drenar(op.as_mut()).unwrap();
        assert_eq!(rows.len(), 4);
        // r está ligada a la ARISTA completa (con sus props).
        assert!(
            matches!(&rows[0].get("r"), Some(Cell::Edge(e)) if e.id == 0 && e.props.contains_key("since"))
        );
        // El LIVES_IN nunca aparece como r.
        assert!(
            rows.iter()
                .all(|r| matches!(r.get("r"), Some(Cell::Edge(e)) if e.label == "KNOWS"))
        );
    }

    #[test]
    fn expand_error_si_from_no_es_nodo() {
        let store = grafo();
        // Plan a mano: Expand cuyo input liga `p`… a una arista (imposible
        // desde el binder; aquí forzamos el caso defensivo con un plan donde
        // `from` apunta a una variable inexistente).
        let plan = LogicalPlan::Expand {
            input: Box::new(LogicalPlan::NodeScan {
                variable: "x".into(),
                label: Some("Person".into()),
            }),
            from: "no_existe".into(),
            rel_variable: None,
            rel_type: None,
            direction: RelDirection::Outgoing,
            to: "f".into(),
        };
        let mut op = compile(&plan, &store).unwrap();
        assert!(matches!(
            drenar(op.as_mut()),
            Err(ExecError::UnboundVariable { variable }) if variable == "no_existe"
        ));
    }

    // ════════════════════════════════════════════════════════════════
    //  Operadores — Filter, Project, CartesianProduct
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn filter_pasa_true_y_descarta_false_y_null() {
        let store = grafo();
        // TRUE pasa: age > 33 → Ana(36), Bo(41), Dani(36).
        let mut op = compilar_interno("MATCH (p:Person) WHERE p.age > 33 RETURN p", &store);
        let rows = drenar(op.as_mut()).unwrap();
        assert_eq!(rows.len(), 3);
        // NULL se descarta: propiedad ausente comparada → NULL → fuera.
        let mut op = compilar_interno("MATCH (p:Person) WHERE p.missing > 33 RETURN p", &store);
        assert_eq!(drenar(op.as_mut()).unwrap().len(), 0);
    }

    #[test]
    fn filter_error_no_booleano_en_ejecucion() {
        let store = grafo();
        // p.age es Any para el plan (schemaless) pero INT en ejecución.
        let mut op = compilar_interno("MATCH (p:Person) WHERE p.age RETURN p", &store);
        assert!(matches!(
            drenar(op.as_mut()),
            Err(ExecError::TypeMismatch { context, expected, got })
                if context == "Filter (WHERE)" && expected == "BOOL" && got == "INT"
        ));
    }

    #[test]
    fn project_nombres_columnas_y_celdas() {
        let store = grafo();
        let rs = run("MATCH (p:Person) RETURN p.name AS nombre, p.age", &store).unwrap();
        assert_eq!(rs.columns, vec!["nombre".to_string(), "p.age".to_string()]);
        assert_eq!(rs.len(), 4);
        // Primera fila: Ana, 36.
        assert_eq!(rs.rows[0][0], Cell::Scalar(Value::String("Ana".into())));
        assert_eq!(rs.rows[0][1], Cell::Scalar(Value::Int(36)));
        // Propiedad ausente proyectada → NULL, no error.
        let rs = run("MATCH (p:Person {name: \"Bo\"}) RETURN p.missing", &store).unwrap();
        assert_eq!(rs.rows, vec![vec![Cell::Scalar(Value::Null)]]);
    }

    #[test]
    fn cartesian_product_materializa_y_cruza() {
        let store = grafo();
        let rs = run("MATCH (p:Person), (c:City) RETURN p.name, c.name", &store).unwrap();
        // 4 personas × 2 ciudades = 8 filas con TODAS las combinaciones.
        assert_eq!(rs.len(), 8);
        let combos: Vec<String> = rs
            .rows
            .iter()
            .map(|r| format!("{}/{}", r[0], r[1]))
            .collect();
        assert!(combos.contains(&"\"Ana\"/\"Madrid\"".to_string()));
        assert!(combos.contains(&"\"Dani\"/\"Lisboa\"".to_string()));
    }

    // ════════════════════════════════════════════════════════════════
    //  Operadores — Limit y Distinct
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn limit_corta_el_pipeline() {
        let store = grafo();
        let plan = parse("MATCH (p:Person) RETURN p.name")
            .unwrap()
            .lower()
            .unwrap();
        let scan = compile(&plan, &store).unwrap();
        let mut limit = LimitOp::new(scan, 2);
        let rows = drenar(&mut limit).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(limit.rows_produced(), 2);
        // Las métricas del árbol siguen viendo el scan COMPLETO pedido:
        // NodeScan produjo 2 porque el pull se cortó de raíz — esa es la
        // ventaja del modelo Volcano.
        let metricas = limit.collect_metrics();
        assert_eq!(metricas[0], ("Limit", 2));
        assert_eq!(metricas[1], ("Project", 2));
        assert_eq!(metricas[2], ("NodeScan", 2));

        // Límite mayor que las filas disponibles: no error.
        let scan = compile(&plan, &store).unwrap();
        let mut limit = LimitOp::new(scan, 100);
        assert_eq!(drenar(&mut limit).unwrap().len(), 4);
    }

    #[test]
    fn distinct_deduplica_filas() {
        let store = grafo();
        let plan = parse("MATCH (p:Person) RETURN p.age")
            .unwrap()
            .lower()
            .unwrap();
        let project = compile(&plan, &store).unwrap();
        let mut distinct = DistinctOp::new(project);
        let rows = drenar(&mut distinct).unwrap();
        // Edades: 36, 41, 29, 36 → tres distintas.
        assert_eq!(rows.len(), 3);
        let edades: Vec<String> = rows
            .iter()
            .map(|r| r.get("p.age").unwrap().to_string())
            .collect();
        assert!(edades.contains(&"36".to_string()));
        assert!(edades.contains(&"41".to_string()));
        assert!(edades.contains(&"29".to_string()));
        // Idempotente: re-open tras close vuelve a empezar de cero.
        assert_eq!(drenar(&mut distinct).unwrap().len(), 3);
    }

    // ════════════════════════════════════════════════════════════════
    //  Hito: consultas de extremo a extremo (parse → lower → execute)
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn hito_consulta_canonica_del_brief() {
        let store = grafo();
        let rs = run(
            "MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE p.name = \"Ana\" RETURN f.name",
            &store,
        )
        .unwrap();
        assert_eq!(rs.columns, vec!["f.name".to_string()]);
        assert_eq!(texto(&rs, 0), vec!["\"Bo\"".to_string()]);
    }

    #[test]
    fn hito_match_solo_y_variedad_de_return() {
        let store = grafo();
        let rs = run("MATCH (p:Person) RETURN p.name", &store).unwrap();
        assert_eq!(
            texto(&rs, 0),
            vec!["\"Ana\"", "\"Bo\"", "\"Carla\"", "\"Dani\""]
        );
        // RETURN de la variable entera → celdas Node.
        let rs = run("MATCH (p:Person {name: \"Ana\"}) RETURN p", &store).unwrap();
        assert_eq!(rs.columns, vec!["p".to_string()]);
        assert!(matches!(&rs.rows[0][0], Cell::Node(n) if n.id == 0));
        // RETURN de la relación entera → celdas Edge.
        let rs = run(
            "MATCH (p:Person)-[r:KNOWS]->(f:Person) WHERE p.name = \"Ana\" RETURN r",
            &store,
        )
        .unwrap();
        assert!(matches!(&rs.rows[0][0], Cell::Edge(e) if e.id == 0));
    }

    #[test]
    fn hito_where_and_or_not() {
        let store = grafo();
        // AND: age > 30 AND name <> "Bo" → Ana(36), Dani(36).
        let rs = run(
            "MATCH (p:Person) WHERE p.age > 30 AND p.name <> \"Bo\" RETURN p.name",
            &store,
        )
        .unwrap();
        assert_eq!(texto(&rs, 0), vec!["\"Ana\"", "\"Dani\""]);
        // OR: Ana o Carla.
        let rs = run(
            "MATCH (p:Person) WHERE p.name = \"Ana\" OR p.name = \"Carla\" RETURN p.name",
            &store,
        )
        .unwrap();
        assert_eq!(texto(&rs, 0), vec!["\"Ana\"", "\"Carla\""]);
        // NOT: !(age > 35) → Carla(29).
        let rs = run(
            "MATCH (p:Person) WHERE NOT p.age > 35 RETURN p.name",
            &store,
        )
        .unwrap();
        assert_eq!(texto(&rs, 0), vec!["\"Carla\""]);
    }

    #[test]
    fn hito_where_con_null_no_pasa_nada() {
        let store = grafo();
        // Nadie tiene la propiedad "nick": NULL = "x" es NULL → 0 filas.
        let rs = run(
            "MATCH (p:Person) WHERE p.nick = \"anita\" RETURN p.name",
            &store,
        )
        .unwrap();
        assert!(rs.is_empty());
        // ...y NOT NULL también es NULL (trivalente): sigue sin pasar.
        let rs = run(
            "MATCH (p:Person) WHERE NOT p.nick = \"anita\" RETURN p.name",
            &store,
        )
        .unwrap();
        assert!(rs.is_empty());
    }

    #[test]
    fn hito_props_inline_y_props_de_arista() {
        let store = grafo();
        let rs = run("MATCH (p:Person {name: \"Ana\"}) RETURN p.age", &store).unwrap();
        assert_eq!(rs.rows, vec![vec![Cell::Scalar(Value::Int(36))]]);
        // Propiedades de la ARISTA: r.since.
        let rs = run(
            "MATCH (p:Person)-[r:KNOWS]->(f:Person) WHERE r.since = 2020 RETURN f.name",
            &store,
        )
        .unwrap();
        assert_eq!(texto(&rs, 0), vec!["\"Bo\""]);
        // El self-loop (sin since) da NULL y queda fuera. Desde el cap. 21
        // run() pasa por el optimizador, que reordena esta cadena para
        // empezar por f (lado entrante): sin ORDER BY el ORDEN de las filas
        // no está garantizado, así que se compara ordenado.
        let rs = run(
            "MATCH (p:Person)-[r:KNOWS]->(f:Person) WHERE r.since > 2019 RETURN p.name",
            &store,
        )
        .unwrap();
        let mut nombres = texto(&rs, 0);
        nombres.sort();
        assert_eq!(nombres, vec!["\"Ana\"", "\"Bo\"", "\"Carla\""]);
    }

    #[test]
    fn hito_patrones_direccionales() {
        let store = grafo();
        // Entrante: ¿quién conoce a Ana? (a recibe de b)
        let rs = run(
            "MATCH (a:Person)<-[:KNOWS]-(b:Person) WHERE a.name = \"Ana\" RETURN b.name",
            &store,
        )
        .unwrap();
        assert_eq!(texto(&rs, 0), vec!["\"Carla\""]);
        // Sin dirección: conocidos de Ana en ambos sentidos → Bo y Carla.
        let rs = run(
            "MATCH (a:Person)-[:KNOWS]-(b:Person) WHERE a.name = \"Ana\" RETURN b.name",
            &store,
        )
        .unwrap();
        let mut nombres = texto(&rs, 0);
        nombres.sort();
        assert_eq!(nombres, vec!["\"Bo\"", "\"Carla\""]);
    }

    #[test]
    fn hito_camino_de_dos_tramos_con_anonimo_intermedio() {
        let store = grafo();
        // "los conocidos de mis conocidos": (a)-[:KNOWS]->()-[:KNOWS]->(c)
        let rs = run(
            "MATCH (a:Person)-[:KNOWS]->()-[:KNOWS]->(c:Person) RETURN c.name",
            &store,
        )
        .unwrap();
        // Ana→Bo→Carla, Bo→Carla→Ana, Carla→Ana→Bo, Dani→Dani→Dani.
        let mut nombres = texto(&rs, 0);
        nombres.sort();
        assert_eq!(nombres, vec!["\"Ana\"", "\"Bo\"", "\"Carla\"", "\"Dani\""]);
    }

    #[test]
    fn hito_self_loop_con_igualdad_de_nodos() {
        let store = grafo();
        // a = b compara IDENTIDAD de nodos: sólo el self-loop de Dani.
        let rs = run(
            "MATCH (a:Person)-[:KNOWS]->(b:Person) WHERE a = b RETURN a.name",
            &store,
        )
        .unwrap();
        assert_eq!(texto(&rs, 0), vec!["\"Dani\""]);
    }

    #[test]
    fn hito_label_inexistente_vacio() {
        let store = grafo();
        let rs = run("MATCH (z:Zombie) RETURN z.name", &store).unwrap();
        assert!(rs.is_empty());
        assert_eq!(rs.columns, vec!["z.name".to_string()]); // columnas intactas
    }

    #[test]
    fn hito_query_execute_coincide_con_run() {
        let store = grafo();
        let src = "MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE p.age < 40 RETURN f.name";
        let por_metodo = parse(src).unwrap().execute(&store).unwrap();
        let por_funcion = run(src, &store).unwrap();
        assert_eq!(por_metodo, por_funcion);
        // age < 40: Ana(36)→Bo, Carla(29)→Ana, Dani(36)→Dani (Bo, 41, fuera).
        let mut nombres = texto(&por_funcion, 0);
        nombres.sort();
        assert_eq!(nombres, vec!["\"Ana\"", "\"Bo\"", "\"Dani\""]);
    }

    #[test]
    fn hito_errores_parse_plan_y_runtime() {
        let store = grafo();
        // Error de sintaxis (cap 18) envuelto.
        let err = run("MATCHH (p:Person) RETURN p", &store).unwrap_err();
        assert!(matches!(err, ExecError::Parse(_)));
        assert!(err.to_string().starts_with("error de sintaxis:"));
        // Error de plan (cap 19) envuelto.
        let err = run("MATCH (p:Person) WHERE q.x = 1 RETURN p", &store).unwrap_err();
        assert!(matches!(
            &err,
            ExecError::Plan(PlanError { kind: PlanErrorKind::UnknownVariable { variable }, .. })
                if variable == "q"
        ));
        // Error en EJECUCIÓN: p.age (INT) como predicado.
        let err = run("MATCH (p:Person) WHERE p.age RETURN p.name", &store).unwrap_err();
        assert!(matches!(
            &err,
            ExecError::TypeMismatch { context, expected, got }
                if *context == "Filter (WHERE)" && *expected == "BOOL" && got == "INT"
        ));
        // std::error::Error: source() existe en los envueltos, no en los puros.
        let envuelto: Box<dyn std::error::Error> = Box::new(run("no-match", &store).unwrap_err());
        assert!(envuelto.source().is_some());
        let puro: Box<dyn std::error::Error> = Box::new(err);
        assert!(puro.source().is_none());
    }

    #[test]
    fn exec_error_display_y_from() {
        let plan_err = parse("MATCH (p:Person) WHERE q.x = 1 RETURN p")
            .unwrap()
            .lower()
            .unwrap_err();
        let err: ExecError = plan_err.into();
        assert!(err.to_string().contains("variable 'q'"));
        let parse_err = parse("no-match").unwrap_err();
        let err: ExecError = parse_err.into();
        assert!(err.to_string().contains("error de sintaxis"));
        // Variantes propias.
        let e = ExecError::UnknownNode(7);
        assert!(e.to_string().contains("nodo 7"));
        let e = ExecError::NotAProjection;
        assert!(e.to_string().contains("Project"));
        let e = ExecError::NotANode {
            variable: "r".into(),
        };
        assert!(e.to_string().contains("'r'"));
    }

    #[test]
    fn executor_rechaza_plan_sin_project_raiz() {
        let store = grafo();
        let plan = LogicalPlan::NodeScan {
            variable: "p".into(),
            label: Some("Person".into()),
        };
        assert!(matches!(
            Executor::new(&plan, &store),
            Err(ExecError::NotAProjection)
        ));
    }

    // ════════════════════════════════════════════════════════════════
    //  Métricas (semilla del explain del cap 21) y ResultSet
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn executor_metricas_por_operador() {
        let store = grafo();
        let plan =
            parse("MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE p.name = \"Ana\" RETURN f.name")
                .unwrap()
                .lower()
                .unwrap();
        let mut exec = Executor::new(&plan, &store).unwrap();
        let rs = exec.execute().unwrap();
        assert_eq!(rs.len(), 1);
        let m = exec.metrics();
        // Pre-orden raíz→hojas: Project, Filter, Expand, NodeScan.
        assert_eq!(
            m.per_operator,
            vec![
                ("Project", 1),
                ("Filter", 1),
                ("Expand", 4),
                ("NodeScan", 4),
            ]
        );
        assert_eq!(m.rows_returned, 1);
        let texto = m.to_string();
        assert!(texto.contains("NodeScan: 4 filas"), "{texto}");
        assert!(texto.contains("filas devueltas: 1"), "{texto}");
    }

    #[test]
    fn executor_metricas_del_cartesiano() {
        let store = grafo();
        let plan = parse("MATCH (p:Person), (c:City) RETURN p.name, c.name")
            .unwrap()
            .lower()
            .unwrap();
        let mut exec = Executor::new(&plan, &store).unwrap();
        let rs = exec.execute().unwrap();
        assert_eq!(rs.len(), 8);
        let m = exec.metrics();
        // CartesianProduct produce 8 (4×2); cada scan produce lo suyo.
        assert_eq!(
            m.per_operator,
            vec![
                ("Project", 8),
                ("CartesianProduct", 8),
                ("NodeScan", 4),
                ("NodeScan", 2),
            ]
        );
    }

    #[test]
    fn result_set_display_tabla_y_column() {
        let store = grafo();
        let rs = run(
            "MATCH (p:Person) WHERE p.age > 35 RETURN p.name AS nombre, p.age",
            &store,
        )
        .unwrap();
        let tabla = rs.to_string();
        // Anchos por columna: max(cabecera, celdas) + separador " | ".
        // col0: "nombre"(6) vs "Ana"(5)/"Bo"(4)/"Dani"(6) → 6; col1: 5.
        let lineas: Vec<String> = tabla.lines().map(|l| l.trim_end().to_string()).collect();
        assert_eq!(lineas[0], "nombre | p.age");
        assert_eq!(lineas[1], "\"Ana\"  | 36");
        assert_eq!(lineas[2], "\"Bo\"   | 41");
        assert_eq!(lineas[3], "\"Dani\" | 36");
        // Búsqueda de columna por nombre.
        assert_eq!(rs.column("nombre"), Some(0));
        assert_eq!(rs.column("p.age"), Some(1));
        assert_eq!(rs.column("no-existe"), None);
        assert_eq!(rs.len(), 3);
        assert!(!rs.is_empty());
    }
}
