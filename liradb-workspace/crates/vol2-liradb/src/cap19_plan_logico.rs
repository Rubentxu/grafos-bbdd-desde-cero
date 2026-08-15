use crate::cap07_modelo::{NodeId, Value};
use crate::cap17_liraql_ast::{
    CompareOp, Expression, NodePattern, PathPattern, Query, RelDirection, Span, display_value,
};
use crate::cap18_lexer_parser::write_span_suffix;

// ─────────────────── Cap 19: Del AST al plan lógico ───────────────────
//
// Los caps. 17-18 completaron la cadena `texto → tokens → AST` (`parse()`).
// Este capítulo da el paso siguiente: convertir el AST en un **plan lógico**,
// un árbol de operadores que declara *qué* hay que calcular sin decidir aún
// *cómo* ejecutarlo (ese es el motor Volcano del cap. 20; el *cómo óptimo*
// es el optimizador del cap. 21).
//
// ```text
//   "MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE p.name = \"Ana\" RETURN f.name"
//        │   parse() (cap 18)  │    lower() (este cap)    │  executor (cap 20)
//        └──► Query (AST) ─────┴──►  LogicalPlan ─────────┴──► filas de resultado
// ```
//
// El plan del ejemplo (pretty-printer de este cap., base de `liradb explain`):
//
// ```text
//   Project(f.name)
//     Filter(f:Person AND p.name = "Ana")
//       Expand(p, KNOWS, OUTGOING, f)
//         NodeScan(Person AS p)
//   ```
//
// Nota sobre el brief: su plan de ejemplo omitía imponer la etiqueta del nodo
// destino (`f:Person`). Aquí esa restricción baja como predicado
// `ScalarExpr::HasLabel` dentro del `Filter`; sin ella la consulta devolvería
// conocidos con CUALQUIER etiqueta, no sólo `Person`.
//
// Responsabilidades del capítulo (brief §cap 19):
//   1. **Operadores**: `NodeScan`, `Expand`, `Filter`, `Project` y
//      `CartesianProduct` (patrones disjuntos separados por coma).
//   2. **Variables**: la tabla de bindings (`Bindings`: nombre → nodo/arista)
//      que el *binder* rellena al recorrer los patrones. Responde a la
//      pregunta crítica del CORPUS: "cómo representar variables ligadas".
//   3. **Expresiones**: `ScalarExpr`, la versión *resuelta* de `Expression`
//      —sin spans ni nombres sin ligar: cada variable ya lleva su tipo de
//      binding y cada propiedad está verificada contra la tabla.
//   4. **Resolución de nombres**: toda variable usada en WHERE/RETURN debe
//      estar declarada en MATCH (`PlanErrorKind::UnknownVariable`).
//   5. **Validación semántica**: sin variables duplicadas, sin re-ligar una
//      variable dentro del mismo patrón, WHERE booleano.
//   6. **Inferencia de tipos básica**: `LogicalType` + `ScalarExpr::type_of`,
//      conservadora porque LiraDB es *schemaless* (las propiedades tipan a
//      `Any` y las comparaciones se resuelven en ejecución).
//
// Límites declarados (conscientes, material para los caps. 20-21):
//   - Un patrón no puede re-ligar una variable (`(a)-[:KNOWS]->(b)-[:X]->(a)`):
//     el *re-binding* es trabajo del executor, no del plan lógico.
//   - Patrones separados por coma que compartan variables exigen un join:
//     `PlanErrorKind::SharedPatternVariables`. Si son disjuntos, el plan es
//     un `CartesianProduct` correcto-pero-ingenuo (el optimizador del cap. 21
//     lo convertirá en join/expansión reordenada).
//   - El `Filter` queda arriba del árbol: el *push-down* de predicados es la
//     primera regla del optimizador del cap. 21, y este plan ingenuo es
//     exactamente el "antes" que aquel capítulo mejorará.

// ─── LogicalType: inferencia de tipos básica ───

/// Tipo lógico de una expresión del plan.
///
/// LiraDB es *schemaless* (cap 7): las propiedades no tienen tipo declarado,
/// así que un acceso a propiedad tipa a [`LogicalType::Any`] y las
/// comparaciones que lo involucran se aceptan (se resolverán en ejecución).
/// La inferencia de este capítulo es deliberadamente conservadora: sólo
/// rechaza lo que *seguro* está mal (p.ej. `WHERE 3` o `p = TRUE` con `p`
/// ligado a un nodo).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalType {
    /// Desconocido / polimórfico (propiedades sin esquema).
    Any,
    /// NULL literal.
    Null,
    /// Booleano.
    Bool,
    /// Entero de 64 bits.
    Int,
    /// Float de 64 bits.
    Float,
    /// Cadena UTF-8.
    String,
    /// Bytes opacos.
    Bytes,
    /// Un nodo del grafo (variable de nodo ligada).
    Node,
    /// Una arista del grafo (variable de relación ligada).
    Edge,
}

impl LogicalType {
    /// ¿Es un tipo numérico (ordenable)?
    pub fn is_numeric(self) -> bool {
        matches!(self, LogicalType::Int | LogicalType::Float)
    }

    /// ¿Comodín que compara con cualquier cosa? (`Any` = sin esquema;
    /// `Null` = comparación con NULL, que en ejecución da NULL/unknown).
    fn is_wildcard(self) -> bool {
        matches!(self, LogicalType::Any | LogicalType::Null)
    }

    /// ¿Dos tipos pueden compararse con `=` / `<>`?
    ///
    /// Reglas: iguales entre sí; `Any`/`Null` con cualquiera; numéricos
    /// cruzados (`Int` vs `Float` se promociona); cualquier otra combinación
    /// concreta (`Bool` vs `Int`, `Node` vs `String`, …) es error de tipos.
    pub fn eq_compatible(a: Self, b: Self) -> bool {
        a == b || a.is_wildcard() || b.is_wildcard() || (a.is_numeric() && b.is_numeric())
    }

    /// ¿Dos tipos pueden ordenarse (`<`, `<=`, `>`, `>=`)?
    ///
    /// Sólo numéricos entre sí o cadenas entre sí (orden lexicográfico).
    /// Booleanos, nodos y aristas NO son ordenables.
    pub fn order_compatible(a: Self, b: Self) -> bool {
        a.is_wildcard()
            || b.is_wildcard()
            || (a.is_numeric() && b.is_numeric())
            || (a == LogicalType::String && b == LogicalType::String)
    }
}

impl std::fmt::Display for LogicalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            LogicalType::Any => "ANY",
            LogicalType::Null => "NULL",
            LogicalType::Bool => "BOOL",
            LogicalType::Int => "INT",
            LogicalType::Float => "FLOAT",
            LogicalType::String => "STRING",
            LogicalType::Bytes => "BYTES",
            LogicalType::Node => "NODE",
            LogicalType::Edge => "EDGE",
        })
    }
}

// ─── Bindings: la tabla de variables ligadas ───

/// Qué clase de elemento del grafo liga una variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
    /// Variable de nodo: `(p:Person)`.
    Node,
    /// Variable de relación: `-[r:KNOWS]->`.
    Edge,
}

impl std::fmt::Display for BindingKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            BindingKind::Node => "NODE",
            BindingKind::Edge => "EDGE",
        })
    }
}

/// Tabla de variables ligadas por el MATCH: nombre → clase de binding.
///
/// Es el corazón del *binder* (la pregunta crítica del cap.19 en CORPUS:
/// "cómo representar variables ligadas"). Se rellena en orden de aparición
/// mientras se baja cada patrón, y consulta después para resolver WHERE y
/// RETURN. Un `Vec` ordenado (no un `HashMap`) mantiene el orden de ligadura
/// —determinista para tests, explain y el executor del cap. 20— con coste
/// O(n) aceptable en un lenguaje didáctico.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bindings {
    entries: Vec<(String, BindingKind)>,
}

impl Bindings {
    /// Tabla vacía.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Declara una variable. Error si ya estaba declarada (los duplicados son
    /// rechazados por el binder del cap 19; re-ligar es cosa del executor).
    pub fn declare(&mut self, name: &str, kind: BindingKind) -> Result<(), PlanErrorKind> {
        if self.contains(name) {
            return Err(PlanErrorKind::DuplicateVariable {
                variable: name.to_string(),
            });
        }
        self.entries.push((name.to_string(), kind));
        Ok(())
    }

    /// Clase de binding de una variable (`None` si no está ligada).
    pub fn get(&self, name: &str) -> Option<BindingKind> {
        self.entries
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, k)| *k)
    }

    /// ¿Está la variable ligada?
    pub fn contains(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// Número de variables ligadas.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// ¿Tabla vacía?
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterar las ligaduras en orden de declaración.
    pub fn iter(&self) -> impl Iterator<Item = (&str, BindingKind)> {
        self.entries.iter().map(|(n, k)| (n.as_str(), *k))
    }
}

