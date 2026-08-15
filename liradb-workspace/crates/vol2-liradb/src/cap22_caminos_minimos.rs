use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use std::fmt;

use crate::cap07_modelo::{Edge, EdgeId, NodeId, Value};
use crate::cap08_graph_store::GraphStore;

// ─────────────────── Cap 22: Caminos mínimos ponderados ───────────────────
//
// Abre la Parte V (algoritmos sobre el grafo persistente). El Vol.I ya enseñó
// Dijkstra y Bellman-Ford ALGORÍTMICAMENTE (cap 4: heap binario, distancias,
// predecesores; cap 9: Johnson). El ángulo del Vol.II es ejecutarlos SOBRE el
// grafo persistente: los pesos ya no vienen en un `Vec<Vec<(usize, i64)>>`
// preparado a mano, sino de las PROPIEDADES de las aristas del store — como
// pide la consulta del brief (§cap 22):
//
// ```text
//   SHORTEST PATH FROM node:1 TO node:42 WEIGHT relationship.distance
// ```
//
// Las piezas del capítulo (brief + CORPUS "Cuándo Dijkstra vs Bellman-Ford
// sobre grafos persistentes"):
//
//   1. **Fuente de pesos** ([`WeightSource`]): de dónde sale el peso de una
//      arista — una propiedad (`WEIGHT relationship.distance`) o una constante
//      (grafo no ponderado → contar saltos). Semántica estricta y tipada
//      ([`edge_weight`]): propiedad ausente o NULL = [`PathError::MissingWeight`],
//      tipo no numérico = [`PathError::InvalidWeight`], NaN/±∞ =
//      [`PathError::NonFiniteWeight`]. Un grafo de propiedades es schemaless:
//      los problemas de calidad de dato deben VERSE, no silenciarse con un
//      default que mienta por omisión.
//
//   2. **Dijkstra** ([`dijkstra`] / [`dijkstra_path`]): min-heap con
//      `BinaryHeap<Reverse<(Cost, NodeId)>>` de std (sin crates), borrado
//      perezoso con nodos settled, predecesores por arista y
//      **finalización anticipada** al destino (cuando el destino sale del
//      heap su distancia es definitiva: el invariante codicioso lo permite).
//
//   3. **Bellman-Ford** ([`bellman_ford`] / [`bellman_ford_path`]): V-1 pasadas
//      con parada temprana si nada cambia, y una pasada extra de verificación
//      que detecta **ciclos negativos alcanzables** desde el origen.
//
// Decisiones de diseño (los porqués):
//
// * **¿Sobre qué estructura? Sobre el trait `GraphStore`** (cap 8), no sobre
//   el CSR del cap 14. Motivo: los pesos viven en `Edge.props` y el acceso
//   EdgeId→Edge es exactamente lo que da el puerto hexagonal; el CSR persiste
//   sólo TOPOLOGÍA (offsets + targets, sin ids de arista), así que no puede
//   responder "¿cuánto pesa esta arista?" sin la proyección con pesos que el
//   cap 26 generalizará. Trabajar contra `&dyn GraphStore` además mantiene el
//   algoritmo agnóstico al backend: MemoryStore hoy, disco mañana. El CSR
//   participa como ORÁCULO de consistencia en un test (la topología que ve la
//   proyección debe coincidir con lo que el algoritmo alcanzó).
//
// * **Pesos negativos**: Dijkstra los RECHAZA con [`PathError::NegativeWeight`]
//   validando TODAS las aristas ANTES de correr (una pasada de sanidad O(E)):
//   su invariante codicioso exige no-negativos y responder con números
//   plausibles pero malos es lo peor que puede hacer una base de datos — se
//   prefiere fallar ruidosamente. Bellman-Ford los ACEPTA (eso es lo que
//   compra con sus V-1 pasadas) y sólo se rinde si hay un ciclo negativo
//   ALCANZABLE desde el origen ([`PathError::NegativeCycle`]): en ese caso las
//   distancias de los nodos aguas abajo del ciclo son -∞ y devolver media
//   tabla sería mentir. Un ciclo negativo en una componente INALCANZABLE no
//   contamina la respuesta (nadie lo alcanza) y no es error.
//
// * **Costes como `f64`**: los pesos nacen de `Value::Int` o `Value::Float`
//   (promoción Int→Float, la misma regla que la comparación del cap 20;
//   documentada la pérdida de precisión más allá de 2^53). Un coste que
//   desborda a infinito se reporta ([`PathError::CostOverflow`]) para que el
//   centinela `INFINITY` (= inalcanzable) nunca se confunda con un coste real.
//
// * **Misma interfaz de resultado** para ambos algoritmos: la tabla
//   [`ShortestPaths`] (distancias + predecesores + [`PathStats`]) y el camino
//   [`Path`] (pasos con arista/origen/destino/peso + coste + stats), más las
//   variantes `_path` punto-a-punto que es la forma de la consulta del brief.

// ─── Cost: f64 con orden total para el heap ───

/// Un coste de camino: `f64` con orden total.
///
/// `f64` NO implementa `Ord` (los NaN rompen el orden total) y
/// `BinaryHeap` necesita `Ord`. Todos los costes que entran en el heap son
/// finitos (los pesos se validan en [`edge_weight`] y los desbordes se
/// reportan como [`PathError::CostOverflow`]), así que el `partial_cmp` no
/// puede fallar y el `expect` es unreachable documentado.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cost(pub f64);

impl Eq for Cost {}

impl Ord for Cost {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .partial_cmp(&other.0)
            .expect("Cost sólo envuelve pesos finitos (validados en edge_weight)")
    }
}

