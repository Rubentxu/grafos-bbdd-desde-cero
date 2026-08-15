use std::cell::Cell;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::collections::HashSet;
use std::fmt;
use std::ops::Range;

use crate::cap07_modelo::{Edge, EdgeId, Node, NodeId};
use crate::cap08_graph_store::{GraphStore, StoreError};
use crate::cap22_caminos_minimos::{
    Cost, Path, PathError, PathStats, PathStep, WeightSource, edge_weight,
};
use crate::cap24_centralidad::GraphDirection;

// ─────────────────── Cap 26: Ejecutar algoritmos sin agotar la memoria ───────────────────
//
// CIERRA la Parte V (algoritmos sobre el grafo persistente). CORPUS
// vol-II-cap-26: "Proyección, streaming, frontiers". Guion (brief §cap 26):
// vista proyectada del grafo, streaming, procesamiento por bloques, frontiers,
// bitsets, paralelismo, snapshots, diferencia entre OLTP y analítica —
// "conecta algoritmos académicos con restricciones reales de una base de
// datos".
//
// EL PORQUÉ del capítulo (la historia que cierra la Parte V): los caps 22-23
// ejecutaron Dijkstra/A*/Bellman-Ford leyendo el store arista a arista
// (`out_edges` + `get_edge` por relajación) — perfecto para UNA consulta, y
// O(iteraciones) re-lecturas para los algoritmos iterativos; los caps 24-25
// lo resolvieron con proyecciones PRIVADAS a medida (la `Proyeccion` no
// ponderada del 24, el `GrafoPonderado` simétrico del 25), cada una con la
// semántica que su familia de algoritmos necesitaba. Este capítulo hace la
// proyección CIUDADANA DE PRIMERA: pública, con pesos, filtrable — y le añade
// la otra mitad del título: ejecutar SIN materializar cuando el grafo no
// cabe (streaming por fronteras con presupuesto).
//
// Las dos estrategias del capítulo (el trade-off central, medible en tests):
//
//   1. **PROYECCIÓN MATERIALIZADA** ([`ProyeccionPonderada`]): el grafo (o un
//      subgrafo filtrado por etiqueta de nodo / tipo de arista) en memoria
//      compacta, con pesos leídos una sola vez con la semántica ESTRICTA del
//      cap 22 ([`WeightSource`]/[`edge_weight`]). El layout es CSR
//      (offsets + targets + pesos + ids de arista) — exactamente lo que el
//      CSR persistente del cap 14 NO podía guardar: allí sólo vive topología
//      (offsets + targets, sin ids de arista), y por eso los algoritmos del
//      cap 22 no podían usarlo para pesar. La proyección ES el CSR del 14
//      completado: mismo layout, más el peso y el id de cada arista. Pagar
//      UNA lectura completa (E `get_edge`) para después iterar K veces sin
//      tocar el store: el cap 13 (BufferPool) puso el precio de cada lectura
//      que falla la caché — este capítulo compra las lecturas por adelantado
//      cuando vamos a necesitar muchas. Es también un **SNAPSHOT**: la
//      proyección es inmutable una vez construida (una foto del store en un
//      instante); el store puede seguir mutando (OLTP) mientras el análisis
//      corre sobre su copia coherente (analítica) — la separación
//      OLTP/analítica del guion, encarnada en un tipo.
//
//   2. **STREAMING POR FRONTERAS** ([`bfs_streaming`]/[`FronterasBfs`]): el
//      extremo opuesto. Cuando la consulta es LOCAL (los vecinos a k saltos
//      de un nodo), materializar el grafo entero es tirar la memoria por la
//      ventana: el 99,9% de lo leído no se va a usar. El BFS por niveles lee
//      las aristas del store BAJO DEMANDA, frontera a frontera: expande la
//      frontera actual, descubre la siguiente, y no toca nada más. La memoria
//      es ∝ nodos VISITADOS (nunca al grafo), y un **presupuesto**
//      ([`Presupuesto`]) acota profundidad, nodos visitados y ARISTAS LEÍDAS
//      — early-stop con [`MotivoParada`] explícito. El [`FronterasBfs`] es
//      un Iterador real: perezoso (pedir 2 fronteras y soltarlo sólo leyó 2
//      niveles), con stats que cuentan cada lectura para PODER demostrar
//      que no se leyó todo.
//
// Piezas menores del guion que viven aquí:
//
//   * **Bitsets** ([`BitSet`]): el conjunto de visitados del streaming, a
//     mano (Vec<u64>, 1 bit por id — 1/8 del espacio de un HashSet<usize>
//     y sin hashing). Regla "primero a mano" del Vol.II.
//   * **Procesamiento por bloques** ([`ProyeccionPonderada::bloques_de_nodos`]):
//     rangos de nodos listos para repartir. El PARALELISMO del guion queda
//     DOCUMENTADO y no implementado: los slices CSR son perfectamente
//     divisibles entre hilos, pero `&dyn GraphStore` no es `Sync` y el
//     workspace no usa crates (nada de rayon); cómo Neo4j GDS y Kùzu
//     paralelizan por bloques es prosa del capítulo, no código.
//   * **Instrumento de medida** ([`ContandoStore`]): un wrapper que cuenta
//     las lecturas que le llegan al store. Es el voltímetro del capítulo:
//     los tests enchufan el BFS o la proyección a un `ContandoStore` y
//     VERIFICAN las lecturas prometidas por las stats (contraste externo,
//     no auto-informe).
//
// Decisiones de diseño (los porqués):
//
// * **Convivencia, no refactor**: las proyecciones privadas de los caps 24
//   (no ponderada, `Both` = unión como CONJUNTO) y 25 (simétrica sumando,
//   self-loops ×2, reconstructible por nivel) NO se tocan. Cada una codifica
//   el contrato de SU familia de algoritmos; unificarlas a la fuerza sería
//   re-asegurar 597 tests por cero valor pedagógico. [`ProyeccionPonderada`]
//   es la API pública dirigida-out con pesos y edge ids que la Parte V
//   esperaba; la deuda del closeness ponderado (cap 24) se paga AQUÍ con
//   [`closeness_ponderado`] (Dijkstra por nodo sobre la proyección: V
//   orígenes, CERO lecturas del store tras materializar) y la del cap 22
//   (consistencia proyección↔algoritmo) con el test
//   `dijkstra_proyeccion_coincide_con_dijkstra_store`. Migrar los caps 24/25
//   a esta estructura queda como refactor futuro documentado, no como
//   requisito.
//
// * **Semántica de pesos heredada, no reinventada**: la proyección lee los
//   pesos con [`edge_weight`] del cap 22 tal cual (prop ausente/NULL =
//   `MissingWeight`, tipo no numérico = `InvalidWeight`, NaN/±∞ =
//   `NonFiniteWeight`, Int promociona a Float) y los envuelve en
//   [`ProyeccionError::Weight`] (`From<PathError>`). Un solo contrato de
//   calidad de dato para toda la Parte V. [`dijkstra_proyeccion`] rechaza
//   pesos negativos EAGER sobre TODA la proyección — la misma política
//   ruidosa del cap 22: una BD prefiere fallar a contestar casi-bien.
//
// * **Multigrafo fiel**: las aristas paralelas se conservan (cada una con su
//   peso) y los self-loops también — la proyección no interpreta el grafo,
//   lo FOTOGRAFÍA. Quien necesita otra lectura (unión, suma, simetrización)
//   la construye encima, como hicieron los caps 24/25.
//
// * **Determinismo**: nodos ordenados por id (compactando los huecos de
//   `delete_node` con el índice denso del cap 24), adyacencia ordenada por
//   (posición destino, id de arista), fronteras del BFS ordenadas por id.
//   Dos ejecuciones = resultados idénticos — un motor debe poder reproducir
//   sus análisis (misma regla que el Louvain del cap 25).
//
// * **Dirección en el streaming**: [`GraphDirection`] del cap 24 se reutiliza
//   tal cual (Out/In/Both). En streaming, `In` es GRATIS (se leen las
//   `in_edges` bajo demanda — nada que transponer) y `Both` lee ambas
//   adyacencias deduplicando por bitset (en un store simetrizado a mano cada
//   par se lee dos veces: documentado en las stats, no escondido).

// ─── Errores ───

/// Errores de la proyección y el streaming del cap 26.
#[derive(Debug, Clone, PartialEq)]
pub enum ProyeccionError {
    /// Peso inválido leído con la semántica estricta del cap 22
    /// (prop ausente/NULL, tipo no numérico, NaN/±∞) o coste que desborda.
    Weight(PathError),
    /// Peso negativo: Dijkstra exige pesos no negativos — misma política
    /// eager del cap 22, aplicada sobre TODA la proyección.
    NegativeWeight { edge: EdgeId, weight: f64 },
    /// El nodo (origen del recorrido, o consultado) no existe en el store.
    UnknownNode(NodeId),
    /// Un límite del presupuesto es inválido: `max_nodos` y `max_lecturas`
    /// deben ser ≥ 1 cuando están presentes (0 significaría "no empezar",
    /// que se consigue mejor no llamando).
    PresupuestoInvalido { campo: &'static str, valor: u64 },
    /// El tamaño de bloque de [`ProyeccionPonderada::bloques_de_nodos`]
    /// debe ser ≥ 1.
    BloqueInvalido { tam: usize },
}

impl fmt::Display for ProyeccionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProyeccionError::Weight(e) => write!(f, "invalid edge weight: {e}"),
            ProyeccionError::NegativeWeight { edge, weight } => write!(
                f,
                "edge {edge} has negative weight {weight} (Dijkstra requires non-negative weights; use bellman_ford)"
            ),
            ProyeccionError::UnknownNode(id) => write!(f, "unknown node id {id}"),
            ProyeccionError::PresupuestoInvalido { campo, valor } => {
                write!(f, "invalid budget: {campo} = {valor} (must be >= 1)")
            }
            ProyeccionError::BloqueInvalido { tam } => {
                write!(f, "invalid block size {tam} (must be >= 1)")
            }
        }
    }
}

impl std::error::Error for ProyeccionError {}

impl From<PathError> for ProyeccionError {
    fn from(e: PathError) -> Self {
        ProyeccionError::Weight(e)
    }
}

// ─── BitSet: el conjunto de visitados, a mano ───

/// Conjunto de ids como bitset: 1 bit por id posible, empaquetado en
/// palabras de 64 bits (`Vec<u64>`).
///
/// Es la pieza "bitsets" del guion, a mano (sin crates). Frente a un
/// `HashSet<NodeId>`: 1/8 del espacio por id en el rango alcanzado, sin
/// hashing ni rehashing, con localidad perfecta para el patrón del BFS
/// ("¿ya visité este vecino?"). Crece bajo demanda (una palabra nueva cada
/// 64 ids); el espacio es O(id_máximo_visitado / 8) — para ids muy dispersos
/// un HashSet puede ganar; documentado, y aquí los ids nacen densos del
/// store (cap 7).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BitSet {
    palabras: Vec<u64>,
}

impl BitSet {
    /// Bitset vacío.
    pub fn new() -> Self {
        BitSet {
            palabras: Vec::new(),
        }
    }

    fn asegurar(&mut self, id: usize) {
        let palabra = id / 64;
        if palabra >= self.palabras.len() {
            self.palabras.resize(palabra + 1, 0);
        }
    }

    /// Marca el id (idempotente).
    pub fn marcar(&mut self, id: usize) {
        self.asegurar(id);
        self.palabras[id / 64] |= 1u64 << (id % 64);
    }

