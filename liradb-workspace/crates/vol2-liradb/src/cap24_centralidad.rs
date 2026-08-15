use std::collections::VecDeque;
use std::fmt;

use crate::cap07_modelo::NodeId;
use crate::cap08_graph_store::GraphStore;

// ─────────────────── Cap 24: Centralidad y PageRank ───────────────────
//
// Parte V (algoritmos sobre el grafo persistente), capítulo 3. El Vol.I ya
// tocó la centralidad dos veces ALGORÍTMICAMENTE (cap 16 espectral:
// Laplaciana y autovalores; cap 24 Vol.I redes: PageRank como ejemplo). El
// ángulo del Vol.II es el mismo de los caps 22-23: ejecutarlas SOBRE el
// grafo del store (`&dyn GraphStore`), con la personalización como primera
// floritura de consulta — no de arista (cap 22) ni de nodo (cap 23), sino
// del VECTOR DE TELEPORT: el usuario decide qué nodos son "el centro del
// mundo". CORPUS vol-II-cap-24: "PageRank personalizado; damping factor".
//
// Las familias del guion (brief §cap 24: grado, closeness, betweenness,
// eigenvector, PageRank, iteraciones, convergencia, coste computacional —
// "no se implementarían todas con optimización industrial; se utilizan para
// explicar familias de algoritmos"):
//
//   1. **Grado** ([`degree_centrality`]): una pasada O(V+E). Con dirección
//      configurable ([`GraphDirection`]: salientes, entrantes o ambas).
//
//   2. **Closeness** ([`closeness_centrality`]): un BFS por nodo
//      (O(V·(V+E))) sobre la proyección NO ponderada (saltos). Corrección
//      de Wasserman-Faust para componentes desconectadas. La versión
//      ponderada (Dijkstra por nodo, cap 22) queda como deuda hacia el cap
//      26: cuando exista la proyección con pesos, el BFS de este capítulo
//      se cambia por `dijkstra` sin tocar nada más.
//
//   3. **Betweenness** ([`betweenness_centrality`]): algoritmo de Brandes
//      (2001) — V BFS con acumulación de dependencias en vez del ingenuo
//      "todos los pares × todos los caminos" (O(V·E) frente a O(V³)).
//      Dirigido; `GraphDirection::Both` simetriza el grafo y la
//      normalización dirigida 1/((n-1)(n-2)) produce entonces los valores
//      de libro de texto no dirigidos.
//
//   4. **Eigenvector** ([`eigenvector_centrality`]): iteración de potencia
//      sobre la adyacencia CRUDA (x_u = Σ_{v→u} x_v), normalizada en L2
//      cada paso. Es el "antes" de la historia: se rompe de DOS maneras en
//      grafos dirigidos reales — la masa de los nodos colgantes (dangling)
//      se ESCAPA, y las componentes periódicas OSCILAN sin converger.
//
//   5. **PageRank** ([`page_rank`]): eigenvector + DOS arreglos — el
//      damping factor d (teleport con probabilidad 1-d) y la
//      redistribución uniforme de la masa colgante. Con ambos, la matriz
//      de transición es positiva ⇒ primitiva ⇒ la iteración de potencia
//      converge siempre y geométricamente (razón ≈ d·λ₂). La demostración
//      vive en un test: el mismo grafo periódico donde eigenvector NO
//      converge, PageRank sí.
//
//   6. **Personalizado** ([`personalized_page_rank`]): el vector de
//      teleport concentra el "mundo" en unas semillas. Es la pieza que el
//      cap 51 (GraphRAG) usará como operador de recuperación: dado un
//      subgrafo de documentos/chunks, PPR sobre la pregunta rankea los
//      trozos relevantes. Por eso [`Teleport`] está SEPARADO del cálculo:
//      `page_rank` = núcleo + teleport uniforme; `personalized_page_rank`
//      = el MISMO núcleo + teleport de semillas. Cero código duplicado,
//      una costura limpia.
//
// Decisiones de diseño (los porqués):
//
// * **¿Sobre qué estructura? Sobre `&dyn GraphStore`, proyectado una vez**
//   ([`Proyeccion`]): los algoritmos iterativos tocan la adyacencia muchas
//   veces por iteración; pagar `out_edges` + `get_edge` por arista en cada
//   ronda sería re-leer el store O(iteraciones) veces. La proyección
//   materializa nodos (ordenados por id: determinismo), índice denso
//   NodeId→posición y vecindarios una sola vez, y TODO el capítulo corre
//   sobre ella. El CSR del cap 14 es exactamente esa idea persistida; la
//   proyección con pesos que los algoritmos de la Parte V esperan llega en
//   el cap 26.
//
// * **Multigrafo**: las aristas paralelas se CONSERVAN en la proyección.
//   Para Brandes cuentan como caminos distintos (semántica correcta de
//   conteo de caminos); para el grado cuentan una por arista; para
//   PageRank cada copia recibe 1/grado_saliente — un enlace duplicado NO
//   duplica el voto del vecino (también duplica el denominador), pero SÍ
//   roba masa a los demás vecinos del mismo origen. Test dedicado.
//
// * **Convergencia por L1** (suma de |Δscore| entre iteraciones): el L1 es
//   la MASA total que se mueve — interpretable como probabilidad ("¿cuánta
//   masa falta por asentar?" < 1e-6) y con el mismo umbral comparable entre
//   grafos de distinto tamaño. El max-delta (máximo cambio por nodo) es
//   más estricto por nodo pero sin lectura probabilística; se documenta y
//   se descarta. El delta final y el historial completo por iteración van
//   en el resultado ([`PageRankResult::history`]) — la convergencia
//   GEOMÉTRICA (razón ≈ d) es contenido del capítulo, no una anécdota.
//
// * **Dangling nodes** (grado de salida 0): su masa se redistribuye
//   UNIFORMEMENTE entre todos los nodos — la decisión clásica de Brin y
//   Page (1998): un surfer que llega a una página sin enlaces teletransporta.
//   La alternativa (descartar la masa y renormalizar al final, "no-scale
//   PageRank") cambia el límite pero no el procedimiento; documentada aquí
//   como variante no implementada. Con redistribución la suma se conserva
//   a 1 en CADA iteración (invariante testeado, no una esperanza).
//
// * **Direccionalidad**: PageRank/eigenvector usan aristas SALIENTES (el
//   surfer sigue enlaces). En un grafo no dirigido el CALLER decide:
//   duplicar cada arista en ambas direcciones antes de insertar (o usar
//   los operadores simétricos de los caps 22-23 para lo no iterativo).
//   Documentado en cada firma.
//
// * **Coste computacional** (la última sección del guion, en las stats):
//   grado O(V+E); closeness O(V·(V+E)); betweenness O(V·E) (Brandes);
//   PageRank O(iter·E). [`CentralidadStats`] cuenta BFS ejecutados,
//   aristas recorridas e iteraciones para poder MEDIRLO, no sólo decirlo.

// ─── Dirección ───

/// Por dónde se cuenta la adyacencia en las centralidades no iterativas
/// (grado, closeness, betweenness).
///
/// PageRank y eigenvector NO la usan: siguen aristas salientes por
/// definición (el surfer pulsa enlaces). El nombre evita colisión con el
/// `Direction` forward/backward del CSR (cap 14) — ambos viven en la API
/// plana del crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphDirection {
    /// Sólo aristas salientes (u → v: v es vecino de u).
    Out,
    /// Sólo aristas entrantes (u ← v: v es vecino de u).
    In,
    /// Unión de ambas — la vista "no dirigida" como CONJUNTO: vecinos
    /// distintos (un store simetrizado a mano no cuenta cada par doble),
    /// self-loops una sola vez (misma convención que el `Expand`
    /// UNDIRECTED del cap 20).
    Both,
}

// ─── Errores ───

/// Errores de las centralidades del cap 24.
#[derive(Debug, Clone, PartialEq)]
pub enum CentralidadError {
    /// El damping factor debe estar en (0,1) ABIERTO: 0 es "sólo teleport"
    /// (una iteración, sin estructura) y 1 es eigenvector puro — que es
    /// exactamente [`eigenvector_centrality`], con sus problemas de
    /// convergencia documentados.
    InvalidDamping { value: f64 },
    /// La tolerancia de convergencia debe ser > 0 y finita.
    InvalidTolerance { value: f64 },
    /// El máximo de iteraciones debe ser ≥ 1.
    InvalidMaxIterations { value: u64 },
    /// Un peso del teleport personalizado es negativo.
    NegativeTeleportWeight { node: NodeId, weight: f64 },
    /// Todos los pesos del teleport son 0: no se puede normalizar.
    ZeroTeleportMass,
    /// Semilla de teleport apunta a un nodo que no existe en el store.
    UnknownNode(NodeId),
}

impl fmt::Display for CentralidadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CentralidadError::InvalidDamping { value } => write!(
                f,
                "invalid damping factor {value}: must be in (0,1) open — use eigenvector_centrality for the undamped limit"
            ),
            CentralidadError::InvalidTolerance { value } => {
                write!(f, "invalid tolerance {value}: must be > 0 and finite")
            }
            CentralidadError::InvalidMaxIterations { value } => {
                write!(f, "invalid max iterations {value}: must be >= 1")
            }
            CentralidadError::NegativeTeleportWeight { node, weight } => {
                write!(f, "teleport weight for node {node} is negative: {weight}")
            }
            CentralidadError::ZeroTeleportMass => {
                write!(f, "teleport weights sum to zero: nothing to normalize")
            }
            CentralidadError::UnknownNode(id) => {
                write!(f, "unknown node id {id}")
            }
        }
    }
}

impl std::error::Error for CentralidadError {}

// ─── Stats y resultados ───

