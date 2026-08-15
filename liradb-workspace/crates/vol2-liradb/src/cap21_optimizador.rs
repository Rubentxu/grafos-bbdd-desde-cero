use crate::cap07_modelo::{NodeId, Value};
use crate::cap08_graph_store::GraphStore;
use crate::cap17_liraql_ast::{CompareOp, RelDirection};
use crate::cap18_lexer_parser::parse;
use crate::cap19_plan_logico::{LogicalPlan, ScalarExpr};
use crate::cap20_volcano::{ExecError, Executor};

// ─────────────────── Cap 21: Un optimizador pequeño pero real ───────────────────
//
// Los caps. 17-20 cerraron la cadena `texto → AST → plan lógico → filas`, pero
// con un plan INGENUO: el `Filter` queda encima de todo (cap 19 lo dejó ahí a
// propósito) y el executor lo obedece tal cual. Este capítulo introduce el
// OPTIMIZADOR: un programa que REESCRIBE el plan antes de ejecutarlo para que
// haga el mismo trabajo con menos esfuerzo. El hito del brief (§cap 21):
//
// ```text
//   liradb explain "MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE f.age < 40 ..."
//
//   Plan ANTES:  Filter encima de Expand encima de NodeScan(Person)
//   Plan DESPUÉS: la consulta empieza por el lado selectivo (f:Person con el
//                 filtro de edad pegado al escaneo) y expande hacia atrás
// ```
//
// Las tres piezas del capítulo (brief + CORPUS "predicate pushdown, join
// ordering, projection pruning"):
//
//   1. **Estadísticas** ([`Catalog`]): una pasada por el `GraphStore` recolecta
//      nodos por etiqueta, grado medio (saliente/entrante) por etiqueta,
//      aristas por tipo y un ÍNDICE DE IGUALDAD (etiqueta, propiedad, valor)
//      → ids. Es el catálogo con el que se estiman y deciden las reglas.
//
//   2. **Estimación de cardinalidad** ([`estimate`]): filas estimadas por
//      operador con heurísticas simples y documentadas — selectividad fija
//      por tipo de predicado (estilo System R) cuando no hay estadística,
//      fracción exacta cuando el índice de igualdad la da, grados medios para
//      los `Expand`.
//
//   3. **Optimizador basado en reglas** ([`optimize`]): cinco reescrituras
//      aplicadas en orden fijo sobre el `LogicalPlan` del cap 19:
//        R1 `rule_selective_start` — elegir el punto inicial más selectivo y
//            reordenar expansiones sencillas (el "join ordering" de grafos).
//        R2 `rule_predicate_pushdown` — bajar los `Filter` hacia los
//            `NodeScan`/`Expand` (dividiendo los AND en átomos).
//        R3 `rule_absorb_label` — el `HasLabel` del nodo escaneado se integra
//            en la etiqueta del `NodeScan` (el "índice de etiquetas").
//        R4 `rule_index_seek` — `Filter(prop = literal) + NodeScan` se
//            convierte en `IndexSeek` con los ids del catálogo (el "usar
//            índices" del brief; su ejemplo canónico).
//        R5 `rule_prune_projections` — elimina proyecciones innecesarias
//            (fusiona `Project` anidados de identidad).
//
// Contrato de equivalencia: las reglas reordenan LIGADURAS, no semántica —
// `Project` re-evalúa por NOMBRE, así que el resultado es el mismo
// multiconjunto de filas. Lo que NO se preserva es el ORDEN de las filas:
// sin `ORDER BY` (que LiraQL aún no tiene) el orden no es parte del
// contrato, exactamente como en SQL. Los tests de equivalencia comparan
// columnas + filas ordenadas.
//
// Errores: el optimizador en sí no falla (las reescrituras son totales);
// [`explain`] puede fallar al parsear/planificar/ejecutar y devuelve
// [`ExecError`] como el resto del pipeline.

// ─── Catálogo: estadísticas recolectadas del store ───

/// Estadísticas de una etiqueta de nodo.
///
/// Los grados se acumulan POR ETIQUETA contando las aristas que tocan nodos
/// con esa etiqueta (un nodo multi-etiqueta contribuye a cada una de ellas).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LabelStats {
    /// Nodos con esta etiqueta.
    pub nodes: u64,
    /// Suma de grados salientes (para estimar `Expand` OUTGOING).
    pub out_degree_sum: u64,
    /// Suma de grados entrantes (para estimar `Expand` INCOMING).
    pub in_degree_sum: u64,
}

impl LabelStats {
    /// Grado saliente medio (0 si no hay nodos con la etiqueta).
    pub fn avg_out_degree(&self) -> f64 {
        if self.nodes == 0 {
            0.0
        } else {
            self.out_degree_sum as f64 / self.nodes as f64
        }
    }

    /// Grado entrante medio (0 si no hay nodos con la etiqueta).
    pub fn avg_in_degree(&self) -> f64 {
        if self.nodes == 0 {
            0.0
        } else {
            self.in_degree_sum as f64 / self.nodes as f64
        }
    }
}

/// Una entrada del índice de igualdad: `(etiqueta, propiedad, valor) → ids`.
///
/// `label: None` es la entrada COMODÍN (sin restricción de etiqueta): los
/// mismos ids indexados también bajo cada etiqueta del nodo. El orden de los
/// ids es el del store — determinista, requisito para que `IndexSeek` produzca
/// las filas en el mismo orden que el `NodeScan` + `Filter` que sustituye.
#[derive(Debug, Clone, PartialEq)]
pub struct EqIndexEntry {
    pub label: Option<String>,
    pub property: String,
    pub value: Value,
    pub ids: Vec<NodeId>,
}

/// El catálogo del optimizador: estadísticas + índice de igualdad.
///
/// Se recolecta con UNA pasada por el store ([`Catalog::collect`]). En un
/// sistema real el catálogo viviría en disco y se mantendría incrementalmente
/// (los índices del cap. 15 son la infraestructura natural); aquí se
/// reconstruye por consulta — el coste es un escaneo, y a cambio el capítulo
/// tiene un catálogo obviamente correcto del que razonar.
///
/// Sin `PartialEq` deliberadamente: el índice de igualdad itera un `HashMap`
/// de propiedades y su ORDEN interno no es determinista (los ids de cada
/// entrada sí lo son, que es lo que importa). Compárese por sus consultas.
pub struct Catalog {
    /// Nodos totales del store.
    pub total_nodes: u64,
    /// Aristas totales del store.
    pub total_edges: u64,
    /// Estadísticas por etiqueta, en orden de aparición.
    per_label: Vec<(String, LabelStats)>,
    /// Aristas por tipo (etiqueta de arista), en orden de aparición.
    edges_per_type: Vec<(String, u64)>,
    /// Índice de igualdad (etiqueta, propiedad, valor) → ids.
    eq_index: Vec<EqIndexEntry>,
}

impl Catalog {
    /// Recolecta las estadísticas del store en una pasada.
    ///
    /// Recorre los nodos una vez (recuentos por etiqueta + índice de
    /// igualdad) y las aristas una vez (tipos + grados por etiqueta de los
    /// extremos). `&dyn GraphStore` (cap 8): funciona con `MemoryStore` y
    /// con cualquier store futuro sin cambios.
    pub fn collect(store: &dyn GraphStore) -> Self {
        let mut catalog = Catalog {
            total_nodes: 0,
            total_edges: 0,
            per_label: Vec::new(),
            edges_per_type: Vec::new(),
            eq_index: Vec::new(),
        };
        for node in store.iter_nodes() {
            catalog.total_nodes += 1;
            for label in &node.labels {
                label_entry(&mut catalog.per_label, label).nodes += 1;
            }
            // Índice de igualdad: bajo cada etiqueta del nodo y bajo el
            // comodín (sin etiqueta). Valor por valor (Value: PartialEq).
            for (property, value) in &node.props {
                for label in &node.labels {
                    eq_push(&mut catalog.eq_index, Some(label), property, value, node.id);
                }
                eq_push(&mut catalog.eq_index, None, property, value, node.id);
            }
        }
        for edge in store.iter_edges() {
            catalog.total_edges += 1;
            if let Some(count) = edges_per_type_entry(&mut catalog.edges_per_type, &edge.label) {
                *count += 1;
            }
            // Grados por etiqueta de los extremos (un extremo multi-etiqueta
            // contribuye a cada etiqueta: el Expand no sabe aún cuál aplica).
            if let Some(source) = store.get_node(edge.source) {
                for label in &source.labels {
                    label_entry(&mut catalog.per_label, label).out_degree_sum += 1;
                }
            }
            if let Some(target) = store.get_node(edge.target) {
                for label in &target.labels {
                    label_entry(&mut catalog.per_label, label).in_degree_sum += 1;
                }
            }
        }
        catalog
    }

    /// Estadísticas de una etiqueta (vacías si no existe en el store).
    pub fn label_stats(&self, label: &str) -> LabelStats {
        self.per_label
            .iter()
            .find(|(l, _)| l == label)
            .map(|(_, s)| s.clone())
            .unwrap_or_default()
    }

    /// Nodos estimados de un escaneo: con la etiqueta, los nodos que la
    /// tienen; sin etiqueta (`None` = ANY), todos.
    pub fn nodes_with_label(&self, label: Option<&str>) -> u64 {
        match label {
            Some(label) => self.label_stats(label).nodes,
            None => self.total_nodes,
        }
    }

    /// Aristas de un tipo (0 si el tipo no existe).
    pub fn edges_of_type(&self, rel_type: &str) -> u64 {
        self.edges_per_type
            .iter()
            .find(|(t, _)| t == rel_type)
            .map(|(_, n)| *n)
            .unwrap_or(0)
    }

    /// Grado medio para recorrer una adyacencia: por etiqueta del nodo de
    /// partida y dirección. `None` = sin etiqueta conocida → media GLOBAL
    /// (aristas/nodos), la mejor suposición sin estadística.
    pub fn avg_degree(&self, label: Option<&str>, direction: RelDirection) -> f64 {
        let stats = label.map(|l| self.label_stats(l));
        match stats {
            Some(stats) => match direction {
                RelDirection::Outgoing => stats.avg_out_degree(),
                RelDirection::Incoming => stats.avg_in_degree(),
                // Sin dirección se recorren las dos adyacencias.
                RelDirection::Undirected => stats.avg_out_degree() + stats.avg_in_degree(),
            },
            None => {
                if self.total_nodes == 0 {
                    0.0
                } else {
                    let global = self.total_edges as f64 / self.total_nodes as f64;
                    match direction {
                        RelDirection::Outgoing | RelDirection::Incoming => global,
                        RelDirection::Undirected => 2.0 * global,
                    }
                }
            }
        }
    }