impl std::fmt::Display for Bindings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("{")?;
        for (i, (name, kind)) in self.entries.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{name}:{kind}")?;
        }
        f.write_str("}")
    }
}

// ─── ScalarExpr: expresiones resueltas (sin Span) ───

/// Expresión del plan lógico: el `Expression` del AST *ya resuelto*.
///
/// Diferencias con `Expression` (cap 17):
/// - No lleva `Span`: ya no apunta al fuente; los errores de plan usan el
///   span de la cláusula que originó la expresión.
/// - `Variable` se convierte en `Var { name, kind }`: el binder *incrusta*
///   la clase de binding en el propio nodo, para que el executor (cap. 20)
///   nunca tenga que re-resolver nombres.
/// - Aparece `HasLabel`: la etiqueta de un nodo que no es el inicial del
///   patrón baja como predicado (`f:Person`) en el `Filter`. No existe en la
///   sintaxis como expresión autónoma —la construye el planner.
#[derive(Debug, Clone, PartialEq)]
pub enum ScalarExpr {
    /// Literal del lenguaje (reutiliza `Value` del cap 7).
    Literal(Value),
    /// Variable ligada, con su clase de binding incrustada.
    Var { name: String, kind: BindingKind },
    /// Acceso a propiedad de una variable ligada: `p.name`.
    Property { variable: String, property: String },
    /// Predicado de etiqueta construido por el planner: `f:Person`.
    HasLabel { variable: String, label: String },
    /// Comparación binaria: `left op right`.
    Compare {
        op: CompareOp,
        left: Box<ScalarExpr>,
        right: Box<ScalarExpr>,
    },
    /// `a AND b` (cortocircuito en el executor del cap. 20).
    And {
        left: Box<ScalarExpr>,
        right: Box<ScalarExpr>,
    },
    /// `a OR b`.
    Or {
        left: Box<ScalarExpr>,
        right: Box<ScalarExpr>,
    },
    /// `NOT a`.
    Not { expr: Box<ScalarExpr> },
}

impl ScalarExpr {
    /// Constructor ergonómico para literales.
    pub fn lit(value: Value) -> Self {
        ScalarExpr::Literal(value)
    }

    /// Constructor ergonómico para variables ligadas.
    pub fn var(name: impl Into<String>, kind: BindingKind) -> Self {
        ScalarExpr::Var {
            name: name.into(),
            kind,
        }
    }

    /// Constructor ergonómico para accesos a propiedad.
    pub fn prop(variable: impl Into<String>, property: impl Into<String>) -> Self {
        ScalarExpr::Property {
            variable: variable.into(),
            property: property.into(),
        }
    }

    /// Constructor ergonómico para predicados de etiqueta.
    pub fn has_label(variable: impl Into<String>, label: impl Into<String>) -> Self {
        ScalarExpr::HasLabel {
            variable: variable.into(),
            label: label.into(),
        }
    }

    /// Constructor ergonómico para igualdades (predicados inline de patrón).
    pub fn eq(left: ScalarExpr, right: ScalarExpr) -> Self {
        ScalarExpr::Compare {
            op: CompareOp::Eq,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Conjunción left-asociativa de una lista de predicados.
    ///
    /// `None` si la lista está vacía (sin predicados no hay `Filter`).
    /// `[a, b, c]` → `And(And(a, b), c)`.
    pub fn and_all(predicates: Vec<ScalarExpr>) -> Option<Self> {
        let mut iter = predicates.into_iter();
        let first = iter.next()?;
        Some(iter.fold(first, |acc, e| ScalarExpr::And {
            left: Box::new(acc),
            right: Box::new(e),
        }))
    }

    /// Infiere el tipo de la expresión (brief §cap 19: "inferencia de tipos
    /// básica"). Devuelve `Err` ante variables sin ligar o tipos incompatibles.
    ///
    /// Conservadora: las propiedades tipan `Any` (schemaless) y `Any` es
    /// compatible con todo; sólo rechaza lo que *seguro* está mal.
    pub fn type_of(&self, bindings: &Bindings) -> Result<LogicalType, PlanErrorKind> {
        match self {
            ScalarExpr::Literal(value) => Ok(match value {
                Value::Null => LogicalType::Null,
                Value::Bool(_) => LogicalType::Bool,
                Value::Int(_) => LogicalType::Int,
                Value::Float(_) => LogicalType::Float,
                Value::String(_) => LogicalType::String,
                Value::Bytes(_) => LogicalType::Bytes,
            }),
            ScalarExpr::Var { kind, .. } => Ok(match kind {
                BindingKind::Node => LogicalType::Node,
                BindingKind::Edge => LogicalType::Edge,
            }),
            ScalarExpr::Property { variable, .. } => {
                // Defensivo: el binder ya verificó la variable, pero el
                // método es público y una ScalarExpr construida a mano
                // podría referenciar una variable sin ligar.
                if !bindings.contains(variable) {
                    return Err(PlanErrorKind::UnknownVariable {
                        variable: variable.clone(),
                    });
                }
                Ok(LogicalType::Any)
            }
            ScalarExpr::HasLabel { variable, .. } => {
                if !bindings.contains(variable) {
                    return Err(PlanErrorKind::UnknownVariable {
                        variable: variable.clone(),
                    });
                }
                Ok(LogicalType::Bool)
            }
            ScalarExpr::Compare { op, left, right } => {
                let lt = left.type_of(bindings)?;
                let rt = right.type_of(bindings)?;
                let (context, ok) = match op {
                    CompareOp::Eq | CompareOp::NotEq => (
                        "comparación de igualdad",
                        LogicalType::eq_compatible(lt, rt),
                    ),
                    CompareOp::Lt | CompareOp::Lte | CompareOp::Gt | CompareOp::Gte => (
                        "comparación de orden",
                        LogicalType::order_compatible(lt, rt),
                    ),
                };
                if ok {
                    Ok(LogicalType::Bool)
                } else {
                    Err(PlanErrorKind::TypeMismatch {
                        context,
                        expected: rt,
                        got: lt,
                    })
                }
            }
            ScalarExpr::And { left, right } => {
                Self::expect_bool(left, bindings, "operando de AND")?;
                Self::expect_bool(right, bindings, "operando de AND")?;
                Ok(LogicalType::Bool)
            }
            ScalarExpr::Or { left, right } => {
                Self::expect_bool(left, bindings, "operando de OR")?;
                Self::expect_bool(right, bindings, "operando de OR")?;
                Ok(LogicalType::Bool)
            }
            ScalarExpr::Not { expr } => {
                Self::expect_bool(expr, bindings, "operando de NOT")?;
                Ok(LogicalType::Bool)
            }
        }
    }

    /// Un operando lógico debe ser `Bool` (o `Any`, que se resuelve en
    /// ejecución). Cualquier otra cosa concreta es error de tipos.
    fn expect_bool(
        expr: &ScalarExpr,
        bindings: &Bindings,
        context: &'static str,
    ) -> Result<(), PlanErrorKind> {
        let ty = expr.type_of(bindings)?;
        if ty == LogicalType::Bool || ty == LogicalType::Any {
            Ok(())
        } else {
            Err(PlanErrorKind::TypeMismatch {
                context,
                expected: LogicalType::Bool,
                got: ty,
            })
        }
    }
}

/// Contexto de precedencia para el pretty-printer de `ScalarExpr`:
/// decide si un operador lógico necesita paréntesis según dónde está anidado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExprCtx {
    /// Raíz del predicado: nunca paréntesis.
    Top,
    /// Operando de un `AND`.
    And,
    /// Operando de un `OR`.
    Or,
    /// Operando de un `NOT`.
    Not,
}

impl std::fmt::Display for ScalarExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.write_expr(f, ExprCtx::Top)
    }
}