    /// ¿Está el id marcado?
    pub fn contiene(&self, id: usize) -> bool {
        match self.palabras.get(id / 64) {
            Some(p) => p & (1u64 << (id % 64)) != 0,
            None => false,
        }
    }

    /// Número de bits marcados (popcount de todas las palabras).
    pub fn unos(&self) -> u64 {
        self.palabras.iter().map(|p| p.count_ones() as u64).sum()
    }

    /// ¿Ningún bit marcado?
    pub fn esta_vacio(&self) -> bool {
        self.palabras.iter().all(|p| *p == 0)
    }
}

// ─── Filtro de subgrafo ───

/// Qué parte del grafo entra en la proyección: subconjunto de etiquetas de
/// nodo y/o de tipos de arista.
///
/// `None` en un campo = "sin restricción". Una arista entra si su tipo pasa
/// el filtro Y su ORIGEN pasa el filtro de nodo (la proyección itera nodos
/// admitidos) Y su DESTINO también — una arista hacia un nodo excluido se
/// DESCARTA (contada en [`ProyeccionStats::descartadas`]): un subgrafo no
/// puede tener aristas colgando hacia nodos que no existen en él.
///
/// ```
/// use vol2_liradb::{Edge, FiltroProyeccion, MemoryStore, Node, ProyeccionPonderada, WeightSource};
/// use vol2_liradb::GraphStore;
///
/// let mut s = MemoryStore::new();
/// s.put_node(Node::new(0, "Persona")).unwrap();
/// s.put_node(Node::new(1, "Ciudad")).unwrap();
/// s.put_edge(Edge::new(0, 0, 1, "VIVE_EN")).unwrap();
///
/// // Sólo personas: la arista hacia la ciudad queda fuera.
/// let filtro = FiltroProyeccion::labels(["Persona"]);
/// let p = ProyeccionPonderada::proyectar(&s, &WeightSource::default(), &filtro).unwrap();
/// assert_eq!(p.num_nodos(), 1);
/// assert_eq!(p.num_aristas(), 0);
/// assert_eq!(p.stats().descartadas, 1);
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FiltroProyeccion {
    labels_nodo: Option<HashSet<String>>,
    tipos_arista: Option<HashSet<String>>,
}

impl FiltroProyeccion {
    /// Sin restricciones: el grafo entero.
    pub fn todo() -> Self {
        FiltroProyeccion {
            labels_nodo: None,
            tipos_arista: None,
        }
    }

    /// Sólo nodos con alguna de estas etiquetas.
    pub fn labels<I: IntoIterator<Item = S>, S: Into<String>>(labels: I) -> Self {
        FiltroProyeccion {
            labels_nodo: Some(labels.into_iter().map(Into::into).collect()),
            tipos_arista: None,
        }
    }

    /// Sólo aristas de estos tipos.
    pub fn tipos_arista<I: IntoIterator<Item = S>, S: Into<String>>(tipos: I) -> Self {
        FiltroProyeccion {
            labels_nodo: None,
            tipos_arista: Some(tipos.into_iter().map(Into::into).collect()),
        }
    }

    /// Añade restricción de etiquetas de nodo (builder).
    pub fn con_labels<I: IntoIterator<Item = S>, S: Into<String>>(mut self, labels: I) -> Self {
        self.labels_nodo = Some(labels.into_iter().map(Into::into).collect());
        self
    }

    /// Añade restricción de tipos de arista (builder).
    pub fn con_tipos_arista<I: IntoIterator<Item = S>, S: Into<String>>(
        mut self,
        tipos: I,
    ) -> Self {
        self.tipos_arista = Some(tipos.into_iter().map(Into::into).collect());
        self
    }

    fn admite_nodo(&self, node: &Node) -> bool {
        match &self.labels_nodo {
            None => true,
            Some(labels) => node.labels.iter().any(|l| labels.contains(l)),
        }
    }

    fn admite_arista(&self, edge: &Edge) -> bool {
        match &self.tipos_arista {
            None => true,
            Some(tipos) => tipos.contains(&edge.label),
        }
    }
}

// ─── Proyección ponderada (CSR en memoria con pesos e ids de arista) ───

/// Estadísticas de la materialización — el precio pagado por adelantado.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProyeccionStats {
    /// Nodos admitidos por el filtro (vivos en la proyección).
    pub nodos: u64,
    /// Aristas materializadas (dentro del filtro, entre nodos admitidos).
    pub aristas: u64,
    /// Lecturas de arista (`get_edge`) durante la materialización: UNA
    /// pasada por las adyacencias de los nodos admitidos.
    pub edges_scanned: u64,
    /// Aristas leídas pero descartadas: tipo no admitido o extremo fuera
    /// del filtro de nodos.
    pub descartadas: u64,
}

/// El grafo (o subgrafo) materializado en memoria compacta CON PESOS.
///
/// Layout CSR — el mismo del cap 14, completado con lo que allí no cabía:
/// ```text
///   nodes:   [id0, id1, ...]            ids ordenados (determinismo)
///   index:   id → posición densa        compacta huecos de delete_node
///   offsets: [0, g0, g0+g1, ...]        fronteras de fila (u32, como cap 14)
///   targets: posiciones destino          ← lo ÚNICO que guarda el CSR del 14
///   pesos:   f64 por arista             ← lo que añade este capítulo
///   aristas: EdgeId por arista          ← lo que añade este capítulo
/// ```
///
/// Es un **snapshot** inmutable: una foto del store en el instante de la
/// proyección. Los algoritmos iterativos (Dijkstra×V, PageRank, Louvain)
/// la recorren K veces sin volver a leer el store — el contrato
/// lectura-una-vez/iteración-muchas de la analítica, frente al OLTP de
/// punto (un `get_edge` puntual sobre el store vivo).
///
/// Dirigida OUT y fiel al multigrafo: paralelas y self-loops conservados;
/// quien necesite la vista no dirigida simetriza el store ANTES de
/// proyectar (como hacen los tests del cap 24).
///
/// ```
/// use vol2_liradb::{demo_graph, dijkstra_proyeccion, FiltroProyeccion, ProyeccionPonderada, WeightSource};
///
/// let store = demo_graph(); // Ana/Bo/Carla/Dani + Madrid/Lisboa
/// let proj = ProyeccionPonderada::proyectar(&store, &WeightSource::default(), &FiltroProyeccion::todo()).unwrap();
/// assert_eq!(proj.num_nodos(), 6);
/// assert_eq!(proj.num_aristas(), 6); // 4 KNOWS (con el self-loop de Dani) + 2 LIVES_IN
///
/// // Sobre la proyección, Dijkstra ya no toca el store:
/// let d = dijkstra_proyeccion(&proj, 0).unwrap();
/// assert_eq!(d.distancia(2), Some(2.0)); // Ana→Bo→Carla
/// assert_eq!(d.distancia(3), None);      // Dani es inalcanzable
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ProyeccionPonderada {
    /// Nodos vivos admitidos, orden ascendente de id (determinismo).
    nodes: Vec<NodeId>,
    /// `index[id]` = posición densa (None = id hueco, excluido o inexistente).
    index: Vec<Option<usize>>,
    /// Fronteras de fila CSR: vecinos(pos i) = targets[offsets[i]..offsets[i+1]].
    offsets: Vec<u32>,
    /// Posición densa del destino de cada arista.
    targets: Vec<u32>,
    /// Peso de cada arista (semántica estricta del cap 22).
    pesos: Vec<f64>,
    /// Id de arista real (lo que el CSR persistente del cap 14 no guarda).
    aristas: Vec<EdgeId>,
    stats: ProyeccionStats,
}

impl ProyeccionPonderada {
    /// Materializa el store (o su subgrafo según `filtro`) con pesos desde
    /// `weight`.
    ///
    /// UNA pasada por las adyacencias de los nodos admitidos: cada arista
    /// se lee (`get_edge`) y se pesa una sola vez; después de aquí, el
    /// store puede quedarse quieto. Errores: los del contrato de pesos del
    /// cap 22 ([`ProyeccionError::Weight`]).
    pub fn proyectar(
        store: &dyn GraphStore,
        weight: &WeightSource,
        filtro: &FiltroProyeccion,
    ) -> Result<Self, ProyeccionError> {
        // Nodos admitidos, ordenados; índice denso que compacta huecos
        // (mismo patrón determinista de los caps 24/25).
        let mut nodes: Vec<NodeId> = store
            .iter_nodes()
            .filter(|n| filtro.admite_nodo(n))
            .map(|n| n.id)
            .collect();
        nodes.sort_unstable();

        let table_len = nodes.last().map_or(0, |&m| m + 1);
        let mut index: Vec<Option<usize>> = vec![None; table_len];
        for (i, &n) in nodes.iter().enumerate() {
            index[n] = Some(i);
        }

        let mut stats = ProyeccionStats {
            nodos: nodes.len() as u64,
            ..ProyeccionStats::default()
        };
        let mut offsets: Vec<u32> = Vec::with_capacity(nodes.len() + 1);
        offsets.push(0);
        let mut targets: Vec<u32> = Vec::new();
        let mut pesos: Vec<f64> = Vec::new();
        let mut aristas: Vec<EdgeId> = Vec::new();

        for &u in &nodes {
            let mut fila: Vec<(usize, EdgeId, f64)> = Vec::new();
            for eid in store.out_edges(u) {
                let edge = store
                    .get_edge(eid)
                    .expect("invariante del store: la adjacencia sólo contiene aristas vivas");
                stats.edges_scanned += 1;
                if !filtro.admite_arista(edge) {
                    stats.descartadas += 1;
                    continue;
                }
                // Arista hacia un nodo fuera del filtro: descartada (un
                // subgrafo no tiene aristas hacia nodos que no contiene).
                let destino = match index.get(edge.target).copied().flatten() {
                    Some(d) => d,
                    None => {
                        stats.descartadas += 1;
                        continue;
                    }
                };
                let w = edge_weight(edge, weight)?;
                fila.push((destino, eid, w));
            }
            // Orden (destino, id de arista): determinismo total incluso con
            // paralelas (mismo destino, ids distintos). total_cmp porque el
            // peso es f64 (sin NaN: la construcción lo rechaza antes).
            fila.sort_unstable_by(|a, b| {
                a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.total_cmp(&b.2))
            });
            for (d, eid, w) in fila {
                targets.push(d as u32);
                aristas.push(eid);
                pesos.push(w);
            }
            offsets.push(targets.len() as u32);
        }
        stats.aristas = targets.len() as u64;