    /// Fracción de aristas que son del tipo dado: aproxima cuántas de las
    /// adyacencias medias sobreviven al filtro `rel_type` del `Expand`.
    /// Sin tipo (`None`) no hay filtro: 1.0.
    pub fn rel_type_fraction(&self, rel_type: Option<&str>) -> f64 {
        match rel_type {
            Some(rel_type) if self.total_edges > 0 => {
                self.edges_of_type(rel_type) as f64 / self.total_edges as f64
            }
            _ => 1.0,
        }
    }

    /// Consulta el índice de igualdad: ids de los nodos con la etiqueta (si
    /// la hay) cuya propiedad vale exactamente `value`. Vacío si la clave no
    /// existe (el valor no ocurre en el store) — que es exactamente la
    /// cardinalidad correcta de esa igualdad.
    ///
    /// Simplificación documentada: igualdad EXACTA de `Value` (sin la
    /// promoción Int/Float de la ejecución; `p.age = 36.0` no encuentra el
    /// `36` almacenado). Las estadísticas estiman; la ejecución decide.
    pub fn equality_lookup(
        &self,
        label: Option<&str>,
        property: &str,
        value: &Value,
    ) -> Vec<NodeId> {
        self.eq_index
            .iter()
            .find(|entry| {
                entry.label.as_deref() == label
                    && entry.property == property
                    && &entry.value == value
            })
            .map(|entry| entry.ids.clone())
            .unwrap_or_default()
    }
}

/// Entrada (o creación) de las estadísticas de una etiqueta.
fn label_entry<'a>(
    per_label: &'a mut Vec<(String, LabelStats)>,
    label: &str,
) -> &'a mut LabelStats {
    if let Some(index) = per_label.iter().position(|(l, _)| l == label) {
        return &mut per_label[index].1;
    }
    per_label.push((label.to_string(), LabelStats::default()));
    let last = per_label.len() - 1;
    &mut per_label[last].1
}

/// Entrada (o creación) del contador de un tipo de arista.
fn edges_per_type_entry<'a>(
    edges_per_type: &'a mut Vec<(String, u64)>,
    rel_type: &str,
) -> Option<&'a mut u64> {
    if let Some(index) = edges_per_type.iter().position(|(t, _)| t == rel_type) {
        return Some(&mut edges_per_type[index].1);
    }
    edges_per_type.push((rel_type.to_string(), 0));
    let last = edges_per_type.len() - 1;
    Some(&mut edges_per_type[last].1)
}

/// Añade un id a la entrada del índice de igualdad (creándola si no existe).
fn eq_push(
    eq_index: &mut Vec<EqIndexEntry>,
    label: Option<&str>,
    property: &str,
    value: &Value,
    id: NodeId,
) {
    if let Some(entry) = eq_index.iter_mut().find(|entry| {
        entry.label.as_deref() == label && entry.property == property && &entry.value == value
    }) {
        if !entry.ids.contains(&id) {
            entry.ids.push(id);
        }
        return;
    }
    eq_index.push(EqIndexEntry {
        label: label.map(str::to_string),
        property: property.to_string(),
        value: value.clone(),
        ids: vec![id],
    });
}

// ─── Selectividad: heurísticas por tipo de predicado ───

/// Selectividad de una igualdad sin estadística (default clásico de System R).
pub const SEL_EQ: f64 = 0.1;
/// Selectividad de una desigualdad `<>` (el complemento razonable de SEL_EQ).
pub const SEL_NOT_EQ: f64 = 0.9;
/// Selectividad de un rango `<`, `<=`, `>`, `>=` (un tercio, System R).
pub const SEL_RANGE: f64 = 1.0 / 3.0;
/// Selectividad de un predicado que no sabemos tipar (neutral).
pub const SEL_UNKNOWN: f64 = 0.5;

/// Entorno de etiquetas: variable → etiqueta declarada para ella.
///
/// Se extrae del propio plan (etiqueta del `NodeScan`/`IndexSeek` y
/// `HasLabel` de los `Filter`). Como el binder del cap. 19 garantiza
/// variables únicas por consulta, el entorno es global y sin ambigüedad.
pub type LabelEnv = Vec<(String, String)>;

/// Etiqueta declarada para una variable en el entorno (la primera gana).
fn env_label<'a>(env: &'a LabelEnv, variable: &str) -> Option<&'a str> {
    env.iter()
        .find(|(v, _)| v == variable)
        .map(|(_, l)| l.as_str())
}

/// Extrae el entorno de etiquetas de un plan: etiquetas de los escaneos y
/// predicados `HasLabel` de los filtros, en pre-orden (el primero gana).
pub fn label_env(plan: &LogicalPlan) -> LabelEnv {
    let mut env = LabelEnv::new();
    collect_label_env(plan, &mut env);
    env
}

fn collect_label_env(plan: &LogicalPlan, env: &mut LabelEnv) {
    match plan {
        LogicalPlan::NodeScan { variable, label }
        | LogicalPlan::IndexSeek {
            variable, label, ..
        } => {
            if let Some(label) = label
                && env_label(env, variable).is_none()
            {
                env.push((variable.clone(), label.clone()));
            }
        }
        LogicalPlan::Expand { input, .. }
        | LogicalPlan::Filter { input, .. }
        | LogicalPlan::Project { input, .. } => collect_label_env(input, env),
        LogicalPlan::CartesianProduct { left, right } => {
            collect_label_env(left, env);
            collect_label_env(right, env);
        }
    }
    if let LogicalPlan::Filter { predicate, .. } = plan {
        let mut atoms = Vec::new();
        split_and(predicate, &mut atoms);
        for atom in atoms {
            if let ScalarExpr::HasLabel { variable, label } = atom
                && env_label(env, &variable).is_none()
            {
                env.push((variable, label));
            }
        }
    }
}

/// Selectividad de un predicado: fracción de filas que se espera que sobrevivan.
///
/// Heurísticas (documentadas, deliberadamente simples):
/// - `HasLabel(v:L)` con la etiqueta de `v` ya declarada en el entorno: 1.0
///   (la declara el propio patrón); con etiqueta distinta: 0.0
///   (contradicción); sin declarar: la fracción de nodos con `L`.
/// - Igualdad `v.prop = literal`: fracción EXACTA si el índice de igualdad
///   la conoce (`ids / nodos de la etiqueta`); 0.0 si el valor no ocurre
///   (la clave no existe); si no es consultable, el default de System R.
/// - Rangos: 1/3 · `<>`: 0.9 · literales booleanos: 1.0/0.0 · resto: 0.5.
/// - `AND` multiplica (independencia), `OR` inclusión-exclusión, `NOT`
///   complementa. Siempre acotado a [0, 1].
pub fn selectivity(pred: &ScalarExpr, catalog: &Catalog, env: &LabelEnv) -> f64 {
    let sel = match pred {
        ScalarExpr::Literal(Value::Bool(true)) => 1.0,
        ScalarExpr::Literal(Value::Bool(false)) | ScalarExpr::Literal(Value::Null) => 0.0,
        ScalarExpr::Literal(_) => SEL_UNKNOWN,
        // Variable o propiedad usadas como condición (schemaless): neutral.
        ScalarExpr::Var { .. } | ScalarExpr::Property { .. } => SEL_UNKNOWN,
        ScalarExpr::HasLabel { variable, label } => match env_label(env, variable) {
            Some(known) if known == label => 1.0,
            Some(_) => 0.0,
            None => {
                let total = catalog.total_nodes.max(1) as f64;
                (catalog.nodes_with_label(Some(label)) as f64 / total).min(1.0)
            }
        },
        ScalarExpr::Compare { op, left, right } => {
            compare_selectivity(*op, left, right, catalog, env)
        }
        ScalarExpr::And { left, right } => {
            selectivity(left, catalog, env) * selectivity(right, catalog, env)
        }
        ScalarExpr::Or { left, right } => {
            let (a, b) = (
                selectivity(left, catalog, env),
                selectivity(right, catalog, env),
            );
            a + b - a * b
        }
        ScalarExpr::Not { expr } => 1.0 - selectivity(expr, catalog, env),
    };
    sel.clamp(0.0, 1.0)
}

fn compare_selectivity(
    op: CompareOp,
    left: &ScalarExpr,
    right: &ScalarExpr,
    catalog: &Catalog,
    env: &LabelEnv,
) -> f64 {
    // Igualdad `v.prop = literal` (en cualquier orden): estadística si existe.
    let prop_literal = match (left, right) {
        (ScalarExpr::Property { variable, property }, ScalarExpr::Literal(value))
        | (ScalarExpr::Literal(value), ScalarExpr::Property { variable, property }) => {
            Some((variable, property, value))
        }
        _ => None,
    };
    if op == CompareOp::Eq
        && let Some((variable, property, value)) = prop_literal
    {
        let ids = catalog.equality_lookup(env_label(env, variable), property, value);
        let base = catalog.nodes_with_label(env_label(env, variable)).max(1) as f64;
        return (ids.len() as f64 / base).min(1.0);
    }
    match op {
        CompareOp::Eq => SEL_EQ,
        CompareOp::NotEq => SEL_NOT_EQ,
        CompareOp::Lt | CompareOp::Lte | CompareOp::Gt | CompareOp::Gte => SEL_RANGE,
    }
}

// ─── Estimación de cardinalidad ───

/// Filas estimadas que producirá un plan (raíz) sobre el store del catálogo.
///
/// ```text
///   NodeScan          → nodos con la etiqueta (todos si ANY)
///   IndexSeek         → ids resueltos (exacto)
///   Filter            → entrada × selectividad del predicado
///   Expand            → entrada × grado medio de la dirección × fracción del tipo
///   Project           → lo que produce su entrada
///   CartesianProduct  → izquierda × derecha
/// ```
pub fn estimate(plan: &LogicalPlan, catalog: &Catalog) -> f64 {
    estimate_with(plan, catalog, &label_env(plan))
}

fn estimate_with(plan: &LogicalPlan, catalog: &Catalog, env: &LabelEnv) -> f64 {
    match plan {
        LogicalPlan::NodeScan { label, .. } => catalog.nodes_with_label(label.as_deref()) as f64,
        LogicalPlan::IndexSeek { ids, .. } => ids.len() as f64,
        LogicalPlan::Filter { input, predicate } => {
            estimate_with(input, catalog, env) * selectivity(predicate, catalog, env)
        }
        LogicalPlan::Expand {
            input,
            from,
            rel_type,
            direction,
            ..
        } => {
            estimate_with(input, catalog, env)
                * catalog.avg_degree(env_label(env, from), *direction)
                * catalog.rel_type_fraction(rel_type.as_deref())
        }
        LogicalPlan::Project { input, .. } => estimate_with(input, catalog, env),
        LogicalPlan::CartesianProduct { left, right } => {
            estimate_with(left, catalog, env) * estimate_with(right, catalog, env)
        }
    }
}

/// Estimación redondeada para mostrar: 0 si no hay nada, y como mínimo 1
/// para cualquier estimación positiva (las fracciones no pintan medias filas).
fn estimacion_mostrada(x: f64) -> u64 {
    if x <= 0.0 {
        0
    } else {
        x.round().max(1.0) as u64
    }
}