impl ScalarExpr {
    /// Escritura con paréntesis mínimos según precedencia NOT > AND > OR.
    ///
    /// Un `OR` se envuelve dentro de `AND` y `NOT` (liga menos que ambos);
    /// un `AND` se envuelve dentro de `OR` y `NOT`. Comparaciones y hojas
    /// nunca necesitan paréntesis: la gramática del cap. 17 sólo permite
    /// comparar primarias.
    fn write_expr(&self, f: &mut std::fmt::Formatter<'_>, ctx: ExprCtx) -> std::fmt::Result {
        let wrap = match self {
            ScalarExpr::Or { .. } => matches!(ctx, ExprCtx::And | ExprCtx::Not),
            ScalarExpr::And { .. } => matches!(ctx, ExprCtx::Or | ExprCtx::Not),
            _ => false,
        };
        if wrap {
            f.write_str("(")?;
        }
        match self {
            ScalarExpr::Literal(value) => display_value(f, value)?,
            ScalarExpr::Var { name, .. } => f.write_str(name)?,
            ScalarExpr::Property { variable, property } => write!(f, "{variable}.{property}")?,
            ScalarExpr::HasLabel { variable, label } => write!(f, "{variable}:{label}")?,
            ScalarExpr::Compare { op, left, right } => {
                left.write_expr(f, ExprCtx::Top)?;
                write!(f, " {op} ")?;
                right.write_expr(f, ExprCtx::Top)?;
            }
            ScalarExpr::And { left, right } => {
                left.write_expr(f, ExprCtx::And)?;
                f.write_str(" AND ")?;
                right.write_expr(f, ExprCtx::And)?;
            }
            ScalarExpr::Or { left, right } => {
                left.write_expr(f, ExprCtx::Or)?;
                f.write_str(" OR ")?;
                right.write_expr(f, ExprCtx::Or)?;
            }
            ScalarExpr::Not { expr } => {
                f.write_str("NOT ")?;
                expr.write_expr(f, ExprCtx::Not)?;
            }
        }
        if wrap {
            f.write_str(")")?;
        }
        Ok(())
    }
}

// ─── Operadores del plan lógico ───

/// Una proyección del RETURN: expresión + alias opcional.
#[derive(Debug, Clone, PartialEq)]
pub struct Projection {
    /// Expresión resuelta a proyectar.
    pub expr: ScalarExpr,
    /// Alias explícito (`AS nombre`). `None` = nombre derivado.
    pub alias: Option<String>,
}

impl Projection {
    /// Nombre de la columna de salida: el alias si existe; si no, se deriva
    /// de la expresión (`p.name` → "p.name", `p` → "p", resto → texto
    /// canónico de la expresión).
    pub fn output_name(&self) -> String {
        if let Some(alias) = &self.alias {
            return alias.clone();
        }
        match &self.expr {
            ScalarExpr::Var { name, .. } => name.clone(),
            ScalarExpr::Property { variable, property } => format!("{variable}.{property}"),
            other => other.to_string(),
        }
    }
}

impl std::fmt::Display for Projection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.alias {
            Some(alias) => write!(f, "{} AS {alias}", self.expr),
            None => write!(f, "{}", self.expr),
        }
    }
}

/// Árbol de operadores lógicos: *qué* calcular, sin *cómo*.
///
/// Diseñado para que el executor Volcano del cap. 20 lo consuma operador a
/// operador (cada variante es un `next()` potencial) y para que el
/// optimizador del cap. 21 lo reescriba (el `Filter` encima de todo es el
/// "antes" del push-down de predicados).
///
/// Los hijos van en `Box` dentro de cada variante —árbol inmutable y sin
/// magia: lo que el `Display` dibuja es exactamente la estructura.
#[derive(Debug, Clone, PartialEq)]
pub enum LogicalPlan {
    /// Escaneo de nodos: liga `variable` a cada nodo con `label` (todos si
    /// `None`). Es siempre la hoja izquierda de un patrón de camino.
    NodeScan {
        variable: String,
        label: Option<String>,
    },
    /// Búsqueda por índice de igualdad: liga `variable` exactamente a los
    /// nodos cuyo `label` (todos si `None`) cumple `property = value`.
    ///
    /// La construye el optimizador del cap. 21 (regla `index_seek`): es la
    /// reescritura canónica del brief (`Filter(name = "Ana") + NodeScan` →
    /// `IndexSeek`). Los `ids` llegan YA resueltos del catálogo de
    /// estadísticas (un plan "semi-ligado", como los planes reales tras el
    /// binder); el Display los oculta y pinta el predicado original:
    /// `IndexSeek(Person.name = "Ana")`.
    IndexSeek {
        variable: String,
        label: Option<String>,
        property: String,
        value: Value,
        /// Nodos que satisfacen la igualdad, en orden del store.
        ids: Vec<NodeId>,
    },
    /// Expansión por adyacencia: dado un binding de `from`, recorre las
    /// aristas de tipo `rel_type` (todas si `None`) en `direction` y liga
    /// `to` (y `rel_variable` si el patrón la nombra) por cada arista.
    Expand {
        input: Box<LogicalPlan>,
        from: String,
        rel_variable: Option<String>,
        rel_type: Option<String>,
        direction: RelDirection,
        to: String,
    },
    /// Filtra los bindings que cumplen `predicate` (WHERE + predicados
    /// inline de los patrones, conjuntados con AND).
    Filter {
        input: Box<LogicalPlan>,
        predicate: ScalarExpr,
    },
    /// Proyección final del RETURN: una columna por `Projection`.
    Project {
        input: Box<LogicalPlan>,
        items: Vec<Projection>,
    },
    /// Producto cartesiano de dos sub-planes con variables disjuntas
    /// (patrones del MATCH separados por coma). Correcto pero ingenuo: el
    /// optimizador del cap. 21 lo reordenará.
    CartesianProduct {
        left: Box<LogicalPlan>,
        right: Box<LogicalPlan>,
    },
}

impl LogicalPlan {
    /// Variables ligadas por este sub-plan, en orden de ligadura y sin
    /// duplicados. El optimizador (cap. 21) lo usará para saber qué
    /// predicados puede empujar bajo cada operador.
    pub fn bound_variables(&self) -> Vec<String> {
        let mut out = Vec::new();
        self.collect_bound(&mut out);
        out
    }

    fn collect_bound(&self, out: &mut Vec<String>) {
        match self {
            LogicalPlan::NodeScan { variable, .. } | LogicalPlan::IndexSeek { variable, .. } => {
                push_unique(out, variable)
            }
            LogicalPlan::Expand {
                input,
                rel_variable,
                to,
                ..
            } => {
                input.collect_bound(out);
                if let Some(rv) = rel_variable {
                    push_unique(out, rv);
                }
                push_unique(out, to);
            }
            LogicalPlan::Filter { input, .. } | LogicalPlan::Project { input, .. } => {
                input.collect_bound(out);
            }
            LogicalPlan::CartesianProduct { left, right } => {
                left.collect_bound(out);
                right.collect_bound(out);
            }
        }
    }

    /// Escribe el árbol con indentación de 2 espacios por nivel.
    ///
    /// Formato idéntico al plan del brief (§cap 19) —es la base de la salida
    /// de `liradb explain` (cap. 21):
    ///
    /// ```text
    /// Project(f.name)
    ///   Filter(f:Person AND p.name = "Ana")
    ///     Expand(p, KNOWS, OUTGOING, f)
    ///       NodeScan(Person AS p)
    /// ```
    fn render(&self, depth: usize, out: &mut Vec<String>) {
        let pad = "  ".repeat(depth);
        match self {
            LogicalPlan::NodeScan { variable, label } => {
                out.push(format!(
                    "{pad}NodeScan({} AS {variable})",
                    label.as_deref().unwrap_or("ANY")
                ));
            }
            LogicalPlan::IndexSeek {
                label,
                property,
                value,
                ..
            } => {
                // El formato del brief §cap 21 (`IndexSeek(Person.name = "Ana")`);
                // sin etiqueta, la propiedad a secas. Los ids resueltos no se
                // pintan: el plan muestra el PREDICADO, no su resolución.
                let target = match label.as_deref() {
                    Some(label) => format!("{label}.{property}"),
                    None => property.clone(),
                };
                out.push(format!(
                    "{pad}IndexSeek({target} = {})",
                    ScalarExpr::lit(value.clone())
                ));
            }
            LogicalPlan::Expand {
                input,
                from,
                rel_variable,
                rel_type,
                direction,
                to,
            } => {
                // El tramo de relación se pinta como en Cypher: `r:KNOWS`,
                // sólo `KNOWS`, sólo `r`, o `ANY` si el patrón no restringe.
                let rel = match (rel_variable.as_deref(), rel_type.as_deref()) {
                    (Some(v), Some(t)) => format!("{v}:{t}"),
                    (Some(v), None) => v.to_string(),
                    (None, Some(t)) => t.to_string(),
                    (None, None) => "ANY".to_string(),
                };
                out.push(format!("{pad}Expand({from}, {rel}, {direction}, {to})"));
                input.render(depth + 1, out);
            }
            LogicalPlan::Filter { input, predicate } => {
                out.push(format!("{pad}Filter({predicate})"));
                input.render(depth + 1, out);
            }
            LogicalPlan::Project { input, items } => {
                let cols: Vec<String> = items.iter().map(|p| p.to_string()).collect();
                out.push(format!("{pad}Project({})", cols.join(", ")));
                input.render(depth + 1, out);
            }
            LogicalPlan::CartesianProduct { left, right } => {
                out.push(format!("{pad}CartesianProduct"));
                left.render(depth + 1, out);
                right.render(depth + 1, out);
            }
        }
    }
}