        Ok(ProyeccionPonderada {
            nodes,
            index,
            offsets,
            targets,
            pesos,
            aristas,
            stats,
        })
    }

    /// Nodos de la proyección, orden ascendente de id.
    pub fn ids(&self) -> &[NodeId] {
        &self.nodes
    }

    /// Número de nodos materializados.
    pub fn num_nodos(&self) -> usize {
        self.nodes.len()
    }

    /// Número de aristas materializadas.
    pub fn num_aristas(&self) -> usize {
        self.aristas.len()
    }

    /// Posición densa de un id de nodo (`None` = no está en la proyección).
    pub fn posicion(&self, id: NodeId) -> Option<usize> {
        self.index.get(id).copied().flatten()
    }

    /// Id real de una posición densa.
    ///
    /// # Panics
    /// Si `pos >= num_nodos()` (contrato interno: las posiciones nacen de
    /// `posicion`/`ids`).
    pub fn id_de(&self, pos: usize) -> NodeId {
        self.nodes[pos]
    }

    /// Vecinos salientes de una posición: `(posición destino, id de arista,
    /// peso)` en orden (destino, arista).
    pub fn vecinos(&self, pos: usize) -> impl Iterator<Item = (usize, EdgeId, f64)> + '_ {
        let (a, b) = (self.offsets[pos] as usize, self.offsets[pos + 1] as usize);
        self.targets[a..b]
            .iter()
            .zip(self.aristas[a..b].iter())
            .zip(self.pesos[a..b].iter())
            .map(|((&t, &e), &w)| (t as usize, e, w))
    }

    /// Grado saliente de una posición (O(1): dos lecturas de offsets).
    pub fn grado_out(&self, pos: usize) -> usize {
        (self.offsets[pos + 1] - self.offsets[pos]) as usize
    }

    /// Las fronteras de fila CSR (`n+1` entradas; `offsets[0] = 0`).
    pub fn offsets(&self) -> &[u32] {
        &self.offsets
    }

    /// Precio pagado por la materialización.
    pub fn stats(&self) -> ProyeccionStats {
        self.stats
    }

    /// Rangos de posiciones de nodos de tamaño `tam` — el "procesamiento
    /// por bloques" del guion: la semilla del paralelismo (cada bloque es
    /// un slice CSR independiente; los hilos llegan cuando el proyecto los
    /// admita — documentado en el banner del capítulo).
    ///
    /// El último bloque puede ser más corto. `tam = 0` →
    /// [`ProyeccionError::BloqueInvalido`].
    pub fn bloques_de_nodos(&self, tam: usize) -> Result<Vec<Range<usize>>, ProyeccionError> {
        if tam == 0 {
            return Err(ProyeccionError::BloqueInvalido { tam });
        }
        Ok((0..self.nodes.len())
            .step_by(tam)
            .map(|i| i..(i + tam).min(self.nodes.len()))
            .collect())
    }
}

// ─── Dijkstra sobre la proyección ───

/// Un paso de predecesor dentro de la proyección: desde la posición
/// `from_pos` cruzando la arista `edge` con peso `peso`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct PasoPred {
    from_pos: usize,
    edge: EdgeId,
    peso: f64,
}

/// Tabla de caminos mínimos calculada SOBRE la proyección (sin tocar el
/// store): distancias y predecesores por posición densa, con los ids
/// reales conservados para reconstruir caminos.
///
/// Hermana de la `ShortestPaths` del cap 22 — misma forma, distinta fuente
/// de datos: allí el store, aquí la memoria ya materializada.
#[derive(Debug, Clone, PartialEq)]
pub struct DistanciasProyeccion {
    /// Origen del cálculo (id real).
    pub origin: NodeId,
    /// Nodos de la proyección (para id ↔ posición).
    ids: Vec<NodeId>,
    /// `dist[pos]`: coste mínimo (INFINITY = inalcanzable).
    dist: Vec<f64>,
    /// `pred[pos]`: último paso que fijó `dist[pos]`.
    pred: Vec<Option<PasoPred>>,
    /// Estadísticas del cálculo (mismo tipo que el cap 22).
    pub stats: PathStats,
}

impl DistanciasProyeccion {
    fn pos_de(&self, id: NodeId) -> Option<usize> {
        self.ids.binary_search(&id).ok()
    }

    /// Distancia mínima al nodo `id`, o `None` si es inalcanzable o no
    /// está en la proyección.
    pub fn distancia(&self, id: NodeId) -> Option<f64> {
        match self.pos_de(id).map(|p| self.dist[p]) {
            Some(d) if d.is_finite() => Some(d),
            _ => None,
        }
    }

    /// Nodos alcanzados (distancia finita), en orden de id.
    pub fn alcanzados(&self) -> Vec<NodeId> {
        self.ids
            .iter()
            .copied()
            .zip(self.dist.iter())
            .filter(|(_, d)| d.is_finite())
            .map(|(n, _)| n)
            .collect()
    }

    /// Reconstruye el camino a `dest` (ids reales, aristas reales), o
    /// `None` si es inalcanzable. Mismo tipo [`Path`] que el cap 22:
    /// comparable directamente contra `dijkstra_path` sobre el store.
    pub fn camino_a(&self, dest: NodeId) -> Option<Path> {
        let cost = self.distancia(dest)?;
        let mut steps: Vec<PathStep> = Vec::new();
        let mut pos = self.pos_de(dest)?;
        while let Some(paso) = &self.pred[pos] {
            steps.push(PathStep {
                edge: paso.edge,
                from: self.ids[paso.from_pos],
                to: self.ids[pos],
                weight: paso.peso,
            });
            pos = paso.from_pos;
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

/// Dijkstra single-source sobre la proyección materializada.
///
/// Misma política de datos que el cap 22: pesos validados eager en TODA la
/// proyección (negativos → [`ProyeccionError::NegativeWeight`], una BD
/// prefiere fallar ruidosamente) y mismo heap
/// (`BinaryHeap<Reverse<(Cost, NodeId)>>` con borrado perezoso). La
/// diferencia la pone la fuente: vecinos desde slices CSR en memoria —
/// CERO lecturas del store durante el cálculo (verificable con
/// [`ContandoStore`]).
///
/// ```
/// use vol2_liradb::{dijkstra_proyeccion, Edge, FiltroProyeccion, MemoryStore, Node, ProyeccionPonderada, Value, WeightSource};
/// use vol2_liradb::GraphStore;
///
/// let mut s = MemoryStore::new();
/// for i in 0..3 { s.put_node(Node::new(i, "N")).unwrap(); }
/// s.put_edge(Edge::new(0, 0, 1, "R").with_prop("w", Value::Float(2.0))).unwrap();
/// s.put_edge(Edge::new(1, 1, 2, "R").with_prop("w", Value::Float(3.0))).unwrap();
/// s.put_edge(Edge::new(2, 0, 2, "R").with_prop("w", Value::Float(10.0))).unwrap();
///
/// let proj = ProyeccionPonderada::proyectar(&s, &WeightSource::property("w"), &FiltroProyeccion::todo()).unwrap();
/// let d = dijkstra_proyeccion(&proj, 0).unwrap();
/// assert_eq!(d.distancia(2), Some(5.0)); // dos saltos baratos > directo caro
/// assert_eq!(d.camino_a(2).unwrap().nodes(), vec![0, 1, 2]);
/// ```
pub fn dijkstra_proyeccion(
    proj: &ProyeccionPonderada,
    origin: NodeId,
) -> Result<DistanciasProyeccion, ProyeccionError> {
    let origen_pos = proj
        .posicion(origin)
        .ok_or(ProyeccionError::UnknownNode(origin))?;
    // Política del cap 22: sanidad eager sobre TODA la proyección.
    validar_pesos_no_negativos(proj)?;
    dijkstra_interna(proj, origen_pos)
}

/// Sanidad eager O(E) sobre la proyección: peso no negativo en TODAS las
/// aristas — la misma decisión documentada del cap 22 (rechazar también las
/// de zonas que la consulta no pisará).
fn validar_pesos_no_negativos(proj: &ProyeccionPonderada) -> Result<(), ProyeccionError> {
    for pos in 0..proj.num_nodos() {
        for (_, eid, w) in proj.vecinos(pos) {
            if w < 0.0 {
                return Err(ProyeccionError::NegativeWeight {
                    edge: eid,
                    weight: w,
                });
            }
        }
    }
    Ok(())
}

/// El núcleo sin validación (la valida una vez quien llama muchas: ver
/// [`closeness_ponderado`]).
fn dijkstra_interna(
    proj: &ProyeccionPonderada,
    origen_pos: usize,
) -> Result<DistanciasProyeccion, ProyeccionError> {
    let n = proj.num_nodos();
    let mut dist = vec![f64::INFINITY; n];
    let mut pred: Vec<Option<PasoPred>> = vec![None; n];
    let mut settled = vec![false; n];
    let mut stats = PathStats::default();

    let mut heap: BinaryHeap<Reverse<(Cost, usize)>> = BinaryHeap::new();
    dist[origen_pos] = 0.0;
    heap.push(Reverse((Cost(0.0), origen_pos)));

    while let Some(Reverse((Cost(d), u))) = heap.pop() {
        stats.popped += 1;
        if settled[u] {
            continue; // entrada obsoleta (lazy deletion)
        }
        settled[u] = true;
        stats.expanded += 1;
        for (v, eid, w) in proj.vecinos(u) {
            stats.relax_attempts += 1;
            let nuevo = d + w; // w >= 0 por la validación eager
            if !nuevo.is_finite() {
                // Desbordamiento: mismo error tipado que el cap 22, para no
                // confundir un coste real desbordado con el centinela de
                // inalcanzable.
                return Err(ProyeccionError::Weight(PathError::CostOverflow {
                    edge: eid,
                }));
            }
            if nuevo < dist[v] {
                dist[v] = nuevo;
                pred[v] = Some(PasoPred {
                    from_pos: u,
                    edge: eid,
                    peso: w,
                });
                stats.relax_updates += 1;
                heap.push(Reverse((Cost(nuevo), v)));
            }
        }
    }

    Ok(DistanciasProyeccion {
        origin: proj.id_de(origen_pos),
        ids: proj.ids().to_vec(),
        dist,
        pred,
        stats,
    })
}

// ─── Closeness ponderado (la deuda del cap 24, saldada) ───

/// Cercanía ponderada: Dijkstra por cada origen SOBRE la proyección.
///
/// La deuda que el cap 24 dejó apuntada ("cuando exista la proyección con
/// pesos, el BFS de saltos se cambia por Dijkstra sin tocar nada más"):
/// misma corrección Wasserman-Faust para componentes desconectadas —
///
/// ```text
///   C(u) = ((r-1)/(n-1)) · ((r-1)/Σd)
/// ```
///
/// con `r` los nodos alcanzables desde `u` y `Σd` la suma de distancias
/// PONDERADAS (no saltos). Devuelve pares `(id, score)` en orden de id.
///
/// El argumento económico del capítulo, medible: V Dijkstras = CERO
/// lecturas del store tras la materialización (la validación de pesos corre
/// UNA vez, no V). Sobre el store directamente (cap 22) cada `dijkstra`
/// re-validaría E aristas: V·E lecturas desperdiciadas.
pub fn closeness_ponderado(
    proj: &ProyeccionPonderada,
) -> Result<Vec<(NodeId, f64)>, ProyeccionError> {
    validar_pesos_no_negativos(proj)?;
    let n = proj.num_nodos();
    let mut out = Vec::with_capacity(n);
    for pos in 0..n {
        let d = dijkstra_interna(proj, pos)?;
        let mut suma_d = 0.0_f64;
        let mut r = 1u64; // el origen cuenta
        for (i, &dist_i) in d.dist.iter().enumerate() {
            if i != pos && dist_i.is_finite() {
                r += 1;
                suma_d += dist_i;
            }
        }
        let score = if n > 1 && suma_d > 0.0 {
            let rf = r as f64;
            ((rf - 1.0) / (n as f64 - 1.0)) * ((rf - 1.0) / suma_d)
        } else {
            0.0
        };
        out.push((proj.id_de(pos), score));
    }
    Ok(out)
}

// ─── Presupuesto del streaming ───

/// Los límites que gobiernan un recorrido por fronteras: el "API que el
/// lector pueda gobernar" del capítulo.
///
/// Cada límite es `None` = sin límite. `max_profundidad` cuenta FRONTERAS
/// (0 = sólo el origen); `max_nodos` acota los nodos visitados (comprobado
/// en cada descubrimiento: exacto); `max_lecturas` acota las aristas leídas
/// del store (comprobado antes de cada `get_edge`: exacto) — acotar
/// lecturas es acotar tiempo Y memoria de trabajo en un store en disco.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Presupuesto {
    /// Fronteras máximas a emitir (el nivel 0 es el origen).
    pub max_profundidad: Option<u32>,
    /// Nodos visitados máximo (≥ 1 si está presente).
    pub max_nodos: Option<u64>,
    /// Aristas leídas del store máximo (≥ 1 si está presente).
    pub max_lecturas: Option<u64>,
}

impl Presupuesto {
    /// Sin ningún límite: el recorrido acaba cuando se agota la componente.
    pub fn sin_limite() -> Self {
        Presupuesto::default()
    }

    /// Presupuesto con límite de profundidad.
    pub fn profundidad(k: u32) -> Self {
        Presupuesto {
            max_profundidad: Some(k),
            ..Presupuesto::default()
        }
    }

    /// Añade límite de profundidad (builder).
    pub fn con_profundidad(mut self, k: u32) -> Self {
        self.max_profundidad = Some(k);
        self
    }

    /// Añade límite de nodos visitados (builder).
    pub fn con_nodos(mut self, m: u64) -> Self {
        self.max_nodos = Some(m);
        self
    }

    /// Añade límite de aristas leídas (builder).
    pub fn con_lecturas(mut self, m: u64) -> Self {
        self.max_lecturas = Some(m);
        self
    }

    fn validar(&self) -> Result<(), ProyeccionError> {
        if self.max_nodos == Some(0) {
            return Err(ProyeccionError::PresupuestoInvalido {
                campo: "max_nodos",
                valor: 0,
            });
        }
        if self.max_lecturas == Some(0) {
            return Err(ProyeccionError::PresupuestoInvalido {
                campo: "max_lecturas",
                valor: 0,
            });
        }
        Ok(())
    }
}

// ─── Streaming por fronteras ───

/// Estadísticas del recorrido — la prueba medible de que NO se leyó todo.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StreamStats {
    /// Nodos visitados (incluido el origen).
    pub nodos_visitados: u64,
    /// Aristas leídas del store (`get_edge`): una por entrada de adyacencia
    /// examinada.
    pub aristas_leidas: u64,
    /// Llamadas de adyacencia (`out_edges`/`in_edges`): una por nodo
    /// expandido y dirección.
    pub adyacencia_consultas: u64,
    /// Fronteras emitidas (la del origen incluida).
    pub fronteras: u32,
}

/// Por qué terminó el recorrido.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotivoParada {
    /// Se agotó la componente alcanzable (respuesta COMPLETA).
    Completo,
    /// Presupuesto de profundidad agotado.
    ProfundidadMaxima,
    /// Presupuesto de nodos visitados agotado.
    PresupuestoNodos,
    /// Presupuesto de aristas leídas agotado.
    PresupuestoLecturas,
}