/// Estadísticas del cálculo — la sección "coste computacional" del guion,
/// medible en vez de declamada.
///
/// - `bfs_runs`: BFS ejecutados (closeness: uno por nodo; betweenness:
///   Brandes, uno por nodo; el resto 0).
/// - `edges_scanned`: aristas recorridas por los BFS o por las iteraciones
///   de potencia (PageRank acumula `iteraciones × E`).
/// - `iterations`: pasadas de la iteración de potencia (familia
///   eigenvector/PageRank); 0 en las unipaso.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CentralidadStats {
    /// BFS ejecutados (closeness/betweenness).
    pub bfs_runs: u64,
    /// Aristas recorridas en total (BFS o iteraciones).
    pub edges_scanned: u64,
    /// Iteraciones de potencia ejecutadas (eigenvector/PageRank).
    pub iterations: u64,
}

/// Resultado de las centralidades unipaso/multi-BFS (grado, closeness,
/// betweenness): scores alineadas con `nodes` (ids ordenados).
///
/// `score(id)` responde `None` para ids huecos o inexistentes (tras
/// `delete_node` el hueco no puntuá — no existe).
#[derive(Debug, Clone, PartialEq)]
pub struct CentralityScores {
    /// Nodos vivos en orden ascendente de id (determinismo).
    nodes: Vec<NodeId>,
    /// Score por posición densa (alineada con `nodes`).
    scores: Vec<f64>,
    /// Estadísticas del cálculo.
    pub stats: CentralidadStats,
}

impl CentralityScores {
    /// Score del nodo `id`, o `None` si no existe en el resultado.
    pub fn score(&self, id: NodeId) -> Option<f64> {
        self.nodes.binary_search(&id).ok().map(|i| self.scores[i])
    }

    /// Pares (nodo, score) en orden de id.
    pub fn entries(&self) -> Vec<(NodeId, f64)> {
        self.nodes
            .iter()
            .copied()
            .zip(self.scores.iter().copied())
            .collect()
    }

    /// Ranking: pares (nodo, score) ordenados por score DESCENDENTE,
    /// desempate por id ascendente (determinismo).
    pub fn ranking(&self) -> Vec<(NodeId, f64)> {
        let mut v = self.entries();
        v.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.0.cmp(&b.0))
        });
        v
    }

    /// Número de nodos puntuados.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// ¿Resultado vacío (store sin nodos)?
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl fmt::Display for CentralityScores {
    /// Formato tipo tabla: `n0=0.500 n1=0.750 ...` en orden de id.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, (n, s)) in self.entries().iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "n{}={:.4}", n, s)?;
        }
        Ok(())
    }
}

/// Resultado de PageRank (global o personalizado).
///
/// Además de los scores ([`CentralityScores`] equivalentes), lleva la
/// TRANSPARENCIA del cálculo: `converged` (¿se alcanzó la tolerancia antes
/// del máximo?), `delta` (L1 de la última iteración), `history` (el delta
/// de CADA iteración — la convergencia geométrica es contenido del
/// capítulo) y `damping`.
#[derive(Debug, Clone, PartialEq)]
pub struct PageRankResult {
    nodes: Vec<NodeId>,
    scores: Vec<f64>,
    /// Damping factor usado.
    pub damping: f64,
    /// ¿Convergió (delta < tolerancia) antes de agotar las iteraciones?
    pub converged: bool,
    /// L1 de la última iteración ejecutada.
    pub delta: f64,
    /// Delta L1 de cada iteración ejecutada, en orden.
    pub history: Vec<f64>,
    /// Estadísticas del cálculo.
    pub stats: CentralidadStats,
}

impl PageRankResult {
    /// Score del nodo `id`, o `None` si no existe en el resultado.
    pub fn score(&self, id: NodeId) -> Option<f64> {
        self.nodes.binary_search(&id).ok().map(|i| self.scores[i])
    }

    /// Pares (nodo, score) en orden de id.
    pub fn entries(&self) -> Vec<(NodeId, f64)> {
        self.nodes
            .iter()
            .copied()
            .zip(self.scores.iter().copied())
            .collect()
    }

    /// Ranking: score descendente, desempate por id ascendente.
    pub fn ranking(&self) -> Vec<(NodeId, f64)> {
        let mut v = self.entries();
        v.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.0.cmp(&b.0))
        });
        v
    }

    /// Suma de los scores: con la redistribución uniforme de dangling debe
    /// ser 1 (invariante por iteración; desviaciones de ~1e-15 son redondeo
    /// de f64).
    pub fn total_mass(&self) -> f64 {
        self.scores.iter().sum()
    }

    /// Número de nodos puntuados.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// ¿Resultado vacío (store sin nodos)?
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl fmt::Display for PageRankResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PageRank(d={}, iteraciones={}, delta={:.2e}, convergido={})",
            self.damping,
            self.stats.iterations,
            self.delta,
            if self.converged { "sí" } else { "no" }
        )
    }
}

/// Resultado de la centralidad eigenvector: mismas piezas de transparencia
/// que [`PageRankResult`] (convergencia no garantizada — es su lección),
/// scores normalizadas en norma L2.
#[derive(Debug, Clone, PartialEq)]
pub struct EigenResult {
    nodes: Vec<NodeId>,
    scores: Vec<f64>,
    /// ¿Convergió antes de agotar las iteraciones?
    pub converged: bool,
    /// L1 de la última iteración ejecutada.
    pub delta: f64,
    /// Estadísticas del cálculo.
    pub stats: CentralidadStats,
}

impl EigenResult {
    /// Score del nodo `id`, o `None` si no existe en el resultado.
    pub fn score(&self, id: NodeId) -> Option<f64> {
        self.nodes.binary_search(&id).ok().map(|i| self.scores[i])
    }

    /// Ranking: score descendente, desempate por id ascendente.
    pub fn ranking(&self) -> Vec<(NodeId, f64)> {
        let mut v: Vec<(NodeId, f64)> = self
            .nodes
            .iter()
            .copied()
            .zip(self.scores.iter().copied())
            .collect();
        v.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.0.cmp(&b.0))
        });
        v
    }
}

// ─── Proyección del store ───

/// Adyacencia materializada una vez: nodos ordenados por id (determinismo),
/// índice denso NodeId→posición y vecindarios por dirección.
///
/// Por qué existe: los algoritmos iterativos tocan la adyacencia muchas
/// veces por iteración; hacerlo vía `out_edges`+`get_edge` en cada ronda
/// re-leería el store O(iteraciones) veces. Materializada aquí es la forma
/// en memoria de lo que el CSR del cap 14 persiste — y la que el cap 26
/// generalizará con pesos.
struct Proyeccion {
    /// Nodos vivos, orden ascendente de id.
    nodes: Vec<NodeId>,
    /// `index[id]` = posición densa (None = id hueco o inexistente).
    index: Vec<Option<usize>>,
    /// Vecindarios por posición densa (con aristas paralelas duplicadas).
    vecinos: Vec<Vec<usize>>,
    /// Aristas leídas del store durante la proyección (semilla del coste).
    stats: CentralidadStats,
}

impl Proyeccion {
    /// Proyecta el store según `dir`. Los ids se ordenan y compactan; los
    /// self-loops en `Both` se cuentan una sola vez (convención del
    /// `Expand` UNDIRECTED del cap 20).
    fn proyectar(store: &dyn GraphStore, dir: GraphDirection) -> Self {
        let mut nodes: Vec<NodeId> = store.iter_nodes().map(|n| n.id).collect();
        nodes.sort_unstable();

        let table_len = nodes.last().map_or(0, |&m| m + 1);
        let mut index: Vec<Option<usize>> = vec![None; table_len];
        for (i, &n) in nodes.iter().enumerate() {
            index[n] = Some(i);
        }

        let mut vecinos: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
        let mut stats = CentralidadStats::default();

        for &u in &nodes {
            let mut vecinos_u: Vec<usize> = Vec::new();
            for eid in store.out_edges(u) {
                let edge = store
                    .get_edge(eid)
                    .expect("invariante del store: la adjacencia sólo contiene aristas vivas");
                if let Some(i) = index[edge.target] {
                    vecinos_u.push(i);
                    stats.edges_scanned += 1;
                }
            }
            // Both: la vista "no dirigida" — unión de out+in como CONJUNTO
            // (vecinos distintos; en un store simetrizado a mano cada par
            // antiparalelo no debe contar doble). El self-loop ya entró
            // por out: una vez basta (convención del Expand UNDIRECTED
            // del cap 20).
            if matches!(dir, GraphDirection::Both) {
                for eid in store.in_edges(u) {
                    let edge = store
                        .get_edge(eid)
                        .expect("invariante del store: la adjacencia sólo contiene aristas vivas");
                    if edge.source == u {
                        continue;
                    }
                    if let Some(i) = index[edge.source] {
                        vecinos_u.push(i);
                        stats.edges_scanned += 1;
                    }
                }
                vecinos_u.sort_unstable();
                vecinos_u.dedup();
            } else {
                vecinos_u.sort_unstable();
            }
            vecinos[index[u].expect("u acaba de indexarse")] = vecinos_u;
        }

        // `dir == In` pide los ENTRANTES como vecindario: transponer la
        // adyacencia SALIENTE pura (con sus paralelas: cada arista u→v
        // aporta v como entrante de u).
        if matches!(dir, GraphDirection::In) {
            let mut t: Vec<Vec<usize>> = vec![Vec::new(); vecinos.len()];
            for (v, outs) in vecinos.iter().enumerate() {
                for &w in outs {
                    t[w].push(v);
                }
            }
            for row in &mut t {
                row.sort_unstable();
            }
            Self {
                nodes,
                index,
                vecinos: t,
                stats,
            }
        } else {
            Self {
                nodes,
                index,
                vecinos,
                stats,
            }
        }
    }