/// Añade sin duplicar (los árboles de plan pueden citar una variable en
/// varios operadores; la lista de bindings es un conjunto ordenado).
fn push_unique(out: &mut Vec<String>, name: &str) {
    if !out.iter().any(|v| v == name) {
        out.push(name.to_string());
    }
}

impl std::fmt::Display for LogicalPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut lines = Vec::new();
        self.render(0, &mut lines);
        f.write_str(&lines.join("\n"))
    }
}

// ─── PlanError: fallos de binding / validación semántica ───

/// Sub-tipo de error del planner (cap 19).
///
/// Cada variante describe un fallo de *binding* o de validación semántica
/// detectado al convertir el AST en plan lógico. El [`Span`] acompañante en
/// [`PlanError`] localiza la cláusula culpable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanErrorKind {
    /// El MATCH no tiene patrones (AST construido a mano; `parse()` lo impide).
    EmptyMatch,
    /// El RETURN no tiene items (AST construido a mano; `parse()` lo impide).
    EmptyReturn,
    /// Variable usada en WHERE/RETURN (o en un predicado inline) que el MATCH
    /// no liga.
    UnknownVariable { variable: String },
    /// Variable declarada dos veces en el MATCH (nodos o relaciones).
    DuplicateVariable { variable: String },
    /// Variable que se liga dos veces dentro del mismo patrón
    /// (`(a)-[:X]->(a)`): el re-binding es trabajo del executor, no del
    /// plan lógico.
    VariableRebind { variable: String },
    /// Patrones separados por coma que comparten variables: exigen un join,
    /// que este plan lógico no planifica (cap. 20-21).
    SharedPatternVariables { variables: Vec<String> },
    /// Tipos incompatibles en `context`: p.ej. `WHERE 3` (se esperaba BOOL,
    /// se obtuvo INT) o `p = TRUE` con `p` ligado a nodo.
    TypeMismatch {
        context: &'static str,
        expected: LogicalType,
        got: LogicalType,
    },
}

/// Error de planificación con posición (el mismo patrón `{ kind, span }` de
/// `QueryError` y `ParseError`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanError {
    pub kind: PlanErrorKind,
    pub span: Span,
}

impl PlanError {
    pub fn new(kind: PlanErrorKind, span: Span) -> Self {
        Self { kind, span }
    }
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            PlanErrorKind::EmptyMatch => {
                f.write_str("MATCH vacío: no hay patrones que planificar")?
            }
            PlanErrorKind::EmptyReturn => {
                f.write_str("RETURN vacío: no hay proyecciones que planificar")?
            }
            PlanErrorKind::UnknownVariable { variable } => {
                write!(f, "variable '{variable}' usada pero no ligada por el MATCH")?
            }
            PlanErrorKind::DuplicateVariable { variable } => {
                write!(f, "variable '{variable}' declarada dos veces en el MATCH")?
            }
            PlanErrorKind::VariableRebind { variable } => write!(
                f,
                "variable '{variable}' se liga dos veces dentro del mismo patrón \
                 (el re-binding se resuelve en ejecución, no en el plan lógico)"
            )?,
            PlanErrorKind::SharedPatternVariables { variables } => write!(
                f,
                "los patrones separados por coma comparten las variables [{}]; \
                 el join entre patrones llega con el optimizador (cap. 21)",
                variables.join(", ")
            )?,
            PlanErrorKind::TypeMismatch {
                context,
                expected,
                got,
            } => write!(
                f,
                "tipos incompatibles en {context}: se esperaba {expected}, se obtuvo {got}"
            )?,
        }
        write_span_suffix(f, self.span)
    }
}

impl std::error::Error for PlanError {}

// ─── Planner: binder + lowering (AST → LogicalPlan) ───

/// Estado del *binder* mientras baja el MATCH: la tabla de ligaduras y el
/// contador de variables internas para nodos/relaciones anónimos.
struct Planner {
    bindings: Bindings,
    next_internal: u32,
}

impl Planner {
    fn new() -> Self {
        Self {
            bindings: Bindings::new(),
            next_internal: 0,
        }
    }

    /// Genera un nombre interno para un elemento anónimo del patrón
    /// (`_n1`, `_e2`, …). Saltarse los nombres ya ocupados evita colisiones
    /// con variables de usuario que empiecen por `_`.
    fn fresh_internal_var(&mut self, prefix: &str) -> String {
        loop {
            self.next_internal += 1;
            let candidate = format!("_{prefix}{}", self.next_internal);
            if !self.bindings.contains(&candidate) {
                return candidate;
            }
        }
    }

    /// Liga un patrón de nodo del camino.
    ///
    /// - Declara su variable (o genera una interna si es anónimo); re-ligar
    ///   una variable existente es `VariableRebind`.
    /// - Si `label_como_predicado` (nodos de la cadena, no el inicial), la
    ///   etiqueta baja como predicado `HasLabel` —el `NodeScan` inicial la
    ///   absorbe directamente— y se devuelve `None`.
    /// - Las propiedades inline (`{edad: 30}`) bajan como igualdades.
    ///
    /// Devuelve `(variable_ligada, label_para_el_scan)`.
    fn bind_node(
        &mut self,
        np: &NodePattern,
        predicates: &mut Vec<ScalarExpr>,
        label_como_predicado: bool,
    ) -> Result<(String, Option<String>), PlanError> {
        let variable = match &np.variable {
            Some(v) => {
                if self.bindings.contains(v) {
                    return Err(PlanError::new(
                        PlanErrorKind::VariableRebind {
                            variable: v.clone(),
                        },
                        np.span,
                    ));
                }
                v.clone()
            }
            None => self.fresh_internal_var("n"),
        };
        self.bindings
            .declare(&variable, BindingKind::Node)
            .map_err(|kind| PlanError::new(kind, np.span))?;

        let scan_label = if label_como_predicado {
            if let Some(label) = &np.label {
                predicates.push(ScalarExpr::has_label(&variable, label));
            }
            None
        } else {
            np.label.clone()
        };

        for (key, value_expr) in &np.properties {
            let value = self.build_scalar(value_expr)?;
            predicates.push(ScalarExpr::eq(ScalarExpr::prop(&variable, key), value));
        }
        Ok((variable, scan_label))
    }

    /// Baja un camino completo `node (rel node)*` a una cadena
    /// `NodeScan + Expand*`, acumulando los predicados inline (etiquetas de
    /// los nodos de la cadena y propiedades) para el `Filter` global.
    fn lower_path(
        &mut self,
        path: &PathPattern,
        predicates: &mut Vec<ScalarExpr>,
    ) -> Result<LogicalPlan, PlanError> {
        // El nodo inicial alimenta el NodeScan con su etiqueta (si la hay):
        // es el único sitio donde una etiqueta NO es un predicado.
        let (start_var, start_label) = self.bind_node(&path.start, predicates, false)?;
        let mut plan = LogicalPlan::NodeScan {
            variable: start_var.clone(),
            label: start_label,
        };
        let mut prev = start_var;

        for (rel, node) in &path.chain {
            let rel_variable = match &rel.variable {
                Some(v) => {
                    if self.bindings.contains(v) {
                        return Err(PlanError::new(
                            PlanErrorKind::VariableRebind {
                                variable: v.clone(),
                            },
                            rel.span,
                        ));
                    }
                    self.bindings
                        .declare(v, BindingKind::Edge)
                        .map_err(|kind| PlanError::new(kind, rel.span))?;
                    Some(v.clone())
                }
                None => None,
            };
            let (to_var, _) = self.bind_node(node, predicates, true)?;
            plan = LogicalPlan::Expand {
                input: Box::new(plan),
                from: prev,
                rel_variable,
                rel_type: rel.rel_type.clone(),
                direction: rel.direction,
                to: to_var.clone(),
            };
            prev = to_var;
        }
        Ok(plan)
    }

    /// Resuelve una `Expression` del AST a `ScalarExpr`: sustituye nombres
    /// por variables ligadas (incrustando su `BindingKind`) y rechaza
    /// cualquier referencia no ligada. Los spans del AST localizan los
    /// errores.
    fn build_scalar(&self, expr: &Expression) -> Result<ScalarExpr, PlanError> {
        match expr {
            Expression::Literal { value, .. } => Ok(ScalarExpr::Literal(value.clone())),
            Expression::Variable { name, span } => match self.bindings.get(name) {
                Some(kind) => Ok(ScalarExpr::Var {
                    name: name.clone(),
                    kind,
                }),
                None => Err(PlanError::new(
                    PlanErrorKind::UnknownVariable {
                        variable: name.clone(),
                    },
                    *span,
                )),
            },
            Expression::PropertyAccess {
                variable,
                property,
                span,
            } => {
                if !self.bindings.contains(variable) {
                    return Err(PlanError::new(
                        PlanErrorKind::UnknownVariable {
                            variable: variable.clone(),
                        },
                        *span,
                    ));
                }
                Ok(ScalarExpr::Property {
                    variable: variable.clone(),
                    property: property.clone(),
                })
            }
            Expression::Compare {
                op, left, right, ..
            } => Ok(ScalarExpr::Compare {
                op: *op,
                left: Box::new(self.build_scalar(left)?),
                right: Box::new(self.build_scalar(right)?),
            }),
            Expression::And { left, right, .. } => Ok(ScalarExpr::And {
                left: Box::new(self.build_scalar(left)?),
                right: Box::new(self.build_scalar(right)?),
            }),
            Expression::Or { left, right, .. } => Ok(ScalarExpr::Or {
                left: Box::new(self.build_scalar(left)?),
                right: Box::new(self.build_scalar(right)?),
            }),
            Expression::Not { expr, .. } => Ok(ScalarExpr::Not {
                expr: Box::new(self.build_scalar(expr)?),
            }),
        }
    }
}

