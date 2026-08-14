use crate::cap07_modelo::Value;

// ─────────────────── Cap 17: Diseñar un lenguaje pequeño ───────────────────
//
// Con la Parte III cerrada (caps 11-16) LiraDB ya sabe *guardar* un grafo:
// páginas, buffer pool, CSR, índices y mantenimiento. Pero todavía no sabe
// *preguntar*. Este capítulo abre la Parte IV "Consultar el grafo" con el
// **diseño del lenguaje**, no con su implementación: definimos el modelo de
// tokens, la gramática (EBNF documentada), el AST y los errores con posición.
// El lexer y el parser descendente llegan en el cap.18; el plan lógico en
// el cap.19 y el motor Volcano en el cap.20. Aquí fijamos *qué* construiremos.
//
// El lenguaje se llama **LiraQL** (mini-Cypher) e intencionadamente recorta
// Cypher: no hay CREATE/MERGE/DELETE (esos son DML del cap.31 de la CLI),
// ni WITH, ni OPTIONAL MATCH, ni recursión (SET en cap.22+). Sólo consulta:
//
//   ```text
//   MATCH (p:Person)-[:KNOWS]->(f:Person)
//   WHERE p.name = "Ana"
//   RETURN f.name, p.age AS edad
//   ```
//
// Las tres cláusulas son obligatorias en este cap (RETURN siempre presente),
// coherente con el hito del brief (§cap 17):
//
//   ```text
//   pub enum AstNode {
//       Match(MatchClause),
//       Where(Expression),
//       Return(ReturnClause),
//   }
//   ```
//
// Modelo de errores comprensibles: TODO AST lleva [`Span`] (rango start..end
// en el fuente). Un [`QueryError`] = { kind, span } apunta al carácter exacto
// donde el usuario se equivocó. El lexer del cap.18 rellenará esos spans.
//
// Gramática EBNF de LiraQL (referencia para el parser del cap.18):
//
//   ```text
//   query         ::= match_clause where_clause? return_clause ;
//   match_clause  ::= 'MATCH' path_pattern (',' path_pattern)* ;
//   path_pattern  ::= node_pattern ( rel_pattern node_pattern )* ;
//   node_pattern  ::= '(' [variable] [':' label] ['{' prop_map '}'] ')' ;
//   rel_pattern   ::= '-[' [variable] [':' rel_type] ']-' ( '>' | '<' )?
//                  |  '<-[' [variable] [':' rel_type] ']-' ;
//   prop_map      ::= ident ':' expression (',' ident ':' expression)* ;
//   where_clause  ::= 'WHERE' expression ;
//   return_clause ::= 'RETURN' return_item (',' return_item)* ;
//   return_item   ::= expression (['AS'] alias)? ;
//   expression    ::= or_expr ;
//   or_expr       ::= and_expr ('OR' and_expr)* ;
//   and_expr      ::= not_expr ('AND' not_expr)* ;
//   not_expr      ::= 'NOT' not_expr | comparison ;
//   comparison    ::= primary ( comp_op primary )? ;
//   comp_op       ::= '=' | '<>' | '<' | '<=' | '>' | '>=' ;
//   primary       ::= literal | property_access | '(' expression ')' ;
//   property_access ::= variable '.' property ;
//   literal       ::= INTEGER | FLOAT | STRING | 'TRUE' | 'FALSE' | 'NULL' ;
//   ```

// ─── Span: posición en el código fuente ───

/// Rango半abierto `[start, end)` en el texto fuente (en bytes UTF-8).
///
/// Todos los nodos del AST llevan un `Span` para que los mensajes de error
/// puedan apuntar al carácter exacto. La convención es la misma que usan
/// `codespan-reporting`, `miette` o `rustc`: offsets de byte desde el inicio
/// del fichero/consulta. El lexer del cap.18 los producirá gratuitamente.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    /// Span vacío en el offset dado (para tokens sintéticos).
    pub fn at(offset: u32) -> Self {
        Self {
            start: offset,
            end: offset,
        }
    }

    /// Span entre dos offsets (el orden se normaliza).
    pub fn new(start: u32, end: u32) -> Self {
        Self {
            start: start.min(end),
            end: start.max(end),
        }
    }

    /// Span que cubre a ambos (unión).
    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// ¿Está vacío (start == end)?
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Longitud en bytes.
    pub fn len(self) -> u32 {
        self.end.saturating_sub(self.start)
    }
}