    fn len(&self) -> usize {
        self.nodes.len()
    }
}

// ─── Grado ───

/// Centralidad de grado: vecinos (según `dir`) normalizados por n-1.
///
/// O(V+E): una proyección. Con n ≤ 1 todas las scores son 0 (no hay "resto
/// del grafo" que dominating). Las aristas paralelas cuentan una por arista
/// y los self-loops una vez por dirección (out e in).
///
/// ```
/// use vol2_liradb::{degree_centrality, Edge, MemoryStore, Node, GraphDirection};
/// use vol2_liradb::GraphStore;
///
/// let mut s = MemoryStore::new();
/// s.put_node(Node::new(0, "P")).unwrap();
/// s.put_node(Node::new(1, "P")).unwrap();
/// s.put_node(Node::new(2, "P")).unwrap();
/// s.put_edge(Edge::new(0, 0, 1, "KNOWS")).unwrap();
/// s.put_edge(Edge::new(1, 0, 2, "KNOWS")).unwrap();
/// s.put_edge(Edge::new(2, 2, 0, "KNOWS")).unwrap();
///
/// let deg = degree_centrality(&s, GraphDirection::Out).unwrap();
/// assert_eq!(deg.score(0), Some(1.0));  // 2 vecinos / (3-1)
/// assert_eq!(deg.score(1), Some(0.0));  // sin salientes
/// assert_eq!(deg.score(2), Some(0.5));  // 1 vecino / 2
/// ```
pub fn degree_centrality(
    store: &dyn GraphStore,
    dir: GraphDirection,
) -> Result<CentralityScores, CentralidadError> {
    let proj = Proyeccion::proyectar(store, dir);
    let n = proj.len();
    let norm = if n > 1 { (n - 1) as f64 } else { 1.0 };
    let scores = proj
        .vecinos
        .iter()
        .map(|v| if n > 1 { v.len() as f64 / norm } else { 0.0 })
        .collect();
    Ok(CentralityScores {
        nodes: proj.nodes,
        scores,
        stats: proj.stats,
    })
}

// ─── Closeness ───

/// Centralidad de cercanía (Freeman con corrección Wasserman-Faust).
///
/// Un BFS por nodo sobre la proyección NO ponderada (distancias = saltos):
///
/// ```text
///   C(u) = ((r-1)/(n-1)) · ((r-1)/Σd)
/// ```
///
/// donde `r` es el número de nodos alcanzables desde `u` (incluido él) y
/// `Σd` la suma de distancias a los alcanzados. En un grafo conexo la
/// corrección es 1 y queda el (n-1)/Σd clásico; en componentes
/// desconectadas penaliza proporcionalmente a lo que NO alcanzas (dos nodos
/// aislados en componentes separadas no son "igual de centrales" que dos
/// nodos en la misma componente). Nodos sin alcanzados (Σd = 0) o n ≤ 1
/// puntúan 0.
///
/// O(V·(V+E)). La variante ponderada (Dijkstra del cap 22 por cada origen)
/// queda como deuda hacia el cap 26 y su proyección con pesos.
///
/// ```
/// use vol2_liradb::{closeness_centrality, Edge, MemoryStore, Node, GraphDirection};
/// use vol2_liradb::GraphStore;
///
/// let mut s = MemoryStore::new();
/// for i in 0..4 { s.put_node(Node::new(i, "P")).unwrap(); }
/// for (a, b) in [(0, 1), (1, 2), (2, 3)] {
///     s.put_edge(Edge::new(a, a, b, "E")).unwrap();
///     s.put_edge(Edge::new(a + 10, b, a, "E")).unwrap(); // simetrizada
/// }
///
/// let c = closeness_centrality(&s, GraphDirection::Both).unwrap();
/// // Camino 0-1-2-3: C = 3/Σd → centro (0.75) > extremos (0.5)
/// assert_eq!(c.score(0), Some(0.5));
/// assert_eq!(c.score(1), Some(0.75));
/// assert_eq!(c.score(2), Some(0.75));
/// assert_eq!(c.score(3), Some(0.5));
/// ```
pub fn closeness_centrality(
    store: &dyn GraphStore,
    dir: GraphDirection,
) -> Result<CentralityScores, CentralidadError> {
    let proj = Proyeccion::proyectar(store, dir);
    let n = proj.len();
    let mut stats = proj.stats;

    let mut scores = vec![0.0; n];
    if n > 1 {
        for origen in 0..n {
            let mut dist: Vec<Option<u32>> = vec![None; n];
            dist[origen] = Some(0);
            let mut cola: VecDeque<usize> = VecDeque::new();
            cola.push_back(origen);
            let mut alcanzados = 1; // el origen cuenta
            let mut suma_d = 0.0_f64;
            while let Some(v) = cola.pop_front() {
                for &w in &proj.vecinos[v] {
                    stats.edges_scanned += 1;
                    if dist[w].is_none() {
                        let d = dist[v].expect("v fue visitado") + 1;
                        dist[w] = Some(d);
                        alcanzados += 1;
                        suma_d += d as f64;
                        cola.push_back(w);
                    }
                }
            }
            stats.bfs_runs += 1;
            if suma_d > 0.0 {
                let r = alcanzados as f64;
                scores[origen] = ((r - 1.0) / (n as f64 - 1.0)) * ((r - 1.0) / suma_d);
            }
        }
    }

    Ok(CentralityScores {
        nodes: proj.nodes,
        scores,
        stats,
    })
}

// ─── Betweenness (Brandes) ───

/// Centralidad de intermediación — algoritmo de Brandes (2001).
///
/// C(u) = Σ_{s≠u≠t} σ_st(u)/σ_st: la fracción de caminos mínimos ENTRE
/// pares que pasan por u. El cálculo ingenuo enumera pares × caminos
/// (O(V³)); Brandes lo reduce a V BFS con acumulación de dependencias
/// hacia atrás: O(V·E) sin pesos.
///
/// * `dir` decide la adyacencia; `GraphDirection::Both` simetriza y, con
///   `normalized`, produce los valores de libro no dirigidos.
/// * `normalized = true` divide por (n-1)(n-2) — la normalización DIRIGIDA
///   (pares ordenados). Sobre el grafo simetrizado equivale a la
///   convención no dirigida 2/((n-1)(n-2)) aplicada a caminos únicos: el
///   test del camino lineal reproduce los 2/3 del libro de texto.
/// * Aristas paralelas: caminos distintos (σ las cuenta); self-loops
///   nunca están en un camino mínimo (distancia 0 con uno mismo).
/// * n ≤ 2: sin pares s≠u≠t posibles, todas 0 (también con `normalized`).
///
/// ```
/// use vol2_liradb::{betweenness_centrality, Edge, MemoryStore, Node, GraphDirection};
/// use vol2_liradb::GraphStore;
///
/// let mut s = MemoryStore::new();
/// for i in 0..4 { s.put_node(Node::new(i, "P")).unwrap(); }
/// for (a, b) in [(0, 1), (1, 2), (2, 3)] {
///     s.put_edge(Edge::new(a, a, b, "E")).unwrap();
///     s.put_edge(Edge::new(a + 10, b, a, "E")).unwrap(); // simetrizada
/// }
///
/// let bc = betweenness_centrality(&s, GraphDirection::Both, true).unwrap();
/// // El camino 0-1-2-3: los intermedios concentran todos los pares.
/// assert_eq!(bc.score(1), Some(2.0 / 3.0));
/// assert_eq!(bc.score(0), Some(0.0));
/// ```
pub fn betweenness_centrality(
    store: &dyn GraphStore,
    dir: GraphDirection,
    normalized: bool,
) -> Result<CentralityScores, CentralidadError> {
    let proj = Proyeccion::proyectar(store, dir);
    let n = proj.len();
    let mut stats = proj.stats;

    let mut bc = vec![0.0_f64; n];
    for origen in 0..n {
        // Fase 1: BFS contando caminos mínimos σ y predecesores.
        let mut sigma = vec![0.0_f64; n];
        sigma[origen] = 1.0;
        let mut dist: Vec<Option<u32>> = vec![None; n];
        dist[origen] = Some(0);
        let mut preds: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut orden: Vec<usize> = Vec::new();
        let mut cola: VecDeque<usize> = VecDeque::new();
        cola.push_back(origen);
        while let Some(v) = cola.pop_front() {
            orden.push(v);
            for &w in &proj.vecinos[v] {
                stats.edges_scanned += 1;
                match dist[w] {
                    None => {
                        dist[w] = Some(dist[v].expect("v fue visitado") + 1);
                        cola.push_back(w);
                        sigma[w] += sigma[v];
                        preds[w].push(v);
                    }
                    Some(dw) => {
                        // ¿v es predecesor de w en ALGÚN camino mínimo?
                        if dw == dist[v].expect("v fue visitado") + 1 {
                            sigma[w] += sigma[v];
                            preds[w].push(v);
                        }
                    }
                }
            }
        }
        stats.bfs_runs += 1;

        // Fase 2: dependencias hacia atrás (el orden inverso del BFS
        // garantiza que delta[w] está completo cuando se usa).
        let mut delta = vec![0.0_f64; n];
        for &w in orden.iter().rev() {
            for &v in &preds[w] {
                delta[v] += (sigma[v] / sigma[w]) * (1.0 + delta[w]);
            }
            if w != origen {
                bc[w] += delta[w];
            }
        }
    }

    if normalized && n > 2 {
        let factor = 1.0 / ((n as f64 - 1.0) * (n as f64 - 2.0));
        for x in &mut bc {
            *x *= factor;
        }
    } else if normalized {
        // n <= 2: no existen pares s != u != t.
        bc.iter_mut().for_each(|x| *x = 0.0);
    }

    Ok(CentralityScores {
        nodes: proj.nodes,
        scores: bc,
        stats,
    })
}