/// Resultado de un recorrido completo por fronteras.
///
/// `niveles[0] = [origen]`, cada nivel ordenado por id (determinismo).
#[derive(Debug, Clone, PartialEq)]
pub struct RecorridoBfs {
    /// Origen del recorrido.
    pub origen: NodeId,
    /// Frontera a frontera: `niveles[k]` = nodos descubiertos a k saltos.
    pub niveles: Vec<Vec<NodeId>>,
    /// Estadísticas de lecturas.
    pub stats: StreamStats,
    /// Por qué paró.
    pub parada: MotivoParada,
}

impl RecorridoBfs {
    /// Todos los nodos visitados, por niveles (cada nivel ordenado).
    pub fn nodos(&self) -> Vec<NodeId> {
        self.niveles.iter().flatten().copied().collect()
    }

    /// Profundidad máxima alcanzada (saltos; 0 = sólo el origen).
    pub fn profundidad(&self) -> u32 {
        self.niveles.len().saturating_sub(1) as u32
    }
}

/// Iterador perezoso de fronteras: cada [`next`](Iterator::next) produce la
/// siguiente frontera leyendo del store SÓLO lo que esa frontera necesita.
///
/// Es el "streaming" del guion hecho tipo: pedir dos fronteras y soltar el
/// iterador deja el grafo intacto salvo dos niveles de adyacencia — la
/// consulta local que NO paga el grafo entero. Los presupuestos viven DENTRO
/// (el iterador para solo); los errores de datos no existen aquí (el BFS no
/// pesa aristas: la semántica estricta de pesos es cosa de la proyección).
///
/// Tras agotarse, [`parada`](FronterasBfs::parada) dice por qué.
pub struct FronterasBfs<'a> {
    store: &'a dyn GraphStore,
    dir: GraphDirection,
    presupuesto: Presupuesto,
    visitado: BitSet,
    frontera: Vec<NodeId>,
    iniciado: bool,
    detenido: bool,
    profundidad: u32,
    stats: StreamStats,
    terminado: Option<MotivoParada>,
}

impl<'a> FronterasBfs<'a> {
    /// Estadísticas de lecturas hasta el momento.
    pub fn stats(&self) -> StreamStats {
        self.stats
    }

    /// Por qué terminó (None mientras queden fronteras por emitir).
    pub fn parada(&self) -> Option<MotivoParada> {
        self.terminado
    }

    /// Expande la frontera actual leyendo el store bajo demanda.
    ///
    /// Devuelve la siguiente frontera (vacía = no quedó nada alcanzable).
    /// Los presupuestos se comprueban ANTES de cada lectura de arista
    /// (lecturas: exacto) y ANTES de cada descubrimiento (nodos: exacto) —
    /// nunca se superan.
    fn expandir(&mut self) -> Vec<NodeId> {
        let store = self.store;
        let actual = std::mem::take(&mut self.frontera);
        let mut siguiente: Vec<NodeId> = Vec::new();

        let usa_out = matches!(self.dir, GraphDirection::Out | GraphDirection::Both);
        let usa_in = matches!(self.dir, GraphDirection::In | GraphDirection::Both);

        'nodos: for u in actual {
            // (ids de arista, ¿tomar el target?) por dirección: en Out
            // seguimos targets, en In seguimos sources, en Both ambas
            // (deduplica el bitset de visitados; un store simetrizado a
            // mano paga cada par dos veces — visible en aristas_leidas).
            let mut listas: Vec<(Vec<EdgeId>, bool)> = Vec::new();
            if usa_out {
                self.stats.adyacencia_consultas += 1;
                listas.push((store.out_edges(u), true));
            }
            if usa_in {
                self.stats.adyacencia_consultas += 1;
                listas.push((store.in_edges(u), false));
            }
            for (eids, tomar_target) in listas {
                for eid in eids {
                    if let Some(max) = self.presupuesto.max_lecturas
                        && self.stats.aristas_leidas >= max
                    {
                        self.terminado = Some(MotivoParada::PresupuestoLecturas);
                        break 'nodos;
                    }
                    let edge = store
                        .get_edge(eid)
                        .expect("invariante del store: la adjacencia sólo contiene aristas vivas");
                    self.stats.aristas_leidas += 1;
                    let v = if tomar_target {
                        edge.target
                    } else {
                        edge.source
                    };
                    if self.visitado.contiene(v) {
                        continue;
                    }
                    if let Some(max) = self.presupuesto.max_nodos
                        && self.stats.nodos_visitados >= max
                    {
                        self.terminado = Some(MotivoParada::PresupuestoNodos);
                        break 'nodos;
                    }
                    self.visitado.marcar(v);
                    self.stats.nodos_visitados += 1;
                    siguiente.push(v);
                }
            }
        }
        siguiente.sort_unstable();
        siguiente
    }
}

impl Iterator for FronterasBfs<'_> {
    type Item = Vec<NodeId>;

    fn next(&mut self) -> Option<Vec<NodeId>> {
        if self.detenido {
            return None;
        }
        if !self.iniciado {
            self.iniciado = true;
            self.visitado.marcar(self.frontera[0]);
            self.stats.nodos_visitados += 1;
            self.stats.fronteras += 1;
            return Some(self.frontera.clone());
        }
        // Presupuesto de profundidad: la frontera emitida ya está al máximo.
        if let Some(max) = self.presupuesto.max_profundidad
            && self.profundidad >= max
        {
            self.terminado = Some(MotivoParada::ProfundidadMaxima);
            self.detenido = true;
            return None;
        }
        let siguiente = self.expandir();
        if siguiente.is_empty() {
            self.detenido = true;
            if self.terminado.is_none() {
                self.terminado = Some(MotivoParada::Completo);
            }
            return None;
        }
        // Si un presupuesto cortó la expansión, ésta es la ÚLTIMA frontera:
        // se emite (sus nodos están visitados) y la siguiente llamada para.
        if self.terminado.is_some() {
            self.detenido = true;
        }
        self.profundidad += 1;
        self.stats.fronteras += 1;
        self.frontera = siguiente.clone();
        Some(siguiente)
    }
}

/// Construye el iterador de fronteras de un BFS bajo demanda.
///
/// Valida que el origen exista y que el presupuesto sea legal
/// ([`Presupuesto::max_nodos`]/[`max_lecturas`](Presupuesto::max_lecturas)
/// ≥ 1). El recorrido es dirigido según `dir` ([`GraphDirection`] del cap
/// 24; `In` lee `in_edges` directamente — nada que transponer).
///
/// ```
/// use vol2_liradb::{bfs_fronteras, Edge, GraphDirection, MemoryStore, Node, Presupuesto};
/// use vol2_liradb::GraphStore;
///
/// let mut s = MemoryStore::new();
/// for i in 0..4 { s.put_node(Node::new(i, "N")).unwrap(); }
/// for (a, b) in [(0, 1), (1, 2), (2, 3)] {
///     s.put_edge(Edge::new(a, a, b, "E")).unwrap();
/// }
///
/// let mut it = bfs_fronteras(&s, 0, GraphDirection::Out, Presupuesto::profundidad(1)).unwrap();
/// assert_eq!(it.next(), Some(vec![0]));  // frontera 0: el origen
/// assert_eq!(it.next(), Some(vec![1]));  // frontera 1
/// assert_eq!(it.next(), None);           // presupuesto de profundidad
/// assert_eq!(it.stats().aristas_leidas, 1); // sólo leyó la arista 0→1
/// ```
pub fn bfs_fronteras(
    store: &dyn GraphStore,
    origen: NodeId,
    dir: GraphDirection,
    presupuesto: Presupuesto,
) -> Result<FronterasBfs<'_>, ProyeccionError> {
    presupuesto.validar()?;
    if store.get_node(origen).is_none() {
        return Err(ProyeccionError::UnknownNode(origen));
    }
    Ok(FronterasBfs {
        store,
        dir,
        presupuesto,
        visitado: BitSet::new(),
        frontera: vec![origen],
        iniciado: false,
        detenido: false,
        profundidad: 0,
        stats: StreamStats::default(),
        terminado: None,
    })
}