// ─── TokenKind: categorías léxicas de LiraQL ───

/// Categorías léxicas de LiraQL.
///
/// El cap.17 fija el *vocabulario* del lenguaje; el lexer del cap.18 produce
/// `Token { kind, text, span }` a partir del texto fuente. Mantener el enum
/// aquí permite que el AST (este cap.) referencie los spans sin depender de
/// la implementación concreta del escáner.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Palabras clave (case-sensitive, mayúsculas por convención Cypher).
    /// `MATCH`.
    Match,
    /// `WHERE`.
    Where,
    /// `RETURN`.
    Return,
    /// `AS` (alias opcional en RETURN).
    As,
    /// `AND`.
    And,
    /// `OR`.
    Or,
    /// `NOT`.
    Not,
    /// `TRUE`.
    True,
    /// `FALSE`.
    False,
    /// `NULL`.
    Null,

    // Identificadores y literales.
    /// Identificador (variable, label, propiedad): `[A-Za-z_][A-Za-z0-9_]*`.
    Ident(String),
    /// Literal entero (`i64`).
    Integer(i64),
    /// Literal flotante (`f64`).
    Float(f64),
    /// Literal string (sin las comillas, ya escapada por el lexer cap.18).
    String(String),

    // Puntuación y patrones.
    /// `(`.
    LParen,
    /// `)`.
    RParen,
    /// `[`.
    LBracket,
    /// `]`.
    RBracket,
    /// `{`.
    LBrace,
    /// `}`.
    RBrace,
    /// `,`.
    Comma,
    /// `:`.
    Colon,
    /// `.`.
    Dot,
    /// `->` (flecha saliente).
    ArrowRight,
    /// `<-` (flecha entrante).
    ArrowLeft,
    /// `--` (guión doble, relación sin dirección).
    DashDash,
    /// `-` (guión simple). Se introduce en cap.18: el lexer lo produce al
    /// encontrar un `-` que no forma parte de `->` ni `--`. Lo necesita el
    /// parser para reconocer los extremos de una relación (`-[ ... ]` y el
    /// cierre `]-` de las relaciones entrantes `<-[ ... ]-`).
    Dash,

    // Operadores de comparación.
    /// `=`.
    Eq,
    /// `<>` (distinto).
    NotEq,
    /// `<`.
    Lt,
    /// `<=`.
    Lte,
    /// `>`.
    Gt,
    /// `>=`.
    Gte,

    /// Fin de fichero.
    Eof,
}

/// Token con su categoría, texto y span.
///
/// El lexer del cap.18 produce `Vec<Token>`; el parser consume ese stream.
/// Aquí lo definimos para que el AST del cap.17 pueda citar spans concretos.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

// ─── Expresiones (WHERE y RETURN) ───

/// Operador de comparación binaria.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    /// `=` (igualdad).
    Eq,
    /// `<>` (distinto).
    NotEq,
    /// `<`.
    Lt,
    /// `<=`.
    Lte,
    /// `>`.
    Gt,
    /// `>=`.
    Gte,
}

impl CompareOp {
    /// Representación textual canónica (para `Display` del AST).
    pub fn as_str(self) -> &'static str {
        match self {
            CompareOp::Eq => "=",
            CompareOp::NotEq => "<>",
            CompareOp::Lt => "<",
            CompareOp::Lte => "<=",
            CompareOp::Gt => ">",
            CompareOp::Gte => ">=",
        }
    }
}