impl PartialOrd for Cost {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// ─── Fuente de pesos ───

/// De dónde sale el peso de una arista.
///
/// Dos fuentes, dos historias:
/// - [`WeightSource::Property`]: el peso es una PROPIEDAD de la arista, como
///   en `WEIGHT relationship.distance` del brief. Es el caso interesante del
///   Vol.II: el dato lo puso un usuario (o un import) y puede estar sucio.
/// - [`WeightSource::Constant`]: todas las aristas pesan lo mismo. Con `1.0`
///   el camino mínimo ponderado degenera en el camino con MENOS SALTOS (lo
///   que hacía el `Expand` encadenado del cap 20 sin saberlo).
///
/// El `Default` es `Constant(1.0)` (contar saltos): lo menos sorprendente
/// cuando nadie ha dicho qué propiedad es el peso.
#[derive(Debug, Clone, PartialEq)]
pub enum WeightSource {
    /// Peso desde la propiedad `props[name]` de la arista (Int o Float).
    Property(String),
    /// Peso fijo para todas las aristas.
    Constant(f64),
}

impl Default for WeightSource {
    fn default() -> Self {
        WeightSource::Constant(1.0)
    }
}

impl WeightSource {
    /// Fuente "propiedad de la arista" (p.ej. `WeightSource::property("distance")`).
    pub fn property(name: impl Into<String>) -> Self {
        WeightSource::Property(name.into())
    }
}

/// Extrae el peso de una arista según la [`WeightSource`], con semántica
/// estricta y errores tipados.
///
/// - Propiedad ausente **o** `Value::Null` → [`PathError::MissingWeight`]
///   (NULL es "sin peso", exactamente como una propiedad ausente: el cap 20
///   ya trató la propiedad ausente como NULL; aquí el NULL se trata como
///   ausencia).
/// - `Bool`/`String`/`Bytes` → [`PathError::InvalidWeight`] informando del
///   tipo encontrado (`Value::type_name`).
/// - `Int(i)` → `i as f64` (promoción Int→Float; pérdida de precisión a
///   partir de 2^53, ver tests).
/// - NaN o ±∞ (Float) → [`PathError::NonFiniteWeight`]: un peso infinito
///   colisionaría con el centinela de inalcanzable y un NaN rompería el
///   orden del heap.
///
/// Los pesos NEGATIVOS no son error aquí: son válidos para Bellman-Ford;
/// Dijkstra los rechaza él mismo (ver [`dijkstra`]).
///
/// ```
/// use vol2_liradb::{edge_weight, Edge, Value, WeightSource};
///
/// let e = Edge::new(0, 1, 2, "ROAD").with_prop("distance", Value::Float(7.5));
/// assert_eq!(edge_weight(&e, &WeightSource::property("distance")).unwrap(), 7.5);
/// assert_eq!(edge_weight(&e, &WeightSource::Constant(1.0)).unwrap(), 1.0);
/// assert!(edge_weight(&e, &WeightSource::property("cost")).is_err()); // sin prop
/// ```
pub fn edge_weight(edge: &Edge, source: &WeightSource) -> Result<f64, PathError> {
    let w = match source {
        WeightSource::Constant(w) => *w,
        WeightSource::Property(name) => match edge.props.get(name) {
            None | Some(Value::Null) => {
                return Err(PathError::MissingWeight {
                    edge: edge.id,
                    prop: name.clone(),
                });
            }
            Some(Value::Int(i)) => *i as f64,
            Some(Value::Float(f)) => *f,
            Some(other) => {
                return Err(PathError::InvalidWeight {
                    edge: edge.id,
                    prop: name.clone(),
                    found: other.type_name().to_string(),
                });
            }
        },
    };
    if !w.is_finite() {
        return Err(PathError::NonFiniteWeight {
            edge: edge.id,
            weight: w,
        });
    }
    Ok(w)
}

// ─── Errores ───

/// Errores de los algoritmos de caminos mínimos (Dijkstra/Bellman-Ford) y,
/// desde el cap 23, de A* y sus heurísticas.
#[derive(Debug, Clone, PartialEq)]
pub enum PathError {
    /// El origen o el destino no existe en el store.
    UnknownNode(NodeId),
    /// La arista no tiene la propiedad del peso (o vale NULL).
    MissingWeight { edge: EdgeId, prop: String },
    /// La propiedad del peso existe pero no es numérica.
    InvalidWeight {
        edge: EdgeId,
        prop: String,
        found: String,
    },
    /// El peso es NaN o ±∞.
    NonFiniteWeight { edge: EdgeId, weight: f64 },
    /// Peso negativo: Dijkstra exige pesos no negativos (Bellman-Ford sí los admite).
    NegativeWeight { edge: EdgeId, weight: f64 },
    /// La suma de costes desbordó a infinito (se reporta para no confundirlo
    /// con el centinela de "inalcanzable").
    CostOverflow { edge: EdgeId },
    /// Bellman-Ford: hay un ciclo negativo alcanzable desde el origen; la
    /// arista señalada todavía relaja tras V-1 pasadas.
    NegativeCycle { edge: EdgeId },
    /// (cap 23) El nodo no tiene la propiedad de coordenada que la heurística
    /// pide (ausente o NULL) — la misma semántica estricta de
    /// [`PathError::MissingWeight`], pero para PROPIEDADES DE NODO.
    MissingCoordinate { node: NodeId, prop: String },
    /// (cap 23) La coordenada existe pero no es un número utilizable
    /// (tipo no numérico, o Float NaN/±∞).
    InvalidCoordinate {
        node: NodeId,
        prop: String,
        found: String,
    },
    /// (cap 23) La heurística devolvió NaN o ±∞ para `node`. Un NaN rompería
    /// el orden total de `Cost` en el heap (panic documentado); se rechaza
    /// ruidosamente en cuanto se consulta.
    NonFiniteHeuristic { node: NodeId, value: f64 },
    /// (cap 23) La heurística devolvió un valor negativo para `node`. En
    /// teoría una h<0 sigue siendo admisible, pero casi siempre es un bug del
    /// caller y además el criterio de parada de A* necesita h(destino)=0
    /// (que la admisibilidad + no-negatividad garantizan).
    NegativeHeuristic { node: NodeId, value: f64 },
    /// (cap 23) [`check_consistency`](crate::check_consistency): la arista
    /// `edge` viola la consistencia `h(from) ≤ w(from,to) + h(to)`.
    InconsistentHeuristic {
        edge: EdgeId,
        h_from: f64,
        bound: f64,
    },
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathError::UnknownNode(id) => write!(f, "unknown node id {id}"),
            PathError::MissingWeight { edge, prop } => {
                write!(
                    f,
                    "edge {edge} has no weight property '{prop}' (missing or NULL)"
                )
            }
            PathError::InvalidWeight { edge, prop, found } => {
                write!(
                    f,
                    "edge {edge}: weight property '{prop}' is {found}, not a number"
                )
            }
            PathError::NonFiniteWeight { edge, weight } => {
                write!(f, "edge {edge}: non-finite weight {weight}")
            }
            PathError::NegativeWeight { edge, weight } => {
                write!(
                    f,
                    "edge {edge}: negative weight {weight} (Dijkstra requires non-negative weights; use bellman_ford)"
                )
            }
            PathError::CostOverflow { edge } => {
                write!(f, "edge {edge}: total cost overflows to infinity")
            }
            PathError::NegativeCycle { edge } => {
                write!(
                    f,
                    "negative cycle reachable from origin (still relaxing edge {edge}); shortest paths are undefined downstream"
                )
            }
            PathError::MissingCoordinate { node, prop } => {
                write!(
                    f,
                    "node {node} has no coordinate property '{prop}' (missing or NULL)"
                )
            }
            PathError::InvalidCoordinate { node, prop, found } => {
                write!(
                    f,
                    "node {node}: coordinate property '{prop}' is {found}, not a usable number"
                )
            }
            PathError::NonFiniteHeuristic { node, value } => {
                write!(f, "heuristic for node {node} is non-finite: {value}")
            }
            PathError::NegativeHeuristic { node, value } => {
                write!(
                    f,
                    "heuristic for node {node} is negative: {value} (A* expects h >= 0)"
                )
            }
            PathError::InconsistentHeuristic {
                edge,
                h_from,
                bound,
            } => {
                write!(
                    f,
                    "inconsistent heuristic: h(from)={h_from} > w + h(to)={bound} across edge {edge}"
                )
            }
        }
    }
}

impl std::error::Error for PathError {}

// ─── Resultado: tabla de caminos y camino individual ───