/// BFS por fronteras con presupuesto, de una tirada.
///
/// Equivalente a consumir [`bfs_fronteras`] hasta el fondo: devuelve los
/// niveles, las stats de lectura y el [`MotivoParada`]. La memoria de
/// trabajo es ∝ nodos visitados (bitset + fronteras), NUNCA al grafo.
///
/// ```
/// use vol2_liradb::{bfs_streaming, demo_graph, GraphDirection, MotivoParada, Presupuesto};
///
/// let store = demo_graph();
/// // Desde Ana, 1 salto: Bo (KNOWS) y Madrid (LIVES_IN) — nada más se lee.
/// let r = bfs_streaming(&store, 0, GraphDirection::Out, Presupuesto::profundidad(1)).unwrap();
/// assert_eq!(r.niveles, vec![vec![0], vec![1, 4]]);
/// assert_eq!(r.parada, MotivoParada::ProfundidadMaxima);
/// assert_eq!(r.stats.nodos_visitados, 3);
/// ```
pub fn bfs_streaming(
    store: &dyn GraphStore,
    origen: NodeId,
    dir: GraphDirection,
    presupuesto: Presupuesto,
) -> Result<RecorridoBfs, ProyeccionError> {
    let mut it = bfs_fronteras(store, origen, dir, presupuesto)?;
    let mut niveles = Vec::new();
    for nivel in it.by_ref() {
        niveles.push(nivel);
    }
    let parada = it.parada().unwrap_or(MotivoParada::Completo);
    Ok(RecorridoBfs {
        origen,
        niveles,
        stats: it.stats(),
        parada,
    })
}

// ─── ContandoStore: el voltímetro del capítulo ───

/// Wrapper de SOLO LECTURA que cuenta las lecturas que llegan al store.
///
/// Es un instrumento de medida — el contraste EXTERNO de las stats: los
/// tests (y el lector) pueden envolver cualquier `&dyn GraphStore` y
/// verificar cuántas lecturas produjo de verdad una proyección o un BFS.
/// Los contadores usan `Cell` (los métodos de lectura del trait son `&self`).
///
/// # Panics
/// `put_node`/`put_edge`/`delete_*` no están soportados: envuelven una
/// referencia inmutable `&dyn GraphStore` — es un voltímetro, no un store.
pub struct ContandoStore<'a> {
    inner: &'a dyn GraphStore,
    lecturas_arista: Cell<u64>,
    lecturas_nodo: Cell<u64>,
    consultas_out: Cell<u64>,
    consultas_in: Cell<u64>,
}

impl fmt::Debug for ContandoStore<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContandoStore")
            .field("lecturas_arista", &self.lecturas_arista.get())
            .field("lecturas_nodo", &self.lecturas_nodo.get())
            .field("consultas_out", &self.consultas_out.get())
            .field("consultas_in", &self.consultas_in.get())
            .finish()
    }
}

impl<'a> ContandoStore<'a> {
    /// Envuelve el store a medir.
    pub fn new(inner: &'a dyn GraphStore) -> Self {
        ContandoStore {
            inner,
            lecturas_arista: Cell::new(0),
            lecturas_nodo: Cell::new(0),
            consultas_out: Cell::new(0),
            consultas_in: Cell::new(0),
        }
    }

    /// Lecturas de arista (`get_edge`).
    pub fn lecturas_arista(&self) -> u64 {
        self.lecturas_arista.get()
    }

    /// Lecturas de nodo (`get_node`).
    pub fn lecturas_nodo(&self) -> u64 {
        self.lecturas_nodo.get()
    }

    /// Consultas de adyacencia saliente (`out_edges`).
    pub fn consultas_out(&self) -> u64 {
        self.consultas_out.get()
    }

    /// Consultas de adyacencia entrante (`in_edges`).
    pub fn consultas_in(&self) -> u64 {
        self.consultas_in.get()
    }

    /// Todas las lecturas juntas (elementos + adyacencias).
    pub fn total_lecturas(&self) -> u64 {
        self.lecturas_arista.get()
            + self.lecturas_nodo.get()
            + self.consultas_out.get()
            + self.consultas_in.get()
    }
}

impl GraphStore for ContandoStore<'_> {
    fn put_node(&mut self, _node: Node) -> Result<(), StoreError> {
        panic!("ContandoStore es un instrumento de medida de sólo lectura")
    }

    fn put_edge(&mut self, _edge: Edge) -> Result<(), StoreError> {
        panic!("ContandoStore es un instrumento de medida de sólo lectura")
    }

    fn get_node(&self, id: NodeId) -> Option<&Node> {
        self.lecturas_nodo.set(self.lecturas_nodo.get() + 1);
        self.inner.get_node(id)
    }

    fn get_edge(&self, id: EdgeId) -> Option<&Edge> {
        self.lecturas_arista.set(self.lecturas_arista.get() + 1);
        self.inner.get_edge(id)
    }

    fn out_edges(&self, u: NodeId) -> Vec<EdgeId> {
        self.consultas_out.set(self.consultas_out.get() + 1);
        self.inner.out_edges(u)
    }

    fn in_edges(&self, u: NodeId) -> Vec<EdgeId> {
        self.consultas_in.set(self.consultas_in.get() + 1);
        self.inner.in_edges(u)
    }

    fn node_count(&self) -> usize {
        self.inner.node_count()
    }

    fn edge_count(&self) -> usize {
        self.inner.edge_count()
    }

    fn delete_node(&mut self, _id: NodeId) -> bool {
        panic!("ContandoStore es un instrumento de medida de sólo lectura")
    }

    fn delete_edge(&mut self, _id: EdgeId) -> bool {
        panic!("ContandoStore es un instrumento de medida de sólo lectura")
    }

    fn iter_nodes(&self) -> Box<dyn Iterator<Item = &Node> + '_> {
        self.inner.iter_nodes()
    }

    fn iter_edges(&self) -> Box<dyn Iterator<Item = &Edge> + '_> {
        self.inner.iter_edges()
    }
}