/// Expresión del lenguaje (usada en WHERE y RETURN).
///
/// Precedencia (de menor a mayor): `OR` < `AND` < `NOT` < comparación < primary.
/// El parser del cap.18 construirá este árbol respetando esa jerarquía.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Literal del lenguaje. Reutiliza el `Value` del cap.7 (Int/Float/String/
    /// Bool/Null) para no duplicar tipos.
    Literal { value: Value, span: Span },
    /// Referencia a una variable ligada (todo el nodo/relación). Se introduce
    /// en cap.18 para soportar el hito del brief `RETURN p` (Cypher permite
    /// retornar la variable entera, no sólo sus propiedades). La distingue de
    /// [`Expression::PropertyAccess`] en que no hay `.propiedad`.
    Variable { name: String, span: Span },
    /// Acceso a propiedad de una variable ligada: `p.name`.
    PropertyAccess {
        variable: String,
        property: String,
        span: Span,
    },
    /// Comparación binaria: `left op right`.
    Compare {
        op: CompareOp,
        left: Box<Expression>,
        right: Box<Expression>,
        span: Span,
    },
    /// `a AND b` (lógica; cortocircuito en el executor del cap.20).
    And {
        left: Box<Expression>,
        right: Box<Expression>,
        span: Span,
    },
    /// `a OR b`.
    Or {
        left: Box<Expression>,
        right: Box<Expression>,
        span: Span,
    },
    /// `NOT a`.
    Not { expr: Box<Expression>, span: Span },
}

impl Expression {
    /// Span que cubre toda la expresión.
    pub fn span(&self) -> Span {
        match self {
            Expression::Literal { span, .. }
            | Expression::Variable { span, .. }
            | Expression::PropertyAccess { span, .. }
            | Expression::Compare { span, .. }
            | Expression::And { span, .. }
            | Expression::Or { span, .. }
            | Expression::Not { span, .. } => *span,
        }
    }

    /// Constructor ergonómico para literales.
    pub fn lit(value: Value, span: Span) -> Self {
        Expression::Literal { value, span }
    }

    /// Constructor ergonómico para referencia a variable (hito cap.18).
    pub fn var(name: impl Into<String>, span: Span) -> Self {
        Expression::Variable {
            name: name.into(),
            span,
        }
    }

    /// Constructor ergonómico para acceso a propiedad.
    pub fn prop(variable: impl Into<String>, property: impl Into<String>, span: Span) -> Self {
        Expression::PropertyAccess {
            variable: variable.into(),
            property: property.into(),
            span,
        }
    }

    /// ¿Referencia la variable `name`? Útil para análisis semántico y
    /// para el planner del cap.19 (push-down de predicados por variable).
    pub fn references_var(&self, name: &str) -> bool {
        match self {
            Expression::Variable { name: n, .. } => n == name,
            Expression::PropertyAccess { variable, .. } => variable == name,
            Expression::Compare { left, right, .. } => {
                left.references_var(name) || right.references_var(name)
            }
            Expression::And { left, right, .. } | Expression::Or { left, right, .. } => {
                left.references_var(name) || right.references_var(name)
            }
            Expression::Not { expr, .. } => expr.references_var(name),
            Expression::Literal { .. } => false,
        }
    }

    /// Variables referenciadas (para validación semántica).
    pub(crate) fn variables(&self, out: &mut Vec<String>) {
        match self {
            Expression::Variable { name, .. } => {
                if !out.iter().any(|v| v == name) {
                    out.push(name.clone());
                }
            }
            Expression::PropertyAccess { variable, .. } => {
                if !out.iter().any(|v| v == variable) {
                    out.push(variable.clone());
                }
            }
            Expression::Compare { left, right, .. } => {
                left.variables(out);
                right.variables(out);
            }
            Expression::And { left, right, .. } | Expression::Or { left, right, .. } => {
                left.variables(out);
                right.variables(out);
            }
            Expression::Not { expr, .. } => expr.variables(out),
            Expression::Literal { .. } => {}
        }
    }
}

// ─── Patrones de camino (MATCH) ───

/// Dirección de una relación en el patrón.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelDirection {
    /// `-[:T]->` saliente (source → target).
    Outgoing,
    /// `<-[:T]-` entrante (target → source, visto desde el primer nodo).
    Incoming,
    /// `--` sin dirección (cualquier extremo).
    Undirected,
}