/// Convierte una `Query` (AST de los caps. 17-18) en su plan lógico.
///
/// Recorrido (el planner opera cláusula a cláusula, como los `AstNode` del
/// cap. 17 anticipaban):
///
/// 1. **MATCH** — un fragmento de plan por patrón; los patrones disjuntos se
///    combinan con `CartesianProduct`, los que comparten variables son error
///    (exigen join). Los predicados inline (etiquetas de la cadena y
///    propiedades) se acumulan para el `Filter`.
/// 2. **WHERE** — se resuelve contra la tabla de bindings, se type-checkea
///    (raíz booleana) y se conjunta con los predicados inline en un único
///    `Filter` sobre el plan del MATCH. Sin push-down: ésa es la primera
///    regla del optimizador del cap. 21.
/// 3. **RETURN** — cada item se resuelve, se type-checkea y forma una
///    `Projection`; el plan se envuelve en `Project`.
///
/// # Errores
///
/// [`PlanErrorKind::UnknownVariable`] (nombre sin ligar), `DuplicateVariable`
/// / `VariableRebind` (re-ligar en el MATCH), `SharedPatternVariables` (join
/// entre patrones), `TypeMismatch` (WHERE no booleano, comparaciones
/// imposibles) y `EmptyMatch`/`EmptyReturn` (ASTs construidos a mano).
pub fn lower(query: &Query) -> Result<LogicalPlan, PlanError> {
    if query.match_clause.patterns.is_empty() {
        return Err(PlanError::new(
            PlanErrorKind::EmptyMatch,
            query.match_clause.span,
        ));
    }

    let mut planner = Planner::new();
    let mut predicates: Vec<ScalarExpr> = Vec::new();

    // 1. MATCH: un fragmento por patrón; comprobar antes de bajar que no
    //    comparte variables con lo ya ligado (eso exigiría un join).
    let mut fragments: Vec<LogicalPlan> = Vec::new();
    for path in &query.match_clause.patterns {
        let shared: Vec<String> = path
            .node_variables()
            .into_iter()
            .chain(path.edge_variables())
            .filter(|v| planner.bindings.contains(v))
            .collect();
        if !shared.is_empty() {
            return Err(PlanError::new(
                PlanErrorKind::SharedPatternVariables { variables: shared },
                path.span,
            ));
        }
        fragments.push(planner.lower_path(path, &mut predicates)?);
    }
    let mut plan = fragments
        .into_iter()
        .reduce(|l, r| LogicalPlan::CartesianProduct {
            left: Box::new(l),
            right: Box::new(r),
        })
        .expect("patrones no vacío verificado arriba");

    // 2. WHERE: resolver, type-checkear (raíz BOOL o ANY) y conjuntar.
    if let Some(where_clause) = &query.where_clause {
        let predicate = planner.build_scalar(&where_clause.expr)?;
        let ty = predicate
            .type_of(&planner.bindings)
            .map_err(|kind| PlanError::new(kind, where_clause.expr.span()))?;
        if ty != LogicalType::Bool && ty != LogicalType::Any {
            return Err(PlanError::new(
                PlanErrorKind::TypeMismatch {
                    context: "WHERE",
                    expected: LogicalType::Bool,
                    got: ty,
                },
                where_clause.expr.span(),
            ));
        }
        predicates.push(predicate);
    }
    if let Some(predicate) = ScalarExpr::and_all(predicates) {
        plan = LogicalPlan::Filter {
            input: Box::new(plan),
            predicate,
        };
    }

    // 3. RETURN: proyecciones resueltas y type-checkeadas.
    if query.return_clause.items.is_empty() {
        return Err(PlanError::new(
            PlanErrorKind::EmptyReturn,
            query.return_clause.span,
        ));
    }
    let mut items = Vec::with_capacity(query.return_clause.items.len());
    for item in &query.return_clause.items {
        let expr = planner.build_scalar(&item.expr)?;
        expr.type_of(&planner.bindings)
            .map_err(|kind| PlanError::new(kind, item.expr.span()))?;
        items.push(Projection {
            expr,
            alias: item.alias.clone(),
        });
    }

    Ok(LogicalPlan::Project {
        input: Box::new(plan),
        items,
    })
}

impl Query {
    /// Atajo de [`lower`] sobre la propia consulta (parse → lower → plan).
    pub fn lower(&self) -> Result<LogicalPlan, PlanError> {
        lower(self)
    }
}

#[cfg(test)]
mod tests_logical_plan {
    use super::*;
    use crate::cap18_lexer_parser::parse;

    // ════════════════════════════════════════════════════════════════
    //  LogicalType — inferencia de tipos básica
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn tipos_display_canonicos() {
        assert_eq!(LogicalType::Any.to_string(), "ANY");
        assert_eq!(LogicalType::Null.to_string(), "NULL");
        assert_eq!(LogicalType::Bool.to_string(), "BOOL");
        assert_eq!(LogicalType::Int.to_string(), "INT");
        assert_eq!(LogicalType::Float.to_string(), "FLOAT");
        assert_eq!(LogicalType::String.to_string(), "STRING");
        assert_eq!(LogicalType::Bytes.to_string(), "BYTES");
        assert_eq!(LogicalType::Node.to_string(), "NODE");
        assert_eq!(LogicalType::Edge.to_string(), "EDGE");
    }

    #[test]
    fn eq_compatible_reglas() {
        // Iguales entre sí y numéricos cruzados.
        assert!(LogicalType::eq_compatible(
            LogicalType::Int,
            LogicalType::Int
        ));
        assert!(LogicalType::eq_compatible(
            LogicalType::Int,
            LogicalType::Float
        ));
        // Comodines (Any = schemaless, Null = comparación con NULL).
        assert!(LogicalType::eq_compatible(
            LogicalType::Any,
            LogicalType::Node
        ));
        assert!(LogicalType::eq_compatible(
            LogicalType::Bool,
            LogicalType::Null
        ));
        // Concretos incompatibles.
        assert!(!LogicalType::eq_compatible(
            LogicalType::Bool,
            LogicalType::Int
        ));
        assert!(!LogicalType::eq_compatible(
            LogicalType::Node,
            LogicalType::String
        ));
        assert!(!LogicalType::eq_compatible(
            LogicalType::Node,
            LogicalType::Edge
        ));
    }

    #[test]
    fn order_compatible_reglas() {
        assert!(LogicalType::order_compatible(
            LogicalType::Int,
            LogicalType::Float
        ));
        assert!(LogicalType::order_compatible(
            LogicalType::String,
            LogicalType::String
        ));
        assert!(LogicalType::order_compatible(
            LogicalType::Any,
            LogicalType::Edge
        ));
        // Booleanos, nodos y aristas no son ordenables; Int vs String tampoco.
        assert!(!LogicalType::order_compatible(
            LogicalType::Bool,
            LogicalType::Bool
        ));
        assert!(!LogicalType::order_compatible(
            LogicalType::Node,
            LogicalType::Node
        ));
        assert!(!LogicalType::order_compatible(
            LogicalType::Int,
            LogicalType::String
        ));
    }

    // ════════════════════════════════════════════════════════════════
    //  Bindings — la tabla de variables ligadas
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn bindings_declara_consulta_y_itera() {
        let mut b = Bindings::new();
        assert!(b.is_empty());
        b.declare("p", BindingKind::Node).unwrap();
        b.declare("r", BindingKind::Edge).unwrap();
        assert_eq!(b.len(), 2);
        assert_eq!(b.get("p"), Some(BindingKind::Node));
        assert_eq!(b.get("r"), Some(BindingKind::Edge));
        assert_eq!(b.get("x"), None);
        assert!(b.contains("p"));
        assert!(!b.contains("x"));
        // El orden de iteración es el de declaración.
        let names: Vec<&str> = b.iter().map(|(n, _)| n).collect();
        assert_eq!(names, vec!["p", "r"]);
        assert_eq!(b.to_string(), "{p:NODE, r:EDGE}");
    }