/// Estadísticas mínimas del cálculo (pedagogía del "cuánto cuesta calcular").
///
/// Campos por algoritmo:
/// - `relax_attempts` / `relax_updates`: aristas consideradas / relajaciones
///   que MEJORARON una distancia (ambos algoritmos).
/// - `popped`: extracciones del heap (sólo Dijkstra y A*; Bellman-Ford no usa
///   heap). Incluye las entradas obsoletas del borrado perezoso.
/// - `rounds`: pasadas sobre la lista de aristas (sólo Bellman-Ford, con la
///   parada temprana si nada cambia; Dijkstra deja 0).
/// - `expanded`: nodos realmente EXPANDIDOS (pops VIVOS: extracciones que no
///   eran entradas obsoletas) — añadido por el cap 23 para poder COMPARAR
///   Dijkstra vs A* (el ahorro de la heurística se mide aquí). En Dijkstra
///   coincide con los pops útiles; en A* puede superar el número de nodos del
///   grafo cuando una heurística inconsistente fuerza re-expansiones. En
///   Bellman-Ford queda a 0 (no expande mediante heap).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PathStats {
    /// Aristas consideradas para relajación.
    pub relax_attempts: u64,
    /// Relajaciones que mejoraron una distancia.
    pub relax_updates: u64,
    /// Extracciones del heap (Dijkstra y A*), obsoletas incluidas.
    pub popped: u64,
    /// Pasadas de aristas ejecutadas (Bellman-Ford).
    pub rounds: u64,
    /// Nodos expandidos de verdad: pops vivos, sin entradas obsoletas (cap 23).
    pub expanded: u64,
}

/// Un paso del camino: cruzar la arista `edge` de `from` a `to` costando `weight`.
#[derive(Debug, Clone, PartialEq)]
pub struct PathStep {
    pub edge: EdgeId,
    pub from: NodeId,
    pub to: NodeId,
    pub weight: f64,
}

/// El camino mínimo entre dos nodos: la respuesta a
/// `SHORTEST PATH FROM ... TO ... WEIGHT ...`.
///
/// `steps` está en orden de avance (origen → destino); `nodes()` lo convierte
/// en la secuencia de nodos. El camino de un nodo a sí mismo tiene 0 pasos y
/// coste 0 (los self-loops positivos nunca mejoran quedarse).
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub origin: NodeId,
    pub destination: NodeId,
    /// Coste total: suma de los pesos de `steps` (o 0.0 si no hay pasos).
    pub cost: f64,
    /// Pasos en orden de avance.
    pub steps: Vec<PathStep>,
    /// Estadísticas del cálculo que produjo este camino.
    pub stats: PathStats,
}

impl Path {
    /// Secuencia de nodos del camino (`[origen, ..., destino]`).
    pub fn nodes(&self) -> Vec<NodeId> {
        let mut nodes = Vec::with_capacity(self.steps.len() + 1);
        nodes.push(self.origin);
        nodes.extend(self.steps.iter().map(|s| s.to));
        nodes
    }

    /// Número de aristas del camino (los "saltos").
    pub fn hops(&self) -> usize {
        self.steps.len()
    }
}

impl fmt::Display for Path {
    /// Formato estilo Cypher: `(n0)-[e2 w=3.5]->(n1)... cost=8.5`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(n{})", self.origin)?;
        for s in &self.steps {
            write!(f, "-[e{} w={}]->(n{})", s.edge, s.weight, s.to)?;
        }
        write!(f, " cost={}", self.cost)
    }
}

/// Tabla de caminos mínimos desde un origen (single-source).
///
/// Invariante: `dist[v] = INFINITY` significa inalcanzable (o id hueco tras
/// borrados); todo coste real es finito porque los desbordes se reportan como
/// [`PathError::CostOverflow`]. Los predecesores guardan el PASO completo
/// (arista + peso), así que [`ShortestPaths::path_to`] reconstruye el camino
/// sin volver a tocar el store.
#[derive(Debug, Clone, PartialEq)]
pub struct ShortestPaths {
    /// Origen del cálculo.
    pub origin: NodeId,
    /// Longitud de las tablas (máximo id de nodo + 1; los huecos quedan INFINITY).
    num_nodes: usize,
    /// `dist[v]`: coste mínimo desde `origin` (INFINITY = inalcanzable).
    dist: Vec<f64>,
    /// `pred[v]`: último paso que fijó `dist[v]` (None si v no fue alcanzado).
    pred: Vec<Option<PathStep>>,
    /// Estadísticas del cálculo.
    pub stats: PathStats,
}

impl ShortestPaths {
    /// Distancia mínima a `v`, o `None` si es inalcanzable (o el id no existe).
    pub fn distance(&self, v: NodeId) -> Option<f64> {
        match self.dist.get(v) {
            Some(d) if d.is_finite() => Some(*d),
            _ => None,
        }
    }

    /// Nodos con distancia finita (alcanzados desde el origen, origen incluido).
    pub fn reached(&self) -> Vec<NodeId> {
        (0..self.num_nodes)
            .filter(|&v| self.dist[v].is_finite())
            .collect()
    }

    /// Reconstruye el camino a `dest`, o `None` si es inalcanzable.
    ///
    /// Seguimos `pred` hacia atrás y damos la vuelta. Los predecesores forman
    /// siempre un árbol: un ciclo en la cadena exigiría distancias estrictamente
    /// decrecientes al rodearlo, es decir, un ciclo negativo — y Bellman-Ford
    /// ya se habría negado a devolver la tabla.
    ///
    /// `stats` es el del cálculo COMPLETO (no sólo de este camino): ver el
    /// trabajo total es parte de la pedagogía del capítulo.
    pub fn path_to(&self, dest: NodeId) -> Option<Path> {
        let cost = self.distance(dest)?;
        let mut steps = Vec::new();
        let mut v = dest;
        while let Some(step) = &self.pred[v] {
            steps.push(step.clone());
            v = step.from;
        }
        steps.reverse();
        Some(Path {
            origin: self.origin,
            destination: dest,
            cost,
            steps,
            stats: self.stats,
        })
    }
}

// ─── Helpers internos ───

/// El nodo debe existir: preguntar por uno inexistente es error del caller.
pub(crate) fn ensure_node(store: &dyn GraphStore, id: NodeId) -> Result<(), PathError> {
    if store.get_node(id).is_some() {
        Ok(())
    } else {
        Err(PathError::UnknownNode(id))
    }
}

/// Tamaño de las tablas: máximo id de nodo + 1 (los ids huecos quedan INFINITY).
pub(crate) fn table_len(store: &dyn GraphStore) -> usize {
    store.iter_nodes().map(|n| n.id).max().map_or(0, |m| m + 1)
}

/// Sanidad eager de pesos O(E) para los algoritmos codiciosos por heap
/// (Dijkstra y, desde el cap 23, A*): peso presente, numérico, finito y NO
/// NEGATIVO en TODAS las aristas del store, antes de contestar.
///
/// ¿Por qué rechazar también las negativas de zonas que esta consulta no va a
/// tocar? Porque una base de datos prefiere FAIL ruidosamente a contestar con
/// números que podrían ser válidos por casualidad; el día que el usuario
/// importe pesos negativos querrá saberlo en TODAS sus consultas, no sólo en
/// las que cruzan esa zona. Para pesos negativos legítimos existe
/// [`bellman_ford`]. Compartida por cap 22 y cap 23 para que el contracto de
/// datos sea EXACTAMENTE el mismo en ambos.
pub(crate) fn validate_edge_weights(
    store: &dyn GraphStore,
    weight: &WeightSource,
) -> Result<(), PathError> {
    for edge in store.iter_edges() {
        let w = edge_weight(edge, weight)?;
        if w < 0.0 {
            return Err(PathError::NegativeWeight {
                edge: edge.id,
                weight: w,
            });
        }
    }
    Ok(())
}