// ══════════════════════════════════════════════════════════════════════
//  Tests
// ══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests_proyeccion {
    use super::*;
    use crate::cap07_modelo::Value;
    use crate::cap08_graph_store::MemoryStore;
    use crate::cap22_caminos_minimos::dijkstra as dijkstra_store;

    /// Tolerancia genérica para comparar f64 contra soluciones a mano.
    const EPS: f64 = 1e-9;

    fn cerca(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    /// Store con nodos 0..n y aristas dirigidas ponderadas (prop "weight",
    /// Float). Como el fixture del cap 22.
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

    fn w() -> WeightSource {
        WeightSource::property("weight")
    }

    fn proj(store: &dyn GraphStore) -> ProyeccionPonderada {
        ProyeccionPonderada::proyectar(store, &w(), &FiltroProyeccion::todo()).unwrap()
    }

    // ════════════════════════════════════════════════════════════════
    //  BitSet
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn bitset_marca_consulta_y_cuenta() {
        let mut b = BitSet::new();
        assert!(b.esta_vacio());
        assert_eq!(b.unos(), 0);
        assert!(!b.contiene(0));

        b.marcar(0);
        b.marcar(63); // último bit de la palabra 0
        b.marcar(64); // primer bit de la palabra 1
        b.marcar(65);
        b.marcar(100);
        b.marcar(0); // idempotente

        assert!(b.contiene(0) && b.contiene(63) && b.contiene(64) && b.contiene(100));
        assert!(!b.contiene(1));
        assert!(!b.contiene(62));
        assert!(!b.contiene(66));
        assert!(!b.contiene(1_000_000)); // muy lejos: false, sin pánico
        assert_eq!(b.unos(), 5);
        assert!(!b.esta_vacio());
    }

    #[test]
    fn bitset_eq_clone_y_default() {
        let mut a = BitSet::new();
        a.marcar(7);
        let b = a.clone();
        assert_eq!(a, b);
        assert_eq!(BitSet::new(), BitSet::default());
        assert_ne!(a, BitSet::new());
    }

    // ════════════════════════════════════════════════════════════════
    //  Proyección: pesos, filtros, layout CSR
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn proyeccion_con_pesos_correcta_vs_recuento_manual() {
        // El diamante del cap 22: 0-1 (1), 1-3 (2), 0-2 (4), 2-3 (3).
        let s = grafo(
            4,
            &[
                (0, 0, 1, 1.0),
                (1, 1, 3, 2.0),
                (2, 0, 2, 4.0),
                (3, 2, 3, 3.0),
            ],
        );
        let p = proj(&s);

        assert_eq!(p.num_nodos(), 4);
        assert_eq!(p.num_aristas(), 4);
        assert_eq!(p.ids(), &[0, 1, 2, 3]);
        assert_eq!(p.stats().edges_scanned, 4);
        assert_eq!(p.stats().descartadas, 0);

        // Adyacencia exacta por nodo (posición, arista, peso).
        let vecinos0: Vec<(usize, EdgeId, f64)> = p.vecinos(0).collect();
        assert_eq!(vecinos0, vec![(1, 0, 1.0), (2, 2, 4.0)]);
        let vecinos1: Vec<_> = p.vecinos(1).collect();
        assert_eq!(vecinos1, vec![(3, 1, 2.0)]);
        let vecinos2: Vec<_> = p.vecinos(2).collect();
        assert_eq!(vecinos2, vec![(3, 3, 3.0)]);
        assert_eq!(p.vecinos(3).count(), 0);

        // Grados O(1) y acceso id ↔ posición.
        assert_eq!(p.grado_out(0), 2);
        assert_eq!(p.grado_out(3), 0);
        assert_eq!(p.posicion(2), Some(2));
        assert_eq!(p.id_de(2), 2);
        assert_eq!(p.posicion(99), None);
    }

    #[test]
    fn proyeccion_layout_csr_offsets_prefijo() {
        // offsets[0] = 0; offsets[i+1] - offsets[i] = grado(i);
        // Σ grados = num_aristas = targets.len().
        let s = grafo(
            6,
            &[
                (0, 0, 1, 1.0),
                (1, 0, 2, 1.0),
                (2, 0, 0, 5.0), // self-loop
                (3, 1, 2, 1.0),
                (4, 4, 5, 1.0),
            ],
        );
        let p = proj(&s);
        assert_eq!(p.offsets().len(), 7); // n+1 entradas
        assert_eq!(p.offsets()[0], 0); // prefijo CSR clásico
        let mut suma = 0;
        for i in 0..p.num_nodos() {
            let g = p.grado_out(i);
            assert_eq!(g, p.vecinos(i).count());
            suma += g;
        }
        assert_eq!(suma, p.num_aristas());
    }

    #[test]
    fn proyeccion_multigrafo_selfloops_e_ids_huecos() {
        // Dos paralelas 0→1 (ids 7 y 3: orden por id dentro del mismo
        // destino) y un self-loop.
        let s = grafo(2, &[(7, 0, 1, 5.0), (3, 0, 1, 2.0), (9, 1, 1, 1.0)]);
        let p = proj(&s);
        assert_eq!(p.num_aristas(), 3);
        let vecinos0: Vec<_> = p.vecinos(0).collect();
        // Mismo destino (1): la arista 3 antes que la 7 (orden (dest, eid)).
        assert_eq!(vecinos0, vec![(1, 3, 2.0), (1, 7, 5.0)]);
        let vecinos1: Vec<_> = p.vecinos(1).collect();
        assert_eq!(vecinos1, vec![(1, 9, 1.0)]); // self-loop conservado

        // Hueco tras delete_node(0): índice denso que compacta.
        let mut s2 = grafo(4, &[(0, 1, 2, 1.0), (1, 2, 3, 1.0)]);
        assert!(s2.delete_node(0));
        let p2 = proj(&s2);
        assert_eq!(p2.num_nodos(), 3);
        assert_eq!(p2.ids(), &[1, 2, 3]);
        assert_eq!(p2.posicion(0), None);
        assert_eq!(p2.posicion(1), Some(0));
        assert_eq!(p2.num_aristas(), 2);
    }

    #[test]
    fn proyeccion_hereda_los_errores_estrictos_de_pesos_del_cap22() {
        // Prop ausente → MissingWeight envuelto en Weight.
        let mut s = MemoryStore::new();
        s.put_node(Node::new(0, "N")).unwrap();
        s.put_node(Node::new(1, "N")).unwrap();
        s.put_edge(Edge::new(0, 0, 1, "R")).unwrap();
        assert_eq!(
            ProyeccionPonderada::proyectar(&s, &w(), &FiltroProyeccion::todo()),
            Err(ProyeccionError::Weight(PathError::MissingWeight {
                edge: 0,
                prop: "weight".into()
            }))
        );
        // From<PathError> permite usar ? desde cap 22.
        let e: ProyeccionError = PathError::UnknownNode(4).into();
        assert_eq!(e, ProyeccionError::Weight(PathError::UnknownNode(4)));

        // Tipo no numérico y no finito: mismo contrato.
        let mut s2 = MemoryStore::new();
        s2.put_node(Node::new(0, "N")).unwrap();
        s2.put_node(Node::new(1, "N")).unwrap();
        s2.put_edge(Edge::new(0, 0, 1, "R").with_prop("weight", Value::Bool(true)))
            .unwrap();
        assert!(matches!(
            ProyeccionPonderada::proyectar(&s2, &w(), &FiltroProyeccion::todo()),
            Err(ProyeccionError::Weight(PathError::InvalidWeight { .. }))
        ));
        let mut s3 = grafo(2, &[]);
        s3.put_edge(Edge::new(0, 0, 1, "R").with_prop("weight", Value::Float(f64::NAN)))
            .unwrap();
        assert!(matches!(
            ProyeccionPonderada::proyectar(&s3, &w(), &FiltroProyeccion::todo()),
            Err(ProyeccionError::Weight(PathError::NonFiniteWeight { .. }))
        ));

        // Los pesos NEGATIVOS NO son error de proyección (Bellman-Ford los
        // admite): son error de dijkstra_proyeccion (política del cap 22).
        let s4 = grafo(2, &[(0, 0, 1, -2.5)]);
        let p4 = proj(&s4);
        assert_eq!(p4.num_aristas(), 1);
        assert_eq!(
            dijkstra_proyeccion(&p4, 0),
            Err(ProyeccionError::NegativeWeight {
                edge: 0,
                weight: -2.5
            })
        );
    }

    #[test]
    fn subgrafo_filtrado_por_label_y_tipo_de_arista() {
        // Red social mínima: Personas y Ciudades; KNOWS y VIVE_EN.
        let mut s = MemoryStore::new();
        s.put_node(Node::new(0, "Persona")).unwrap();
        s.put_node(Node::new(1, "Persona")).unwrap();
        s.put_node(Node::new(4, "Ciudad")).unwrap();
        s.put_edge(Edge::new(0, 0, 1, "KNOWS").with_prop("weight", Value::Float(1.0)))
            .unwrap();
        s.put_edge(Edge::new(1, 1, 4, "VIVE_EN").with_prop("weight", Value::Float(1.0)))
            .unwrap();
        s.put_edge(Edge::new(2, 4, 0, "VIVE_EN").with_prop("weight", Value::Float(1.0)))
            .unwrap();

        // Sólo Personas: las VIVE_EN hacia/hacia la ciudad se descartan.
        let p = ProyeccionPonderada::proyectar(&s, &w(), &FiltroProyeccion::labels(["Persona"]))
            .unwrap();
        assert_eq!(p.num_nodos(), 2);
        assert_eq!(p.num_aristas(), 1);
        // Sólo 1 descartada visible (1→4): la 4→0 NI SE LEE — su nodo
        // origen (la ciudad) está fuera del filtro y su adyacencia no se
        // itera. Ahí está parte del ahorro del subgrafo.
        assert_eq!(p.stats().descartadas, 1);
        // Se iteraron las adyacencias de los 2 nodos admitidos: 2 lecturas.
        assert_eq!(p.stats().edges_scanned, 2);

        // Sólo aristas KNOWS sobre el grafo entero: mismo resultado aquí.
        let p2 =
            ProyeccionPonderada::proyectar(&s, &w(), &FiltroProyeccion::tipos_arista(["KNOWS"]))
                .unwrap();
        assert_eq!(p2.num_nodos(), 3);
        assert_eq!(p2.num_aristas(), 1);
        assert_eq!(p2.stats().descartadas, 2);

        // Combinado (builder): personas Y sólo KNOWS.
        let filtro = FiltroProyeccion::todo()
            .con_labels(["Persona"])
            .con_tipos_arista(["KNOWS"]);
        let p3 = ProyeccionPonderada::proyectar(&s, &w(), &filtro).unwrap();
        assert_eq!(p3.num_nodos(), 2);
        assert_eq!(p3.num_aristas(), 1);

        // Sin filtros: todo entra (3 nodos, 3 aristas).
        let p4 = proj(&s);
        assert_eq!(p4.num_nodos(), 3);
        assert_eq!(p4.num_aristas(), 3);
        assert_eq!(p4.stats().descartadas, 0);
    }

    #[test]
    fn proyeccion_vacia_y_sin_aristas() {
        let p = proj(&MemoryStore::new());
        assert_eq!(p.num_nodos(), 0);
        assert_eq!(p.num_aristas(), 0);
        assert_eq!(p.stats(), ProyeccionStats::default());

        // Sin aristas pero con nodos: offsets = [0; n+1].
        let s = grafo(3, &[]);
        let p2 = proj(&s);
        assert_eq!(p2.num_nodos(), 3);
        assert_eq!(p2.num_aristas(), 0);
        for i in 0..3 {
            assert_eq!(p2.grado_out(i), 0);
        }
    }

    #[test]
    fn proyeccion_determinista_dos_ejecuciones_identicas() {
        let s = grafo(
            5,
            &[
                (0, 0, 1, 2.0),
                (1, 0, 1, 1.0),
                (2, 1, 2, 1.0),
                (3, 3, 3, 1.0),
            ],
        );
        let a = proj(&s);
        let b = proj(&s);
        assert_eq!(a, b); // PartialEq estructural: MISMA proyección
    }

    #[test]
    fn bloques_de_nodos_reparto_y_ultimo_parcial() {
        let s = grafo(10, &[(0, 0, 1, 1.0)]);
        let p = proj(&s);
        let bloques = p.bloques_de_nodos(4).unwrap();
        assert_eq!(
            bloques,
            vec![0..4, 4..8, 8..10] // el último bloque es más corto
        );
        // Cada bloque es un rango válido de posiciones y cubren todo.
        let cubierto: usize = bloques.iter().map(|r| r.len()).sum();
        assert_eq!(cubierto, p.num_nodos());

        // tam 0 → error tipado; proyección vacía → sin bloques.
        assert_eq!(
            p.bloques_de_nodos(0),
            Err(ProyeccionError::BloqueInvalido { tam: 0 })
        );
        let vacia = proj(&MemoryStore::new());
        assert!(vacia.bloques_de_nodos(4).unwrap().is_empty());
    }

    // ════════════════════════════════════════════════════════════════
    //  Dijkstra sobre la proyección: LA DEUDA DEL CAP 22
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn dijkstra_proyeccion_coincide_con_dijkstra_store() {
        // La deuda documentada en el banner del cap 22: la proyección con
        // pesos debe dar EL MISMO resultado que el algoritmo sobre el store.
        // Grafo con paralelas, self-loop, componente inalcanzable.
        let s = grafo(
            6,
            &[
                (0, 0, 1, 7.0),
                (1, 0, 1, 3.0), // paralela más barata
                (2, 1, 2, 1.0),
                (3, 0, 0, 5.0), // self-loop (nunca ayuda)
                (4, 2, 1, 0.5),
                (5, 4, 5, 1.0), // componente aparte
            ],
        );
        let p = proj(&s);
        let dp = dijkstra_proyeccion(&p, 0).unwrap();
        let ds = dijkstra_store(&s, 0, &w()).unwrap();

        for id in 0..6 {
            assert_eq!(
                dp.distancia(id),
                ds.distance(id),
                "dist({id}) difiere entre proyección y store"
            );
        }
        // Mismo camino punto a punto (nodos Y aristas: la paralela barata).
        let camino_p = dp.camino_a(2).unwrap();
        let camino_s = ds.path_to(2).unwrap();
        assert_eq!(camino_p.nodes(), camino_s.nodes());
        assert_eq!(camino_p.cost, camino_s.cost);
        assert_eq!(camino_p.steps[0].edge, 1); // la arista 1 (peso 3), no la 0
        // La componente {4,5} es inalcanzable en ambas.
        assert_eq!(dp.alcanzados(), vec![0, 1, 2]);
        assert_eq!(dp.alcanzados(), ds.reached());
    }

    #[test]
    fn dijkstra_proyeccion_camino_con_ids_reales_y_valido() {
        let s = grafo(
            5,
            &[
                (0, 0, 2, 3.0),
                (1, 2, 3, 4.0),
                (2, 3, 1, 2.0),
                (3, 1, 4, 1.0),
            ],
        );
        let p = proj(&s);
        let d = dijkstra_proyeccion(&p, 0).unwrap();
        let path = d.camino_a(4).unwrap();
        assert_eq!(path.nodes(), vec![0, 2, 3, 1, 4]);
        assert_eq!(path.cost, 10.0);

        // Validación del camino contra el STORE (como el helper del cap 22):
        // continuidad, aristas reales apuntando donde dicen, coste = suma.
        let mut actual = path.origin;
        let mut suma = 0.0;
        for paso in &path.steps {
            let edge = s.get_edge(paso.edge).unwrap();
            assert_eq!(edge.source, paso.from);
            assert_eq!(edge.target, paso.to);
            assert_eq!(edge.source, actual);
            assert_eq!(paso.weight, edge_weight(edge, &w()).unwrap());
            suma += paso.weight;
            actual = paso.to;
        }
        assert_eq!(actual, 4);
        assert_eq!(suma, path.cost);
    }

    #[test]
    fn dijkstra_proyeccion_casos_borde() {
        let s = grafo(3, &[(0, 0, 1, 1.0)]);
        let p = proj(&s);

        // Origen inexistente (o hueco): UnknownNode.
        assert_eq!(
            dijkstra_proyeccion(&p, 9),
            Err(ProyeccionError::UnknownNode(9))
        );
        // Origen = destino: coste 0, camino vacío.
        let d = dijkstra_proyeccion(&p, 2).unwrap();
        let path = d.camino_a(2).unwrap();
        assert_eq!(path.cost, 0.0);
        assert!(path.steps.is_empty());
        // Destino inalcanzable desde el origen: None (respuesta, no error).
        let d0 = dijkstra_proyeccion(&p, 0).unwrap();
        assert_eq!(d0.distancia(2), None);
        assert_eq!(d0.camino_a(2), None);
        // Stats coherentes (heredan el significado del cap 22).
        assert!(d0.stats.relax_updates <= d0.stats.relax_attempts);
        assert!(d0.stats.expanded <= d0.stats.popped);

        // CostOverflow tipado (1e308 + 1e308 desborda): mismo error que cap22.
        let s2 = grafo(3, &[(0, 0, 1, 1e308), (1, 1, 2, 1e308)]);
        let p2 = proj(&s2);
        assert!(dijkstra_proyeccion(&p2, 0).is_err());
        // ...y sobre el store también (contraste).
        assert!(dijkstra_store(&s2, 0, &w()).is_err());
    }

    #[test]
    fn proyeccion_y_dijkstra_en_demo_graph() {
        // Contando saltos (Constant): la proyección fotografía el grafo demo
        // completo; Dijkstra sobre ella replica al del cap 22.
        let s = crate::cap20_volcano::demo_graph();
        let p =
            ProyeccionPonderada::proyectar(&s, &WeightSource::default(), &FiltroProyeccion::todo())
                .unwrap();
        assert_eq!(p.num_nodos(), 6);
        assert_eq!(p.num_aristas(), 6); // KNOWS 0→1,1→2,2→0,self 3→3 + LIVES_IN ×2

        let dp = dijkstra_proyeccion(&p, 0).unwrap();
        let ds = dijkstra_store(&s, 0, &WeightSource::default()).unwrap();
        for id in 0..6 {
            assert_eq!(dp.distancia(id), ds.distance(id));
        }
        assert_eq!(dp.alcanzados(), ds.reached()); // Dani (3) inalcanzable

        // Con pesos reales ("since"): la sanidad estricta heredada del cap 22
        // reporta la PRIMERA arista sin el dato. La proyección itera nodos
        // en orden ascendente: la adyacencia de Ana (nodo 0) contiene e0
        // (KNOWS, con since) y e4 (LIVES_IN, SIN since) — la primera que
        // falla es la LIVES_IN de Ana (edge 4), no el self-loop de Dani.
        assert_eq!(
            ProyeccionPonderada::proyectar(
                &s,
                &WeightSource::property("since"),
                &FiltroProyeccion::todo()
            ),
            Err(ProyeccionError::Weight(PathError::MissingWeight {
                edge: 4,
                prop: "since".into()
            }))
        );
    }

    #[test]
    fn economia_multiorigen_una_lectura_por_adelantado() {
        // EL argumento económico del capítulo, medido con el voltímetro:
        // materializar (E lecturas) + K Dijkstras = E lecturas del store;
        // K Dijkstras directos sobre el store ≥ K·E (cada llamada del cap 22
        // re-valida TODAS las aristas eager).
        let n = 12;
        let aristas: Vec<(EdgeId, NodeId, NodeId, f64)> =
            (0..n - 1).map(|i| (i, i, i + 1, 1.0 + i as f64)).collect();
        let s = grafo(n, &aristas);
        let total = s.edge_count() as u64;

        let contando = ContandoStore::new(&s);
        let p = ProyeccionPonderada::proyectar(&contando, &w(), &FiltroProyeccion::todo()).unwrap();
        assert_eq!(contando.lecturas_arista(), total); // UNA pasada
        for origen in 0..5u64 {
            dijkstra_proyeccion(&p, origen as NodeId).unwrap();
        }
        // K Dijkstras después: SIGUE siendo exactamente E lecturas.
        assert_eq!(contando.lecturas_arista(), total);

        // Contraste: 5 Dijkstras del cap 22 sobre el store re-leen las
        // aristas CADA VEZ (la validación eager usa iter_edges, que el
        // voltímetro no cuenta como get_edge; las relajaciones sí). En la
        // cadena de 12, el origen i expande los nodos i..11 → lee 11-i
        // aristas: 11+10+9+8+7 = 45 lecturas, CUATRO veces las de la vía
        // proyección (11) — y la brecha crece con cada origen extra.
        let contando2 = ContandoStore::new(&s);
        for origen in 0..5u64 {
            dijkstra_store(&contando2, origen as NodeId, &w()).unwrap();
        }
        assert_eq!(contando2.lecturas_arista(), 45);
        assert!(contando2.lecturas_arista() > contando.lecturas_arista());
    }

    #[test]
    fn closeness_ponderado_paga_la_deuda_del_cap24() {
        // Cadena dirigida 0→1→2→3 con pesos 1, 5, 1.
        let s = grafo(4, &[(0, 0, 1, 1.0), (1, 1, 2, 5.0), (2, 2, 3, 1.0)]);

        // CONSENSO con el cap 24: pesos Constant(1.0) ≡ closeness por
        // saltos de `closeness_centrality(Out)` sobre el MISMO grafo.
        let pu = ProyeccionPonderada::proyectar(
            &s,
            &WeightSource::Constant(1.0),
            &FiltroProyeccion::todo(),
        )
        .unwrap();
        let cu = closeness_ponderado(&pu).unwrap();
        let saltos = crate::cap24_centralidad::closeness_centrality(
            &s,
            crate::cap24_centralidad::GraphDirection::Out,
        )
        .unwrap();
        for &(n, score) in &cu {
            assert!(
                cerca(score, saltos.score(n).unwrap(), EPS),
                "closeness constante ≠ saltos en n{n}: {score}"
            );
        }
        // Valores de libro (saltos): 0 → 3/6, 1 → 4/9, 2 → 1/3, 3 → 0.
        assert!(cerca(cu[0].1, 3.0 / 6.0, EPS));
        assert!(cerca(cu[1].1, 4.0 / 9.0, EPS));
        assert!(cerca(cu[2].1, 1.0 / 3.0, EPS));
        assert_eq!(cu[3].1, 0.0);

        // PONDERADO (la deuda): a mano, Wasserman-Faust con d ponderado —
        //  0: Σd = 1+6+7 = 14, r=4 → (3/3)(3/14) = 3/14
        //  1: Σd = 5+6 = 11,    r=3 → (2/3)(2/11) = 4/33
        //  2: Σd = 1,           r=2 → (1/3)(1)    = 1/3 (su mundo no toca
        //  la arista cara: igual que a saltos)   3: Σd = 0 → 0.
        let c = closeness_ponderado(&proj(&s)).unwrap();
        assert!(cerca(c[0].1, 3.0 / 14.0, EPS));
        assert!(cerca(c[1].1, 4.0 / 33.0, EPS));
        assert!(cerca(c[2].1, 1.0 / 3.0, EPS));
        assert_eq!(c[3].1, 0.0);
        // La arista cara DEVALÚA a quien la cruzaría (0 y 1); el 2 ni se
        // entera — la fuente de pesos cambia la respuesta (lección cap 22).
        assert!(c[0].1 < cu[0].1);
        assert!(c[1].1 < cu[1].1);
        assert!(cerca(c[2].1, cu[2].1, EPS));
        // En orden de id.
        assert_eq!(
            c.iter().map(|&(n, _)| n).collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );

        // La economía del capítulo: V Dijkstras sin UNA lectura del store.
        let mut sim = MemoryStore::new();
        for i in 0..4 {
            sim.put_node(Node::new(i, "N")).unwrap();
        }
        for &(eid, a, b, w) in &[
            (0usize, 0usize, 1usize, 1.0),
            (1, 1, 2, 5.0),
            (2, 2, 3, 1.0),
        ] {
            sim.put_edge(Edge::new(eid, a, b, "R").with_prop("weight", Value::Float(w)))
                .unwrap();
            sim.put_edge(Edge::new(eid + 10, b, a, "R").with_prop("weight", Value::Float(w)))
                .unwrap();
        }
        let contando = ContandoStore::new(&sim);
        let pc =
            ProyeccionPonderada::proyectar(&contando, &w(), &FiltroProyeccion::todo()).unwrap();
        let lecturas_tras_proyeccion = contando.lecturas_arista();
        closeness_ponderado(&pc).unwrap();
        assert_eq!(contando.lecturas_arista(), lecturas_tras_proyeccion);

        // Pesos negativos: rechazo eager de la política del cap 22.
        let sneg = grafo(3, &[(0, 0, 1, -1.0), (1, 1, 2, 1.0)]);
        assert_eq!(
            closeness_ponderado(&proj(&sneg)),
            Err(ProyeccionError::NegativeWeight {
                edge: 0,
                weight: -1.0
            })
        );

        // Casos degenerados: vacío y n ≤ 1 sin pánico.
        assert!(
            closeness_ponderado(&proj(&MemoryStore::new()))
                .unwrap()
                .is_empty()
        );
        let uno = grafo(1, &[]);
        assert_eq!(closeness_ponderado(&proj(&uno)).unwrap(), vec![(0, 0.0)]);
    }

    // ════════════════════════════════════════════════════════════════
    //  Streaming por fronteras: presupuestos y lecturas medidas
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn bfs_streaming_niveles_ordenados_y_deterministas() {
        // Diamante + ramal: 0→{1,2}, 1→3, 2→3, 3→4.
        let s = grafo(
            5,
            &[
                (0, 0, 1, 1.0),
                (1, 0, 2, 1.0),
                (2, 1, 3, 1.0),
                (3, 2, 3, 1.0),
                (4, 3, 4, 1.0),
            ],
        );
        let r = bfs_streaming(&s, 0, GraphDirection::Out, Presupuesto::sin_limite()).unwrap();
        assert_eq!(r.niveles, vec![vec![0], vec![1, 2], vec![3], vec![4]]);
        assert_eq!(r.parada, MotivoParada::Completo);
        assert_eq!(r.profundidad(), 3);
        assert_eq!(r.nodos(), vec![0, 1, 2, 3, 4]);
        // Dos ejecuciones: idénticas (determinismo).
        let r2 = bfs_streaming(&s, 0, GraphDirection::Out, Presupuesto::sin_limite()).unwrap();
        assert_eq!(r, r2);
    }

    #[test]
    fn bfs_streaming_no_lee_todo_el_grafo() {
        // LA tesis del capítulo en un test: cadena de 500 nodos (499
        // aristas); BFS con profundidad 2 desde el 0 lee 3 aristas y
        // visita 3 nodos. El resto del grafo NO EXISTIÓ para la consulta.
        let n = 500;
        let aristas: Vec<(EdgeId, NodeId, NodeId, f64)> =
            (0..n - 1).map(|i| (i, i, i + 1, 1.0)).collect();
        let s = grafo(n, &aristas);

        let contando = ContandoStore::new(&s);
        let r = bfs_streaming(
            &contando,
            0,
            GraphDirection::Out,
            Presupuesto::profundidad(2),
        )
        .unwrap();

        assert_eq!(r.niveles, vec![vec![0], vec![1], vec![2]]);
        assert_eq!(r.parada, MotivoParada::ProfundidadMaxima);
        // Presupuesto cumplido con exactitud. Con profundidad 2 sólo se
        // EXPANDEN los nodos 0 y 1 (para descubrir el 2 basta leer la
        // arista de 1; el 2 no se expande porque su frontera ya excede el
        // presupuesto): 2 consultas de adyacencia, 2 aristas leídas.
        assert_eq!(r.stats.nodos_visitados, 3);
        assert_eq!(r.stats.aristas_leidas, 2); // out_edges de 0 y 1
        assert_eq!(r.stats.adyacencia_consultas, 2);
        assert_eq!(r.stats.fronteras, 3);
        // Verificación EXTERNA (el voltímetro confirma el auto-informe):
        assert_eq!(contando.lecturas_arista(), 2);
        assert_eq!(contando.consultas_out(), 2);
        // ...y el contraste: 2 de 499 aristas = menos del 0,5%.
        assert!(r.stats.aristas_leidas < contando.edge_count() as u64 / 100);
    }

    #[test]
    fn bfs_streaming_completo_solo_toca_la_componente_alcanzable() {
        // Dos cadenas: 0→1→2 y 100→101→102 (ids dispersos via delete? no:
        // simplemente nodos 0..5 con dos componentes).
        let s = grafo(
            6,
            &[
                (0, 0, 1, 1.0),
                (1, 1, 2, 1.0),
                (2, 3, 4, 1.0),
                (3, 4, 5, 1.0),
            ],
        );
        let contando = ContandoStore::new(&s);
        let r =
            bfs_streaming(&contando, 0, GraphDirection::Out, Presupuesto::sin_limite()).unwrap();
        assert_eq!(r.nodos(), vec![0, 1, 2]);
        assert_eq!(r.parada, MotivoParada::Completo);
        // La otra componente no se leyó NI se visitó:
        assert_eq!(contando.lecturas_arista(), 2);
        assert_eq!(contando.consultas_out(), 3); // 0, 1 y 2 (el 2 no lleva a nadie nuevo)
        // Desde el 3: simétrico.
        let r2 = bfs_streaming(&s, 3, GraphDirection::Out, Presupuesto::sin_limite()).unwrap();
        assert_eq!(r2.nodos(), vec![3, 4, 5]);
    }

    #[test]
    fn bfs_streaming_presupuesto_nodos_exacto() {
        // Estrella 0→{1,2,3,4}: presupuesto de 3 nodos → origen + 2 hojas.
        let s = grafo(
            5,
            &[
                (0, 0, 1, 1.0),
                (1, 0, 2, 1.0),
                (2, 0, 3, 1.0),
                (3, 0, 4, 1.0),
            ],
        );
        let r = bfs_streaming(
            &s,
            0,
            GraphDirection::Out,
            Presupuesto::sin_limite().con_nodos(3),
        )
        .unwrap();
        assert_eq!(r.stats.nodos_visitados, 3);
        assert_eq!(r.nodos(), vec![0, 1, 2]); // descubiertos en orden de id
        assert_eq!(r.parada, MotivoParada::PresupuestoNodos);
        // El límite también corta en cuanto el nivel 0 basta: presupuesto 1.
        let r1 = bfs_streaming(
            &s,
            0,
            GraphDirection::Out,
            Presupuesto::sin_limite().con_nodos(1),
        )
        .unwrap();
        assert_eq!(r1.niveles, vec![vec![0]]);
        assert_eq!(r1.parada, MotivoParada::PresupuestoNodos);
        // Un nodo aislado con presupuesto 1: COMPLETO (no había nada más).
        let solo = grafo(2, &[]);
        let rc = bfs_streaming(
            &solo,
            1,
            GraphDirection::Out,
            Presupuesto::sin_limite().con_nodos(1),
        )
        .unwrap();
        assert_eq!(rc.parada, MotivoParada::Completo);
    }

    #[test]
    fn bfs_streaming_presupuesto_lecturas_exacto() {
        // Cadena 0→1→2→3→4: 2 lecturas llegan hasta descubrir el 2 (leyó
        // las aristas de 0 y de 1), y para ANTES de expandir el 2.
        let s = grafo(
            5,
            &[
                (0, 0, 1, 1.0),
                (1, 1, 2, 1.0),
                (2, 2, 3, 1.0),
                (3, 3, 4, 1.0),
            ],
        );
        let contando = ContandoStore::new(&s);
        let r = bfs_streaming(
            &contando,
            0,
            GraphDirection::Out,
            Presupuesto::sin_limite().con_lecturas(2),
        )
        .unwrap();
        assert_eq!(r.stats.aristas_leidas, 2); // EXACTO
        assert_eq!(contando.lecturas_arista(), 2);
        assert_eq!(r.parada, MotivoParada::PresupuestoLecturas);
        assert_eq!(r.nodos(), vec![0, 1, 2]); // lo descubierto antes del corte
    }

    #[test]
    fn bfs_iterador_perezoso_una_frontera() {
        // El iterador es de verdad perezoso: pedir 2 fronteras lee sólo lo
        // necesario y soltarlo deja el resto del grafo intacto.
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
        let contando = ContandoStore::new(&s);
        let mut it =
            bfs_fronteras(&contando, 0, GraphDirection::Out, Presupuesto::sin_limite()).unwrap();
        assert_eq!(it.parada(), None);
        assert_eq!(it.next(), Some(vec![0]));
        assert_eq!(it.next(), Some(vec![1]));
        // Soltamos el iterador sin agotarlo: sólo se expandió la frontera
        // [0] (la adyacencia de 1 se consulta al EXPANDIR, nunca antes).
        assert_eq!(contando.lecturas_arista(), 1); // sólo la arista de 0
        assert_eq!(contando.consultas_out(), 1);
        // stats accesibles en caliente:
        assert_eq!(it.stats().nodos_visitados, 2);
        assert_eq!(it.stats().fronteras, 2);
    }

    #[test]
    fn bfs_direcciones_out_in_y_both() {
        // Cadena 0→1→2: cada dirección cuenta una historia distinta.
        let s = grafo(3, &[(0, 0, 1, 1.0), (1, 1, 2, 1.0)]);

        let out = bfs_streaming(&s, 2, GraphDirection::Out, Presupuesto::sin_limite()).unwrap();
        assert_eq!(out.niveles, vec![vec![2]]); // 2 no apunta a nadie
        assert_eq!(out.parada, MotivoParada::Completo);

        // In: desde 2 se alcanza toda la cadena hacia atrás (sin transponer
        // nada: las in_edges se leen bajo demanda — ventaja del streaming).
        let inn = bfs_streaming(&s, 2, GraphDirection::In, Presupuesto::sin_limite()).unwrap();
        assert_eq!(inn.niveles, vec![vec![2], vec![1], vec![0]]);
        assert_eq!(inn.stats.aristas_leidas, 2); // in_edges de 2 y de 1

        // Both: desde 1, los dos lados.
        let both = bfs_streaming(&s, 1, GraphDirection::Both, Presupuesto::sin_limite()).unwrap();
        assert_eq!(both.niveles, vec![vec![1], vec![0, 2]]);
        assert_eq!(both.stats.aristas_leidas, 4); // out(1) + in(1) + out(0) + in(2)... se miden ambas adyacencias
    }

    #[test]
    fn bfs_casos_borde_errores_y_vacio() {
        // Grafo vacío: origen desconocido.
        assert_eq!(
            bfs_streaming(
                &MemoryStore::new(),
                0,
                GraphDirection::Out,
                Presupuesto::sin_limite()
            ),
            Err(ProyeccionError::UnknownNode(0))
        );

        let s = grafo(3, &[(0, 1, 1, 1.0)]); // self-loop en 1; 0 y 2 aislados

        // Origen aislado: una frontera, cero lecturas, COMPLETO.
        let r = bfs_streaming(&s, 0, GraphDirection::Out, Presupuesto::sin_limite()).unwrap();
        assert_eq!(r.niveles, vec![vec![0]]);
        assert_eq!(r.stats.aristas_leidas, 0);
        assert_eq!(r.stats.adyacencia_consultas, 1);
        assert_eq!(r.parada, MotivoParada::Completo);

        // Self-loop: no re-visita (el bitset lo bloquea).
        let r2 = bfs_streaming(&s, 1, GraphDirection::Out, Presupuesto::sin_limite()).unwrap();
        assert_eq!(r2.niveles, vec![vec![1]]);
        assert_eq!(r2.stats.aristas_leidas, 1);
        assert_eq!(r2.parada, MotivoParada::Completo);

        // Presupuestos inválidos: rechazo tipado.
        assert_eq!(
            bfs_streaming(
                &s,
                0,
                GraphDirection::Out,
                Presupuesto::sin_limite().con_nodos(0)
            ),
            Err(ProyeccionError::PresupuestoInvalido {
                campo: "max_nodos",
                valor: 0
            })
        );
        assert_eq!(
            bfs_streaming(
                &s,
                0,
                GraphDirection::Out,
                Presupuesto::sin_limite().con_lecturas(0)
            ),
            Err(ProyeccionError::PresupuestoInvalido {
                campo: "max_lecturas",
                valor: 0
            })
        );

        // Profundidad 0: sólo el origen (frontera única) y parada por
        // profundidad — distinta del caso "aislado" (que fue Completo).
        let r3 = bfs_streaming(&s, 1, GraphDirection::Out, Presupuesto::profundidad(0)).unwrap();
        assert_eq!(r3.niveles, vec![vec![1]]);
        assert_eq!(r3.stats.aristas_leidas, 0);
        assert_eq!(r3.parada, MotivoParada::ProfundidadMaxima);
    }

    #[test]
    fn bfs_en_demo_graph_alcance_y_contraste_con_dijkstra() {
        // demo_graph desde Ana (Out): nivel 1 = {Bo, Madrid}; nivel 2 =
        // {Carla, Lisboa}; Dani inalcanzable (sólo su self-loop).
        let s = crate::cap20_volcano::demo_graph();
        let r = bfs_streaming(&s, 0, GraphDirection::Out, Presupuesto::sin_limite()).unwrap();
        assert_eq!(r.niveles, vec![vec![0], vec![1, 4], vec![2, 5]]);
        assert_eq!(r.parada, MotivoParada::Completo);

        // Contraste con la proyección: el alcanzado del BFS == los
        // alcanzados de Dijkstra sobre la proyección (misma componente).
        let p =
            ProyeccionPonderada::proyectar(&s, &WeightSource::default(), &FiltroProyeccion::todo())
                .unwrap();
        let d = dijkstra_proyeccion(&p, 0).unwrap();
        let mut bfs_nodos = r.nodos();
        bfs_nodos.sort_unstable();
        assert_eq!(bfs_nodos, d.alcanzados());
    }

    // ════════════════════════════════════════════════════════════════
    //  Errores y utilidades
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn errores_display_y_std_error() {
        let errs = vec![
            ProyeccionError::Weight(PathError::MissingWeight {
                edge: 2,
                prop: "w".into(),
            }),
            ProyeccionError::NegativeWeight {
                edge: 5,
                weight: -1.5,
            },
            ProyeccionError::UnknownNode(9),
            ProyeccionError::PresupuestoInvalido {
                campo: "max_nodos",
                valor: 0,
            },
            ProyeccionError::BloqueInvalido { tam: 0 },
        ];
        for e in &errs {
            assert!(!e.to_string().is_empty());
            let _: &dyn std::error::Error = e;
        }
        assert!(errs[1].to_string().contains("bellman_ford"));
        assert!(errs[3].to_string().contains("max_nodos"));
    }

    #[test]
    fn contando_store_mide_y_delega_sin_perder_datos() {
        let s = grafo(3, &[(0, 0, 1, 1.0), (1, 1, 2, 1.0)]);
        let c = ContandoStore::new(&s);

        // Delegación fiel:
        assert_eq!(c.node_count(), 3);
        assert_eq!(c.edge_count(), 2);
        assert_eq!(c.out_edges(0), vec![0]);
        assert_eq!(c.in_edges(2), vec![1]);
        assert!(c.get_node(0).is_some());
        assert!(c.get_edge(0).is_some());
        assert_eq!(c.iter_nodes().count(), 3);
        assert_eq!(c.iter_edges().count(), 2);

        // Medición:
        assert_eq!(c.lecturas_nodo(), 1);
        assert_eq!(c.lecturas_arista(), 1);
        assert_eq!(c.consultas_out(), 1);
        assert_eq!(c.consultas_in(), 1);
        assert_eq!(c.total_lecturas(), 4);

        // La proyección a través del voltímetro cuenta sus E lecturas
        // (una pasada por las adyacencias de los nodos vivos).
        let c2 = ContandoStore::new(&s);
        let p = ProyeccionPonderada::proyectar(&c2, &w(), &FiltroProyeccion::todo()).unwrap();
        assert_eq!(p.stats().edges_scanned, 2);
        assert_eq!(c2.lecturas_arista(), 2);
        assert_eq!(c2.consultas_out(), 3); // un out_edges por nodo vivo
    }
}