    #[test]
    fn bindings_rechaza_duplicados() {
        let mut b = Bindings::new();
        b.declare("a", BindingKind::Node).unwrap();
        let err = b.declare("a", BindingKind::Edge).unwrap_err();
        assert!(matches!(
            err,
            PlanErrorKind::DuplicateVariable { ref variable } if variable == "a"
        ));
        // El duplicado no se insertó.
        assert_eq!(b.len(), 1);
        assert_eq!(b.get("a"), Some(BindingKind::Node));
    }

    // ════════════════════════════════════════════════════════════════
    //  ScalarExpr — display, conjunción y tipos
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn scalar_display_sin_parentesis_redundantes() {
        // And(HasLabel, Compare) en la raíz: sin paréntesis, como el brief.
        let e = ScalarExpr::And {
            left: Box::new(ScalarExpr::has_label("f", "Person")),
            right: Box::new(ScalarExpr::eq(
                ScalarExpr::prop("p", "name"),
                ScalarExpr::lit(Value::String("Ana".into())),
            )),
        };
        assert_eq!(e.to_string(), "f:Person AND p.name = \"Ana\"");
    }

    #[test]
    fn scalar_display_parentesis_minimos_por_precedencia() {
        let a = || ScalarExpr::var("a", BindingKind::Node);
        let b = || ScalarExpr::var("b", BindingKind::Node);
        let c = || ScalarExpr::var("c", BindingKind::Node);
        // (a OR b) AND c — el OR dentro de AND necesita paréntesis.
        let e1 = ScalarExpr::And {
            left: Box::new(ScalarExpr::Or {
                left: Box::new(a()),
                right: Box::new(b()),
            }),
            right: Box::new(c()),
        };
        assert_eq!(e1.to_string(), "(a OR b) AND c");
        // (a AND b) OR c — el AND dentro de OR necesita paréntesis.
        let e2 = ScalarExpr::Or {
            left: Box::new(ScalarExpr::And {
                left: Box::new(a()),
                right: Box::new(b()),
            }),
            right: Box::new(c()),
        };
        assert_eq!(e2.to_string(), "(a AND b) OR c");
        // NOT (a AND b) — el AND dentro de NOT necesita paréntesis.
        let e3 = ScalarExpr::Not {
            expr: Box::new(ScalarExpr::And {
                left: Box::new(a()),
                right: Box::new(b()),
            }),
        };
        assert_eq!(e3.to_string(), "NOT (a AND b)");
        // NOT a AND b — asociación correcta sin paréntesis: (NOT a) AND b.
        let e4 = ScalarExpr::And {
            left: Box::new(ScalarExpr::Not {
                expr: Box::new(a()),
            }),
            right: Box::new(b()),
        };
        assert_eq!(e4.to_string(), "NOT a AND b");
        // Asociativos: sin paréntesis.
        let e5 = ScalarExpr::And {
            left: Box::new(ScalarExpr::And {
                left: Box::new(a()),
                right: Box::new(b()),
            }),
            right: Box::new(c()),
        };
        assert_eq!(e5.to_string(), "a AND b AND c");
    }

    #[test]
    fn scalar_and_all_conjuncion_left_asociativa() {
        assert!(ScalarExpr::and_all(Vec::new()).is_none());
        let only = ScalarExpr::lit(Value::Bool(true));
        let one = ScalarExpr::and_all(vec![only.clone()]).unwrap();
        assert_eq!(one, only);
        let a = ScalarExpr::lit(Value::Bool(true));
        let b = ScalarExpr::lit(Value::Bool(false));
        let c = ScalarExpr::has_label("p", "Person");
        // [a, b, c] → And(And(a, b), c).
        let expr = ScalarExpr::and_all(vec![a.clone(), b.clone(), c.clone()]).unwrap();
        assert_eq!(
            expr,
            ScalarExpr::And {
                left: Box::new(ScalarExpr::And {
                    left: Box::new(a),
                    right: Box::new(b),
                }),
                right: Box::new(c),
            }
        );
    }

    #[test]
    fn type_of_literales_y_variables() {
        let mut b = Bindings::new();
        b.declare("p", BindingKind::Node).unwrap();
        b.declare("r", BindingKind::Edge).unwrap();
        assert_eq!(
            ScalarExpr::lit(Value::Null).type_of(&b).unwrap(),
            LogicalType::Null
        );
        assert_eq!(
            ScalarExpr::lit(Value::Bool(true)).type_of(&b).unwrap(),
            LogicalType::Bool
        );
        assert_eq!(
            ScalarExpr::lit(Value::Int(3)).type_of(&b).unwrap(),
            LogicalType::Int
        );
        assert_eq!(
            ScalarExpr::lit(Value::Float(2.5)).type_of(&b).unwrap(),
            LogicalType::Float
        );
        assert_eq!(
            ScalarExpr::lit(Value::String("x".into()))
                .type_of(&b)
                .unwrap(),
            LogicalType::String
        );
        assert_eq!(
            ScalarExpr::lit(Value::Bytes(vec![1])).type_of(&b).unwrap(),
            LogicalType::Bytes
        );
        assert_eq!(
            ScalarExpr::var("p", BindingKind::Node).type_of(&b).unwrap(),
            LogicalType::Node
        );
        assert_eq!(
            ScalarExpr::var("r", BindingKind::Edge).type_of(&b).unwrap(),
            LogicalType::Edge
        );
        // Propiedades: schemaless → Any. HasLabel → Bool.
        assert_eq!(
            ScalarExpr::prop("p", "name").type_of(&b).unwrap(),
            LogicalType::Any
        );
        assert_eq!(
            ScalarExpr::has_label("p", "Person").type_of(&b).unwrap(),
            LogicalType::Bool
        );
        // Variable sin ligar: error defensivo (el binder ya lo habría cazado).
        assert!(matches!(
            ScalarExpr::prop("x", "y").type_of(&b),
            Err(PlanErrorKind::UnknownVariable { .. })
        ));
    }

    #[test]
    fn type_of_comparaciones_y_logicos() {
        let mut b = Bindings::new();
        b.declare("p", BindingKind::Node).unwrap();
        let num = ScalarExpr::lit(Value::Int(1));
        let flo = ScalarExpr::lit(Value::Float(2.5));
        let booleano = ScalarExpr::lit(Value::Bool(true));
        let any = ScalarExpr::prop("p", "edad");

        // Numéricos cruzados y wildcards: OK, resultado BOOL.
        let cmp = ScalarExpr::Compare {
            op: CompareOp::Lt,
            left: Box::new(num.clone()),
            right: Box::new(flo.clone()),
        };
        assert_eq!(cmp.type_of(&b).unwrap(), LogicalType::Bool);
        assert_eq!(
            ScalarExpr::eq(any.clone(), booleano.clone())
                .type_of(&b)
                .unwrap(),
            LogicalType::Bool
        );

        // Orden sobre Bool: TypeMismatch.
        let bad_order = ScalarExpr::Compare {
            op: CompareOp::Gte,
            left: Box::new(booleano.clone()),
            right: Box::new(booleano.clone()),
        };
        assert!(matches!(
            bad_order.type_of(&b),
            Err(PlanErrorKind::TypeMismatch { context, expected, got })
                if context == "comparación de orden"
                    && expected == LogicalType::Bool
                    && got == LogicalType::Bool
        ));

        // Igualdad imposible: Int vs Bool.
        let bad_eq = ScalarExpr::eq(num, booleano);
        assert!(matches!(
            bad_eq.type_of(&b),
            Err(PlanErrorKind::TypeMismatch { context, expected, got })
                if context == "comparación de igualdad"
                    && expected == LogicalType::Bool
                    && got == LogicalType::Int
        ));

        // AND/OR/NOT con operandos booleanos: BOOL.
        let andy = ScalarExpr::And {
            left: Box::new(any.clone()),
            right: Box::new(ScalarExpr::Not {
                expr: Box::new(any.clone()),
            }),
        };
        assert_eq!(andy.type_of(&b).unwrap(), LogicalType::Bool);
        // AND con operando Int: TypeMismatch.
        let int_lit = ScalarExpr::lit(Value::Int(3));
        let bad_and = ScalarExpr::And {
            left: Box::new(int_lit.clone()),
            right: Box::new(int_lit),
        };
        assert!(matches!(
            bad_and.type_of(&b),
            Err(PlanErrorKind::TypeMismatch { context, expected, got })
                if context == "operando de AND"
                    && expected == LogicalType::Bool
                    && got == LogicalType::Int
        ));
    }