// ─── Utilidades sobre predicados ───

/// Añade sin duplicar.
fn push_unique(out: &mut Vec<String>, name: &str) {
    if !out.iter().any(|v| v == name) {
        out.push(name.to_string());
    }
}

/// Parte un predicado en sus átomos conjuntivos (aplanan los `AND`).
///
/// `a AND (b AND c)` → `[a, b, c]`. El orden se conserva — determinismo
/// para el explain y para los tests.
fn split_and(pred: &ScalarExpr, out: &mut Vec<ScalarExpr>) {
    match pred {
        ScalarExpr::And { left, right } => {
            split_and(left, out);
            split_and(right, out);
        }
        other => out.push(other.clone()),
    }
}

/// Variables que menciona un predicado, en orden de aparición y sin duplicar.
fn pred_variables(pred: &ScalarExpr, out: &mut Vec<String>) {
    match pred {
        ScalarExpr::Literal(_) => {}
        ScalarExpr::Var { name, .. } => push_unique(out, name),
        ScalarExpr::Property { variable, .. } | ScalarExpr::HasLabel { variable, .. } => {
            push_unique(out, variable)
        }
        ScalarExpr::Compare { left, right, .. }
        | ScalarExpr::And { left, right }
        | ScalarExpr::Or { left, right } => {
            pred_variables(left, out);
            pred_variables(right, out);
        }
        ScalarExpr::Not { expr } => pred_variables(expr, out),
    }
}

/// ¿El predicado sólo menciona variables de `allowed`? (un átomo sin
/// variables — un literal — se considera movible: es constante en todo nodo).
fn references_only(pred: &ScalarExpr, allowed: &[String]) -> bool {
    let mut vars = Vec::new();
    pred_variables(pred, &mut vars);
    vars.iter().all(|v| allowed.contains(v))
}

/// Reparte los átomos en (mencionan sólo `allowed`, el resto), conservando el
/// orden relativo en cada partición.
fn partition_atoms(
    atoms: Vec<ScalarExpr>,
    allowed: &[String],
) -> (Vec<ScalarExpr>, Vec<ScalarExpr>) {
    let mut inside = Vec::new();
    let mut outside = Vec::new();
    for atom in atoms {
        if references_only(&atom, allowed) {
            inside.push(atom);
        } else {
            outside.push(atom);
        }
    }
    (inside, outside)
}

/// Envuelve un plan con `Filter` salvo que no queden átomos (entonces el
/// `Filter` desaparece: proyección de filtro vacío).
fn wrap_filter(input: LogicalPlan, atoms: Vec<ScalarExpr>) -> LogicalPlan {
    match ScalarExpr::and_all(atoms) {
        Some(predicate) => LogicalPlan::Filter {
            input: Box::new(input),
            predicate,
        },
        None => input,
    }
}

/// Extrae `(propiedad, valor)` de una igualdad `v.prop = literal` (o espejo)
/// sobre la variable dada; `None` si el átomo no es de esa forma.
fn equality_atom(atom: &ScalarExpr, variable: &str) -> Option<(String, Value)> {
    let ScalarExpr::Compare {
        op: CompareOp::Eq,
        left,
        right,
    } = atom
    else {
        return None;
    };
    match (left.as_ref(), right.as_ref()) {
        (
            ScalarExpr::Property {
                variable: v,
                property,
            },
            ScalarExpr::Literal(value),
        ) if v == variable => Some((property.clone(), value.clone())),
        (
            ScalarExpr::Literal(value),
            ScalarExpr::Property {
                variable: v,
                property,
            },
        ) if v == variable => Some((property.clone(), value.clone())),
        _ => None,
    }
}

/// Dirección recorrida al cruzar una relación en sentido contrario.
fn invert_direction(direction: RelDirection) -> RelDirection {
    match direction {
        RelDirection::Outgoing => RelDirection::Incoming,
        RelDirection::Incoming => RelDirection::Outgoing,
        RelDirection::Undirected => RelDirection::Undirected,
    }
}

// ─── El optimizador: cinco reglas en orden fijo ───

/// Reescribe el plan ingenuo del cap. 19 en el plan que ejecutará el motor.
///
/// Orden de las reglas y por qué:
/// 1. [`rule_selective_start`] PRIMERO: decide por dónde empezar a ligar el
///    patrón; el resto de reglas cuelgan los predicados del plan resultante.
/// 2. [`rule_predicate_pushdown`]: baja cada átomo lo más profundo posible.
/// 3. [`rule_absorb_label`]: la etiqueta del nodo escaneado entra en el
///    `NodeScan` (que filtra al escanear) y desaparece del `Filter`.
/// 4. [`rule_index_seek`]: la igualdad que quedó sobre el escaneo se
///    resuelve con el índice del catálogo (`NodeScan` → `IndexSeek`).
/// 5. [`rule_prune_projections`]: limpieza de proyecciones redundantes.
pub fn optimize(plan: &LogicalPlan, catalog: &Catalog) -> LogicalPlan {
    let plan = rule_selective_start(plan, catalog);
    let plan = rule_predicate_pushdown(&plan);
    let plan = rule_absorb_label(&plan);
    let plan = rule_index_seek(&plan, catalog);
    rule_prune_projections(&plan)
}

/// Un tramo de una cadena de expansión (sólo lo que cambia al reordenar).
struct Hop {
    rel_variable: Option<String>,
    rel_type: Option<String>,
    direction: RelDirection,
}

/// R1 — punto inicial más selectivo + reordenación de expansiones.
///
/// Una cadena `NodeScan + Expand*` puede empezarse por CUALQUIER variable del
/// patrón: los tramos a la derecha se recorren tal cual y los de la izquierda
/// con la dirección invertida. Para cada candidato se estima
///
/// ```text
///   coste(i) = filas_escaneadas(vi) × Π grados medios de cada tramo desde vi
/// ```
///
/// donde `filas_escaneadas(vi)` incorpora la selectividad de los predicados
/// que sólo mencionan a `vi` (los que podrían pegarse a su escaneo). Gana el
/// coste estrictamente menor; el empate conserva el orden original (las
/// estimaciones iguales no justifican tocar nada — y mantiene los planes
/// estables para tests y explain).
fn rule_selective_start(plan: &LogicalPlan, catalog: &Catalog) -> LogicalPlan {
    match plan {
        LogicalPlan::Filter { input, predicate } => {
            let mut atoms = Vec::new();
            split_and(predicate, &mut atoms);
            let input = rule_selective_start(input, catalog);
            LogicalPlan::Filter {
                input: Box::new(reorder_chain(&input, &atoms, catalog)),
                predicate: predicate.clone(),
            }
        }
        LogicalPlan::Project { input, items } => LogicalPlan::Project {
            input: Box::new(reorder_chain(
                &rule_selective_start(input, catalog),
                &[],
                catalog,
            )),
            items: items.clone(),
        },
        LogicalPlan::CartesianProduct { left, right } => LogicalPlan::CartesianProduct {
            // Cada lado es (en planes de lower) una cadena; si no lo es, la
            // reordenación lo devuelve intacto.
            left: Box::new(reorder_chain(
                &rule_selective_start(left, catalog),
                &[],
                catalog,
            )),
            right: Box::new(reorder_chain(
                &rule_selective_start(right, catalog),
                &[],
                catalog,
            )),
        },
        // Dentro de una cadena: la reordena el nodo cabeza (Filter/Project/
        // CartesianProduct que la cuelga). Los escaneos sueltos no tienen tramos.
        LogicalPlan::Expand { .. }
        | LogicalPlan::NodeScan { .. }
        | LogicalPlan::IndexSeek { .. } => plan.clone(),
    }
}

/// Reordena una cadena `NodeScan + Expand*` si conviene; cualquier otra forma
/// se devuelve tal cual. `atoms` son los predicados en juego (para estimar).
fn reorder_chain(child: &LogicalPlan, atoms: &[ScalarExpr], catalog: &Catalog) -> LogicalPlan {
    // Despellejar la cadena: hops y destinos de fuera hacia dentro, scan al
    // fondo. Una forma que no sea NodeScan+Expand* no se toca.
    let mut hops_out_in: Vec<Hop> = Vec::new();
    let mut tos_out_in: Vec<String> = Vec::new();
    let mut cursor = child;
    let (scan_var, scan_label) = loop {
        match cursor {
            LogicalPlan::Expand {
                input,
                from: _,
                rel_variable,
                rel_type,
                direction,
                to,
            } => {
                hops_out_in.push(Hop {
                    rel_variable: rel_variable.clone(),
                    rel_type: rel_type.clone(),
                    direction: *direction,
                });
                tos_out_in.push(to.clone());
                cursor = input.as_ref();
            }
            LogicalPlan::NodeScan { variable, label } => break (variable.clone(), label.clone()),
            // No es una cadena limpia (p.ej. un CartesianProduct anidado).
            _ => return child.clone(),
        }
    };
    if hops_out_in.is_empty() {
        return child.clone();
    }
    let mut hops = hops_out_in;
    hops.reverse(); // hop j conecta v(j-1) → vj, tal como se escribió el patrón

    // Variables de la cadena en orden: v0 (scan original), v1..vn (destinos).
    let mut vars = vec![scan_var.clone()];
    vars.extend(tos_out_in.into_iter().rev());

    // Entorno de etiquetas: etiqueta del scan original + HasLabel de los
    // predicados en juego.
    let mut env = LabelEnv::new();
    if let Some(label) = &scan_label {
        env.push((scan_var.clone(), label.clone()));
    }
    for atom in atoms {
        if let ScalarExpr::HasLabel { variable, label } = atom
            && env_label(&env, variable).is_none()
        {
            env.push((variable.clone(), label.to_string()));
        }
    }

    // Coste de empezar por cada candidato.
    let n = hops.len();
    let cost_of = |start: usize| -> f64 {
        let mut est = catalog.nodes_with_label(env_label(&env, &vars[start])) as f64;
        for atom in atoms {
            if references_only(atom, &[vars[start].clone()]) {
                est *= selectivity(atom, catalog, &env);
            }
        }
        // Tramos a la derecha: tal cual, desde v(j-1) hacia vj.
        for j in start + 1..=n {
            est *= catalog.avg_degree(env_label(&env, &vars[j - 1]), hops[j - 1].direction)
                * catalog.rel_type_fraction(hops[j - 1].rel_type.as_deref());
        }
        // Tramos a la izquierda: dirección invertida, desde vj hacia v(j-1).
        for j in (1..=start).rev() {
            est *= catalog.avg_degree(
                env_label(&env, &vars[j]),
                invert_direction(hops[j - 1].direction),
            ) * catalog.rel_type_fraction(hops[j - 1].rel_type.as_deref());
        }
        est
    };
    let mut best = 0;
    let mut best_cost = cost_of(0);
    for candidate in 1..=n {
        let cost = cost_of(candidate);
        if cost < best_cost {
            best = candidate;
            best_cost = cost;
        }
    }
    if best == 0 {
        return child.clone();
    }

    // Reconstruir empezando por v(best): primero los tramos a la derecha,
    // luego los de la izquierda (con su dirección invertida).
    let mut plan = LogicalPlan::NodeScan {
        variable: vars[best].clone(),
        label: env_label(&env, &vars[best]).map(str::to_string),
    };
    for j in best + 1..=n {
        let hop = &hops[j - 1];
        plan = LogicalPlan::Expand {
            input: Box::new(plan),
            from: vars[j - 1].clone(),
            rel_variable: hop.rel_variable.clone(),
            rel_type: hop.rel_type.clone(),
            direction: hop.direction,
            to: vars[j].clone(),
        };
    }
    for j in (1..=best).rev() {
        let hop = &hops[j - 1];
        plan = LogicalPlan::Expand {
            input: Box::new(plan),
            from: vars[j].clone(),
            rel_variable: hop.rel_variable.clone(),
            rel_type: hop.rel_type.clone(),
            direction: invert_direction(hop.direction),
            to: vars[j - 1].clone(),
        };
    }
    plan
}