/// Patrón de nodo: `(variable:Label { props })`. Todas las partes opcionales.
#[derive(Debug, Clone, PartialEq)]
pub struct NodePattern {
    /// Variable ligada al nodo (e.g. `p`). `None` = nodo anónimo `()`.
    pub variable: Option<String>,
    /// Etiqueta exigida (e.g. `Person`). `None` = cualquier etiqueta.
    pub label: Option<String>,
    /// Propiedades literales exigidas en el patrón (inline predicates).
    pub properties: Vec<(String, Expression)>,
    pub span: Span,
}

impl NodePattern {
    /// Nodo anónimo sin restricciones: `()`.
    pub fn anonymous(span: Span) -> Self {
        Self {
            variable: None,
            label: None,
            properties: Vec::new(),
            span,
        }
    }

    /// Variable declarada por este patrón (si la tiene).
    pub fn declared_variable(&self) -> Option<&str> {
        self.variable.as_deref()
    }
}

/// Patrón de relación: `-[variable:TYPE]->`.
#[derive(Debug, Clone, PartialEq)]
pub struct RelationshipPattern {
    /// Variable ligada a la arista (e.g. `r`). `None` = anónima.
    pub variable: Option<String>,
    /// Tipo de relación exigido (e.g. `KNOWS`).
    pub rel_type: Option<String>,
    pub direction: RelDirection,
    pub span: Span,
}

impl RelationshipPattern {
    /// Relación saliente anónima sin tipo: `-[]->` (el patrón más común).
    pub fn outgoing_anonymous(span: Span) -> Self {
        Self {
            variable: None,
            rel_type: None,
            direction: RelDirection::Outgoing,
            span,
        }
    }

    /// Variable declarada por esta relación (si la tiene).
    pub fn declared_variable(&self) -> Option<&str> {
        self.variable.as_deref()
    }
}

/// Un camino del MATCH: `node (rel node)*`.
///
/// Ejemplo: `(p:Person)-[:KNOWS]->(f:Person)` →
/// `start = (p:Person)`, `chain = [(-[:KNOWS]->, (f:Person))]`.
#[derive(Debug, Clone, PartialEq)]
pub struct PathPattern {
    pub start: NodePattern,
    pub chain: Vec<(RelationshipPattern, NodePattern)>,
    pub span: Span,
}

impl PathPattern {
    /// Todos los patrones de nodo del camino (start + chain).
    pub fn node_patterns(&self) -> impl Iterator<Item = &NodePattern> {
        std::iter::once(&self.start).chain(self.chain.iter().map(|(_, n)| n))
    }

    /// Variables declaradas por los nodos de este camino.
    pub fn node_variables(&self) -> Vec<String> {
        let mut out = Vec::new();
        for n in self.node_patterns() {
            if let Some(v) = &n.variable {
                out.push(v.clone());
            }
        }
        out
    }

    /// Variables declaradas por las relaciones de este camino.
    pub fn edge_variables(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (r, _) in &self.chain {
            if let Some(v) = &r.variable {
                out.push(v.clone());
            }
        }
        out
    }
}

// ─── Cláusulas ───

/// `MATCH pattern, pattern, ...`.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchClause {
    /// Uno o más caminos separados por coma. Mínimo 1 (validado).
    pub patterns: Vec<PathPattern>,
    pub span: Span,
}

impl MatchClause {
    /// Variables ligadas a nodos en TODOS los patrones del MATCH.
    pub fn bound_node_variables(&self) -> Vec<String> {
        let mut out = Vec::new();
        for p in &self.patterns {
            for v in p.node_variables() {
                if !out.iter().any(|x| x == &v) {
                    out.push(v);
                }
            }
        }
        out
    }

    /// Variables ligadas a aristas en TODOS los patrones del MATCH.
    pub fn bound_edge_variables(&self) -> Vec<String> {
        let mut out = Vec::new();
        for p in &self.patterns {
            for v in p.edge_variables() {
                if !out.iter().any(|x| x == &v) {
                    out.push(v);
                }
            }
        }
        out
    }
}

/// `WHERE expression`. Opcional en la consulta.
#[derive(Debug, Clone, PartialEq)]
pub struct WhereClause {
    pub expr: Expression,
    pub span: Span,
}