    // ════════════════════════════════════════════════════════════════
    //  Lowering — casos base
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn lower_match_nodo_solo() {
        let plan = parse("MATCH (p:Person) RETURN p").unwrap().lower().unwrap();
        let LogicalPlan::Project { input, items } = &plan else {
            panic!("la raíz siempre es Project");
        };
        let LogicalPlan::NodeScan { variable, label } = input.as_ref() else {
            panic!("sin predicados no hay Filter: Project sobre NodeScan");
        };
        assert_eq!(variable, "p");
        assert_eq!(label.as_deref(), Some("Person"));
        assert_eq!(items.len(), 1);
        assert!(matches!(
            &items[0].expr,
            ScalarExpr::Var { name, kind } if name == "p" && *kind == BindingKind::Node
        ));
        assert_eq!(plan.to_string(), "Project(p)\n  NodeScan(Person AS p)");
    }

    #[test]
    fn lower_display_ejemplo_canonico_del_brief() {
        // El ejemplo del brief §cap 19, con una corrección: el plan del brief
        // omitía imponer `f:Person`; aquí baja como predicado en el Filter.
        let plan =
            parse("MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE p.name = \"Ana\" RETURN f.name")
                .unwrap()
                .lower()
                .unwrap();
        assert_eq!(
            plan.to_string(),
            "Project(f.name)\n  \
             Filter(f:Person AND p.name = \"Ana\")\n    \
             Expand(p, KNOWS, OUTGOING, f)\n      \
             NodeScan(Person AS p)"
        );
    }