// ─── Eigenvector ───

/// Centralidad eigenvector: iteración de potencia sobre la adyacencia CRUDA.
///
/// `x_u ← Σ_{v→u} x_v` (normalizado en L2 cada paso para que la masa que
/// ESCAPA por los nodos colgantes no arrastre el vector a 0). Es "lo
/// importante es ser apuntado por lo importante" SIN ningún correctivo:
///
/// * Un nodo sin aristas entrantes converge a **0** por muy bien conectado
///   que esté hacia fuera (test de la estrella).
/// * Un grafo con estructura periódica (cola + 3-ciclo) OSCILA y no
///   converge: `converged = false` tras agotar las iteraciones — y ése es
///   exactamente el grafo donde `page_rank` SÍ converge. El damping no es
///   un truco numérico: hace la matriz positiva (primitiva) y garantiza
///   la convergencia.
///
/// Arranque uniforme; convergencia por L1 < `tol`; máximo `max_iterations`.
/// Grafo sin aristas: todo vector es autovector (autovalor 0) — se devuelve
/// el uniforme con `converged = true` sin iterar.
///
/// ```
/// use vol2_liradb::{eigenvector_centrality, Edge, MemoryStore, Node};
/// use vol2_liradb::GraphStore;
///
/// let mut s = MemoryStore::new();
/// for i in 0..4 { s.put_node(Node::new(i, "P")).unwrap(); }
/// // Estrella: las hojas apuntan al centro → todo el prestigio al centro.
/// for hoja in 0..3 {
///     s.put_edge(Edge::new(hoja, hoja, 3, "E")).unwrap();
/// }
///
/// let ev = eigenvector_centrality(&s, 100, 1e-9).unwrap();
/// assert!(ev.score(3).unwrap() > 0.99);       // el centro se lo lleva todo
/// assert!(ev.score(0).unwrap() < 1e-6);       // las hojas, a cero
/// assert!(ev.converged);
/// ```
pub fn eigenvector_centrality(
    store: &dyn GraphStore,
    max_iterations: u64,
    tol: f64,
) -> Result<EigenResult, CentralidadError> {
    validar_parametros_iterativos(max_iterations, tol)?;
    let proj = Proyeccion::proyectar(store, GraphDirection::Out);
    let n = proj.len();
    let mut stats = proj.stats;

    // Store sin nodos: nada que iterar.
    if n == 0 {
        return Ok(EigenResult {
            nodes: proj.nodes,
            scores: Vec::new(),
            converged: true,
            delta: 0.0,
            stats,
        });
    }

    // Sin aristas no hay estructura que iterar (autovalor 0, cualquier
    // vector sirve): uniforme y a casa.
    let sin_aristas = proj.vecinos.iter().all(|v| v.is_empty());
    if sin_aristas {
        return Ok(EigenResult {
            nodes: proj.nodes,
            scores: vec![1.0; n],
            converged: true,
            delta: 0.0,
            stats,
        });
    }

    // Arranque uniforme con norma L2 = 1.
    let mut x = vec![1.0 / (n as f64).sqrt(); n];
    let mut converged = false;
    let mut delta = 0.0;

    for it in 1..=max_iterations {
        stats.iterations = it;
        let mut y = vec![0.0; n];
        for (v, vecinos_v) in proj.vecinos.iter().enumerate() {
            let xv = x[v];
            if xv == 0.0 {
                continue;
            }
            for &w in vecinos_v {
                y[w] += xv;
                stats.edges_scanned += 1;
            }
        }
        // Normalización L2: la masa que escapa por los colgantes no debe
        // colapsar el vector.
        let norma = y.iter().map(|s| s * s).sum::<f64>().sqrt();
        if norma == 0.0 {
            // Todo el vector se escapó (p. ej. DAG terminado): el límite
            // ya se alcanzó en la iteración anterior.
            converged = true;
            break;
        }
        for s in &mut y {
            *s /= norma;
        }
        delta = y.iter().zip(&x).map(|(a, b)| (a - b).abs()).sum();
        x = y;
        if delta < tol {
            converged = true;
            break;
        }
    }

    Ok(EigenResult {
        nodes: proj.nodes,
        scores: x,
        converged,
        delta,
        stats,
    })
}

// ─── Teleport: la costura del PageRank personalizado ───

/// De dónde teletransporta el surfer aleatorio: el vector de reinicio.
///
/// Ésta es LA pieza que separa el PageRank global del personalizado, y la
/// que el cap 51 (GraphRAG) usará como operador de recuperación: PPR sobre
/// las semillas = la pregunta rankea el subgrafo de documentos. Mantenerlo
/// como parámetro independiente del núcleo garantiza que global y
/// personalizado comparten el MISMO código de iteración.
///
/// - [`Teleport::Uniform`]: 1/n en cada nodo — el PageRank clásico.
/// - [`Teleport::Personalized`]: pesos sobre un subconjunto semilla,
///   normalizados a 1. Pesos negativos o todos-cero son rechazados
///   ([`CentralidadError`]); las semillas deben existir en el store.
#[derive(Debug, Clone, PartialEq)]
pub enum Teleport {
    /// Reinicio uniforme (PageRank global).
    Uniform,
    /// Reinicio ponderado sobre semillas (PageRank personalizado).
    Personalized(Vec<(NodeId, f64)>),
}

impl Teleport {
    /// Teleport personalizado desde semillas ponderadas.
    ///
    /// Los pesos se normalizan a masa 1 en el primer uso (documentado en la
    /// variante). Validación eager de negativos aquí; la existencia de los
    /// nodos se valida contra el store en el cálculo.
    pub fn personalizado(seeds: Vec<(NodeId, f64)>) -> Self {
        Teleport::Personalized(seeds)
    }

    /// Vector denso normalizado sobre la proyección.
    fn densificar(&self, proj: &Proyeccion) -> Result<Vec<f64>, CentralidadError> {
        let n = proj.len();
        match self {
            Teleport::Uniform => Ok(vec![1.0 / n as f64; n]),
            Teleport::Personalized(pesos) => {
                let mut t = vec![0.0; n];
                let mut masa = 0.0;
                for &(node, w) in pesos {
                    if w < 0.0 {
                        return Err(CentralidadError::NegativeTeleportWeight { node, weight: w });
                    }
                    let i = proj
                        .index
                        .get(node)
                        .copied()
                        .flatten()
                        .ok_or(CentralidadError::UnknownNode(node))?;
                    t[i] += w; // semillas repetidas: se suman
                    masa += w;
                }
                if masa <= 0.0 {
                    return Err(CentralidadError::ZeroTeleportMass);
                }
                for s in &mut t {
                    *s /= masa;
                }
                Ok(t)
            }
        }
    }
}

// ─── PageRank ───

/// PageRank global (teleport uniforme) sobre el grafo del store.
///
/// Iteración de potencia de la cadena del surfer aleatorio:
///
/// ```text
///   x_{k+1}[u] = (1-d)·t[u] + d·( Σ_{v→u} x_k[v]/gradOut(v) + D_k/n )
/// ```
///
/// con `t` el teleport (aquí uniforme), `D_k` la masa total en nodos
/// colgantes (grado de salida 0) redistribuida UNIFORMEMENTE — la decisión
/// clásica de Brin y Page (1998): el surfer que llega a una página sin
/// enlaces teletransporta. La alternativa (descartar esa masa y
/// renormalizar al final) cambia el límite pero no el procedimiento; no
/// implementada. Con redistribución la masa total es 1 en CADA iteración.
///
/// * `damping` ∈ (0,1) ABIERTO — default conceptual 0.85 (el valor del
///   paper original, usado en casi toda la literatura): 0 sería puro
///   teleport y 1 es eigenvector puro ([`eigenvector_centrality`]).
/// * Convergencia por **L1** (masa que se mueve) < `tol`; máx.
///   `max_iterations` (100 por convención del ecosistema — pásalo a mano si
///   `converged` llega false: una BD prefiere contestar "no convergió" a
///   devolver números casi-buenos en silencio).
/// * Direccionalidad: aristas SALIENTES. Grafo no dirigido: el CALLER
///   inserta cada arista en ambas direcciones.
/// * Semántica multigrafo: cada arista paralela recibe 1/grado — un
///   duplicado no duplica el voto, pero roba masa a los otros vecinos
///   (test dedicado).
///
/// ```
/// use vol2_liradb::{demo_graph, page_rank};
///
/// let store = demo_graph();
/// let pr = page_rank(&store, 0.85, 200, 1e-9).unwrap();
/// assert!((pr.total_mass() - 1.0).abs() < 1e-9);
/// // El triángulo KNOWS y el self-loop de Dani quedan por encima de las
/// // ciudades (sumideros sin voto propio).
/// assert!(pr.score(0).unwrap() > pr.score(4).unwrap());
/// ```
pub fn page_rank(
    store: &dyn GraphStore,
    damping: f64,
    max_iterations: u64,
    tol: f64,
) -> Result<PageRankResult, CentralidadError> {
    validar_parametros_pagerank(damping, max_iterations, tol)?;
    let proj = Proyeccion::proyectar(store, GraphDirection::Out);
    let t = Teleport::Uniform.densificar(&proj)?;
    Ok(iteracion_de_potencia(
        &proj,
        damping,
        &t,
        max_iterations,
        tol,
    ))
}