/// R2 — push-down de predicados: cada átomo baja lo más profundo posible.
///
/// - Bajo un `Expand`: baja lo que sólo menciona variables ligadas POR DEBAJO
///   (la variable `from`); lo que menciona `to` o la relación se queda arriba
///   (aún no están ligadas).
/// - Sobre un `Filter`: se fusionan y se reintenta (los planes a mano pueden
///   traer filtros apilados).
/// - En un `CartesianProduct`: cada átomo baja hacia el lado que liga todas
///   sus variables; los que mezclan lados se quedan arriba (son el join).
/// - `NodeScan`/`IndexSeek`/`Project` son fronteras: ahí se detiene.
fn rule_predicate_pushdown(plan: &LogicalPlan) -> LogicalPlan {
    match plan {
        LogicalPlan::Filter { input, predicate } => {
            let input = rule_predicate_pushdown(input);
            sink(predicate, input)
        }
        LogicalPlan::Project { input, items } => LogicalPlan::Project {
            input: Box::new(rule_predicate_pushdown(input)),
            items: items.clone(),
        },
        LogicalPlan::CartesianProduct { left, right } => LogicalPlan::CartesianProduct {
            left: Box::new(rule_predicate_pushdown(left)),
            right: Box::new(rule_predicate_pushdown(right)),
        },
        LogicalPlan::Expand {
            input,
            from,
            rel_variable,
            rel_type,
            direction,
            to,
        } => LogicalPlan::Expand {
            input: Box::new(rule_predicate_pushdown(input)),
            from: from.clone(),
            rel_variable: rel_variable.clone(),
            rel_type: rel_type.clone(),
            direction: *direction,
            to: to.clone(),
        },
        LogicalPlan::NodeScan { .. } | LogicalPlan::IndexSeek { .. } => plan.clone(),
    }
}

/// Intenta hundir `pred` dentro de `input` (ya reescrito en sus hijos).
fn sink(pred: &ScalarExpr, input: LogicalPlan) -> LogicalPlan {
    let mut atoms = Vec::new();
    split_and(pred, &mut atoms);
    match input {
        LogicalPlan::Expand {
            input: inner,
            from,
            rel_variable,
            rel_type,
            direction,
            to,
        } => {
            let below = inner.bound_variables();
            let (inside, outside) = partition_atoms(atoms, &below);
            let inner = inside
                .into_iter()
                .fold(*inner, |acc, atom| sink(&atom, acc));
            let expand = LogicalPlan::Expand {
                input: Box::new(inner),
                from,
                rel_variable,
                rel_type,
                direction,
                to,
            };
            wrap_filter(expand, outside)
        }
        LogicalPlan::Filter {
            input: inner,
            predicate: existing,
        } => {
            // Fusionar (los nuevos primero) y reintentar sobre el input.
            let mut combined = atoms;
            split_and(&existing, &mut combined);
            match ScalarExpr::and_all(combined) {
                Some(merged) => sink(&merged, *inner),
                None => LogicalPlan::Filter {
                    input: inner,
                    predicate: existing,
                },
            }
        }
        LogicalPlan::CartesianProduct { left, right } => {
            let left_vars = left.bound_variables();
            let right_vars = right.bound_variables();
            let (to_left, rest) = partition_atoms(atoms, &left_vars);
            let (to_right, rest) = partition_atoms(rest, &right_vars);
            let left = to_left
                .into_iter()
                .fold(*left, |acc, atom| sink(&atom, acc));
            let right = to_right
                .into_iter()
                .fold(*right, |acc, atom| sink(&atom, acc));
            wrap_filter(
                LogicalPlan::CartesianProduct {
                    left: Box::new(left),
                    right: Box::new(right),
                },
                rest,
            )
        }
        // NodeScan, IndexSeek y Project: fronteras del push-down.
        other => LogicalPlan::Filter {
            input: Box::new(other),
            predicate: pred.clone(),
        },
    }
}

/// R3 — absorber el `HasLabel` del nodo escaneado en la etiqueta del
/// `NodeScan`: el escaneo filtra por etiqueta al iterar, así que el predicado
/// sobra. Con etiqueta DISTINTA declarada, el átomo se conserva (contradicción:
/// el runtime devolverá cero filas, que es lo correcto).
fn rule_absorb_label(plan: &LogicalPlan) -> LogicalPlan {
    match plan {
        LogicalPlan::Filter { input, predicate } => {
            let inner = rule_absorb_label(input);
            if let LogicalPlan::NodeScan { variable, label } = &inner {
                let mut atoms = Vec::new();
                split_and(predicate, &mut atoms);
                let mut new_label = label.clone();
                let mut rest = Vec::new();
                for atom in atoms {
                    if let ScalarExpr::HasLabel {
                        variable: v,
                        label: l,
                    } = &atom
                        && v == variable
                    {
                        match &new_label {
                            // Se absorbe: el escaneo filtra por etiqueta.
                            None => {
                                new_label = Some(l.clone());
                                continue;
                            }
                            // Redundante: el escaneo ya la impone.
                            Some(existing) if existing == l => continue,
                            // Contradicción: el átomo se queda arriba.
                            Some(_) => {}
                        }
                    }
                    rest.push(atom);
                }
                let scan = LogicalPlan::NodeScan {
                    variable: variable.clone(),
                    label: new_label,
                };
                return wrap_filter(scan, rest);
            }
            LogicalPlan::Filter {
                input: Box::new(inner),
                predicate: predicate.clone(),
            }
        }
        LogicalPlan::Project { input, items } => LogicalPlan::Project {
            input: Box::new(rule_absorb_label(input)),
            items: items.clone(),
        },
        LogicalPlan::CartesianProduct { left, right } => LogicalPlan::CartesianProduct {
            left: Box::new(rule_absorb_label(left)),
            right: Box::new(rule_absorb_label(right)),
        },
        LogicalPlan::Expand {
            input,
            from,
            rel_variable,
            rel_type,
            direction,
            to,
        } => LogicalPlan::Expand {
            input: Box::new(rule_absorb_label(input)),
            from: from.clone(),
            rel_variable: rel_variable.clone(),
            rel_type: rel_type.clone(),
            direction: *direction,
            to: to.clone(),
        },
        LogicalPlan::NodeScan { .. } | LogicalPlan::IndexSeek { .. } => plan.clone(),
    }
}

/// R4 — `Filter(v.prop = literal) + NodeScan` → `IndexSeek`.
///
/// La igualdad se resuelve contra el índice del catálogo y los ids quedan en
/// el plan (semi-ligado); el ejecutor sólo tiene que leer esos nodos. Sólo se
/// aplica si AHORRA: si la búsqueda devolviera tantos nodos como el escaneo,
/// no vale la pena sustituirlo (se deja el `NodeScan` + `Filter`). Un valor
/// ausente da cero ids — la reescritura más rentable de todas (cortar el
/// pipeline de raíz) y también la correcta.
fn rule_index_seek(plan: &LogicalPlan, catalog: &Catalog) -> LogicalPlan {
    match plan {
        LogicalPlan::Filter { input, predicate } => {
            let inner = rule_index_seek(input, catalog);
            if let LogicalPlan::NodeScan { variable, label } = &inner {
                let mut atoms = Vec::new();
                split_and(predicate, &mut atoms);
                if let Some(index) = atoms
                    .iter()
                    .position(|a| equality_atom(a, variable).is_some())
                {
                    let (property, value) =
                        equality_atom(&atoms[index], variable).expect("verificado en position");
                    let ids = catalog.equality_lookup(label.as_deref(), &property, &value);
                    let scan_rows = catalog.nodes_with_label(label.as_deref());
                    if (ids.len() as u64) < scan_rows {
                        atoms.remove(index);
                        let seek = LogicalPlan::IndexSeek {
                            variable: variable.clone(),
                            label: label.clone(),
                            property,
                            value,
                            ids,
                        };
                        return wrap_filter(seek, atoms);
                    }
                }
            }
            LogicalPlan::Filter {
                input: Box::new(inner),
                predicate: predicate.clone(),
            }
        }
        LogicalPlan::Project { input, items } => LogicalPlan::Project {
            input: Box::new(rule_index_seek(input, catalog)),
            items: items.clone(),
        },
        LogicalPlan::CartesianProduct { left, right } => LogicalPlan::CartesianProduct {
            left: Box::new(rule_index_seek(left, catalog)),
            right: Box::new(rule_index_seek(right, catalog)),
        },
        LogicalPlan::Expand {
            input,
            from,
            rel_variable,
            rel_type,
            direction,
            to,
        } => LogicalPlan::Expand {
            input: Box::new(rule_index_seek(input, catalog)),
            from: from.clone(),
            rel_variable: rel_variable.clone(),
            rel_type: rel_type.clone(),
            direction: *direction,
            to: to.clone(),
        },
        LogicalPlan::NodeScan { .. } | LogicalPlan::IndexSeek { .. } => plan.clone(),
    }
}