    #[test]
    fn lower_estructura_del_ejemplo_canonico() {
        // Además del texto: la estructura exacta del árbol.
        let plan =
            parse("MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE p.name = \"Ana\" RETURN f.name")
                .unwrap()
                .lower()
                .unwrap();
        let LogicalPlan::Project { input, items } = &plan else {
            panic!("la raíz siempre es Project");
        };
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].output_name(), "f.name");
        assert!(items[0].alias.is_none());
        let LogicalPlan::Filter { input, predicate } = input.as_ref() else {
            panic!("debajo del Project está el Filter");
        };
        // El Filter queda ENCIMA del Expand: el push-down es cap. 21.
        assert!(matches!(
            input.as_ref(),
            LogicalPlan::Expand {
                from, to, rel_type, direction,
                ..
            } if from == "p" && to == "f"
                  && rel_type.as_deref() == Some("KNOWS")
                  && *direction == RelDirection::Outgoing
        ));
        // Predicado = And(HasLabel(f, Person), p.name = "Ana").
        assert_eq!(
            predicate,
            &ScalarExpr::And {
                left: Box::new(ScalarExpr::has_label("f", "Person")),
                right: Box::new(ScalarExpr::eq(
                    ScalarExpr::prop("p", "name"),
                    ScalarExpr::lit(Value::String("Ana".into())),
                )),
            }
        );
    }

    #[test]
    fn lower_nodo_anonimo_genera_variable_interna() {
        let plan = parse("MATCH (p:Person)-[:KNOWS]->() RETURN p")
            .unwrap()
            .lower()
            .unwrap();
        // `()` sin label ni props NO añade predicados: no hay Filter; el
        // destino se liga con una variable interna (_n1).
        let LogicalPlan::Project { input, .. } = &plan else {
            panic!("la raíz siempre es Project");
        };
        assert!(matches!(
            input.as_ref(),
            LogicalPlan::Expand { to, .. } if to.starts_with("_n")
        ));
        let texto = plan.to_string();
        assert!(
            texto.contains("Expand(p, KNOWS, OUTGOING, _n1)"),
            "plan: {texto}"
        );
        assert_eq!(
            plan.bound_variables(),
            vec!["p".to_string(), "_n1".to_string()]
        );
    }

    #[test]
    fn lower_sin_label_any() {
        let plan = parse("MATCH (p) RETURN p").unwrap().lower().unwrap();
        let LogicalPlan::Project { input, .. } = &plan else {
            panic!("la raíz siempre es Project");
        };
        let LogicalPlan::NodeScan { variable, label } = input.as_ref() else {
            panic!("sin predicados no hay Filter");
        };
        assert_eq!(variable, "p");
        assert!(label.is_none());
        assert_eq!(plan.to_string(), "Project(p)\n  NodeScan(ANY AS p)");
    }

    #[test]
    fn lower_propiedades_inline_bajan_al_filter() {
        let plan = parse("MATCH (p:Person {edad: 30}) RETURN p")
            .unwrap()
            .lower()
            .unwrap();
        assert_eq!(
            plan.to_string(),
            "Project(p)\n  Filter(p.edad = 30)\n    NodeScan(Person AS p)"
        );
    }

    #[test]
    fn lower_where_y_props_inline_se_conjuntan_en_un_filter() {
        let plan = parse("MATCH (p:Person {edad: 30}) WHERE p.nombre = \"Ana\" RETURN p")
            .unwrap()
            .lower()
            .unwrap();
        assert_eq!(
            plan.to_string(),
            "Project(p)\n  \
             Filter(p.edad = 30 AND p.nombre = \"Ana\")\n    \
             NodeScan(Person AS p)"
        );
    }

    #[test]
    fn lower_path_de_tres_nodos_encadena_expands() {
        let plan = parse("MATCH (a:Person)-[:KNOWS]->(b:Person)-[:KNOWS]->(c:Person) RETURN c")
            .unwrap()
            .lower()
            .unwrap();
        assert_eq!(
            plan.to_string(),
            "Project(c)\n  \
             Filter(b:Person AND c:Person)\n    \
             Expand(b, KNOWS, OUTGOING, c)\n      \
             Expand(a, KNOWS, OUTGOING, b)\n        \
             NodeScan(Person AS a)"
        );
        assert_eq!(
            plan.bound_variables(),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn lower_direccion_entrante_y_sin_definir() {
        let entrante = parse("MATCH (a:Person)<-[:KNOWS]-(b:Person) RETURN a")
            .unwrap()
            .lower()
            .unwrap();
        assert_eq!(
            entrante.to_string(),
            "Project(a)\n  \
             Filter(b:Person)\n    \
             Expand(a, KNOWS, INCOMING, b)\n      \
             NodeScan(Person AS a)"
        );
        let indefinida = parse("MATCH (a:Person)-[:KNOWS]-(f:Person) RETURN f")
            .unwrap()
            .lower()
            .unwrap();
        assert!(
            indefinida
                .to_string()
                .contains("Expand(a, KNOWS, UNDIRECTED, f)")
        );
    }

    #[test]
    fn lower_relacion_con_variable_y_sin_tipo() {
        let plan = parse("MATCH (p:Person)-[r:KNOWS]->(f:Person) RETURN r")
            .unwrap()
            .lower()
            .unwrap();
        assert!(plan.to_string().contains("Expand(p, r:KNOWS, OUTGOING, f)"));
        // r está ligada como EDGE y es retornable.
        assert!(plan.bound_variables().contains(&"r".to_string()));
        let LogicalPlan::Project { items, .. } = &plan else {
            panic!("raíz");
        };
        assert!(matches!(
            &items[0].expr,
            ScalarExpr::Var { name, kind } if name == "r" && *kind == BindingKind::Edge
        ));

        // Relación anónima sin tipo: ANY.
        let sin_tipo = parse("MATCH (p:Person)-[]->(f:Person) RETURN f")
            .unwrap()
            .lower()
            .unwrap();
        assert!(sin_tipo.to_string().contains("Expand(p, ANY, OUTGOING, f)"));
    }

    #[test]
    fn lower_patrones_disjuntos_cartesian_product() {
        let plan = parse("MATCH (a:Person), (b:City) RETURN a, b")
            .unwrap()
            .lower()
            .unwrap();
        assert_eq!(
            plan.to_string(),
            "Project(a, b)\n  \
             CartesianProduct\n    \
             NodeScan(Person AS a)\n    \
             NodeScan(City AS b)"
        );
        assert_eq!(
            plan.bound_variables(),
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn lower_return_alias_y_nombres_derivados() {
        let plan = parse("MATCH (p:Person) RETURN p.nombre AS nombre, p")
            .unwrap()
            .lower()
            .unwrap();
        let LogicalPlan::Project { items, .. } = &plan else {
            panic!("raíz");
        };
        assert_eq!(items.len(), 2);
        // Con alias explícito.
        assert_eq!(items[0].alias.as_deref(), Some("nombre"));
        assert_eq!(items[0].output_name(), "nombre");
        // Sin alias: nombre derivado de la expresión.
        assert!(items[1].alias.is_none());
        assert_eq!(items[1].output_name(), "p");
        assert!(
            plan.to_string()
                .starts_with("Project(p.nombre AS nombre, p)")
        );

        // Propiedad sin alias: "var.prop" (formato Cypher).
        let plan2 = parse("MATCH (f:Person) RETURN f.name")
            .unwrap()
            .lower()
            .unwrap();
        let LogicalPlan::Project { items, .. } = &plan2 else {
            panic!("raíz");
        };
        assert_eq!(items[0].output_name(), "f.name");
    }

    // ════════════════════════════════════════════════════════════════
    //  Lowering — errores semánticos
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn lower_where_variable_no_ligada() {
        let err = parse("MATCH (p:Person) WHERE x.name = \"A\" RETURN p")
            .unwrap()
            .lower()
            .unwrap_err();
        assert!(matches!(
            &err.kind,
            PlanErrorKind::UnknownVariable { variable } if variable == "x"
        ));
        // El span apunta al acceso ofensivo (x.name), no a toda la cláusula.
        assert!(!err.span.is_empty());
    }

    #[test]
    fn lower_return_variable_no_ligada() {
        let err = parse("MATCH (p:Person) RETURN f.name")
            .unwrap()
            .lower()
            .unwrap_err();
        assert!(matches!(
            &err.kind,
            PlanErrorKind::UnknownVariable { variable } if variable == "f"
        ));
    }

    #[test]
    fn lower_propiedad_inline_variable_no_ligada() {
        // El valor de una propiedad inline también se resuelve contra bindings:
        // {amigo: q.nombre} referencia q, que no está ligada.
        let err = parse("MATCH (p:Person {amigo: q.nombre}) RETURN p")
            .unwrap()
            .lower()
            .unwrap_err();
        assert!(matches!(
            &err.kind,
            PlanErrorKind::UnknownVariable { variable } if variable == "q"
        ));
    }

    #[test]
    fn lower_variable_relidada_en_el_mismo_patron() {
        let err = parse("MATCH (a:Person)-[:KNOWS]->(a:Person) RETURN a")
            .unwrap()
            .lower()
            .unwrap_err();
        assert!(matches!(
            &err.kind,
            PlanErrorKind::VariableRebind { variable } if variable == "a"
        ));
    }

    #[test]
    fn lower_patrones_que_comparten_variables_exigen_join() {
        let err = parse("MATCH (a:Person)-[:KNOWS]->(b:Person), (b)-[:KNOWS]->(c:Person) RETURN c")
            .unwrap()
            .lower()
            .unwrap_err();
        assert!(matches!(
            &err.kind,
            PlanErrorKind::SharedPatternVariables { variables } if variables == &vec!["b".to_string()]
        ));
    }

    #[test]
    fn lower_match_y_return_vacios() {
        // `parse()` no puede producirlos; un AST a mano sí (campos pub).
        let mut q = parse("MATCH (p:Person) RETURN p").unwrap();
        q.match_clause.patterns.clear();
        let err = lower(&q).unwrap_err();
        assert!(matches!(err.kind, PlanErrorKind::EmptyMatch));

        let mut q2 = parse("MATCH (p:Person) RETURN p").unwrap();
        q2.return_clause.items.clear();
        let err2 = lower(&q2).unwrap_err();
        assert!(matches!(err2.kind, PlanErrorKind::EmptyReturn));
    }

    // ════════════════════════════════════════════════════════════════
    //  Lowering — type-check del WHERE y RETURN
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn lower_where_no_booleano() {
        let err = parse("MATCH (p:Person) WHERE 3 RETURN p")
            .unwrap()
            .lower()
            .unwrap_err();
        assert!(matches!(
            &err.kind,
            PlanErrorKind::TypeMismatch { context, expected, got }
                if *context == "WHERE"
                    && *expected == LogicalType::Bool
                    && *got == LogicalType::Int
        ));
    }

    #[test]
    fn lower_where_igualdad_imposible() {
        // p es NODE: compararlo con TRUE es error de tipos.
        let err = parse("MATCH (p:Person) WHERE p = TRUE RETURN p")
            .unwrap()
            .lower()
            .unwrap_err();
        assert!(matches!(
            &err.kind,
            PlanErrorKind::TypeMismatch { context, expected, got }
                if *context == "comparación de igualdad"
                    && *expected == LogicalType::Bool
                    && *got == LogicalType::Node
        ));
    }

    #[test]
    fn lower_where_orden_imposible() {
        let err = parse("MATCH (p:Person) WHERE TRUE < FALSE RETURN p")
            .unwrap()
            .lower()
            .unwrap_err();
        assert!(matches!(
            &err.kind,
            PlanErrorKind::TypeMismatch { context, .. }
                if *context == "comparación de orden"
        ));
    }

    #[test]
    fn lower_where_property_schemaless_pasa() {
        // p.edad tipa ANY: compararla con TRUE es aceptable (schemaless),
        // la comparación concreta se resuelve en ejecución (cap. 20).
        let plan = parse("MATCH (p:Person) WHERE p.edad = TRUE RETURN p")
            .unwrap()
            .lower()
            .unwrap();
        assert!(plan.to_string().contains("Filter(p.edad = TRUE)"));
    }

    #[test]
    fn lower_where_bool_literal_y_and_sobre_no_bool() {
        // WHERE TRUE es válido (raíz BOOL) aunque inútil.
        let plan = parse("MATCH (p:Person) WHERE TRUE RETURN p")
            .unwrap()
            .lower()
            .unwrap();
        assert!(plan.to_string().contains("Filter(TRUE)"));

        // 1 AND 2: operandos INT dentro de un AND → TypeMismatch.
        let err = parse("MATCH (p:Person) WHERE 1 AND 2 RETURN p")
            .unwrap()
            .lower()
            .unwrap_err();
        assert!(matches!(
            &err.kind,
            PlanErrorKind::TypeMismatch { context, .. } if *context == "operando de AND"
        ));
    }

    #[test]
    fn lower_not_sobre_no_booleano() {
        let err = parse("MATCH (p:Person) WHERE NOT 3 RETURN p")
            .unwrap()
            .lower()
            .unwrap_err();
        assert!(matches!(
            &err.kind,
            PlanErrorKind::TypeMismatch { context, .. } if *context == "operando de NOT"
        ));
    }

    #[test]
    fn lower_return_item_type_checkeado() {
        // RETURN con un NOT sobre un entero: el error de tipos del item se
        // detecta en el lowering, no en ejecución.
        let err = parse("MATCH (p:Person) WHERE p.nombre = \"A\" RETURN NOT 3")
            .unwrap()
            .lower()
            .unwrap_err();
        assert!(matches!(
            &err.kind,
            PlanErrorKind::TypeMismatch { context, .. } if *context == "operando de NOT"
        ));
    }

    // ════════════════════════════════════════════════════════════════
    //  Integración parse → lower → plan, Display y errores
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn integracion_parse_lower_plan_pipeline_completo() {
        // Consulta del cap. 19 del brief, cadena completa cap 18 + cap 19.
        let src = "MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE p.name = \"Ana\" RETURN f.name";
        let query = parse(src).unwrap();
        assert!(query.validate().is_empty());
        let plan = lower(&query).unwrap();
        // El método de Query y la función libre coinciden.
        assert_eq!(query.lower().unwrap(), plan);
        // Variables ligadas visibles para el executor del cap. 20.
        assert_eq!(
            plan.bound_variables(),
            vec!["p".to_string(), "f".to_string()]
        );
    }

    #[test]
    fn plan_display_es_estable_e_idempotente() {
        let plan = parse(
            "MATCH (a:Person)-[:KNOWS]->(b:Person) WHERE a.age > 30 OR b.age > 40 RETURN a, b",
        )
        .unwrap()
        .lower()
        .unwrap();
        let s1 = plan.to_string();
        let s2 = format!("{plan}");
        assert_eq!(s1, s2);
        assert!(s1.contains("Filter(b:Person AND (a.age > 30 OR b.age > 40))"));
    }

    #[test]
    fn plan_error_display_localiza_y_es_std_error() {
        let err = parse("MATCH (p:Person) WHERE q.x = 1 RETURN p")
            .unwrap()
            .lower()
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("variable 'q' usada pero no ligada por el MATCH"),
            "{msg}"
        );
        assert!(msg.contains("(en "), "{msg}");
        // Implementa std::error::Error (usable con Box<dyn Error>, ?, anyhow…).
        let boxed: Box<dyn std::error::Error> = Box::new(err);
        assert!(boxed.to_string().contains("'q'"));
    }

    #[test]
    fn plan_error_shared_variables_display() {
        let err = parse("MATCH (a:Person), (a:City) RETURN a")
            .unwrap()
            .lower()
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("comparten las variables [a]"), "{msg}");
        assert!(msg.contains("join"), "{msg}");
    }

    #[test]
    fn lower_dos_patrones_con_anonimos_no_colisionan() {
        // Dos nodos anónimos generan variables internas distintas. (Los
        // `()` desnudos los rechaza el validate() del cap 17; el binder del
        // cap 19 es más permisivo y los liga con internas.)
        let plan = parse("MATCH (), () RETURN 1").unwrap().lower().unwrap();
        let vars = plan.bound_variables();
        assert_eq!(vars.len(), 2, "vars: {vars:?}");
        assert_ne!(vars[0], vars[1]);
        assert!(vars.iter().all(|v| v.starts_with("_n")));
    }
}