/// `RETURN item, item, ...` con `item = expr [AS alias]`.
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnItem {
    pub expr: Expression,
    /// Alias explícito (`f.name AS edad` o `f.name edad`). `None` = sin alias.
    pub alias: Option<String>,
    pub span: Span,
}

/// `RETURN ...`. Siempre presente en LiraQL cap.17.
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnClause {
    /// Mínimo 1 item (validado).
    pub items: Vec<ReturnItem>,
    pub span: Span,
}

// ─── AstNode: el enum del hito del brief ───

/// Bloque de construcción del AST.
///
/// El brief (§cap 17) fija el hito:
///
/// ```text
/// pub enum AstNode {
///     Match(MatchClause),
///     Where(Expression),
///     Return(ReturnClause),
/// }
/// ```
///
/// Una `Query` real siempre es `Match → Where? → Return`, pero exponer los
/// nodos como enum permite construir sub-árboles en tests y en el planner
/// del cap.19 (que opera cláusula a cláusula).
#[derive(Debug, Clone, PartialEq)]
pub enum AstNode {
    Match(MatchClause),
    Where(WhereClause),
    Return(ReturnClause),
}

// ─── Query: la consulta completa ───

/// Una consulta LiraQL completa: `MATCH ... [WHERE ...] RETURN ...`.
///
/// Invariante (validada por [`Query::validate`]):
///   - `match_clause` tiene ≥ 1 patrón con ≥ 1 nodo.
///   - Toda variable usada en WHERE/RETURN está declarada en MATCH.
///   - No hay variables duplicadas en un mismo MATCH.
///   - `return_clause` tiene ≥ 1 item.
#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub match_clause: MatchClause,
    pub where_clause: Option<WhereClause>,
    pub return_clause: ReturnClause,
    pub span: Span,
}

// ─── Errores tipados ───

/// Tipo de error semántico de una consulta LiraQL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryErrorKind {
    /// El MATCH está vacío (sin patrones).
    EmptyMatch,
    /// Un patrón de nodo del MATCH no declara ni variable ni label ni props
    /// (es `()` puro). Permitido en Cypher pero inútil pedagógicamente.
    EmptyNodePattern,
    /// Variable declarada dos veces en el mismo MATCH.
    DuplicateVariable { variable: String },
    /// Variable usada en WHERE/RETURN pero no declarada en MATCH.
    UnknownVariable { variable: String },
    /// RETURN sin items.
    EmptyReturn,
    /// Alias vacío en un ReturnItem.
    EmptyAlias,
}

/// Error de validación de una consulta, con la posición del fuente.
///
/// El patrón `{ kind, span }` es el mismo que usan rustc/miette: el `kind`
/// dice *qué* pasó y el `span` dice *dónde*. El lexer del cap.18 rellena
/// los spans; aquí los constructores de tests los simulan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryError {
    pub kind: QueryErrorKind,
    pub span: Span,
}

impl QueryError {
    pub fn new(kind: QueryErrorKind, span: Span) -> Self {
        Self { kind, span }
    }
}

impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            QueryErrorKind::EmptyMatch => {
                write!(f, "MATCH vacío: se requiere al menos un patrón de nodo")
            }
            QueryErrorKind::EmptyNodePattern => write!(
                f,
                "patrón de nodo vacío '()': declare una variable, label o propiedad"
            ),
            QueryErrorKind::DuplicateVariable { variable } => {
                write!(f, "variable '{variable}' declarada dos veces en MATCH")
            }
            QueryErrorKind::UnknownVariable { variable } => {
                write!(f, "variable '{variable}' usada pero no declarada en MATCH")
            }
            QueryErrorKind::EmptyReturn => {
                write!(f, "RETURN vacío: se requiere al menos una expresión")
            }
            QueryErrorKind::EmptyAlias => write!(f, "alias vacío en RETURN"),
        }?;
        // Incluir el span como ayuda de localización (estilo rustc).
        if !self.span.is_empty() {
            write!(f, " (en {}..{})", self.span.start, self.span.end)
        } else {
            write!(f, " (en offset {})", self.span.start)
        }
    }
}

impl std::error::Error for QueryError {}

// ─── Validación semántica ───