// ─── Dijkstra ───

/// Dijkstra single-source sobre el grafo del store.
///
/// Devuelve la [`ShortestPaths`] tabla completa. Para la variante punto-a-
/// punto con finalización anticipada ver [`dijkstra_path`].
///
/// Contracto de pesos: TODAS las aristas se validan antes de correr (una
/// pasada de sanidad O(E)): peso presente y numérico, finito, no negativo.
/// ¿Por qué rechazar también las negativas de zonas que esta consulta no va a
/// tocar? Porque una base de datos prefiere FAIL ruidosamente a contestar con
/// números que podrían ser válidos por casualidad; el día que el usuario
/// importe pesos negativos querrá saberlo en TODAS sus consultas, no sólo en
/// las que cruzan esa zona. Para pesos negativos legítimos existe
/// [`bellman_ford`].
///
/// ```
/// use vol2_liradb::{demo_graph, dijkstra, MemoryStore};
///
/// let store = demo_graph();
/// let sp = dijkstra(&store, 0, &Default::default()).unwrap(); // pesos = 1.0 (saltos)
/// assert_eq!(sp.distance(2), Some(2.0)); // 0 -KNOWS-> 1 -KNOWS-> 2
/// assert_eq!(sp.distance(5), Some(2.0)); // 0 -KNOWS-> 1 -LIVES_IN-> 5 (Lisboa)
/// assert_eq!(sp.distance(3), None);      // Dani (sólo self-loop) es inalcanzable
/// ```
pub fn dijkstra(
    store: &dyn GraphStore,
    origin: NodeId,
    weight: &WeightSource,
) -> Result<ShortestPaths, PathError> {
    ensure_node(store, origin)?;
    dijkstra_impl(store, origin, weight, None)
}

/// Dijkstra punto-a-punto con **finalización anticipada**: en cuanto el
/// destino sale del heap su distancia es DEFINITIVA (invariante codicioso:
/// todo lo que queda en el heap pesa igual o más) y dejamos de trabajar.
///
/// `Ok(None)` = destino inalcanzable (no es error: es una respuesta).
/// Los errores son del contracto de pesos o de nodos inexistentes.
pub fn dijkstra_path(
    store: &dyn GraphStore,
    origin: NodeId,
    dest: NodeId,
    weight: &WeightSource,
) -> Result<Option<Path>, PathError> {
    ensure_node(store, origin)?;
    ensure_node(store, dest)?;
    let sp = dijkstra_impl(store, origin, weight, Some(dest))?;
    Ok(sp.path_to(dest))
}

/// Núcleo compartido: `target = Some(d)` corta cuando `d` sale del heap.
fn dijkstra_impl(
    store: &dyn GraphStore,
    origin: NodeId,
    weight: &WeightSource,
    target: Option<NodeId>,
) -> Result<ShortestPaths, PathError> {
    // Sanidad de datos ANTES de contestar: la respuesta de una BD no debe
    // depender de qué zona del grafo llegó a pisar la consulta.
    validate_edge_weights(store, weight)?;

    let num_nodes = table_len(store);
    let mut dist = vec![f64::INFINITY; num_nodes];
    let mut pred: Vec<Option<PathStep>> = vec![None; num_nodes];
    let mut settled = vec![false; num_nodes];
    let mut stats = PathStats::default();

    // Min-heap: std::collections::BinaryHeap es un max-heap; Reverse lo
    // volta. La clave (Cost, NodeId) ordena por coste y desempata por id
    // (determinismo). Las entradas obsoletas se detectan con `settled`.
    let mut heap: BinaryHeap<Reverse<(Cost, NodeId)>> = BinaryHeap::new();
    dist[origin] = 0.0;
    heap.push(Reverse((Cost(0.0), origin)));

    while let Some(Reverse((Cost(d), u))) = heap.pop() {
        stats.popped += 1;
        if settled[u] {
            continue; // entrada obsoleta (lazy deletion)
        }
        settled[u] = true;
        stats.expanded += 1; // (cap 23) pop VIVO: aquí se expande de verdad
        if target == Some(u) {
            break; // finalización anticipada: dist[u] ya es definitiva
        }
        for eid in store.out_edges(u) {
            let edge = store
                .get_edge(eid)
                .expect("invariante del store: la adjacencia sólo contiene aristas vivas");
            let w = edge_weight(edge, weight)?; // ya validado arriba; relectura ≤ 1 vez por arista
            stats.relax_attempts += 1;
            let new = d + w; // w ≥ 0 por la validación eager
            if !new.is_finite() {
                return Err(PathError::CostOverflow { edge: eid });
            }
            let v = edge.target;
            if new < dist[v] {
                dist[v] = new;
                pred[v] = Some(PathStep {
                    edge: eid,
                    from: u,
                    to: v,
                    weight: w,
                });
                stats.relax_updates += 1;
                heap.push(Reverse((Cost(new), v)));
            }
        }
    }

    Ok(ShortestPaths {
        origin,
        num_nodes,
        dist,
        pred,
        stats,
    })
}

// ─── Bellman-Ford ───

/// Bellman-Ford single-source: admite pesos NEGATIVOS, detecta ciclos
/// negativos alcanzables desde el origen.
///
/// Mismo contrato de datos que [`dijkstra`] (peso presente, numérico, finito)
/// pero SIN el requisito de no-negatividad — eso es exactamente lo que compran
/// sus V-1 pasadas sobre TODA la lista de aristas (Dijkstra toca cada arista
/// una vez; Bellman-Ford hasta V-1, con parada temprana si una pasada no
/// cambia nada).
///
/// Ciclo negativo ALCANZABLE desde el origen → [`PathError::NegativeCycle`]
/// señalando una arista que aún relaja: las distancias aguas abajo del ciclo
/// tienden a -∞ y media tabla válida sería mentir. Un ciclo negativo en una
/// componente inalcanzable NO es error (nadie puede llegar a él).
///
/// ```
/// use vol2_liradb::{bellman_ford, Edge, MemoryStore, Node, Value, WeightSource};
/// use vol2_liradb::GraphStore;
///
/// let mut s = MemoryStore::new();
/// s.put_node(Node::new(0, "A")).unwrap();
/// s.put_node(Node::new(1, "B")).unwrap();
/// s.put_edge(Edge::new(0, 0, 1, "R").with_prop("w", Value::Int(-4))).unwrap();
/// let w = WeightSource::property("w");
/// assert_eq!(bellman_ford(&s, 0, &w).unwrap().distance(1), Some(-4.0));
/// ```
pub fn bellman_ford(
    store: &dyn GraphStore,
    origin: NodeId,
    weight: &WeightSource,
) -> Result<ShortestPaths, PathError> {
    ensure_node(store, origin)?;

    let num_nodes = table_len(store);
    // Materializamos la lista de relajación UNA vez: los pesos se leen de las
    // props una sola pasada (leerlas en cada ronda sería V-1 búsquedas hash
    // por arista para el mismo valor).
    let mut relax: Vec<(NodeId, NodeId, EdgeId, f64)> = Vec::new();
    for edge in store.iter_edges() {
        let w = edge_weight(edge, weight)?;
        relax.push((edge.source, edge.target, edge.id, w));
    }

    let mut dist = vec![f64::INFINITY; num_nodes];
    let mut pred: Vec<Option<PathStep>> = vec![None; num_nodes];
    dist[origin] = 0.0;
    let mut stats = PathStats::default();

    // V-1 pasadas (una cadena más larga que V-1 saltos repetiría nodo).
    let max_rounds = num_nodes.saturating_sub(1);
    for _ in 0..max_rounds {
        stats.rounds += 1;
        let mut changed = false;
        for &(u, v, eid, w) in &relax {
            stats.relax_attempts += 1;
            let du = dist[u];
            if du == f64::INFINITY {
                continue; // u inalcanzable: relajar desde él no significa nada
            }
            let new = du + w;
            if !new.is_finite() {
                return Err(PathError::CostOverflow { edge: eid });
            }
            if new < dist[v] {
                dist[v] = new;
                pred[v] = Some(PathStep {
                    edge: eid,
                    from: u,
                    to: v,
                    weight: w,
                });
                stats.relax_updates += 1;
                changed = true;
            }
        }
        if !changed {
            break; // parada temprana: nada cambió, otra pasada no cambiaría nada
        }
    }

    // Pasada de verificación: si algo TODAVÍA relaja con origen alcanzable,
    // hay un ciclo negativo alcanzable desde el origen.
    for &(u, v, eid, w) in &relax {
        if dist[u] != f64::INFINITY && dist[u] + w < dist[v] {
            return Err(PathError::NegativeCycle { edge: eid });
        }
    }

    Ok(ShortestPaths {
        origin,
        num_nodes,
        dist,
        pred,
        stats,
    })
}