/// R5 — eliminar proyecciones innecesarias.
///
/// Fusiona un `Project` cuyo input es OTRO `Project` de identidad (todos sus
/// items son la variable `x` proyectada como `x`, sin renombrar): pasar por
/// ella no cambia ninguna columna y puede eliminarse. Un `Project` que
/// transforma (`p.name AS p`, literales…) NO se fusiona: tras él las
/// variables dejan de estar ligadas a nodos y las expresiones del `Project`
/// exterior dejarían de ver lo que esperan.
///
/// Hoy `lower()` produce un único `Project` raíz, así que esta regla es
/// defensiva para planes construidos a mano — y la demostración del patrón
/// clásico "merge adjacent projections" con la arquitectura de este libro.
fn rule_prune_projections(plan: &LogicalPlan) -> LogicalPlan {
    match plan {
        LogicalPlan::Project { input, items } => {
            let inner = rule_prune_projections(input);
            if let LogicalPlan::Project {
                input: inner_input,
                items: inner_items,
            } = inner
            {
                let es_identidad = inner_items.iter().all(|item| {
                    matches!(&item.expr, ScalarExpr::Var { name, .. } if item.output_name() == *name)
                });
                if es_identidad {
                    // El Project interior no aporta: se elimina.
                    LogicalPlan::Project {
                        input: inner_input,
                        items: items.clone(),
                    }
                } else {
                    LogicalPlan::Project {
                        input: Box::new(LogicalPlan::Project {
                            input: inner_input,
                            items: inner_items,
                        }),
                        items: items.clone(),
                    }
                }
            } else {
                LogicalPlan::Project {
                    input: Box::new(inner),
                    items: items.clone(),
                }
            }
        }
        LogicalPlan::Filter { input, predicate } => LogicalPlan::Filter {
            input: Box::new(rule_prune_projections(input)),
            predicate: predicate.clone(),
        },
        LogicalPlan::CartesianProduct { left, right } => LogicalPlan::CartesianProduct {
            left: Box::new(rule_prune_projections(left)),
            right: Box::new(rule_prune_projections(right)),
        },
        LogicalPlan::Expand {
            input,
            from,
            rel_variable,
            rel_type,
            direction,
            to,
        } => LogicalPlan::Expand {
            input: Box::new(rule_prune_projections(input)),
            from: from.clone(),
            rel_variable: rel_variable.clone(),
            rel_type: rel_type.clone(),
            direction: *direction,
            to: to.clone(),
        },
        LogicalPlan::NodeScan { .. } | LogicalPlan::IndexSeek { .. } => plan.clone(),
    }
}

// ─── explain: el hito del capítulo ───

/// El hito del cap. 21: plan ANTES / DESPUÉS con cardinalidades estimadas.
///
/// Pipeline de diagnóstico: `parse` (cap 18) → `lower` (cap 19: el ANTES) →
/// `Catalog::collect` → `optimize` (el DESPUÉS) → estimaciones por operador.
/// Al final ejecuta el plan optimizado para CONTRASTAR la estimación de la
/// raíz con las filas reales (LiraQL es de sólo lectura: ejecutar no tiene
/// efectos). El texto es el que imprime `liradb explain`.
///
/// ```
/// use vol2_liradb::{demo_graph, explain};
///
/// let store = demo_graph();
/// let texto = explain("MATCH (p:Person {name: \"Ana\"}) RETURN p.age", &store).unwrap();
/// assert!(texto.contains("Plan ANTES"));
/// assert!(texto.contains("Plan DESPUÉS"));
/// // La reescritura canónica del brief: el Filter de igualdad + NodeScan…
/// assert!(texto.contains("IndexSeek(Person.name = \"Ana\")"));
/// // …y las filas reales contrastan con la estimación.
/// assert!(texto.contains("Filas reales al ejecutar el plan optimizado: 1"));
/// ```
pub fn explain(src: &str, store: &dyn GraphStore) -> Result<String, ExecError> {
    let query = parse(src)?;
    let antes = query.lower()?;
    let catalog = Catalog::collect(store);
    let despues = optimize(&antes, &catalog);

    let mut out = String::new();
    out.push_str("liradb explain — optimizador (cap. 21)\n");
    out.push_str(&format!("Consulta: {src}\n\n"));
    out.push_str(&resumen_catalogo(&catalog));
    out.push_str("\nPlan ANTES (lower, cap. 19):\n");
    out.push_str(&plan_con_estimaciones(&antes, &catalog));
    out.push_str("\n\nPlan DESPUÉS (optimize, cap. 21):\n");
    out.push_str(&plan_con_estimaciones(&despues, &catalog));
    if antes == despues {
        out.push_str("\n\n(El optimizador no encontró mejoras para esta consulta.)");
    }

    let mut executor = Executor::new(&despues, store)?;
    let rs = executor.execute()?;
    out.push_str(&format!(
        "\n\nFilas reales al ejecutar el plan optimizado: {} (raíz estimada: {})",
        rs.len(),
        estimacion_mostrada(estimate(&despues, &catalog))
    ));
    Ok(out)
}

/// Resumen legible del catálogo (la sección "estadísticas" del capítulo).
fn resumen_catalogo(catalog: &Catalog) -> String {
    let mut out = format!(
        "Catálogo (estadísticas del store): {} nodos · {} aristas\n",
        catalog.total_nodes, catalog.total_edges
    );
    for (label, stats) in &catalog.per_label {
        out.push_str(&format!(
            "  {label}: {} nodos · grado medio out {:.2} / in {:.2}\n",
            stats.nodes,
            stats.avg_out_degree(),
            stats.avg_in_degree()
        ));
    }
    if !catalog.edges_per_type.is_empty() {
        let tipos: Vec<String> = catalog
            .edges_per_type
            .iter()
            .map(|(rel_type, count)| format!("{rel_type} {count}"))
            .collect();
        out.push_str(&format!("  aristas por tipo: {}\n", tipos.join(", ")));
    }
    out
}

/// Estimaciones por operador en PRE-ORDEN (el mismo con el que el `Display`
/// del cap. 19 pinta las líneas), para emparejar 1:1 con `plan.to_string()`.
fn estimaciones_en_preorden(plan: &LogicalPlan, catalog: &Catalog) -> Vec<u64> {
    let env = label_env(plan);
    let mut out = Vec::new();
    walk_estimaciones(plan, catalog, &env, &mut out);
    out
}

fn walk_estimaciones(plan: &LogicalPlan, catalog: &Catalog, env: &LabelEnv, out: &mut Vec<u64>) {
    out.push(estimacion_mostrada(estimate_with(plan, catalog, env)));
    match plan {
        LogicalPlan::Expand { input, .. }
        | LogicalPlan::Filter { input, .. }
        | LogicalPlan::Project { input, .. } => {
            walk_estimaciones(input, catalog, env, out);
        }
        LogicalPlan::CartesianProduct { left, right } => {
            walk_estimaciones(left, catalog, env, out);
            walk_estimaciones(right, catalog, env, out);
        }
        LogicalPlan::NodeScan { .. } | LogicalPlan::IndexSeek { .. } => {}
    }
}

/// El plan (Display del cap. 19) con la cardinalidad estimada al final de
/// cada línea — el formato del hito del brief (`… estimated: 12`).
fn plan_con_estimaciones(plan: &LogicalPlan, catalog: &Catalog) -> String {
    let texto = plan.to_string();
    let lineas: Vec<&str> = texto.lines().collect();
    let estimaciones = estimaciones_en_preorden(plan, catalog);
    debug_assert_eq!(
        lineas.len(),
        estimaciones.len(),
        "render y estimación recorren el plan con la misma forma"
    );
    let ancho = lineas.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    lineas
        .into_iter()
        .zip(estimaciones)
        .map(|(linea, est)| format!("{linea:<ancho$}  est. {est} filas"))
        .collect::<Vec<_>>()
        .join("\n")
}

// ─────────────────── Tests ───────────────────

#[cfg(test)]
mod tests_optimizer {
    use super::*;
    use crate::cap07_modelo::Node;
    use crate::cap08_graph_store::MemoryStore;
    use crate::cap19_plan_logico::{BindingKind, Projection};
    use crate::cap20_volcano::{ResultSet, demo_graph, run};

    /// El fixture de siempre: Ana/Bo/Carla/Dani + Madrid/Lisboa.
    fn grafo() -> MemoryStore {
        demo_graph()
    }

    /// Ejecuta el plan INGENUO (lower tal cual, sin optimizador).
    fn naive(src: &str, store: &MemoryStore) -> ResultSet {
        let plan = parse(src).unwrap().lower().unwrap();
        Executor::new(&plan, store).unwrap().execute().unwrap()
    }

    /// Filas como texto, ORDENADAS: sin ORDER BY el orden no es parte del
    /// contrato (las reglas reordenan ligaduras), así que la equivalencia se
    /// verifica como multiconjunto.
    fn filas_ordenadas(rs: &ResultSet) -> Vec<Vec<String>> {
        let mut filas: Vec<Vec<String>> = rs
            .rows
            .iter()
            .map(|fila| fila.iter().map(|c| c.to_string()).collect())
            .collect();
        filas.sort();
        filas
    }

    fn optimizado(src: &str, store: &MemoryStore) -> String {
        let plan = parse(src).unwrap().lower().unwrap();
        let catalog = Catalog::collect(store);
        optimize(&plan, &catalog).to_string()
    }

    // ════════════════════════════════════════════════════════════════
    //  Catálogo — estadísticas del store
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn catalogo_recuentos_del_grafo_demo() {
        let catalog = Catalog::collect(&grafo());
        assert_eq!(catalog.total_nodes, 6);
        assert_eq!(catalog.total_edges, 6);
        assert_eq!(catalog.nodes_with_label(Some("Person")), 4);
        assert_eq!(catalog.nodes_with_label(Some("City")), 2);
        assert_eq!(catalog.nodes_with_label(None), 6);
        assert_eq!(catalog.nodes_with_label(Some("Zombie")), 0);
        assert_eq!(catalog.edges_of_type("KNOWS"), 4);
        assert_eq!(catalog.edges_of_type("LIVES_IN"), 2);
        assert_eq!(catalog.edges_of_type("NADA"), 0);
    }