impl Query {
    /// Valida la consulta y devuelve la lista de errores encontrados.
    ///
    /// Vacía = consulta bien formada semánticamente. No es un parser: asume
    /// que la estructura del AST ya es sintácticamente correcta (cap.18).
    /// Aquí comprobamos las reglas de *alcance* de variables y los mínimos.
    pub fn validate(&self) -> Vec<QueryError> {
        let mut errors = Vec::new();

        // 1. MATCH no vacío.
        if self.match_clause.patterns.is_empty() {
            errors.push(QueryError::new(
                QueryErrorKind::EmptyMatch,
                self.match_clause.span,
            ));
            return errors; // Sin patrones no podemos validar variables.
        }

        // 2. Patrones de nodo no triviales + variables duplicadas.
        let mut declared: Vec<String> = Vec::new();
        for path in &self.match_clause.patterns {
            for node in path.node_patterns() {
                let trivial =
                    node.variable.is_none() && node.label.is_none() && node.properties.is_empty();
                if trivial {
                    errors.push(QueryError::new(QueryErrorKind::EmptyNodePattern, node.span));
                }
                if let Some(v) = &node.variable {
                    if declared.iter().any(|x| x == v) {
                        errors.push(QueryError::new(
                            QueryErrorKind::DuplicateVariable {
                                variable: v.clone(),
                            },
                            node.span,
                        ));
                    } else {
                        declared.push(v.clone());
                    }
                }
            }
            // Las variables de arista también entran en el alcance y no se
            // pueden duplicar (ni contra nodos ni contra otras aristas).
            for (rel, _node) in &path.chain {
                if let Some(v) = &rel.variable {
                    if declared.iter().any(|x| x == v) {
                        errors.push(QueryError::new(
                            QueryErrorKind::DuplicateVariable {
                                variable: v.clone(),
                            },
                            rel.span,
                        ));
                    } else {
                        declared.push(v.clone());
                    }
                }
            }
        }

        // 3. RETURN no vacío.
        if self.return_clause.items.is_empty() {
            errors.push(QueryError::new(
                QueryErrorKind::EmptyReturn,
                self.return_clause.span,
            ));
        }

        // 4. Alias vacíos.
        for item in &self.return_clause.items {
            if let Some(alias) = &item.alias
                && alias.trim().is_empty()
            {
                errors.push(QueryError::new(QueryErrorKind::EmptyAlias, item.span));
            }
        }

        // 5. Variables usadas en WHERE/RETURN deben estar declaradas.
        if let Some(where_c) = &self.where_clause {
            let mut used = Vec::new();
            where_c.expr.variables(&mut used);
            for v in used {
                if !declared.iter().any(|d| d == &v) {
                    errors.push(QueryError::new(
                        QueryErrorKind::UnknownVariable { variable: v },
                        where_c.expr.span(),
                    ));
                }
            }
        }
        for item in &self.return_clause.items {
            let mut used = Vec::new();
            item.expr.variables(&mut used);
            for v in used {
                if !declared.iter().any(|d| d == &v) {
                    errors.push(QueryError::new(
                        QueryErrorKind::UnknownVariable { variable: v },
                        item.expr.span(),
                    ));
                }
            }
        }

        errors
    }

    /// ¿Es semánticamente válida? (atajo de `self.validate().is_empty()`).
    pub fn is_valid(&self) -> bool {
        self.validate().is_empty()
    }

    /// Variables ligadas a nodos en el MATCH (alcance visible en WHERE/RETURN).
    pub fn bound_node_variables(&self) -> Vec<String> {
        self.match_clause.bound_node_variables()
    }

    /// Variables ligadas a aristas en el MATCH.
    pub fn bound_edge_variables(&self) -> Vec<String> {
        self.match_clause.bound_edge_variables()
    }
}

// ─── Pretty-printer (Display) del AST ───
//
// Regenera una representación canónica de la consulta. Útil para:
//   1. Tests: comparar AST esperado vs parseado (cap.18).
//   2. `liradb explain` (cap.21): mostrar la consulta normalizada.
//   3. Round-trip: parse(display(ast)) debe ser idempotente.
//
// La salida NO conserva whitespace/commas originales; produce una forma
// canónica con indentación consistente.