/// PageRank personalizado (PPR): el teleport concentra en las semillas.
///
/// Misma iteración que [`page_rank`] — sólo cambia el vector `t`. Las
/// semillas (con pesos ≥ 0, no todos cero, nodos existentes) definen "el
/// centro del mundo": la masa que escapa por el damping vuelve a ELLAS, no
/// a todo el grafo. Éste es el operador de recuperación que el cap 51
/// (GraphRAG) usará sobre el subgrafo de documentos.
///
/// ```
/// use vol2_liradb::{personalized_page_rank, Edge, MemoryStore, Node};
/// use vol2_liradb::GraphStore;
///
/// let mut s = MemoryStore::new();
/// for i in 0..4 { s.put_node(Node::new(i, "P")).unwrap(); }
/// // Dos 2-ciclos desconectados: (0,1) y (2,3).
/// for (a, b) in [(0, 1), (1, 0), (2, 3), (3, 2)] {
///     s.put_edge(Edge::new(a, a, b, "E")).unwrap();
/// }
///
/// // Semilla en el nodo 0: la componente (2,3) queda FUERA del mundo.
/// let ppr = personalized_page_rank(&s, &[(0, 1.0)], 0.85, 200, 1e-9).unwrap();
/// assert!(ppr.score(0).unwrap() > 0.5);
/// assert!(ppr.score(2).unwrap() < 1e-9);
/// assert!((ppr.total_mass() - 1.0).abs() < 1e-9);
/// ```
pub fn personalized_page_rank(
    store: &dyn GraphStore,
    seeds: &[(NodeId, f64)],
    damping: f64,
    max_iterations: u64,
    tol: f64,
) -> Result<PageRankResult, CentralidadError> {
    validar_parametros_pagerank(damping, max_iterations, tol)?;
    let proj = Proyeccion::proyectar(store, GraphDirection::Out);
    let t = Teleport::Personalized(seeds.to_vec()).densificar(&proj)?;
    Ok(iteracion_de_potencia(
        &proj,
        damping,
        &t,
        max_iterations,
        tol,
    ))
}

/// Validación de los parámetros de la familia PageRank.
fn validar_parametros_pagerank(
    damping: f64,
    max_iterations: u64,
    tol: f64,
) -> Result<(), CentralidadError> {
    // (0,1) ABIERTO por ambos extremos: `Range::contains` no sirve (el
    // inicio es inclusivo) — comparación explícita.
    if !damping.is_finite() || damping <= 0.0 || damping >= 1.0 {
        return Err(CentralidadError::InvalidDamping { value: damping });
    }
    validar_parametros_iterativos(max_iterations, tol)
}

/// Validación compartida eigenvector/PageRank.
fn validar_parametros_iterativos(max_iterations: u64, tol: f64) -> Result<(), CentralidadError> {
    if max_iterations < 1 {
        return Err(CentralidadError::InvalidMaxIterations {
            value: max_iterations,
        });
    }
    if !tol.is_finite() || tol <= 0.0 {
        return Err(CentralidadError::InvalidTolerance { value: tol });
    }
    Ok(())
}

/// El núcleo: iteración de potencia de la cadena del surfer con damping,
/// teleport arbitrario y redistribución uniforme de la masa colgante.
///
/// Precondiciones (validadas por las funciones públicas): damping ∈ (0,1),
/// tol > 0, max_iterations ≥ 1, `t` denso de masa 1 sobre la proyección.
fn iteracion_de_potencia(
    proj: &Proyeccion,
    damping: f64,
    t: &[f64],
    max_iterations: u64,
    tol: f64,
) -> PageRankResult {
    let n = proj.len();
    let mut stats = proj.stats;

    if n == 0 {
        return PageRankResult {
            nodes: proj.nodes.clone(),
            scores: Vec::new(),
            damping,
            converged: true,
            delta: 0.0,
            history: Vec::new(),
            stats,
        };
    }

    // Arranque: el propio teleport (uniforme en el global; las semillas en
    // el personalizado — arrancar "en el mundo" ahorra las primeras
    // iteraciones de mezclado).
    let mut x = t.to_vec();
    let mut converged = false;
    let mut delta = 0.0;
    let mut history = Vec::new();

    for it in 1..=max_iterations {
        stats.iterations = it;

        // Masa colgante: grado de salida 0 → teletransporte uniforme.
        let masa_colgante: f64 = proj
            .vecinos
            .iter()
            .zip(&x)
            .filter(|(vecinos_v, _)| vecinos_v.is_empty())
            .map(|(_, xv)| *xv)
            .sum();

        // Semilla del vector nuevo: teleport + cuota uniforme de colgantes.
        let colgante_por_nodo = damping * masa_colgante / n as f64;
        let mut y: Vec<f64> = t
            .iter()
            .map(|tu| (1.0 - damping) * tu + colgante_por_nodo)
            .collect();

        // Votos siguiendo enlaces salientes: cada arista vale 1/grado.
        for (v, vecinos_v) in proj.vecinos.iter().enumerate() {
            let grado = vecinos_v.len();
            if grado == 0 {
                continue;
            }
            let cuota = damping * x[v] / grado as f64;
            for &w in vecinos_v {
                y[w] += cuota;
                stats.edges_scanned += 1;
            }
        }

        delta = y.iter().zip(&x).map(|(a, b)| (a - b).abs()).sum();
        x = y;
        history.push(delta);
        if delta < tol {
            converged = true;
            break;
        }
    }

    PageRankResult {
        nodes: proj.nodes.clone(),
        scores: x,
        damping,
        converged,
        delta,
        history,
        stats,
    }
}