    #[test]
    fn catalogo_grados_medios_por_etiqueta() {
        let catalog = Catalog::collect(&grafo());
        let person = catalog.label_stats("Person");
        // Salientes por persona: Ana 2 (KNOWS+LIVES_IN), Bo 2, Carla 1, Dani 1.
        assert_eq!(person.nodes, 4);
        assert!((person.avg_out_degree() - 1.5).abs() < 1e-9);
        // Entrantes: Ana 1 (de Carla), Bo 1, Carla 1, Dani 1 (self-loop).
        assert!((person.avg_in_degree() - 1.0).abs() < 1e-9);
        let city = catalog.label_stats("City");
        assert_eq!(city.nodes, 2);
        assert!((city.avg_out_degree() - 0.0).abs() < 1e-9);
        // Ambas ciudades reciben un LIVES_IN: (1 + 1) / 2.
        assert!((city.avg_in_degree() - 1.0).abs() < 1e-9);
        // Sin etiqueta conocida: media global = 6 aristas / 6 nodos.
        assert!((catalog.avg_degree(None, RelDirection::Outgoing) - 1.0).abs() < 1e-9);
        assert!((catalog.avg_degree(None, RelDirection::Incoming) - 1.0).abs() < 1e-9);
        assert!((catalog.avg_degree(None, RelDirection::Undirected) - 2.0).abs() < 1e-9);
        // Por etiqueta y dirección (undirected = out + in).
        assert!((catalog.avg_degree(Some("Person"), RelDirection::Undirected) - 2.5).abs() < 1e-9);
        // Fracción de tipo: 4 de 6 aristas son KNOWS.
        assert!((catalog.rel_type_fraction(Some("KNOWS")) - 4.0 / 6.0).abs() < 1e-9);
        assert!((catalog.rel_type_fraction(None) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn catalogo_vacio_sin_panic() {
        let catalog = Catalog::collect(&MemoryStore::new());
        assert_eq!(catalog.total_nodes, 0);
        assert_eq!(catalog.total_edges, 0);
        assert_eq!(catalog.nodes_with_label(None), 0);
        assert_eq!(catalog.avg_degree(None, RelDirection::Outgoing), 0.0);
        assert_eq!(catalog.edges_of_type("KNOWS"), 0);
        assert_eq!(catalog.rel_type_fraction(Some("KNOWS")), 1.0); // sin aristas → sin filtro
        assert!(
            catalog
                .equality_lookup(Some("Person"), "name", &Value::String("Ana".into()))
                .is_empty()
        );
    }

    #[test]
    fn catalogo_indice_de_igualdad() {
        let catalog = Catalog::collect(&grafo());
        let busca = |label: Option<&str>, prop: &str, valor: &str| {
            catalog.equality_lookup(label, prop, &Value::String(valor.into()))
        };
        assert_eq!(busca(Some("Person"), "name", "Ana"), vec![0]);
        // Edad 36: Ana y Dani (en orden del store).
        assert_eq!(
            catalog.equality_lookup(Some("Person"), "age", &Value::Int(36)),
            vec![0, 3]
        );
        // Comodín sin etiqueta: misma búsqueda sobre todos los nodos.
        assert_eq!(busca(None, "name", "Ana"), vec![0]);
        assert_eq!(busca(None, "name", "Madrid"), vec![4]);
        // Valores y propiedades ausentes: clave que no existe → vacío.
        assert!(busca(Some("Person"), "name", "Zoe").is_empty());
        assert!(busca(Some("Person"), "nick", "Ana").is_empty());
        // La etiqueta restringe: "Madrid" no está bajo Person.
        assert!(busca(Some("Person"), "name", "Madrid").is_empty());
    }

    #[test]
    fn catalogo_dos_pasadas_mismos_ids() {
        // El orden de las entradas puede variar (HashMap de props), pero los
        // IDs de cada búsqueda son siempre los del store: determinismo.
        let c1 = Catalog::collect(&grafo());
        let c2 = Catalog::collect(&grafo());
        for (label, prop, value) in [
            (Some("Person"), "name", Value::String("Ana".into())),
            (None, "age", Value::Int(36)),
            (Some("City"), "name", Value::String("Lisboa".into())),
        ] {
            assert_eq!(
                c1.equality_lookup(label, prop, &value),
                c2.equality_lookup(label, prop, &value)
            );
        }
    }

    // ════════════════════════════════════════════════════════════════
    //  Selectividad — heurísticas documentadas
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn selectividad_defaults_de_system_r() {
        let catalog = Catalog::collect(&grafo());
        let env = LabelEnv::new();
        let var = |nombre: &str| ScalarExpr::var(nombre, BindingKind::Node);
        // Igualdad variable-variable (sin estadística): 0.1.
        let eq = ScalarExpr::eq(var("a"), var("b"));
        assert!((selectivity(&eq, &catalog, &env) - SEL_EQ).abs() < 1e-9);
        // Rango: 1/3 · Desigualdad: 0.9 · Desconocido: 0.5.
        let rango = ScalarExpr::Compare {
            op: CompareOp::Lt,
            left: Box::new(var("a")),
            right: Box::new(var("b")),
        };
        assert!((selectivity(&rango, &catalog, &env) - SEL_RANGE).abs() < 1e-9);
        let distinto = ScalarExpr::Compare {
            op: CompareOp::NotEq,
            left: Box::new(var("a")),
            right: Box::new(var("b")),
        };
        assert!((selectivity(&distinto, &catalog, &env) - SEL_NOT_EQ).abs() < 1e-9);
        assert!((selectivity(&var("a"), &catalog, &env) - SEL_UNKNOWN).abs() < 1e-9);
    }

    #[test]
    fn selectividad_igualdad_con_estadistica_y_etiquetas() {
        let catalog = Catalog::collect(&grafo());
        let env = vec![("p".to_string(), "Person".to_string())];
        // Igualdad con índice: 1 Ana entre 4 personas = 0.25 (exacta).
        let eq = ScalarExpr::eq(
            ScalarExpr::prop("p", "name"),
            ScalarExpr::lit(Value::String("Ana".into())),
        );
        assert!((selectivity(&eq, &catalog, &env) - 0.25).abs() < 1e-9);
        // Valor ausente: 0 filas.
        let ausente = ScalarExpr::eq(
            ScalarExpr::prop("p", "name"),
            ScalarExpr::lit(Value::String("Zoe".into())),
        );
        assert_eq!(selectivity(&ausente, &catalog, &env), 0.0);
        // HasLabel con la etiqueta ya declarada: 1.0; distinta: 0.0.
        assert_eq!(
            selectivity(&ScalarExpr::has_label("p", "Person"), &catalog, &env),
            1.0
        );
        assert_eq!(
            selectivity(&ScalarExpr::has_label("p", "City"), &catalog, &env),
            0.0
        );
        // Sin declarar: fracción de nodos con la etiqueta (4/6).
        let vacio = LabelEnv::new();
        assert!(
            (selectivity(&ScalarExpr::has_label("x", "Person"), &catalog, &vacio) - 4.0 / 6.0)
                .abs()
                < 1e-9
        );
    }

    #[test]
    fn selectividad_logica_and_or_not() {
        let catalog = Catalog::collect(&grafo());
        let env = vec![("p".to_string(), "Person".to_string())];
        let eq = || {
            ScalarExpr::eq(
                ScalarExpr::prop("p", "name"),
                ScalarExpr::lit(Value::String("Ana".into())),
            )
        };
        let rango = || ScalarExpr::Compare {
            op: CompareOp::Lt,
            left: Box::new(ScalarExpr::prop("p", "age")),
            right: Box::new(ScalarExpr::lit(Value::Int(40))),
        };
        // AND: producto (independencia): 0.25 × 1/3.
        let y = ScalarExpr::And {
            left: Box::new(eq()),
            right: Box::new(rango()),
        };
        assert!((selectivity(&y, &catalog, &env) - 0.25 * SEL_RANGE).abs() < 1e-9);
        // OR: inclusión-exclusión.
        let o = ScalarExpr::Or {
            left: Box::new(eq()),
            right: Box::new(rango()),
        };
        assert!(
            (selectivity(&o, &catalog, &env) - (0.25 + SEL_RANGE - 0.25 * SEL_RANGE)).abs() < 1e-9
        );
        // NOT: complemento.
        let no = ScalarExpr::Not {
            expr: Box::new(eq()),
        };
        assert!((selectivity(&no, &catalog, &env) - 0.75).abs() < 1e-9);
        // Literales: TRUE pasa todo, FALSE nada.
        assert_eq!(
            selectivity(&ScalarExpr::lit(Value::Bool(true)), &catalog, &env),
            1.0
        );
        assert_eq!(
            selectivity(&ScalarExpr::lit(Value::Bool(false)), &catalog, &env),
            0.0
        );
    }

    // ════════════════════════════════════════════════════════════════
    //  Estimación de cardinalidad por operador
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn estimacion_scan_filter_expand() {
        let store = grafo();
        let catalog = Catalog::collect(&store);
        let plan =
            parse("MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE p.name = \"Ana\" RETURN f.name")
                .unwrap()
                .lower()
                .unwrap();
        // NodeScan(Person) = 4.
        let scan = LogicalPlan::NodeScan {
            variable: "p".into(),
            label: Some("Person".into()),
        };
        assert_eq!(estimate(&scan, &catalog), 4.0);
        // Expand = 4 × grado out de Person (1.5) × fracción KNOWS (4/6) = 4.
        let expand = LogicalPlan::Expand {
            input: Box::new(scan.clone()),
            from: "p".into(),
            rel_variable: None,
            rel_type: Some("KNOWS".into()),
            direction: RelDirection::Outgoing,
            to: "f".into(),
        };
        assert!((estimate(&expand, &catalog) - 4.0).abs() < 1e-9);
        // Filter = 4 × sel(f:Person AND p.name="Ana") = 4 × (1.0 × 0.25).
        let env = label_env(&plan);
        let predicado = match &plan {
            LogicalPlan::Project { input, .. } => match input.as_ref() {
                LogicalPlan::Filter { predicate, .. } => predicate,
                _ => panic!("Filter bajo el Project"),
            },
            _ => panic!("Project raíz"),
        };
        assert!((selectivity(predicado, &catalog, &env) - 0.25).abs() < 1e-9);
        assert!((estimate(&plan, &catalog) - 1.0).abs() < 1e-9);
        // Escaneo sin etiqueta: todos los nodos.
        let cualquier = LogicalPlan::NodeScan {
            variable: "x".into(),
            label: None,
        };
        assert_eq!(estimate(&cualquier, &catalog), 6.0);
    }

    #[test]
    fn estimacion_index_seek_y_cartesiano() {
        let store = grafo();
        let catalog = Catalog::collect(&store);
        // IndexSeek: exacta (los ids ya están resueltos).
        let seek = LogicalPlan::IndexSeek {
            variable: "p".into(),
            label: Some("Person".into()),
            property: "age".into(),
            value: Value::Int(36),
            ids: vec![0, 3],
        };
        assert_eq!(estimate(&seek, &catalog), 2.0);
        // CartesianProduct: producto de estimaciones.
        let cp = LogicalPlan::CartesianProduct {
            left: Box::new(LogicalPlan::NodeScan {
                variable: "p".into(),
                label: Some("Person".into()),
            }),
            right: Box::new(LogicalPlan::NodeScan {
                variable: "c".into(),
                label: Some("City".into()),
            }),
        };
        assert_eq!(estimate(&cp, &catalog), 8.0);
    }

    // ════════════════════════════════════════════════════════════════
    //  R1 — punto inicial más selectivo / reordenar expansiones
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn reordenar_empieza_por_el_lado_selectivo() {
        let store = grafo();
        // El filtro de edad es de f: empezar por f (y expandir hacia atrás)
        // deja el filtro pegado al escaneo en vez de sobre las expansiones.
        let texto = optimizado(
            "MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE f.age < 40 RETURN p.name, f.name",
            &store,
        );
        assert!(texto.contains("NodeScan(Person AS f)"), "plan: {texto}");
        assert!(
            texto.contains("Expand(f, KNOWS, INCOMING, p)"),
            "plan: {texto}"
        );
        assert!(!texto.contains("OUTGOING"), "plan: {texto}");
    }

    #[test]
    fn reordenar_conserva_el_lado_que_ya_gana() {
        let store = grafo();
        let catalog = Catalog::collect(&store);
        // Con p.name = "Ana" el lado p es el selectivo (coste 1.0 vs 2.67):
        // la cadena se queda como está y mejoran las demás reglas.
        let canonico = "MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE p.name = \"Ana\" RETURN f.name";
        let plan = parse(canonico).unwrap().lower().unwrap();
        let solo_reorden = rule_selective_start(&plan, &catalog);
        assert!(solo_reorden.to_string().contains("NodeScan(Person AS p)"));
        assert!(solo_reorden.to_string().contains("OUTGOING"));
        // Filtro exclusivo del nodo inicial: tampoco hay nada que ganar.
        let exclusivo = "MATCH (a:Person)-[:KNOWS]->(b:Person) WHERE a.age < 40 RETURN b.name";
        let plan2 = parse(exclusivo).unwrap().lower().unwrap();
        let solo_reorden2 = rule_selective_start(&plan2, &catalog);
        assert!(solo_reorden2.to_string().contains("NodeScan(Person AS a)"));
    }

    #[test]
    fn reordenar_camino_de_tres_nodos_arranca_por_el_final() {
        let store = grafo();
        let texto = optimizado(
            "MATCH (a:Person)-[:KNOWS]->(b:Person)-[:KNOWS]->(c:Person) RETURN c",
            &store,
        );
        // El b:Person baja hasta quedar ENTRE las dos expansiones (es lo más
        // profundo donde b ya está ligada); el c:Person se absorbió en el scan.
        assert_eq!(
            texto,
            "Project(c)\n  \
             Expand(b, KNOWS, INCOMING, a)\n    \
             Filter(b:Person)\n      \
             Expand(c, KNOWS, INCOMING, b)\n        \
             NodeScan(Person AS c)"
        );
    }

    #[test]
    fn reordenar_no_toca_formas_que_no_son_cadenas() {
        let store = grafo();
        let catalog = Catalog::collect(&store);
        // Un CartesianProduct bajo Filter: las CADENAS (los lados) se
        // reordenan individualmente si conviene, pero la estructura CP queda.
        let plan = parse("MATCH (a:Person), (c:City) RETURN a, c")
            .unwrap()
            .lower()
            .unwrap();
        let reorden = rule_selective_start(&plan, &catalog);
        assert!(matches!(reorden, LogicalPlan::Project { .. }));
        assert!(
            optimizado("MATCH (a:Person), (c:City) RETURN a, c", &store)
                .contains("CartesianProduct")
        );
    }

    // ════════════════════════════════════════════════════════════════
    //  R2 — push-down de predicados
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn pushdown_canonico_del_brief_con_index_seek() {
        let store = grafo();
        // ANTES (cap 19): Filter(f:Person AND p.name="Ana") encima de todo.
        // DESPUÉS: p.name="Ana" baja al escaneo (y se vuelve IndexSeek);
        // f:Person se queda sobre el Expand (f la liga el Expand).
        let texto = optimizado(
            "MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE p.name = \"Ana\" RETURN f.name",
            &store,
        );
        assert_eq!(
            texto,
            "Project(f.name)\n  \
             Filter(f:Person)\n    \
             Expand(p, KNOWS, OUTGOING, f)\n      \
             IndexSeek(Person.name = \"Ana\")"
        );
    }

    #[test]
    fn pushdown_divide_el_and_entre_los_lados() {
        let store = grafo();
        // f (con etiqueta y filtro de edad) es el lado barato: la cadena se
        // reordena para empezar por él, sus dos predicados bajan al escaneo
        // (f:Person se absorbe) y p.age > 30 se queda sobre el Expand (p la
        // liga la expansión: no puede bajar más).
        let texto = optimizado(
            "MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE p.age > 30 AND f.age < 40 RETURN p.name",
            &store,
        );
        assert_eq!(
            texto,
            "Project(p.name)\n  \
             Filter(p.age > 30)\n    \
             Expand(f, KNOWS, INCOMING, p)\n      \
             Filter(f.age < 40)\n        \
             NodeScan(Person AS f)"
        );
    }

    #[test]
    fn pushdown_reparte_el_cartesiano_y_busca_indices() {
        let store = grafo();
        // Cada átomo baja a su lado; el de la ciudad se vuelve IndexSeek.
        let texto = optimizado(
            "MATCH (a:Person), (c:City) WHERE a.age > 35 AND c.name = \"Madrid\" RETURN a.name, c.name",
            &store,
        );
        assert_eq!(
            texto,
            "Project(a.name, c.name)\n  \
             CartesianProduct\n    \
             Filter(a.age > 35)\n      \
             NodeScan(Person AS a)\n    \
             IndexSeek(City.name = \"Madrid\")"
        );
    }

    #[test]
    fn pushdown_no_cruza_project_ni_fusiona_filtrables() {
        // Plan a mano: Filter SOBRE un Project (frontera): se queda arriba.
        let plan = LogicalPlan::Filter {
            input: Box::new(LogicalPlan::Project {
                input: Box::new(LogicalPlan::NodeScan {
                    variable: "p".into(),
                    label: Some("Person".into()),
                }),
                items: vec![Projection {
                    expr: ScalarExpr::prop("p", "name"),
                    alias: None,
                }],
            }),
            predicate: ScalarExpr::eq(
                ScalarExpr::prop("p", "name"),
                ScalarExpr::lit(Value::String("Ana".into())),
            ),
        };
        let resultado = rule_predicate_pushdown(&plan);
        assert!(matches!(resultado, LogicalPlan::Filter { .. }));
        // Dos Filters apilados a mano: se fusionan al hundirse.
        let apilados = LogicalPlan::Filter {
            input: Box::new(LogicalPlan::Filter {
                input: Box::new(LogicalPlan::NodeScan {
                    variable: "p".into(),
                    label: Some("Person".into()),
                }),
                predicate: ScalarExpr::Compare {
                    op: CompareOp::Gt,
                    left: Box::new(ScalarExpr::prop("p", "age")),
                    right: Box::new(ScalarExpr::lit(Value::Int(30))),
                },
            }),
            predicate: ScalarExpr::eq(
                ScalarExpr::prop("p", "name"),
                ScalarExpr::lit(Value::String("Ana".into())),
            ),
        };
        let fusion = rule_predicate_pushdown(&apilados);
        let LogicalPlan::Filter { predicate, .. } = &fusion else {
            panic!("sigue habiendo Filter");
        };
        // Átomos entrantes primero, existentes después.
        assert_eq!(predicate.to_string(), "p.name = \"Ana\" AND p.age > 30");
    }

    // ════════════════════════════════════════════════════════════════
    //  R3 — absorber HasLabel en el escaneo
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn absorb_integra_etiqueta_y_elimina_filtro() {
        let store = grafo();
        // Real: el HasLabel de f queda sobre el tras reordenar por p... usar
        // planes a mano para las tres variantes de la regla.
        let scan = |label: Option<String>| LogicalPlan::NodeScan {
            variable: "p".into(),
            label,
        };
        let filtro = |label: &str| LogicalPlan::Filter {
            input: Box::new(scan(None)),
            predicate: ScalarExpr::has_label("p", label),
        };
        // Sin etiqueta en el scan: se absorbe.
        assert_eq!(
            rule_absorb_label(&filtro("Person")).to_string(),
            "NodeScan(Person AS p)"
        );
        // Etiqueta redundante: el filtro desaparece.
        let redundante = LogicalPlan::Filter {
            input: Box::new(scan(Some("Person".into()))),
            predicate: ScalarExpr::has_label("p", "Person"),
        };
        assert_eq!(
            rule_absorb_label(&redundante).to_string(),
            "NodeScan(Person AS p)"
        );
        // Etiqueta contradictoria: el átomo se conserva (runtime → vacío).
        let contradiccion = LogicalPlan::Filter {
            input: Box::new(scan(Some("Person".into()))),
            predicate: ScalarExpr::And {
                left: Box::new(ScalarExpr::has_label("p", "City")),
                right: Box::new(ScalarExpr::eq(
                    ScalarExpr::prop("p", "name"),
                    ScalarExpr::lit(Value::String("Ana".into())),
                )),
            },
        };
        let texto = rule_absorb_label(&contradiccion).to_string();
        assert_eq!(
            texto,
            "Filter(p:City AND p.name = \"Ana\")\n  NodeScan(Person AS p)"
        );
        // Caso real: reordenar por f deja NodeScan(Person AS f) y el atom
        // f:Person se absorbe (queda sólo el filtro de edad).
        let optimizado_real = optimizado(
            "MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE f.age < 40 RETURN p.name, f.name",
            &store,
        );
        assert!(optimizado_real.contains("Filter(f.age < 40)"));
        assert!(!optimizado_real.contains("f:Person"));
    }

    // ════════════════════════════════════════════════════════════════
    //  R4 — NodeScan → IndexSeek
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn index_seek_valor_ausente_corta_de_raiz() {
        let store = grafo();
        // "Zoe" no existe: el seek es vacío (0 filas) y sustituye al escaneo.
        let plan = parse("MATCH (p:Person) WHERE p.name = \"Zoe\" RETURN p.name")
            .unwrap()
            .lower()
            .unwrap();
        let catalog = Catalog::collect(&store);
        let optimizado = optimize(&plan, &catalog);
        let LogicalPlan::Project { input, .. } = &optimizado else {
            panic!("raíz");
        };
        let LogicalPlan::IndexSeek { ids, .. } = input.as_ref() else {
            panic!("debería ser un IndexSeek: {optimizado}");
        };
        assert!(ids.is_empty());
        // Y la consulta devuelve 0 filas con columnas intactas.
        let rs = run(
            "MATCH (p:Person) WHERE p.name = \"Zoe\" RETURN p.name",
            &store,
        )
        .unwrap();
        assert!(rs.is_empty());
        assert_eq!(rs.columns, vec!["p.name".to_string()]);
    }

    #[test]
    fn index_seek_no_aplica_si_no_ahorra_ni_con_rangos() {
        // Store con 2 personas que comparten edad: buscar esa edad devuelve
        // TODAS las filas del escaneo → no hay ahorro → se queda el scan.
        let mut store = MemoryStore::new();
        store
            .put_node(Node::new(0, "Person").with_prop("age", Value::Int(40)))
            .unwrap();
        store
            .put_node(Node::new(1, "Person").with_prop("age", Value::Int(40)))
            .unwrap();
        let plan = parse("MATCH (p:Person) WHERE p.age = 40 RETURN p")
            .unwrap()
            .lower()
            .unwrap();
        let catalog = Catalog::collect(&store);
        let optimizado = optimize(&plan, &catalog);
        assert!(
            optimizado.to_string().contains("NodeScan(Person AS p)"),
            "plan: {optimizado}"
        );
        // Un rango nunca se resuelve con el índice de igualdad.
        let store2 = grafo();
        let plan2 = parse("MATCH (p:Person) WHERE p.age < 40 RETURN p.name")
            .unwrap()
            .lower()
            .unwrap();
        let catalog2 = Catalog::collect(&store2);
        let texto = optimize(&plan2, &catalog2).to_string();
        assert!(texto.contains("Filter(p.age < 40)"));
        assert!(texto.contains("NodeScan(Person AS p)"));
        assert!(!texto.contains("IndexSeek"));
    }

    // ════════════════════════════════════════════════════════════════
    //  R5 — eliminar proyecciones innecesarias
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn prune_fusiona_proyecciones_de_identidad() {
        let exterior = |input: LogicalPlan| LogicalPlan::Project {
            input: Box::new(input),
            items: vec![Projection {
                expr: ScalarExpr::prop("p", "name"),
                alias: None,
            }],
        };
        // Interior de identidad (Var p AS p): se elimina.
        let identidad = LogicalPlan::Project {
            input: Box::new(LogicalPlan::NodeScan {
                variable: "p".into(),
                label: Some("Person".into()),
            }),
            items: vec![Projection {
                expr: ScalarExpr::var("p", BindingKind::Node),
                alias: None,
            }],
        };
        let plan = exterior(identidad);
        assert_eq!(
            rule_prune_projections(&plan).to_string(),
            "Project(p.name)\n  NodeScan(Person AS p)"
        );
        // Interior que TRANSFORMA (p.name AS p): NO se fusiona (las variables
        // dejarían de estar ligadas a nodos bajo el project exterior).
        let transformadora = LogicalPlan::Project {
            input: Box::new(LogicalPlan::NodeScan {
                variable: "p".into(),
                label: Some("Person".into()),
            }),
            items: vec![Projection {
                expr: ScalarExpr::prop("p", "name"),
                alias: Some("p".into()),
            }],
        };
        let plan = exterior(transformadora);
        let texto = rule_prune_projections(&plan).to_string();
        assert_eq!(texto.matches("Project").count(), 2, "plan: {texto}");
    }

    // ════════════════════════════════════════════════════════════════
    //  Equivalencia: resultados idénticos antes/después de optimizar
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn equivalencia_antes_y_despues_sobre_bateria_de_consultas() {
        let store = grafo();
        let consultas = [
            // 1. La canónica del brief (index seek + filtro de etiqueta).
            "MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE p.name = \"Ana\" RETURN f.name",
            // 2. Filtro del lado destino (reordenación + push-down + absorbe).
            "MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE f.age < 40 RETURN p.name, f.name",
            // 3. Dirección entrante.
            "MATCH (a:Person)<-[:KNOWS]-(b:Person) WHERE a.name = \"Ana\" RETURN b.name",
            // 4. Sin dirección (self-loop una vez).
            "MATCH (a:Person)-[:KNOWS]-(b:Person) WHERE a.name = \"Ana\" RETURN b.name",
            // 5. Camino de tres nodos (reordenación completa de la cadena).
            "MATCH (a:Person)-[:KNOWS]->(b:Person)-[:KNOWS]->(c:Person) RETURN c.name",
            // 6. Dos tramos con anónimo intermedio.
            "MATCH (a:Person)-[:KNOWS]->()-[:KNOWS]->(c:Person) RETURN c.name",
            // 7. CartesianProduct con filtros en ambos lados.
            "MATCH (a:Person), (c:City) WHERE a.age > 35 AND c.name = \"Madrid\" RETURN a.name, c.name",
            // 8. Propiedad inline.
            "MATCH (p:Person {name: \"Ana\"}) RETURN p.age",
            // 9. Propiedad de arista.
            "MATCH (p:Person)-[r:KNOWS]->(f:Person) WHERE r.since > 2019 RETURN p.name",
            // 10. Self-loop por identidad de nodos.
            "MATCH (a:Person)-[:KNOWS]->(b:Person) WHERE a = b RETURN a.name",
            // 11. Etiqueta inexistente (pipeline vacío, columnas intactas).
            "MATCH (z:Zombie) RETURN z.name",
            // 12. OR (átomo no divisible: baja entero o se queda entero).
            "MATCH (p:Person) WHERE p.name = \"Ana\" OR p.age > 40 RETURN p.name",
        ];
        for src in consultas {
            let ingenuo = naive(src, &store);
            let optimizado = run(src, &store).unwrap_or_else(|e| panic!("{src}: {e}"));
            assert_eq!(ingenuo.columns, optimizado.columns, "columnas de: {src}");
            assert_eq!(
                filas_ordenadas(&ingenuo),
                filas_ordenadas(&optimizado),
                "filas de: {src}\ningenuo:\n{ingenuo}\noptimizado:\n{optimizado}"
            );
        }
    }

    #[test]
    fn equivalencia_run_pasa_por_el_optimizador() {
        let store = grafo();
        let src = "MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE f.age < 40 RETURN p.name, f.name";
        // Tres caminos al mismo resultado: plan ingenuo, plan optimizado a
        // mano y run() (que integra el optimizador desde este capítulo).
        let ingenuo = naive(src, &store);
        let plan = parse(src).unwrap().lower().unwrap();
        let catalog = Catalog::collect(&store);
        let optimizado_a_mano = Executor::new(&optimize(&plan, &catalog), &store)
            .unwrap()
            .execute()
            .unwrap();
        let por_run = run(src, &store).unwrap();
        assert_eq!(filas_ordenadas(&ingenuo), filas_ordenadas(&por_run));
        assert_eq!(
            filas_ordenadas(&optimizado_a_mano),
            filas_ordenadas(&por_run)
        );
        assert_eq!(por_run.len(), 3);
        // Y el plan optimizado es DISTINTO del ingenuo (de verdad optimizó).
        assert_ne!(plan.to_string(), optimize(&plan, &catalog).to_string());
    }

    #[test]
    fn optimizar_es_idempotente_y_conservador() {
        let store = grafo();
        let catalog = Catalog::collect(&store);
        // Consultas sin margen: el plan no cambia.
        for src in [
            "MATCH (p:Person) RETURN p.name, p.age",
            "MATCH (p:Person) WHERE p.age < 40 RETURN p.name",
            "MATCH (a:Person), (c:City) RETURN a, c",
        ] {
            let plan = parse(src).unwrap().lower().unwrap();
            assert_eq!(
                optimize(&plan, &catalog).to_string(),
                plan.to_string(),
                "no debería cambiar: {src}"
            );
        }
        // Optimizar dos veces da lo mismo (reglas convergentes).
        let src = "MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE p.name = \"Ana\" RETURN f.name";
        let plan = parse(src).unwrap().lower().unwrap();
        let una_vez = optimize(&plan, &catalog);
        let dos_veces = optimize(&una_vez, &catalog);
        assert_eq!(una_vez.to_string(), dos_veces.to_string());
    }

    // ════════════════════════════════════════════════════════════════
    //  explain — el hito del capítulo
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn explain_canonico_con_antes_despues_y_estimaciones() {
        let store = grafo();
        let texto = explain(
            "MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE p.name = \"Ana\" RETURN f.name",
            &store,
        )
        .unwrap();
        assert!(texto.contains("Plan ANTES (lower, cap. 19):"), "{texto}");
        assert!(
            texto.contains("Plan DESPUÉS (optimize, cap. 21):"),
            "{texto}"
        );
        assert!(
            texto.contains("Catálogo (estadísticas del store): 6 nodos · 6 aristas"),
            "{texto}"
        );
        assert!(
            texto.contains("Person: 4 nodos · grado medio out 1.50 / in 1.00"),
            "{texto}"
        );
        assert!(
            texto.contains("aristas por tipo: KNOWS 4, LIVES_IN 2"),
            "{texto}"
        );
        // ANTES: escaneo de 4 y expansión estimada en 4; el filtro lo deja en 1.
        assert!(texto.contains("NodeScan(Person AS p)"), "{texto}");
        assert!(texto.contains("est. 4 filas"), "{texto}");
        assert!(texto.contains("est. 1 filas"), "{texto}");
        // DESPUÉS: la reescritura canónica del brief.
        assert!(
            texto.contains("IndexSeek(Person.name = \"Ana\")"),
            "{texto}"
        );
        // Contraste estimación vs realidad.
        assert!(
            texto.contains("Filas reales al ejecutar el plan optimizado: 1"),
            "{texto}"
        );
    }

    #[test]
    fn explain_la_consulta_del_reordenado() {
        let store = grafo();
        let texto = explain(
            "MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE f.age < 40 RETURN p.name, f.name",
            &store,
        )
        .unwrap();
        // ANTES mantiene el Filter sobre el Expand empezando por p…
        assert!(texto.contains("Filter(f:Person AND f.age < 40)"), "{texto}");
        // …y DESPUÉS arranca por f con el filtro pegado al escaneo.
        assert!(texto.contains("NodeScan(Person AS f)"), "{texto}");
        assert!(texto.contains("Expand(f, KNOWS, INCOMING, p)"), "{texto}");
        // La heurística subestima (1/3 vs 3/4 reales): 3 filas reales.
        assert!(
            texto.contains("Filas reales al ejecutar el plan optimizado: 3"),
            "{texto}"
        );
    }

    #[test]
    fn explain_sin_mejoras_lo_dice() {
        let store = grafo();
        let texto = explain("MATCH (p:Person) RETURN p.name", &store).unwrap();
        assert!(texto.contains("no encontró mejoras"), "{texto}");
        // Los dos bloques existen igualmente (el explain no exige cambios):
        // Project y NodeScan estiman 4 en el ANTES y en el DESPUÉS.
        assert_eq!(texto.matches("est. 4 filas").count(), 4, "{texto}");
    }

    #[test]
    fn explain_erroneo_devuelve_exec_error() {
        let store = grafo();
        assert!(matches!(
            explain("SELECCIONA", &store),
            Err(ExecError::Parse(_))
        ));
        assert!(matches!(
            explain("MATCH (p:Person) WHERE q.x = 1 RETURN p", &store),
            Err(ExecError::Plan(_))
        ));
    }

    #[test]
    fn display_de_index_seek_con_y_sin_etiqueta() {
        let con = LogicalPlan::IndexSeek {
            variable: "p".into(),
            label: Some("Person".into()),
            property: "name".into(),
            value: Value::String("Ana".into()),
            ids: vec![0],
        };
        assert_eq!(con.to_string(), "IndexSeek(Person.name = \"Ana\")");
        assert_eq!(con.bound_variables(), vec!["p".to_string()]);
        let sin = LogicalPlan::IndexSeek {
            variable: "x".into(),
            label: None,
            property: "age".into(),
            value: Value::Int(36),
            ids: vec![0, 3],
        };
        assert_eq!(sin.to_string(), "IndexSeek(age = 36)");
    }
}