impl std::fmt::Display for CompareOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::fmt::Display for RelDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelDirection::Outgoing => f.write_str("OUTGOING"),
            RelDirection::Incoming => f.write_str("INCOMING"),
            RelDirection::Undirected => f.write_str("UNDIRECTED"),
        }
    }
}

impl std::fmt::Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expression::Literal { value, .. } => display_value(f, value),
            Expression::Variable { name, .. } => f.write_str(name),
            Expression::PropertyAccess {
                variable, property, ..
            } => write!(f, "{variable}.{property}"),
            Expression::Compare {
                op, left, right, ..
            } => write!(f, "({left} {op} {right})"),
            Expression::And { left, right, .. } => write!(f, "({left} AND {right})"),
            Expression::Or { left, right, .. } => write!(f, "({left} OR {right})"),
            Expression::Not { expr, .. } => write!(f, "(NOT {expr})"),
        }
    }
}

/// Formatea un `Value` como literal de LiraQL.
pub(crate) fn display_value(f: &mut std::fmt::Formatter<'_>, value: &Value) -> std::fmt::Result {
    match value {
        Value::Null => f.write_str("NULL"),
        Value::Bool(b) => {
            if *b {
                f.write_str("TRUE")
            } else {
                f.write_str("FALSE")
            }
        }
        Value::Int(i) => write!(f, "{i}"),
        Value::Float(x) => write!(f, "{x}"),
        Value::String(s) => write!(f, "\"{s}\""),
        Value::Bytes(b) => write!(f, "0x{}", hex_bytes(b)),
    }
}

/// Hex mínimo sin dependencias (para `Value::Bytes` en Display).
pub(crate) fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[char] = &[
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
    ];
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize]);
        s.push(HEX[(b & 0x0f) as usize]);
    }
    s
}

impl std::fmt::Display for NodePattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("(")?;
        if let Some(v) = &self.variable {
            f.write_str(v)?;
        }
        if let Some(l) = &self.label {
            write!(f, ":{l}")?;
        }
        if !self.properties.is_empty() {
            f.write_str(" {")?;
            for (i, (k, v)) in self.properties.iter().enumerate() {
                if i > 0 {
                    f.write_str(", ")?;
                }
                write!(f, "{k}: {v}")?;
            }
            f.write_str("}")?;
        }
        f.write_str(")")
    }
}

impl std::fmt::Display for RelationshipPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.direction {
            RelDirection::Outgoing => {
                f.write_str("-[")?;
                display_rel_inner(f, self)?;
                f.write_str("]->")
            }
            RelDirection::Incoming => {
                f.write_str("<-[")?;
                display_rel_inner(f, self)?;
                f.write_str("]-")
            }
            RelDirection::Undirected => {
                f.write_str("-[")?;
                display_rel_inner(f, self)?;
                f.write_str("]-")
            }
        }
    }
}

fn display_rel_inner(f: &mut std::fmt::Formatter<'_>, r: &RelationshipPattern) -> std::fmt::Result {
    if let Some(v) = &r.variable {
        f.write_str(v)?;
    }
    if let Some(t) = &r.rel_type {
        write!(f, ":{t}")?;
    }
    Ok(())
}

impl std::fmt::Display for PathPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.start)?;
        for (rel, node) in &self.chain {
            write!(f, "{rel}{node}")?;
        }
        Ok(())
    }
}

impl std::fmt::Display for MatchClause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MATCH ")?;
        for (i, p) in self.patterns.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{p}")?;
        }
        Ok(())
    }
}

impl std::fmt::Display for WhereClause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WHERE {}", self.expr)
    }
}

impl std::fmt::Display for ReturnItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.expr)?;
        if let Some(a) = &self.alias {
            write!(f, " AS {a}")?;
        }
        Ok(())
    }
}

impl std::fmt::Display for ReturnClause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RETURN ")?;
        for (i, item) in self.items.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{item}")?;
        }
        Ok(())
    }
}

impl std::fmt::Display for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.match_clause)?;
        if let Some(w) = &self.where_clause {
            write!(f, " {w}")?;
        }
        write!(f, " {}", self.return_clause)
    }
}