// ══════════════════════════════════════════════════════════════════════
//  Tests
// ══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests_centralidad {
    use super::*;
    use crate::cap07_modelo::{Edge, Node};
    use crate::cap08_graph_store::MemoryStore;

    /// Tolerancia genérica para comparar f64 contra soluciones a mano.
    const EPS: f64 = 1e-6;

    /// ¿Está `a` a menos de `eps` de `b`?
    fn cerca(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    /// Store con nodos 0..n y las aristas dirigidas dadas.
    fn dirigido(n: usize, aristas: &[(usize, usize)]) -> MemoryStore {
        let mut s = MemoryStore::new();
        for i in 0..n {
            s.put_node(Node::new(i, "N")).unwrap();
        }
        for (k, &(a, b)) in aristas.iter().enumerate() {
            s.put_edge(Edge::new(k, a, b, "E")).unwrap();
        }
        s
    }

    /// Store con nodos 0..n y las aristas NO dirigidas dadas (ambas
    /// direcciones — la forma en que el caller simetriza para PageRank).
    fn simetrizado(n: usize, aristas: &[(usize, usize)]) -> MemoryStore {
        let mut s = MemoryStore::new();
        for i in 0..n {
            s.put_node(Node::new(i, "N")).unwrap();
        }
        let mut k = 0;
        for &(a, b) in aristas {
            s.put_edge(Edge::new(k, a, b, "E")).unwrap();
            s.put_edge(Edge::new(k + 1, b, a, "E")).unwrap();
            k += 2;
        }
        s
    }

    // ─── Grado ───

    #[test]
    fn grado_por_direccion_y_normalizado() {
        // 0 → 1, 0 → 2, 2 → 0.
        let s = dirigido(3, &[(0, 1), (0, 2), (2, 0)]);

        let out = degree_centrality(&s, GraphDirection::Out).unwrap();
        assert_eq!(out.score(0), Some(1.0)); // 2 / (3-1)
        assert_eq!(out.score(1), Some(0.0));
        assert_eq!(out.score(2), Some(0.5));

        let inn = degree_centrality(&s, GraphDirection::In).unwrap();
        assert_eq!(inn.score(0), Some(0.5)); // apuntado por 2
        assert_eq!(inn.score(1), Some(0.5));
        assert_eq!(inn.score(2), Some(0.5));

        let both = degree_centrality(&s, GraphDirection::Both).unwrap();
        // 0: {1,2}, 1: {0}, 2: {0} — todos con grado distinto.
        assert_eq!(both.score(0), Some(1.0));
        assert_eq!(both.score(1), Some(0.5));
        assert_eq!(both.score(2), Some(0.5));

        // Stats: la proyección recorrió las aristas una vez por dirección.
        assert_eq!(out.stats.edges_scanned, 3);
        assert_eq!(both.stats.edges_scanned, 6); // out + in
    }

    #[test]
    fn grado_vacio_un_nodo_self_loop_y_multigrafo() {
        let vacio = MemoryStore::new();
        let deg = degree_centrality(&vacio, GraphDirection::Both).unwrap();
        assert!(deg.is_empty());
        assert_eq!(deg.score(0), None);

        // Un solo nodo: n-1 = 0 → score 0 por definición (sin "resto").
        let uno = dirigido(1, &[]);
        let deg = degree_centrality(&uno, GraphDirection::Out).unwrap();
        assert_eq!(deg.score(0), Some(0.0));

        // Self-loop: cuenta en Out y en In; en Both una sola vez.
        let mut s = MemoryStore::new();
        s.put_node(Node::new(0, "N")).unwrap();
        s.put_edge(Edge::new(0, 0, 0, "E")).unwrap();
        s.put_node(Node::new(1, "N")).unwrap();
        assert_eq!(
            degree_centrality(&s, GraphDirection::Out).unwrap().score(0),
            Some(1.0)
        );
        assert_eq!(
            degree_centrality(&s, GraphDirection::Both)
                .unwrap()
                .score(0),
            Some(1.0)
        );

        // Multigrafo: dos aristas paralelas cuentan dos.
        let s = dirigido(2, &[(0, 1), (0, 1)]);
        assert_eq!(
            degree_centrality(&s, GraphDirection::Out).unwrap().score(0),
            Some(2.0)
        );
    }

    // ─── Closeness ───

    #[test]
    fn closeness_camino_lineal_valores_de_libro() {
        // 0 - 1 - 2 - 3 simetrizado: C = (n-1)/Σd.
        let s = simetrizado(4, &[(0, 1), (1, 2), (2, 3)]);
        let c = closeness_centrality(&s, GraphDirection::Both).unwrap();
        assert!(cerca(c.score(0).unwrap(), 3.0 / 6.0, EPS)); // Σd = 1+2+3
        assert!(cerca(c.score(1).unwrap(), 3.0 / 4.0, EPS)); // Σd = 1+1+2
        assert!(cerca(c.score(2).unwrap(), 3.0 / 4.0, EPS));
        assert!(cerca(c.score(3).unwrap(), 3.0 / 6.0, EPS));
        // Un BFS por nodo. Coste medido: 12 lecturas de arista en la
        // proyección (6 out + 6 in) + 24 vecindarios pisados en los BFS
        // (6 entradas deduplicadas por BFS × 4 orígenes).
        assert_eq!(c.stats.bfs_runs, 4);
        assert_eq!(c.stats.edges_scanned, 12 + 4 * 6);
    }

    #[test]
    fn closeness_componentes_desconectadas_wasserman_faust() {
        // Dos 2-ciclos: (0,1) y (2,3). Cada nodo alcanza a 1 de los 3
        // posibles: C = (1/3)·(1/1) = 1/3 — penalizado, no inflado.
        let s = simetrizado(4, &[(0, 1), (2, 3)]);
        let c = closeness_centrality(&s, GraphDirection::Both).unwrap();
        for i in 0..4 {
            assert!(cerca(c.score(i).unwrap(), 1.0 / 3.0, EPS));
        }
    }

    #[test]
    fn closeness_dirigido_out_vs_in() {
        // Cadena 0 → 1 → 2: por Out sólo 0 alcanza algo.
        let s = dirigido(3, &[(0, 1), (1, 2)]);

        let out = closeness_centrality(&s, GraphDirection::Out).unwrap();
        // 0: alcanza {1,2}, Σd = 3, r = 3 → (2/2)·(2/3) = 2/3.
        assert!(cerca(out.score(0).unwrap(), 2.0 / 3.0, EPS));
        // 1: alcanza {2}, Σd = 1, r = 2 → (1/2)·(1/1) = 0.5.
        assert!(cerca(out.score(1).unwrap(), 0.5, EPS));
        // 2: no alcanza a nadie (Σd = 0) → 0.
        assert_eq!(out.score(2), Some(0.0));

        // Por In se invierte la historia: 2 es el "alcanzable por todos".
        let inn = closeness_centrality(&s, GraphDirection::In).unwrap();
        assert_eq!(inn.score(0), Some(0.0));
        assert!(cerca(inn.score(1).unwrap(), 0.5, EPS));
        assert!(cerca(inn.score(2).unwrap(), 2.0 / 3.0, EPS));
    }

    // ─── Betweenness ───

    #[test]
    fn betweenness_camino_lineal_valores_de_libro() {
        // 0 - 1 - 2 - 3 simetrizado. Crudo (dirigido sobre simetrizado):
        // cada camino aparece en ambos sentidos → 1 y 2 acumulan 4;
        // normalizado ×1/((n-1)(n-2)) = 1/6 → 2/3, el valor de libro.
        let s = simetrizado(4, &[(0, 1), (1, 2), (2, 3)]);

        let crudo = betweenness_centrality(&s, GraphDirection::Both, false).unwrap();
        assert_eq!(crudo.score(0), Some(0.0));
        assert_eq!(crudo.score(1), Some(4.0));
        assert_eq!(crudo.score(2), Some(4.0));
        assert_eq!(crudo.score(3), Some(0.0));

        let norm = betweenness_centrality(&s, GraphDirection::Both, true).unwrap();
        assert!(cerca(norm.score(1).unwrap(), 2.0 / 3.0, EPS));
        assert!(cerca(norm.score(2).unwrap(), 2.0 / 3.0, EPS));
        assert_eq!(norm.score(0), Some(0.0));

        // Brandes: V BFS; aristas pisadas por BFS que las alcanza.
        assert_eq!(crudo.stats.bfs_runs, 4);
        assert!(crudo.stats.edges_scanned > 0);
    }

    #[test]
    fn betweenness_estrella_el_centro_lo_es_todo() {
        // Centro 0, hojas 1..3 (simetrizado). Todos los pares de hojas
        // pasan por 0: crudo = 2·C(3,2)·1 = 6 (ambos sentidos),
        // normalizado 6/6 = 1 — el máximo de la escala.
        let s = simetrizado(4, &[(0, 1), (0, 2), (0, 3)]);
        let bc = betweenness_centrality(&s, GraphDirection::Both, true).unwrap();
        assert!(cerca(bc.score(0).unwrap(), 1.0, EPS));
        for hoja in 1..4 {
            assert_eq!(bc.score(hoja), Some(0.0));
        }
    }

    #[test]
    fn betweenness_dirigido_y_caminos_multiples() {
        // Cadena dirigida 0 → 1 → 2: sólo el par (0,2) tiene camino, por 1.
        let s = dirigido(3, &[(0, 1), (1, 2)]);
        let crudo = betweenness_centrality(&s, GraphDirection::Out, false).unwrap();
        assert_eq!(crudo.score(1), Some(1.0));
        assert_eq!(crudo.score(0), Some(0.0));
        let norm = betweenness_centrality(&s, GraphDirection::Out, true).unwrap();
        assert!(cerca(norm.score(1).unwrap(), 0.5, EPS)); // 1/((3-1)(3-2))

        // Diamante 0→{1,2}→{3}: dos caminos mínimos 0→3, la mitad por
        // cada intermedio — σ cuenta, Brandes reparte.
        let s = dirigido(4, &[(0, 1), (0, 2), (1, 3), (2, 3)]);
        let bc = betweenness_centrality(&s, GraphDirection::Out, false).unwrap();
        assert!(cerca(bc.score(1).unwrap(), 0.5, EPS));
        assert!(cerca(bc.score(2).unwrap(), 0.5, EPS));
        assert_eq!(bc.score(0), Some(0.0));
        assert_eq!(bc.score(3), Some(0.0));
    }

    #[test]
    fn betweenness_paralelas_y_pequenos() {
        // Aristas paralelas = caminos distintos: σ(0→2) = 2, ambas por 1,
        // pero la fracción σ_st(u)/σ_st sigue siendo 1 (todas pasan por 1).
        let s = dirigido(3, &[(0, 1), (0, 1), (1, 2)]);
        let bc = betweenness_centrality(&s, GraphDirection::Out, false).unwrap();
        assert_eq!(bc.score(1), Some(1.0));

        // n = 2: sin pares s≠u≠t, normalizado a cero sin pánico.
        let s = dirigido(2, &[(0, 1)]);
        let bc = betweenness_centrality(&s, GraphDirection::Out, true).unwrap();
        assert_eq!(bc.score(0), Some(0.0));
        assert_eq!(bc.score(1), Some(0.0));
    }

    // ─── Eigenvector ───

    #[test]
    fn eigenvector_estrella_las_hojas_a_cero() {
        // Hojas → centro: sin correctivo, todo el prestigio al centro y
        // las hojas mueren a 0 (la limitación clásica que PageRank arregla).
        let s = dirigido(4, &[(0, 3), (1, 3), (2, 3)]);
        let ev = eigenvector_centrality(&s, 200, 1e-12).unwrap();
        assert!(ev.converged);
        assert!(ev.score(3).unwrap() > 0.999999);
        for hoja in 0..3 {
            assert!(ev.score(hoja).unwrap() < 1e-6);
        }
        // Normalizado en L2.
        let l2: f64 = ev.ranking().iter().map(|(_, s)| s * s).sum::<f64>().sqrt();
        assert!(cerca(l2, 1.0, EPS));
    }

    #[test]
    fn eigenvector_no_converge_en_periodico_y_pagerank_si() {
        // Cola + 3-ciclo: T→A, A→B, B→C, C→A. Tras morir la cola, la masa
        // rota en el ciclo sin asentarse (periodicidad): la iteración de
        // potencia OSCILA. PageRank con damping converge en el MISMO grafo
        // — el porqué del damping, demostrado.
        let s = dirigido(4, &[(0, 1), (1, 2), (2, 3), (3, 1)]);

        let ev = eigenvector_centrality(&s, 100, 1e-9).unwrap();
        assert!(!ev.converged);
        assert_eq!(ev.stats.iterations, 100);
        assert!(ev.delta > 1e-9);

        let pr = page_rank(&s, 0.85, 200, 1e-9).unwrap();
        assert!(pr.converged);
        assert!((pr.total_mass() - 1.0).abs() < EPS);
    }

    #[test]
    fn eigenvector_sin_aristas_y_parametros_invalidos() {
        // Sin aristas: cualquier vector es autovector (autovalor 0) —
        // uniforme, convergido, sin iterar.
        let s = dirigido(3, &[]);
        let ev = eigenvector_centrality(&s, 10, 1e-6).unwrap();
        assert!(ev.converged);
        assert_eq!(ev.stats.iterations, 0);
        for (_, v) in ev.ranking() {
            assert!(cerca(v, 1.0, EPS));
        }

        assert_eq!(
            eigenvector_centrality(&s, 0, 1e-6),
            Err(CentralidadError::InvalidMaxIterations { value: 0 })
        );
        assert_eq!(
            eigenvector_centrality(&s, 10, 0.0),
            Err(CentralidadError::InvalidTolerance { value: 0.0 })
        );
    }

    // ─── PageRank: casos analíticos ───

    #[test]
    fn pagerank_ciclos_compartidos_solucion_a_mano() {
        // A↔B y A↔C (A reparte entre B y C, ambos le devuelven todo).
        // Por simetría x_B = x_C = y:
        //   x = (1-d)/3 + d·2y ;  y = (1-d)/3 + d·x/2 ;  x + 2y = 1
        // Con d = 0.85: y = (0.05 + 0.425)/1.85, x = 1 - 2y.
        let s = dirigido(3, &[(0, 1), (1, 0), (0, 2), (2, 0)]);
        let pr = page_rank(&s, 0.85, 500, 1e-12).unwrap();

        assert!(pr.converged);
        let y = (0.05_f64 + 0.425) / 1.85;
        assert!(cerca(pr.score(0).unwrap(), 1.0 - 2.0 * y, EPS));
        assert!(cerca(pr.score(1).unwrap(), y, EPS));
        assert!(cerca(pr.score(2).unwrap(), y, EPS));
        assert!((pr.total_mass() - 1.0).abs() < EPS);
    }

    #[test]
    fn pagerank_cadena_con_dangling_solucion_a_mano() {
        // 0 → 1 → 2 (2 colgante). Sistema lineal (d = 0.85, t = 1/3):
        //   a = 0.05 + 0.85·c/3 ; b = 0.05 + 0.85·(a + c/3) ;
        //   c = 0.05 + 0.85·(b + c/3)
        // Solución (verificada por sustitución): la masa fluye río abajo
        // y la colgante vuelve repartida — el ranking invierte al grado.
        let s = dirigido(3, &[(0, 1), (1, 2)]);
        let pr = page_rank(&s, 0.85, 500, 1e-12).unwrap();

        assert!(pr.converged);
        assert!(cerca(pr.score(0).unwrap(), 0.18441678192715533, EPS));
        assert!(cerca(pr.score(1).unwrap(), 0.34117104656523745, EPS));
        assert!(cerca(pr.score(2).unwrap(), 0.47441217150760706, EPS));
        assert!((pr.total_mass() - 1.0).abs() < EPS);
        // El colgante NO absorbe la masa: la redistribución la devuelve.
        assert!(pr.score(2).unwrap() < 0.5);
    }

    #[test]
    fn pagerank_suma_uno_y_convergencia_geometrica() {
        // La cadena con colgante (0→1→2): el arranque (teleport uniforme)
        // NO es el estacionario, así que el historial de deltas muestra la
        // convergencia GEOMÉTRICA de verdad: cada iteración multiplica el
        // error por ~d·λ₂ < 1 hasta caer bajo la tolerancia.
        let s = dirigido(3, &[(0, 1), (1, 2)]);
        let pr = page_rank(&s, 0.85, 500, 1e-10).unwrap();

        assert!(pr.converged);
        assert!((pr.total_mass() - 1.0).abs() < EPS);

        let h = &pr.history;
        assert!(h.len() >= 5, "historial corto: {h:?}");
        // Monótona desde la segunda iteración (la primera mezcla).
        for k in 1..h.len() {
            assert!(h[k] <= h[k - 1] + 1e-12, "delta creció en {k}: {h:?}");
        }
        // La razón entre consecutivos del final: contracción < 1 (aquí se
        // asienta cerca de d·λ₂; oscila por el colgante, nunca supera 1).
        let razon = h[h.len() - 1] / h[h.len() - 2];
        assert!(
            razon < 0.99 && razon > 0.01,
            "razón de convergencia {razon}"
        );
        // El delta final quedó por debajo de la tolerancia pedida.
        assert!(pr.delta < 1e-10);

        // Contraste: en los dos 2-ciclos simétricos el teleport uniforme
        // YA es el estacionario — converge en una iteración con delta 0
        // (el historial también cuenta esa historia).
        let s = simetrizado(4, &[(0, 1), (2, 3)]);
        let pr = page_rank(&s, 0.85, 500, 1e-10).unwrap();
        assert!(pr.converged);
        assert_eq!(pr.history.len(), 1);
        assert_eq!(pr.history[0], 0.0);
        for i in 0..4 {
            assert!(cerca(pr.score(i).unwrap(), 0.25, EPS));
        }
    }

    #[test]
    fn pagerank_componentes_desconectadas_masa_por_componente() {
        // Con teleport uniforme cada componente recibe masa ∝ su tamaño:
        // aquí 2 nodos de 4 → 0.5 por componente (el teleport es lo que
        // hace la cadena irreducible; sin él las componentes no se hablan).
        let s = simetrizado(4, &[(0, 1), (2, 3)]);
        let pr = page_rank(&s, 0.85, 500, 1e-10).unwrap();

        let masa_c1: f64 = [0, 1].iter().map(|&i| pr.score(i).unwrap()).sum();
        let masa_c2: f64 = [2, 3].iter().map(|&i| pr.score(i).unwrap()).sum();
        assert!(cerca(masa_c1, 0.5, EPS));
        assert!(cerca(masa_c2, 0.5, EPS));

        // Un 2-ciclo grande y uno pequeño: masa 3/5 vs 2/5.
        let s = simetrizado(5, &[(0, 1), (1, 2), (2, 0), (3, 4)]);
        let pr = page_rank(&s, 0.85, 500, 1e-10).unwrap();
        let masa_c1: f64 = [0, 1, 2].iter().map(|&i| pr.score(i).unwrap()).sum();
        assert!(cerca(masa_c1, 0.6, EPS));
    }

    #[test]
    fn pagerank_damping_extremos() {
        // d → 0: el teleport lo domina todo; los scores ≈ uniformes.
        let s = dirigido(3, &[(0, 1), (1, 2)]);
        let pr = page_rank(&s, 0.01, 500, 1e-10).unwrap();
        for i in 0..3 {
            assert!(cerca(pr.score(i).unwrap(), 1.0 / 3.0, 0.01));
        }

        // d → 1: convergencia más lenta — MÁS iteraciones para la misma
        // tolerancia (la razón de contracción se acerca a 1).
        let rapido = page_rank(&s, 0.5, 500, 1e-9).unwrap();
        let lento = page_rank(&s, 0.99, 500, 1e-9).unwrap();
        assert!(lento.stats.iterations > rapido.stats.iterations);
        assert!(rapido.converged && lento.converged);

        // d fuera de (0,1): rechazado ruidosamente, incluidos los bordes.
        // (NaN se comprueba con matches! porque NaN != NaN bajo PartialEq.)
        for d in [0.0, 1.0, -0.5, 1.5] {
            assert_eq!(
                page_rank(&s, d, 100, 1e-6),
                Err(CentralidadError::InvalidDamping { value: d })
            );
        }
        assert!(matches!(
            page_rank(&s, f64::NAN, 100, 1e-6),
            Err(CentralidadError::InvalidDamping { .. })
        ));
    }

    #[test]
    fn pagerank_parametros_invalidos_y_vacio_un_nodo() {
        let s = dirigido(2, &[(0, 1)]);

        assert_eq!(
            page_rank(&s, 0.85, 0, 1e-6),
            Err(CentralidadError::InvalidMaxIterations { value: 0 })
        );
        assert_eq!(
            page_rank(&s, 0.85, 100, 0.0),
            Err(CentralidadError::InvalidTolerance { value: 0.0 })
        );
        assert_eq!(
            page_rank(&s, 0.85, 100, f64::NEG_INFINITY),
            Err(CentralidadError::InvalidTolerance {
                value: f64::NEG_INFINITY
            })
        );

        // Grafo vacío: nada que rankear — resultado vacío y convergido.
        let vacio = MemoryStore::new();
        let pr = page_rank(&vacio, 0.85, 100, 1e-6).unwrap();
        assert!(pr.is_empty());
        assert!(pr.converged);
        assert_eq!(pr.total_mass(), 0.0);

        // Un nodo (colgante): retiene su propia masa redistribuida → 1.
        let uno = dirigido(1, &[]);
        let pr = page_rank(&uno, 0.85, 100, 1e-9).unwrap();
        assert!(cerca(pr.score(0).unwrap(), 1.0, EPS));

        // Un nodo con self-loop: idéntico (el voto vuelve a sí mismo).
        let mut s = MemoryStore::new();
        s.put_node(Node::new(0, "N")).unwrap();
        s.put_edge(Edge::new(0, 0, 0, "E")).unwrap();
        let pr = page_rank(&s, 0.85, 100, 1e-9).unwrap();
        assert!(cerca(pr.score(0).unwrap(), 1.0, EPS));
    }

    #[test]
    fn pagerank_multigrafo_el_duplicado_roba_masa() {
        // Con normalización por grado de salida, duplicar 0→2 NO duplica
        // el voto de 2 (también duplica el denominador)... pero sí le
        // ROBA la mitad de la cuota a 1: 2 sube, 1 baja.
        let simple = dirigido(3, &[(0, 1), (0, 2)]);
        let duplicado = dirigido(3, &[(0, 1), (0, 2), (0, 2)]);

        let a = page_rank(&simple, 0.85, 500, 1e-10).unwrap();
        let b = page_rank(&duplicado, 0.85, 500, 1e-10).unwrap();

        assert!(b.score(2).unwrap() > a.score(2).unwrap());
        assert!(b.score(1).unwrap() < a.score(1).unwrap());
        assert!((a.total_mass() - 1.0).abs() < EPS);
        assert!((b.total_mass() - 1.0).abs() < EPS);
    }

    #[test]
    fn pagerank_huecos_tras_delete_node() {
        // Borrar el nodo central de una cadena deja un hueco: los ids
        // restantes siguen puntuando (compactados internamente) y el id
        // borrado responde None — no existe, no puntúa.
        let mut s = dirigido(4, &[(0, 1), (1, 2), (2, 3)]);
        assert!(s.delete_node(1));
        let pr = page_rank(&s, 0.85, 500, 1e-10).unwrap();

        assert_eq!(pr.score(1), None);
        assert_eq!(pr.len(), 3);
        assert!((pr.total_mass() - 1.0).abs() < EPS);
        // 0 es colgante ahora; 3 también: todos reciben teleport + cuota.
        assert!(pr.score(0).is_some() && pr.score(3).is_some());
    }

    // ─── PageRank personalizado ───

    #[test]
    fn ppr_concentra_en_semillas_solucion_a_mano() {
        // Dos 2-ciclos; semilla en el nodo 0. La componente (2,3) queda
        // fuera del mundo: masa exactamente 0. En la de la semilla:
        //   a = 0.15 + 0.85·b ; b = 0.85·a  →  a = 0.15/(1-0.85²).
        let s = simetrizado(4, &[(0, 1), (2, 3)]);
        let ppr = personalized_page_rank(&s, &[(0, 1.0)], 0.85, 500, 1e-12).unwrap();

        assert!(cerca(
            ppr.score(0).unwrap(),
            0.15 / (1.0 - 0.85 * 0.85),
            EPS
        ));
        assert!(cerca(
            ppr.score(1).unwrap(),
            0.85 * 0.15 / (1.0 - 0.85 * 0.85),
            EPS
        ));
        assert_eq!(ppr.score(2), Some(0.0));
        assert_eq!(ppr.score(3), Some(0.0));
        assert!((ppr.total_mass() - 1.0).abs() < EPS);

        // vs el GLOBAL: mismas aristas, mundos distintos — el ranking de
        // la componente lejana se hunde SOLO en el personalizado.
        let global = page_rank(&s, 0.85, 500, 1e-12).unwrap();
        assert!(cerca(global.score(2).unwrap(), 0.25, EPS));
        assert!(global.score(2).unwrap() > ppr.score(2).unwrap());
    }

    #[test]
    fn ppr_pesos_relativos_y_multiples_semillas() {
        // Semillas ponderadas 3:1 en un 2-ciclo (0,1) + lejana (2,3):
        // la masa inicial se parte 3/4 y 1/4 entre los mundos.
        let s = simetrizado(4, &[(0, 1), (2, 3)]);
        let ppr = personalized_page_rank(&s, &[(0, 3.0), (2, 1.0)], 0.85, 500, 1e-10).unwrap();

        let cerca_de_0: f64 = [0, 1].iter().map(|&i| ppr.score(i).unwrap()).sum();
        let cerca_de_2: f64 = [2, 3].iter().map(|&i| ppr.score(i).unwrap()).sum();
        assert!(cerca(cerca_de_0 / cerca_de_2, 3.0, 1e-3));
        assert!((ppr.total_mass() - 1.0).abs() < EPS);
    }

    #[test]
    fn ppr_errores_de_teleport() {
        let s = simetrizado(4, &[(0, 1), (2, 3)]);

        // Peso negativo: rechazado con el nodo señalado.
        assert_eq!(
            personalized_page_rank(&s, &[(0, 1.0), (2, -0.5)], 0.85, 100, 1e-6),
            Err(CentralidadError::NegativeTeleportWeight {
                node: 2,
                weight: -0.5
            })
        );

        // Todos los pesos a cero: nada que normalizar.
        assert_eq!(
            personalized_page_rank(&s, &[(0, 0.0)], 0.85, 100, 1e-6),
            Err(CentralidadError::ZeroTeleportMass)
        );

        // Semilla que no existe en el store.
        assert_eq!(
            personalized_page_rank(&s, &[(99, 1.0)], 0.85, 100, 1e-6),
            Err(CentralidadError::UnknownNode(99))
        );
    }

    // ─── Integración con demo_graph ───

    #[test]
    fn demo_graph_ranking_plausible() {
        // KNOWS: 0→1→2→0 y self-loop 3→3; LIVES_IN: 0→4, 1→5. Sorpresa
        // pedagógica: Dani (3) ARRASA (≈0.386) — su self-loop le devuelve
        // cada voto y encima cobra la cuota uniforme de la masa colgante
        // de las ciudades; sin salida neta, es una trampa de acumulación.
        // El triángulo reparte hacia las ciudades, que empatan con quien
        // las alimenta (1↔4, 2↔5 por la simetría rotacional 0→1→2).
        let s = crate::cap20_volcano::demo_graph();
        let pr = page_rank(&s, 0.85, 500, 1e-10).unwrap();

        assert!(pr.converged);
        assert!((pr.total_mass() - 1.0).abs() < EPS);
        assert!(cerca(pr.score(3).unwrap(), 0.3855087315, EPS));
        assert!(cerca(pr.score(0).unwrap(), 0.1510610136, EPS));
        assert!(cerca(pr.score(4).unwrap(), 0.1220272405, EPS));
        // Empates estructurales: la ciudad 4 con quien la apunta desde el
        // triángulo (1), y la 5 con el 2.
        assert!(cerca(pr.score(1).unwrap(), pr.score(4).unwrap(), EPS));
        assert!(cerca(pr.score(2).unwrap(), pr.score(5).unwrap(), EPS));

        // Ranking: Dani primero, triángulo y ciudades detrás.
        let ranking = pr.ranking();
        assert_eq!(ranking[0].0, 3);
        assert!(pr.score(0).unwrap() > pr.score(4).unwrap());
        assert!(pr.score(0).unwrap() > pr.score(5).unwrap());

        // Degree Out: 0 y 1 (KNOWS + LIVES_IN) encabezan — otra métrica,
        // otra historia (aquí 3 es un nodo casi marginal).
        let deg = degree_centrality(&s, GraphDirection::Out).unwrap();
        assert_eq!(deg.score(0), Some(2.0 / 5.0));
        assert_eq!(deg.score(3), Some(1.0 / 5.0)); // self-loop
        assert_eq!(deg.score(4), Some(0.0));
    }

    #[test]
    fn ppr_vs_global_en_demo_graph() {
        // Personalizar en Madrid (4): la ciudad sube (el teleport vuelve a
        // ella) y el grafo se RE-CENTRA — quien no está en la órbita de la
        // semilla pierde teleport sin ganar nada equivalente: incluso el
        // nodo 0 (que la apunta) BAJA, y la trampa de acumulación de Dani
        // se desinfla al dejar de recibir teleport uniforme.
        let s = crate::cap20_volcano::demo_graph();
        let global = page_rank(&s, 0.85, 500, 1e-10).unwrap();
        let ppr = personalized_page_rank(&s, &[(4, 1.0)], 0.85, 500, 1e-10).unwrap();

        assert!(ppr.score(4).unwrap() > global.score(4).unwrap());
        assert!(ppr.score(3).unwrap() < global.score(3).unwrap());
        assert!(ppr.score(0).unwrap() < global.score(0).unwrap());
        assert!((ppr.total_mass() - 1.0).abs() < EPS);
        assert!(ppr.converged);
    }

    // ─── Errores y utilidades ───

    #[test]
    fn errores_display_y_std_error() {
        let e = CentralidadError::InvalidDamping { value: 1.5 };
        assert!(e.to_string().contains("1.5"));
        assert!(e.to_string().contains("(0,1)"));
        let _: &dyn std::error::Error = &e;

        let e = CentralidadError::InvalidTolerance { value: -1.0 };
        assert!(e.to_string().contains("-1"));
        let e = CentralidadError::InvalidMaxIterations { value: 0 };
        assert!(e.to_string().contains("0"));
        let e = CentralidadError::NegativeTeleportWeight {
            node: 7,
            weight: -2.0,
        };
        assert!(e.to_string().contains("7"));
        let e = CentralidadError::ZeroTeleportMass;
        assert!(!e.to_string().is_empty());
        let e = CentralidadError::UnknownNode(42);
        assert!(e.to_string().contains("42"));
    }

    #[test]
    fn resultados_accessores_y_display() {
        let s = simetrizado(4, &[(0, 1), (2, 3)]);
        let pr = page_rank(&s, 0.85, 500, 1e-10).unwrap();

        // Ranking ordenado por score desc, desempate id asc.
        let r = pr.ranking();
        assert_eq!(r.len(), 4);
        for w in r.windows(2) {
            assert!(w[0].1 >= w[1].1);
        }
        // Todas 0.25: desempate por id.
        assert_eq!(r[0], (0, r[0].1));
        assert_eq!(r[1].0, 1);

        // entries() en orden de id; Display tipo tabla.
        let e = pr.entries();
        assert_eq!(e[0].0, 0);
        let disp = format!("{}", pr);
        assert!(disp.contains("PageRank(d=0.85"));
        assert!(disp.contains("convergido=sí"));

        let deg = degree_centrality(&s, GraphDirection::Both).unwrap();
        assert!(format!("{}", deg).starts_with("n0="));
        assert_eq!(deg.ranking().len(), 4);
        assert!(deg.score(99).is_none());

        // Teleport::personalizado expone la variante.
        let t = Teleport::personalizado(vec![(0, 2.0)]);
        assert_eq!(t, Teleport::Personalized(vec![(0, 2.0)]));
    }

    #[test]
    fn proyeccion_in_transpone_la_adyacencia() {
        // 0→1: para In, el vecindario de 1 es {0} y el de 0 queda vacío.
        let s = dirigido(2, &[(0, 1)]);
        let deg_out = degree_centrality(&s, GraphDirection::Out).unwrap();
        let deg_in = degree_centrality(&s, GraphDirection::In).unwrap();
        assert_eq!(deg_out.score(0), Some(1.0));
        assert_eq!(deg_out.score(1), Some(0.0));
        assert_eq!(deg_in.score(0), Some(0.0));
        assert_eq!(deg_in.score(1), Some(1.0));
    }
}