/// Bellman-Ford punto-a-punto: misma interfaz que [`dijkstra_path`].
///
/// Sin finalización anticipada por destino (no hay invariante codicioso que
/// la justifique: con pesos negativos un camino más largo puede ganar
/// después); la parada temprana de Bellman-Ford es la de PASADAS (parar cuando
/// nada cambia), que sí está implementada.
pub fn bellman_ford_path(
    store: &dyn GraphStore,
    origin: NodeId,
    dest: NodeId,
    weight: &WeightSource,
) -> Result<Option<Path>, PathError> {
    ensure_node(store, origin)?;
    ensure_node(store, dest)?;
    let sp = bellman_ford(store, origin, weight)?;
    Ok(sp.path_to(dest))
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests_caminos {
    use super::*;
    use crate::cap07_modelo::Node;
    use crate::cap08_graph_store::MemoryStore;
    use crate::cap14_csr::Csr;

    // ════════════════════════════════════════════════════════════════
    //  Fixtures y helpers
    // ════════════════════════════════════════════════════════════════

    /// Grafo dirigido ponderado desde `(nodos, [(id, from, to, peso)])`.
    /// Los pesos van a la propiedad "weight" (Value::Float).
    fn grafo(n: usize, aristas: &[(EdgeId, NodeId, NodeId, f64)]) -> MemoryStore {
        let mut s = MemoryStore::new();
        for id in 0..n {
            s.put_node(Node::new(id, "N")).unwrap();
        }
        for &(eid, from, to, w) in aristas {
            s.put_edge(Edge::new(eid, from, to, "REL").with_prop("weight", Value::Float(w)))
                .unwrap();
        }
        s
    }

    /// Como [`grafo`] pero con pesos Int (para la promoción Int→Float).
    fn grafo_int(n: usize, aristas: &[(EdgeId, NodeId, NodeId, i64)]) -> MemoryStore {
        let mut s = MemoryStore::new();
        for id in 0..n {
            s.put_node(Node::new(id, "N")).unwrap();
        }
        for &(eid, from, to, w) in aristas {
            s.put_edge(Edge::new(eid, from, to, "REL").with_prop("weight", Value::Int(w)))
                .unwrap();
        }
        s
    }

    fn w() -> WeightSource {
        WeightSource::property("weight")
    }

    /// Verifica que un camino es VÁLIDO contra el store: continuidad
    /// (from/to encadenan desde el origen hasta el destino), las aristas
    /// existen y apuntan donde dicen, y el coste es la suma de los pesos.
    fn assert_camino_valido(store: &dyn GraphStore, path: &Path, weight: &WeightSource) {
        let mut actual = path.origin;
        let mut suma = 0.0;
        for step in &path.steps {
            let edge = store.get_edge(step.edge).expect("arista del camino existe");
            assert_eq!(edge.source, step.from);
            assert_eq!(edge.target, step.to);
            assert_eq!(
                edge.source, actual,
                "el paso encadena desde el nodo anterior"
            );
            assert_eq!(step.weight, edge_weight(edge, weight).unwrap());
            suma += step.weight;
            actual = step.to;
        }
        assert_eq!(actual, path.destination, "el camino termina en el destino");
        assert_eq!(path.cost, suma, "el coste es la suma de los pesos");
    }

    // ════════════════════════════════════════════════════════════════
    //  Cost y extracción de pesos
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn cost_orden_total_con_negativos_y_cero() {
        assert!(Cost(-3.0) < Cost(0.0));
        assert!(Cost(0.0) < Cost(0.5));
        assert!(Cost(2.5) < Cost(10.0));
        assert_eq!(Cost(1.0).cmp(&Cost(1.0)), Ordering::Equal);
    }

    #[test]
    fn peso_desde_propiedad_constante_y_default() {
        let e = Edge::new(7, 0, 1, "ROAD").with_prop("distance", Value::Float(7.5));
        assert_eq!(
            edge_weight(&e, &WeightSource::property("distance")).unwrap(),
            7.5
        );
        assert_eq!(edge_weight(&e, &WeightSource::Constant(2.0)).unwrap(), 2.0);
        // Default: Constant(1.0) — contar saltos.
        assert_eq!(edge_weight(&e, &WeightSource::default()).unwrap(), 1.0);
    }

    #[test]
    fn peso_ausente_o_null_es_missing() {
        let sin_prop = Edge::new(0, 0, 1, "R");
        assert_eq!(
            edge_weight(&sin_prop, &w()),
            Err(PathError::MissingWeight {
                edge: 0,
                prop: "weight".into()
            })
        );
        let con_null = Edge::new(1, 0, 1, "R").with_prop("weight", Value::Null);
        assert_eq!(
            edge_weight(&con_null, &w()),
            Err(PathError::MissingWeight {
                edge: 1,
                prop: "weight".into()
            })
        );
    }

    #[test]
    fn peso_de_tipo_no_numerico_es_invalid() {
        let e = Edge::new(3, 0, 1, "R").with_prop("weight", Value::String("mucho".into()));
        assert_eq!(
            edge_weight(&e, &w()),
            Err(PathError::InvalidWeight {
                edge: 3,
                prop: "weight".into(),
                found: "String".into()
            })
        );
        let e_bool = Edge::new(4, 0, 1, "R").with_prop("weight", Value::Bool(true));
        assert!(matches!(
            edge_weight(&e_bool, &w()),
            Err(PathError::InvalidWeight { found, .. }) if found == "Bool"
        ));
    }

    #[test]
    fn peso_nan_o_infinito_es_no_finito() {
        let nan = Edge::new(0, 0, 1, "R").with_prop("weight", Value::Float(f64::NAN));
        assert!(matches!(
            edge_weight(&nan, &w()),
            Err(PathError::NonFiniteWeight { .. })
        ));
        let inf = Edge::new(1, 0, 1, "R").with_prop("weight", Value::Float(f64::INFINITY));
        assert!(matches!(
            edge_weight(&inf, &w()),
            Err(PathError::NonFiniteWeight { .. })
        ));
        // Constant también se valida.
        assert!(matches!(
            edge_weight(&nan, &WeightSource::Constant(f64::NEG_INFINITY)),
            Err(PathError::NonFiniteWeight { .. })
        ));
    }

    #[test]
    fn errores_display_y_std_error() {
        let errs = vec![
            PathError::UnknownNode(9),
            PathError::MissingWeight {
                edge: 2,
                prop: "cost".into(),
            },
            PathError::InvalidWeight {
                edge: 2,
                prop: "cost".into(),
                found: "Bool".into(),
            },
            PathError::NonFiniteWeight {
                edge: 2,
                weight: f64::NAN,
            },
            PathError::NegativeWeight {
                edge: 5,
                weight: -1.5,
            },
            PathError::CostOverflow { edge: 6 },
            PathError::NegativeCycle { edge: 7 },
        ];
        for e in &errs {
            assert!(!e.to_string().is_empty());
            let _: &dyn std::error::Error = e; // trait implementado
        }
        assert!(errs[0].to_string().contains("9"));
        assert!(errs[4].to_string().contains("bellman_ford"));
    }

    // ════════════════════════════════════════════════════════════════
    //  Dijkstra: caminos, pesos desde props, casos límite
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn dijkstra_camino_clasico_del_diamante() {
        // El grafo del cap. 4 del Vol.I: 0-1 (1), 1-3 (2), 0-2 (4), 2-3 (3).
        // Óptimo a 3: 0→1→3 con coste 3 (la vía por 2 cuesta 7).
        let s = grafo(
            4,
            &[
                (0, 0, 1, 1.0),
                (1, 1, 3, 2.0),
                (2, 0, 2, 4.0),
                (3, 2, 3, 3.0),
            ],
        );
        let path = dijkstra_path(&s, 0, 3, &w()).unwrap().unwrap();
        assert_eq!(path.nodes(), vec![0, 1, 3]);
        assert_eq!(path.cost, 3.0);
        assert_eq!(path.hops(), 2);
        assert_eq!(path.steps[0].edge, 0);
        assert_eq!(path.steps[1].edge, 1);
        assert_camino_valido(&s, &path, &w());
    }

    #[test]
    fn dijkstra_la_ruta_con_menos_saltos_no_es_la_mas_barata() {
        // Directo caro (10) vs dos saltos baratos (2+3): Dijkstra elige 2 saltos.
        let s = grafo(3, &[(0, 0, 1, 2.0), (1, 1, 2, 3.0), (2, 0, 2, 10.0)]);
        let path = dijkstra_path(&s, 0, 2, &w()).unwrap().unwrap();
        assert_eq!(path.nodes(), vec![0, 1, 2]);
        assert_eq!(path.cost, 5.0);
    }

    #[test]
    fn la_fuente_de_pesos_cambia_la_respuesta() {
        // Mismo grafo: ponderado por "weight" gana la ruta 2 saltos (coste 5);
        // no ponderado (Constant 1.0) gana el salto directo (coste 1).
        let s = grafo(3, &[(0, 0, 1, 2.0), (1, 1, 2, 3.0), (2, 0, 2, 10.0)]);
        let ponderado = dijkstra_path(&s, 0, 2, &w()).unwrap().unwrap();
        assert_eq!(ponderado.cost, 5.0);
        let saltos = dijkstra_path(&s, 0, 2, &WeightSource::Constant(1.0))
            .unwrap()
            .unwrap();
        assert_eq!(saltos.cost, 1.0);
        assert_eq!(saltos.nodes(), vec![0, 2]);
    }

    #[test]
    fn dijkstra_multigrafo_elige_la_arista_paralela_mas_barata() {
        // Dos aristas 0→1: la e1 (peso 3) debe ganar sobre la e0 (peso 7).
        let s = grafo(2, &[(0, 0, 1, 7.0), (1, 0, 1, 3.0)]);
        let path = dijkstra_path(&s, 0, 1, &w()).unwrap().unwrap();
        assert_eq!(path.cost, 3.0);
        assert_eq!(path.steps[0].edge, 1);
        assert_eq!(path.hops(), 1);
    }

    #[test]
    fn dijkstra_destino_inalcanzable_devuelve_none() {
        // 3 no recibe aristas y no hay camino desde 0.
        let s = grafo(4, &[(0, 0, 1, 1.0), (1, 1, 2, 1.0)]);
        assert!(dijkstra_path(&s, 0, 3, &w()).unwrap().is_none());
        let sp = dijkstra(&s, 0, &w()).unwrap();
        assert_eq!(sp.distance(3), None);
        assert_eq!(sp.reached(), vec![0, 1, 2]);
    }

    #[test]
    fn dijkstra_origen_igual_destino_coste_cero() {
        let s = grafo(3, &[(0, 0, 1, 1.0)]);
        let path = dijkstra_path(&s, 2, 2, &w()).unwrap().unwrap();
        assert_eq!(path.cost, 0.0);
        assert!(path.steps.is_empty());
        assert_eq!(path.nodes(), vec![2]);
    }

    #[test]
    fn dijkstra_self_loop_positivo_no_ayuda() {
        // 0→0 peso 5: quedarse (coste 0) siempre gana. Y un self-loop en el
        // camino (1→1) se relaja una vez y pierde.
        let s = grafo(
            3,
            &[
                (0, 0, 0, 5.0),
                (1, 0, 1, 1.0),
                (2, 1, 1, 2.0),
                (3, 1, 2, 1.0),
            ],
        );
        let path = dijkstra_path(&s, 0, 0, &w()).unwrap().unwrap();
        assert_eq!(path.cost, 0.0);
        assert!(path.steps.is_empty());
        let sp = dijkstra(&s, 0, &w()).unwrap();
        assert_eq!(sp.distance(1), Some(1.0));
        assert_eq!(sp.distance(2), Some(2.0));
    }

    #[test]
    fn dijkstra_nodos_desconocidos_en_grafo_vacio_y_con_huecos() {
        let vacio = MemoryStore::new();
        assert_eq!(
            dijkstra_path(&vacio, 0, 1, &w()),
            Err(PathError::UnknownNode(0))
        );
        // delete_node(0) deja un hueco: preguntar por él es UnknownNode.
        let mut s = grafo(3, &[(0, 1, 2, 1.0)]);
        assert!(s.delete_node(0));
        assert_eq!(dijkstra(&s, 0, &w()), Err(PathError::UnknownNode(0)));
        assert_eq!(
            dijkstra_path(&s, 1, 0, &w()),
            Err(PathError::UnknownNode(0))
        );
        // El grafo restante sigue respondiendo.
        assert_eq!(dijkstra(&s, 1, &w()).unwrap().distance(2), Some(1.0));
    }

    #[test]
    fn dijkstra_rechaza_negativos_aun_en_zonas_no_visitadas() {
        // El negativo está en la componente 3⇄4, inalcanzable desde 0:
        // la sanidad eager lo rechaza igualmente (decisión documentada).
        let s = grafo(5, &[(0, 0, 1, 1.0), (1, 3, 4, -2.0), (2, 4, 3, 1.0)]);
        assert_eq!(
            dijkstra(&s, 0, &w()),
            Err(PathError::NegativeWeight {
                edge: 1,
                weight: -2.0
            })
        );
    }

    #[test]
    fn dijkstra_coste_que_desborda_es_error_tipado() {
        let s = grafo(3, &[(0, 0, 1, 1e308), (1, 1, 2, 1e308)]);
        assert_eq!(
            dijkstra_path(&s, 0, 2, &w()),
            Err(PathError::CostOverflow { edge: 1 })
        );
    }

    #[test]
    fn dijkstra_finalizacion_anticipada_extrae_menos_nodos() {
        // Cadena 0→1→...→5: preguntar por 1 debe asentar sólo {0, 1}; la
        // tabla completa asienta los 6.
        let s = grafo(
            6,
            &[
                (0, 0, 1, 1.0),
                (1, 1, 2, 1.0),
                (2, 2, 3, 1.0),
                (3, 3, 4, 1.0),
                (4, 4, 5, 1.0),
            ],
        );
        let corto = dijkstra_path(&s, 0, 1, &w()).unwrap().unwrap();
        let tabla = dijkstra(&s, 0, &w()).unwrap();
        assert!(corto.stats.popped < tabla.stats.popped);
        assert!(
            corto.stats.popped <= 2,
            "asentar el destino no debe requerir más pops: {}",
            corto.stats.popped
        );
        // El atajo no cambia la respuesta:
        assert_eq!(corto.cost, tabla.distance(1).unwrap());
        // Coherencia mínima de stats:
        assert!(tabla.stats.relax_updates <= tabla.stats.relax_attempts);
    }

    #[test]
    fn dijkstra_tabla_completa_con_distancias() {
        let s = grafo(
            5,
            &[
                (0, 0, 1, 10.0),
                (1, 0, 2, 3.0),
                (2, 2, 3, 4.0),
                (3, 3, 1, 2.0),
                (4, 1, 4, 1.0),
            ],
        );
        let sp = dijkstra(&s, 0, &w()).unwrap();
        assert_eq!(sp.distance(0), Some(0.0));
        assert_eq!(sp.distance(1), Some(9.0)); // 0→2→3→1 = 3+4+2
        assert_eq!(sp.distance(2), Some(3.0));
        assert_eq!(sp.distance(3), Some(7.0)); // 0→2→3
        assert_eq!(sp.distance(4), Some(10.0)); // +1 desde 1
        // path_to coincide con la distancia de la tabla:
        let p4 = sp.path_to(4).unwrap();
        assert_eq!(p4.cost, 10.0);
        assert_eq!(p4.nodes(), vec![0, 2, 3, 1, 4]);
        assert_camino_valido(&s, &p4, &w());
    }

    #[test]
    fn pesos_int_se_promocionan_a_float_con_perdida_documentada() {
        // 2^53 + 1 no es representable en f64: se promociona a 2^53.
        let s = grafo_int(2, &[(0, 0, 1, 9_007_199_254_740_993)]);
        let sp = dijkstra(&s, 0, &w()).unwrap();
        assert_eq!(sp.distance(1), Some(9_007_199_254_740_992.0));
        // ...y valores razonables son exactos:
        let s2 = grafo_int(3, &[(0, 0, 1, 3), (1, 1, 2, 4)]);
        assert_eq!(dijkstra(&s2, 0, &w()).unwrap().distance(2), Some(7.0));
    }

    // ════════════════════════════════════════════════════════════════
    //  Bellman-Ford: negativos, ciclos, consistencia con Dijkstra
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn bellman_ford_coincide_con_dijkstra_sin_negativos() {
        // Sin pesos negativos ambos deben dar LA MISMA tabla de distancias.
        let s = grafo(
            6,
            &[
                (0, 0, 1, 7.0),
                (1, 0, 2, 9.0),
                (2, 0, 5, 14.0),
                (3, 1, 2, 10.0),
                (4, 1, 3, 15.0),
                (5, 2, 3, 11.0),
                (6, 2, 5, 2.0),
                (7, 3, 4, 6.0),
                (8, 5, 4, 9.0),
            ],
        );
        let dj = dijkstra(&s, 0, &w()).unwrap();
        let bf = bellman_ford(&s, 0, &w()).unwrap();
        for v in 0..6 {
            assert_eq!(dj.distance(v), bf.distance(v), "dist({v}) difiere");
        }
        // Y la misma respuesta punto-a-punto:
        let pd = dijkstra_path(&s, 0, 4, &w()).unwrap().unwrap();
        let pb = bellman_ford_path(&s, 0, 4, &w()).unwrap().unwrap();
        assert_eq!(pd.cost, pb.cost);
        assert_eq!(pd.nodes(), pb.nodes());
        assert_eq!(pd.cost, 20.0); // 0→2→5→4 = 9+2+9
        assert_camino_valido(&s, &pb, &w());
    }

    #[test]
    fn bellman_ford_explota_un_peso_negativo_que_dijkstra_rechaza() {
        // 0→2 (3) y 2→3 (-4): la ruta 0→2→3 cuesta -1, mejor que el directo
        // 0→3 (1). Dijkstra ni intenta; Bellman-Ford la encuentra.
        let s = grafo(4, &[(0, 0, 2, 3.0), (1, 2, 3, -4.0), (2, 0, 3, 1.0)]);
        assert_eq!(
            dijkstra(&s, 0, &w()),
            Err(PathError::NegativeWeight {
                edge: 1,
                weight: -4.0
            })
        );
        let path = bellman_ford_path(&s, 0, 3, &w()).unwrap().unwrap();
        assert_eq!(path.cost, -1.0);
        assert_eq!(path.nodes(), vec![0, 2, 3]);
        assert_camino_valido(&s, &path, &w());
        // La tabla entera es coherente:
        let sp = bellman_ford(&s, 0, &w()).unwrap();
        assert_eq!(sp.distance(2), Some(3.0));
        assert_eq!(sp.distance(3), Some(-1.0));
    }

    #[test]
    fn bellman_ford_detecta_ciclo_negativo_alcanzable() {
        // 1→2 (1) y 2→1 (-3): el ciclo suma -2 y es alcanzable desde 0.
        // La pasada de verificación señala la PRIMERA arista (por id) que
        // todavía relaja: la 1→2, cuya mejora ya no puede propagarse en las
        // pasadas disponibles.
        let s = grafo(4, &[(0, 0, 1, 1.0), (1, 1, 2, 1.0), (2, 2, 1, -3.0)]);
        assert_eq!(
            bellman_ford(&s, 0, &w()),
            Err(PathError::NegativeCycle { edge: 1 })
        );
        // También en la variante punto-a-punto:
        assert_eq!(
            bellman_ford_path(&s, 0, 3, &w()),
            Err(PathError::NegativeCycle { edge: 1 })
        );
    }

    #[test]
    fn bellman_ford_ciclo_negativo_inalcanzable_no_contamina() {
        // Ciclo negativo en la componente {4, 5}, inalcanzable desde 0:
        // la parte alcanzable responde con normalidad (decisión documentada).
        let s = grafo(
            6,
            &[
                (0, 0, 1, 2.0),
                (1, 1, 2, 2.0),
                (2, 0, 2, 10.0),
                (3, 4, 5, 1.0),
                (4, 5, 4, -3.0),
            ],
        );
        let sp = bellman_ford(&s, 0, &w()).unwrap();
        assert_eq!(sp.distance(2), Some(4.0));
        assert_eq!(sp.reached(), vec![0, 1, 2]);
        // ...pero si el ciclo es alcanzable, sí es error (control):
        let s2 = grafo(4, &[(0, 0, 1, 1.0), (1, 1, 2, 1.0), (2, 2, 1, -3.0)]);
        assert!(matches!(
            bellman_ford(&s2, 0, &w()),
            Err(PathError::NegativeCycle { .. })
        ));
    }

    #[test]
    fn bellman_ford_self_loop_negativo_es_ciclo_negativo() {
        // Un self-loop con peso -1 ya es un ciclo negativo de longitud 1.
        let s = grafo(2, &[(0, 0, 1, 1.0), (1, 1, 1, -1.0)]);
        assert_eq!(
            bellman_ford(&s, 0, &w()),
            Err(PathError::NegativeCycle { edge: 1 })
        );
    }

    #[test]
    fn bellman_ford_para_temprano_cuando_nada_cambia() {
        // Cadena 0→1→2→3: converge en 2 pasadas (la 3ª no cambia nada y la
        // 4ª ni se ejecuta), muchas menos que las V-1 = 3 posibles... la
        // parada temprana garantiza rounds ≤ las necesarias + 1.
        let s = grafo(
            5,
            &[
                (0, 0, 1, 1.0),
                (1, 1, 2, 1.0),
                (2, 2, 3, 1.0),
                (3, 3, 4, 1.0),
            ],
        );
        let sp = bellman_ford(&s, 0, &w()).unwrap();
        assert_eq!(sp.distance(4), Some(4.0));
        assert_eq!(
            sp.stats.rounds, 2,
            "una cadena converge en la 2ª pasada y la 3ª (sin cambios) corta"
        );
        assert_eq!(sp.stats.popped, 0, "Bellman-Ford no usa heap");
        assert!(sp.stats.relax_updates <= sp.stats.relax_attempts);
    }

    #[test]
    fn bellman_ford_destino_inalcanzable_y_nodos_desconocidos() {
        let s = grafo(4, &[(0, 0, 1, 1.0)]);
        assert!(bellman_ford_path(&s, 0, 3, &w()).unwrap().is_none());
        assert_eq!(
            bellman_ford_path(&s, 9, 0, &w()),
            Err(PathError::UnknownNode(9))
        );
        assert_eq!(
            bellman_ford(&MemoryStore::new(), 0, &w()),
            Err(PathError::UnknownNode(0))
        );
    }

    #[test]
    fn bellman_ford_tambien_valida_los_pesos_estrictamente() {
        // Missing / inválido / no finito: mismas reglas que Dijkstra.
        let mut s = MemoryStore::new();
        s.put_node(Node::new(0, "A")).unwrap();
        s.put_node(Node::new(1, "B")).unwrap();
        s.put_edge(Edge::new(0, 0, 1, "R")).unwrap(); // sin "weight"
        assert_eq!(
            bellman_ford(&s, 0, &w()),
            Err(PathError::MissingWeight {
                edge: 0,
                prop: "weight".into()
            })
        );
        let mut s2 = MemoryStore::new();
        s2.put_node(Node::new(0, "A")).unwrap();
        s2.put_node(Node::new(1, "B")).unwrap();
        s2.put_edge(Edge::new(0, 0, 1, "R").with_prop("weight", Value::String("?".into())))
            .unwrap();
        assert!(matches!(
            bellman_ford(&s2, 0, &w()),
            Err(PathError::InvalidWeight { .. })
        ));
    }

    // ════════════════════════════════════════════════════════════════
    //  Integración: CSR como oráculo + grafo demo + Display
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn proyeccion_csr_consistente_con_lo_alcanzado_por_dijkstra() {
        // El CSR del cap 14 sólo ve TOPOLOGÍA: la proyección del grafo a
        // (source, target) debe estar de acuerdo con lo que Dijkstra dice
        // que es alcanzable. Es el roundtrip proyección→algoritmo posible
        // sin la proyección con pesos (que llega en el cap 26).
        let s = grafo(
            6,
            &[
                (0, 0, 1, 5.0),
                (1, 0, 2, 1.0),
                (2, 2, 3, 1.0),
                (3, 3, 1, 1.0),
                (4, 4, 5, 1.0), // componente aparte
            ],
        );
        let sp = dijkstra(&s, 0, &w()).unwrap();

        let csr = Csr::from_edges(vec![(0, 1), (0, 2), (2, 3), (3, 1), (4, 5)]).unwrap();
        // Alcanzabilidad por BFS sobre el CSR forward:
        let mut seen = vec![false; csr.num_nodes as usize];
        let mut stack = vec![0];
        seen[0] = true;
        while let Some(u) = stack.pop() {
            for &v in csr.neighbors_out(u) {
                if !seen[v] {
                    seen[v] = true;
                    stack.push(v);
                }
            }
        }
        let csr_reach: Vec<NodeId> = (0..csr.num_nodes as usize).filter(|&v| seen[v]).collect();
        assert_eq!(sp.reached(), csr_reach);
    }

    #[test]
    fn demo_graph_con_pesos_reales_y_calidad_de_dato() {
        // El grafo demo del cap 20: KNOWS con "since", LIVES_IN SIN props.
        let s = crate::cap20_volcano::demo_graph();
        // Contando saltos (Default = Constant(1.0)): Ana(0)→Bo(1)→Carla(2).
        let saltos = dijkstra_path(&s, 0, 2, &WeightSource::default())
            .unwrap()
            .unwrap();
        assert_eq!(saltos.cost, 2.0);
        assert_eq!(saltos.nodes(), vec![0, 1, 2]);
        // Con pesos reales ("since"): las KNOWS 0-2 lo tienen, pero el
        // self-loop de Dani (edge 3) y las LIVES_IN no — la sanidad eager
        // reporta la PRIMERA arista sin el dato.
        assert_eq!(
            dijkstra(&s, 0, &WeightSource::property("since")),
            Err(PathError::MissingWeight {
                edge: 3,
                prop: "since".into()
            })
        );
        // Dani(3) sólo tiene su self-loop: desde Ana es inalcanzable.
        assert!(
            dijkstra_path(&s, 0, 3, &WeightSource::default())
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn camino_display_estilo_cypher() {
        let s = grafo(3, &[(4, 0, 1, 1.5), (7, 1, 2, 2.0)]);
        let path = dijkstra_path(&s, 0, 2, &w()).unwrap().unwrap();
        assert_eq!(
            path.to_string(),
            "(n0)-[e4 w=1.5]->(n1)-[e7 w=2]->(n2) cost=3.5"
        );
        // Y el camino vacío (origen = destino):
        let vacio = dijkstra_path(&s, 1, 1, &w()).unwrap().unwrap();
        assert_eq!(vacio.to_string(), "(n1) cost=0");
    }
}
