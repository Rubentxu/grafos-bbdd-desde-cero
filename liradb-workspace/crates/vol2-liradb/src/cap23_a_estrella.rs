use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::cap07_modelo::{NodeId, Value};
use crate::cap08_graph_store::GraphStore;
use crate::cap22_caminos_minimos::{
    Cost, Path, PathError, PathStats, PathStep, WeightSource, edge_weight, ensure_node, table_len,
    validate_edge_weights,
};

// ─────────────────── Cap 23: A*, heurísticas y búsquedas dirigidas ───────────────────
//
// Continúa la Parte V (algoritmos sobre el grafo persistente). El Vol.I ya
// enseñó A* ALGORÍTMICAMENTE (cap 9, sobre su grafo `Vec<Vec<...>>` en
// memoria; el cap 29 del Vol.I lo usaba en robótica y videojuegos). El ángulo
// del Vol.II es distinto en los dos ejes que importan:
//
//   1. A* corre SOBRE el grafo del store (`&dyn GraphStore`), reutilizando la
//      maquinaria del cap 22 tal cual: [`WeightSource`] / [`edge_weight`] con
//      su semántica estricta, la sanidad eager de pesos (ahora compartida en
//      `validate_edge_weights`), [`PathError`], [`Path`] y [`PathStats`].
//   2. La heurística la aporta el USUARIO de la API (trait [`Heuristic`]),
//      no viene horneada en el algoritmo: coordenadas euclídeas guardadas
//      como props `x`/`y` de los NODOS ([`EuclideanHeuristic`]), la
//      heurística nula que degenera en Dijkstra ([`ZeroHeuristic`]), o
//      cualquier otra que el caller implemente (por grados, por landmarks —
//      ver los tests con heurísticas ad-hoc). Es la PRIMERA VEZ que un
//      algoritmo de la Parte V necesita datos del NODO, no sólo de la arista:
//      por eso [`Heuristic::estimate`] recibe el store.
//
// La idea central: Dijkstra (cap 22) explora "en círculo" ordenando por el
// coste ya acumulado g(n); A* ordena el heap por
//
// ```text
//   f(n) = g(n) + h(n)      (coste acumulado + estimación hasta el destino)
// ```
//
// y sesga la exploración HACIA el destino. Con una heurística ADMISIBLE
// (h(n) ≤ coste real de n al destino: nunca sobre-estima) el primer pop vivo
// del destino tiene g óptimo — mismo estilo de garantía que la finalización
// anticipada de Dijkstra, pero pagando menos expansiones (medible en
// `PathStats::expanded`, añadido por este capítulo).
//
// Piezas del capítulo (brief §cap 23 + CORPUS "Heurísticas admisibles y
// consistentes"):
//
//   1. **Trait [`Heuristic`]**: el contrato `h(n) -> f64` con acceso al store.
//   2. **A\* punto-a-punto** ([`a_star`]): heap por f con re-apertura de nodos.
//   3. **Validación honesta** (ver "Decisiones" abajo): eager lo barato,
//      documentado+testeado lo caro, utilidad [`check_consistency`] para lo
//      localmente verificable.
//   4. **Comparativa Dijkstra vs A\*** en los tests: mismo coste, menos nodos
//      expandidos (el hito del brief: rutas sobre una red de ciudades).
//
// Decisiones de diseño (los porqués):
//
// * **¿Trait o closure para h?** Trait. Tres razones: (a) las heurísticas son
//   FAMILIAS con estado — la euclídea recuerda el destino y los nombres de
//   las props; una de landmarks pre-calcularía distancias a hitos — y un
//   trait con campos lo expresa naturalmente; (b) el contrato (finita, ≥ 0,
//   y h(destino)=0 si quiere admisibilidad) se documenta e valida EN UN SITIO
//   ([`a_star`] revisa cada estimación que usa) en vez de repartirse por cada
//   caller; (c) `&dyn Heuristic` mantiene la firma de `a_star` sin genéricos
//   que contagien. Una closure `Fn(NodeId) -> f64` cubriría el caso ad-hoc
//   pero NO PODRÍA leer el store sin capturarlo — y capturar el `&dyn
//   GraphStore` prestado bloquea pasarle otro store. Los tests muestran que
//   un trait también se implementa en tres líneas para casos ad-hoc (ver
//   `Fija` en los tests: la tabla h por nodo de los casos patológicos).
//
// * **¿Qué se valida eager y qué no? (la decisión honesta del capítulo)** El
//   cap 22 validó eager los pesos negativos porque era O(E) y saltarse esa
//   validación producía respuestas MENTIRA. Aquí la escala de costes manda:
//   - **Pesos** (herencia cap 22): eager O(E), literalmente la misma función
//     `validate_edge_weights` — A* comparte el invariante codicioso de
//     Dijkstra y exige no-negativos por las mismas razones.
//   - **h finita y h ≥ 0**: se revisa EN CADA estimación (cacheada: ≤ 1
//     consulta por nodo). No es negociable: un NaN rompería el orden total
//     de `Cost` y haría PANIC dentro del heap; un valor negativo casi siempre
//     es un bug del caller, y además el criterio de parada necesita
//     h(dest)=0 — que se SIGUE de admisibilidad + no-negatividad.
//     [`PathError::NonFiniteHeuristic`] / [`PathError::NegativeHeuristic`].
//   - **Admisibilidad** (h ≤ coste real): NO verificable sin resolver el
//     problema — el coste real de cada n al destino ES un Dijkstra completo.
//     Verificarla eager costaría más que el propio A*. Se DOCUMENTA el
//     contrato, se DEMUESTRA el riesgo (test con h sobre-estimada → camino
//     subóptimo SIN error: A* responde plausible-pero-malo, exactamente lo
//     que el brief llama "cuándo A* no ayuda") y la euclídea lo garantiza POR
//     CONSTRUCCIÓN cuando los pesos son distancias en las unidades de las
//     coordenadas (una carretera nunca es más corta que la línea recta; mezclar
//     unidades — km con minutos — la rompe: testeado).
//   - **Consistencia** (h(n) ≤ w(n,n') + h(n') para toda arista): a diferencia
//     de la admisibilidad es LOCAL y verificable en O(E) + ≤2V estimaciones →
//     utilidad [`check_consistency`] para debugging/diagnóstico. PERO A* NO la
//     exige (no es un error de datos y no produce mentiras): con h admisible
//     pero inconsistente nuestra implementación RE-ABRE nodos y sigue
//     devolviendo el camino óptimo, sólo que expande más (testeado y medido
//     en `expanded`). Rechazar una heurística inconsistente sería rechazar
//     respuestas correctas.
//
// * **Re-apertura (re-opening) en vez de `settled`**: Dijkstra puede marcar
//   nodos como definitivos porque h≡0 es trivialmente consistente. Con h
//   inconsistente un nodo ya expandido puede MEJORAR su g después y hay que
//   re-expandirlo para no perder caminos. Detectamos entradas obsoletas del
//   heap comparando el g de la entrada con el g vigente (`g_entrada > g[v]`):
//   las entradas vivas llevan exactamente el g actual.
//
// * **Sólo punto-a-punto** (no hay variante de tabla completa): el sesgo hacia
//   el destino es toda la gracia; las distancias intermedias que A* va
//   fijando NO están garantizadas (con h inconsistente ni las de los nodos
//   tocados). `dijkstra` del cap 22 sigue siendo la herramienta single-source.
//
// * **f infinita permitida**: f = g + h es sólo una PRIORIDAD; si desborda a
//   ∞ el orden sigue siendo total (±∞ no rompe `Cost`; sólo NaN, ya revisado).
//   El g del camino sí se valida finito ([`PathError::CostOverflow`],
//   herencia cap 22) para que el centinela INFINITY de "inalcanzable" nunca
//   se confunda con un coste real.

// ─── El contrato de la heurística ───

/// La estimación h(n): lo que el CALLER sabe del problema que el grafo no
/// dice — "¿cuánto queda para llegar al destino?".
///
/// Contrato (lo que [`a_star`] revisa en cada llamada):
/// - el resultado debe ser FINITO (un NaN rompería el orden del heap) y
///   NO NEGATIVO;
/// - si quiere ADMISIBILIDAD (h(n) ≤ coste real hasta el destino), es cosa
///   del implementador: no hay forma barata de verificarla desde aquí (ver
///   [`check_consistency`] para la propiedad hermana, la consistencia, que sí
///   es local).
///
/// El destino se liga AL CONSTRUIR la heurística (una heurística apunta a UN
/// destino: por eso `estimate` no lo recibe). Recibe el `store` porque las
/// heurísticas interesantes leen PROPIEDADES DE NODO — coordenadas, grados,
/// tablas precalculadas.
///
/// Implementaciones del capítulo: [`ZeroHeuristic`] (h≡0 = Dijkstra) y
/// [`EuclideanHeuristic`] (distancia recta por props x/y). Los tests añaden
/// implementaciones ad-hoc (incluida una admisible-no-consistente y una
/// sobre-estimada) para demostrar que el trait está abierto.
pub trait Heuristic {
    /// Estima el coste de ir de `node` al destino fijado en construcción.
    fn estimate(&self, store: &dyn GraphStore, node: NodeId) -> Result<f64, PathError>;
}

/// h(n) ≡ 0: admisible, consistente… y completamente inútil para dirigir la
/// búsqueda — A* degenera EXACTAMENTE en Dijkstra (mismo orden de pops,
/// testeado). Su valor es doble: como test de consistencia (todo camino que
/// A* con h≡0 encuentre debe coincidir con
/// [`dijkstra_path`](crate::dijkstra_path)) y como línea base para medir
/// cuánto ahorra una heurística real.
///
/// ```
/// use vol2_liradb::{a_star, Edge, GraphStore, MemoryStore, Node, ZeroHeuristic};
///
/// let mut s = MemoryStore::new();
/// s.put_node(Node::new(0, "A")).unwrap();
/// s.put_node(Node::new(1, "B")).unwrap();
/// s.put_edge(Edge::new(0, 0, 1, "R")).unwrap();
/// // Sin pesos en las aristas la fuente por defecto cuenta saltos:
/// let path = a_star(&s, 0, 1, &Default::default(), &ZeroHeuristic)
///     .unwrap()
///     .unwrap();
/// assert_eq!(path.cost, 1.0);
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ZeroHeuristic;

impl Heuristic for ZeroHeuristic {
    fn estimate(&self, _store: &dyn GraphStore, _node: NodeId) -> Result<f64, PathError> {
        Ok(0.0)
    }
}

/// Lee una propiedad de nodo como coordenada, con la MISMA semántica estricta
/// que [`edge_weight`](crate::edge_weight) aplica a las aristas (cap 22):
/// ausente o NULL → [`PathError::MissingCoordinate`], tipo no numérico o Float
/// no finito → [`PathError::InvalidCoordinate`], Int promocionado a Float.
fn node_coord(store: &dyn GraphStore, node: NodeId, prop: &str) -> Result<f64, PathError> {
    let raw = match store.get_node(node) {
        Some(n) => match n.props.get(prop) {
            Some(Value::Int(i)) => *i as f64,
            Some(Value::Float(f)) => *f,
            Some(Value::Null) | None => {
                return Err(PathError::MissingCoordinate {
                    node,
                    prop: prop.to_string(),
                });
            }
            Some(other) => {
                return Err(PathError::InvalidCoordinate {
                    node,
                    prop: prop.to_string(),
                    found: other.type_name().to_string(),
                });
            }
        },
        None => {
            // Inalcanzable en la práctica: a_star valida los extremos y sólo
            // estima nodos que salen del heap (que existen). Defense in depth.
            return Err(PathError::MissingCoordinate {
                node,
                prop: prop.to_string(),
            });
        }
    };
    if !raw.is_finite() {
        return Err(PathError::InvalidCoordinate {
            node,
            prop: prop.to_string(),
            found: "non-finite Float".to_string(),
        });
    }
    Ok(raw)
}

/// Heurística euclídea: la distancia en línea recta de `node` al destino,
/// leyendo las coordenadas de las props del nodo (p.ej. `x`/`y`).
///
/// Es LA heurística canónica del capítulo (el "uso de coordenadas" del brief)
/// y la del hito (red de ciudades): si los pesos de las aristas son DISTANCIAS
/// en las mismas unidades que las coordenadas, entonces cada arista satisface
/// w(u,v) ≥ dist_recta(u,v) (una carretera nunca es más corta que la línea
/// recta) y de la desigualdad triangular se sigue que h es ADMISIBLE y
/// CONSISTENTE — el caso ideal: camino óptimo, sin re-expansiones. Mezclar
/// unidades (coordenadas en km, pesos en minutos) la rompe: testeado.
///
/// La construcción valida las coordenadas del DESTINO eager (barato: 2 props)
/// y las del resto on-demand al estimar cada nodo que la búsqueda toca: un
/// grafo schemaless delata sus huecos cuando se pisan, no antes (misma
/// filosofía que las props de peso por arista del cap 22).
///
/// ```
/// use vol2_liradb::{a_star, Edge, EuclideanHeuristic, GraphStore, MemoryStore, Node, Value, WeightSource};
///
/// // Tres ciudades en línea: A(0,0) -10-> B(10,0) -10-> C(20,0), y un atajo
/// // caro A -25-> C. La recta A→C son 20, así que la ruta por B (20) gana.
/// let mut s = MemoryStore::new();
/// for (id, x) in [(0, 0.0), (1, 10.0), (2, 20.0)] {
///     s.put_node(
///         Node::new(id, "City")
///             .with_prop("x", Value::Float(x))
///             .with_prop("y", Value::Float(0.0)),
///     )
///     .unwrap();
/// }
/// for (eid, from, to, km) in [(0, 0, 1, 10.0), (1, 1, 2, 10.0), (2, 0, 2, 25.0)] {
///     s.put_edge(Edge::new(eid, from, to, "ROAD").with_prop("km", Value::Float(km)))
///         .unwrap();
/// }
///
/// let h = EuclideanHeuristic::new(&s, 2, "x", "y").unwrap();
/// let path = a_star(&s, 0, 2, &WeightSource::property("km"), &h)
///     .unwrap()
///     .unwrap();
/// assert_eq!(path.nodes(), vec![0, 1, 2]);
/// assert_eq!(path.cost, 20.0);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct EuclideanHeuristic {
    /// Destino al que apunta la heurística (ligado en construcción).
    dest: NodeId,
    dest_x: f64,
    dest_y: f64,
    x_prop: String,
    y_prop: String,
}

impl EuclideanHeuristic {
    /// Construye la heurística hacia `dest`, leyendo y validando SUS
    /// coordenadas ahora (fallar antes de empezar es más barato que fallar
    /// en medio de la búsqueda).
    pub fn new(
        store: &dyn GraphStore,
        dest: NodeId,
        x_prop: impl Into<String>,
        y_prop: impl Into<String>,
    ) -> Result<Self, PathError> {
        let x_prop = x_prop.into();
        let y_prop = y_prop.into();
        let dest_x = node_coord(store, dest, &x_prop)?;
        let dest_y = node_coord(store, dest, &y_prop)?;
        Ok(Self {
            dest,
            dest_x,
            dest_y,
            x_prop,
            y_prop,
        })
    }

    /// El destino al que apunta (para diagnóstico).
    pub fn destination(&self) -> NodeId {
        self.dest
    }
}

impl Heuristic for EuclideanHeuristic {
    fn estimate(&self, store: &dyn GraphStore, node: NodeId) -> Result<f64, PathError> {
        let x = node_coord(store, node, &self.x_prop)?;
        let y = node_coord(store, node, &self.y_prop)?;
        // hypot (std, sin crates): raíz de x²+y² sin desbordes intermedios.
        // Siempre ≥ 0 y finita (coordenadas validadas finitas).
        Ok((x - self.dest_x).hypot(y - self.dest_y))
    }
}

// ─── A*: el algoritmo ───

/// Estima y REVISA el contrato de la heurística para `node` (finita, ≥ 0),
/// con caché: la heurística se consulta a lo sumo UNA vez por nodo (la
/// euclídea reelería las props en cada inserción en el heap). NaN = "todavía
/// no estimada".
fn h_of(
    heuristic: &dyn Heuristic,
    store: &dyn GraphStore,
    cache: &mut [f64],
    node: NodeId,
) -> Result<f64, PathError> {
    if cache[node].is_nan() {
        let v = heuristic.estimate(store, node)?;
        if !v.is_finite() {
            return Err(PathError::NonFiniteHeuristic { node, value: v });
        }
        if v < 0.0 {
            return Err(PathError::NegativeHeuristic { node, value: v });
        }
        cache[node] = v;
    }
    Ok(cache[node])
}

/// A\* punto-a-punto sobre el grafo del store: mismo contrato de datos que
/// [`dijkstra_path`](crate::dijkstra_path) (pesos presentes, numéricos,
/// finitos y NO negativos en TODO el store — la sanidad eager es literalmente
/// la misma función) y misma forma de respuesta (`Ok(None)` = inalcanzable,
/// que es una respuesta, no un error).
///
/// El heap se ordena por **f(n) = g(n) + h(n)**: cuando el destino sale del
/// heap como entrada VIVA, su g es óptimo — con h admisible (y h ≥ 0, que sí
/// se revisa) cualquier camino aún por descubrir pasa por un nodo del heap
/// cuyo f es cota inferior de su coste, y todos eran ≥ f(destino).
///
/// A diferencia de Dijkstra, A* **re-abre nodos**: con una heurística
/// admisible pero INCONSISTENTE un nodo expandido puede mejorar después, y se
/// re-expande (por eso `stats.expanded` puede superar el número de nodos).
/// El resultado sigue siendo óptimo; el coste de la inconsistencia se mide,
/// no se esconde.
///
/// Lo que NO verifica (ver la sección de decisiones del módulo): la
/// ADMISIBILIDAD — h ≤ coste real exigiría resolver Dijkstra hacia el
/// destino. Una heurística sobre-estimadora produce caminos SUBÓPTIMOS sin
/// ningún error (riesgo documentado y demostrado en los tests). Para
/// diagnosticar la propiedad hermana, local y barata, usar
/// [`check_consistency`].
///
/// ```
/// use vol2_liradb::{a_star, Edge, GraphStore, MemoryStore, Node, Value, WeightSource, ZeroHeuristic};
///
/// let mut s = MemoryStore::new();
/// s.put_node(Node::new(0, "A")).unwrap();
/// s.put_node(Node::new(1, "B")).unwrap();
/// s.put_node(Node::new(2, "C")).unwrap();
/// for (eid, from, to, w) in [(0, 0, 1, 1.0), (1, 1, 2, 2.0), (2, 0, 2, 4.5)] {
///     s.put_edge(Edge::new(eid, from, to, "R").with_prop("w", Value::Float(w)))
///         .unwrap();
/// }
/// let peso = WeightSource::property("w");
/// // Con h ≡ 0 esto ES Dijkstra: el camino 0→1→2 (coste 3) gana al directo.
/// let path = a_star(&s, 0, 2, &peso, &ZeroHeuristic).unwrap().unwrap();
/// assert_eq!(path.nodes(), vec![0, 1, 2]);
/// assert_eq!(path.cost, 3.0);
/// ```
pub fn a_star(
    store: &dyn GraphStore,
    origin: NodeId,
    dest: NodeId,
    weight: &WeightSource,
    heuristic: &dyn Heuristic,
) -> Result<Option<Path>, PathError> {
    ensure_node(store, origin)?;
    ensure_node(store, dest)?;

    // Misma sanidad eager que Dijkstra (cap 22): A* comparte su invariante
    // codicioso — f puede mentir si los pesos son negativos.
    validate_edge_weights(store, weight)?;

    let num_nodes = table_len(store);
    let mut g = vec![f64::INFINITY; num_nodes];
    let mut pred: Vec<Option<PathStep>> = vec![None; num_nodes];
    let mut h_cache = vec![f64::NAN; num_nodes];
    let mut stats = PathStats::default();

    // Min-heap por f = g + h. La clave (f, g, nodo) — Cost, no f64: el heap
    // exige Ord y los NaN lo rompen (todo lo que entra ya está validado
    // finito) — ordena por prioridad y desempata por g y luego id
    // (determinismo, mismo estilo que cap 22; con h ≡ 0 la clave degenera en
    // la de Dijkstra y el orden de pops es IDÉNTICO: es el test
    // "heurística cero == Dijkstra").
    let mut heap: BinaryHeap<Reverse<(Cost, Cost, NodeId)>> = BinaryHeap::new();
    let h0 = h_of(heuristic, store, &mut h_cache, origin)?;
    g[origin] = 0.0;
    heap.push(Reverse((Cost(h0), Cost(0.0), origin)));

    while let Some(Reverse((_, Cost(g_u), u))) = heap.pop() {
        stats.popped += 1;
        // Entrada obsoleta: su g ya fue superado (la entrada viva —o una
        // mejor— sigue en el heap). Comparación exacta: ambos valores salen
        // de las mismas sumas en f64.
        if g_u > g[u] {
            continue;
        }
        stats.expanded += 1;
        if u == dest {
            // Criterio de parada: f(dest) = g(dest) + h(dest) era el mínimo
            // del heap, y todo camino por descubrir cuesta ≥ su f ≥ éste.
            // Válido con h admisible (h(dest)=0 se sigue de h ≥ 0 + admisible).
            break;
        }
        for eid in store.out_edges(u) {
            let edge = store
                .get_edge(eid)
                .expect("invariante del store: la adjacencia sólo contiene aristas vivas");
            let w = edge_weight(edge, weight)?; // validado eager; relectura ≤ 1 por arista viva
            stats.relax_attempts += 1;
            let new = g_u + w; // w ≥ 0 por la validación eager
            if !new.is_finite() {
                return Err(PathError::CostOverflow { edge: eid });
            }
            let v = edge.target;
            if new < g[v] {
                g[v] = new;
                pred[v] = Some(PathStep {
                    edge: eid,
                    from: u,
                    to: v,
                    weight: w,
                });
                stats.relax_updates += 1;
                let hv = h_of(heuristic, store, &mut h_cache, v)?;
                // f sólo es PRIORIDAD: puede desbordar a ∞ sin romper el
                // orden (Cost tolera ±∞; el enemigo NaN ya se revisó).
                heap.push(Reverse((Cost(new + hv), Cost(new), v)));
            }
        }
    }

    if !g[dest].is_finite() {
        return Ok(None); // inalcanzable: una respuesta, no un error
    }

    // Reconstrucción por predecesores (misma mecánica que
    // ShortestPaths::path_to del cap 22): los pred forman árbol porque cada
    // asignación exige mejora ESTRICTA de g.
    let mut steps = Vec::new();
    let mut v = dest;
    while let Some(step) = &pred[v] {
        steps.push(step.clone());
        v = step.from;
    }
    debug_assert_eq!(v, origin, "la cadena de predecesores llega al origen");
    steps.reverse();
    Ok(Some(Path {
        origin,
        destination: dest,
        cost: g[dest],
        steps,
        stats,
    }))
}

// ─── Utilidad de diagnóstico: consistencia ───

/// Verifica la CONSISTENCIA de `heuristic` contra los pesos del store:
/// para toda arista (u → v) debe cumplir `h(u) ≤ w(u,v) + h(v)`.
///
/// A diferencia de la admisibilidad (global: exigiría el coste real de cada
/// nodo al destino, i.e. un Dijkstra), la consistencia es LOCAL: una pasada
/// O(E) más las estimaciones (≤ 2V). Por eso existe esta utilidad y no una
/// `check_admissibility`: es lo único verificable sin resolver el problema.
///
/// A* NO exige consistencia ([`a_star`] re-abre nodos y permanece óptimo con
/// h sólo admisible); su valor es DIAGNÓSTICO: una heurística inconsistente
/// sigue siendo correcta pero expande de más — si tu A* explora más de lo
/// esperado, ejecuta esto antes de culpar al grafo. La primera violación que
/// se encuentra al iterar las aristas se reporta como
/// [`PathError::InconsistentHeuristic`].
///
/// ```
/// use vol2_liradb::{
///     check_consistency, Edge, EuclideanHeuristic, GraphStore, MemoryStore, Node, Value,
///     WeightSource,
/// };
///
/// let mut s = MemoryStore::new();
/// s.put_node(
///     Node::new(0, "A")
///         .with_prop("x", Value::Float(0.0))
///         .with_prop("y", Value::Float(0.0)),
/// )
/// .unwrap();
/// s.put_node(
///     Node::new(1, "B")
///         .with_prop("x", Value::Float(3.0))
///         .with_prop("y", Value::Float(4.0)),
/// )
/// .unwrap();
/// // La recta A→B son exactamente 5; un peso ≥ 5 mantiene h consistente:
/// s.put_edge(Edge::new(0, 0, 1, "ROAD").with_prop("km", Value::Float(6.0)))
///     .unwrap();
/// let h = EuclideanHeuristic::new(&s, 1, "x", "y").unwrap();
/// let peso = WeightSource::property("km");
/// assert!(check_consistency(&s, &peso, &h).is_ok());
/// // ...pero un túnel de 4 km (¡más corto que la recta!) la rompe:
/// s.put_edge(Edge::new(1, 0, 1, "TUNNEL").with_prop("km", Value::Float(4.0)))
///     .unwrap();
/// assert!(check_consistency(&s, &peso, &h).is_err());
/// ```
pub fn check_consistency(
    store: &dyn GraphStore,
    weight: &WeightSource,
    heuristic: &dyn Heuristic,
) -> Result<(), PathError> {
    // Pesos primero (misma sanidad que los algoritmos): no tiene sentido
    // diagnosticar una heurística contra datos que los propios algoritmos
    // rechazarían.
    validate_edge_weights(store, weight)?;
    let mut h_cache = vec![f64::NAN; table_len(store)];
    for edge in store.iter_edges() {
        let w = edge_weight(edge, weight)?;
        let h_from = h_of(heuristic, store, &mut h_cache, edge.source)?;
        let h_to = h_of(heuristic, store, &mut h_cache, edge.target)?;
        if h_from > w + h_to {
            return Err(PathError::InconsistentHeuristic {
                edge: edge.id,
                h_from,
                bound: w + h_to,
            });
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests_a_estrella {
    use super::*;
    use crate::cap07_modelo::{Edge, EdgeId, Node};
    use crate::cap08_graph_store::MemoryStore;
    use crate::cap22_caminos_minimos::dijkstra_path;

    // ════════════════════════════════════════════════════════════════
    //  Fixtures y helpers
    // ════════════════════════════════════════════════════════════════

    /// Heurística ad-hoc por tabla: h(n) = valores[n]. La forma mínima de
    /// implementar el trait — demuestra que cualquiera puede aportar h sin
    /// tocar el algoritmo (y sirve para construir los casos patológicos:
    /// inconsistente, sobre-estimada, negativa, NaN).
    struct Fija(Vec<f64>);

    impl Heuristic for Fija {
        fn estimate(&self, _store: &dyn GraphStore, node: NodeId) -> Result<f64, PathError> {
            Ok(self.0[node])
        }
    }

    /// Grafo dirigido ponderado desde `(nodos, [(id, from, to, peso)])`,
    /// pesos en la propiedad "weight" — hereda el fixture del cap 22.
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

    /// Nodo con coordenadas x/y (para la heurística euclídea).
    fn ciudad(id: NodeId, x: f64, y: f64) -> Node {
        Node::new(id, "City")
            .with_prop("x", Value::Float(x))
            .with_prop("y", Value::Float(y))
    }

    /// Arista ROAD con "weight" en km (≥ distancia recta ⇒ heurística
    /// euclídea admisible y consistente).
    fn carretera(eid: EdgeId, from: NodeId, to: NodeId, km: f64) -> Edge {
        Edge::new(eid, from, to, "ROAD").with_prop("weight", Value::Float(km))
    }

    /// Como el `assert_camino_valido` del cap 22: el camino encadena, las
    /// aristas existen y apuntan donde dicen, y el coste suma.
    fn assert_camino_valido(store: &dyn GraphStore, path: &Path, weight: &WeightSource) {
        let mut actual = path.origin;
        let mut suma = 0.0;
        for step in &path.steps {
            let edge = store.get_edge(step.edge).expect("arista del camino existe");
            assert_eq!(edge.source, step.from);
            assert_eq!(edge.target, step.to);
            assert_eq!(edge.source, actual, "el paso encadena desde el anterior");
            assert_eq!(step.weight, edge_weight(edge, weight).unwrap());
            suma += step.weight;
            actual = step.to;
        }
        assert_eq!(actual, path.destination, "el camino termina en el destino");
        assert_eq!(path.cost, suma, "el coste es la suma de los pesos");
    }

    // ════════════════════════════════════════════════════════════════
    //  h ≡ 0: A* ES Dijkstra
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn heuristica_cero_es_dijkstra_exactamente() {
        // Con h ≡ 0 la clave del heap (f, g, nodo) degenera en la de
        // Dijkstra (g, nodo): MISMO camino, MISMO coste y MISMO trabajo
        // (hasta el orden de pops es idéntico — el test de equivalencia más
        // fuerte que se puede pedir).
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
        let pa = a_star(&s, 0, 4, &w(), &ZeroHeuristic).unwrap().unwrap();
        let pd = dijkstra_path(&s, 0, 4, &w()).unwrap().unwrap();
        assert_eq!(pa.cost, pd.cost);
        assert_eq!(pa.cost, 20.0); // 0→2→5→4 = 9+2+9
        assert_eq!(pa.nodes(), pd.nodes());
        assert_eq!(pa.nodes(), vec![0, 2, 5, 4]);
        assert_eq!(pa.stats.popped, pd.stats.popped, "mismo orden de pops");
        assert_eq!(pa.stats.expanded, pd.stats.expanded);
        assert_eq!(pa.stats.relax_updates, pd.stats.relax_updates);
        assert_camino_valido(&s, &pa, &w());
    }

    #[test]
    fn heuristica_cero_coincide_en_destino_inalcanzable_y_trivia() {
        let s = grafo(4, &[(0, 0, 1, 1.0), (1, 1, 2, 1.0)]);
        // Inalcanzable: misma respuesta (None) que Dijkstra.
        assert_eq!(
            a_star(&s, 0, 3, &w(), &ZeroHeuristic).unwrap(),
            dijkstra_path(&s, 0, 3, &w()).unwrap() // None
        );
        // Origen == destino: camino vacío, coste 0 (aunque la heurística se
        // consulte para el origen: el contrato se revisa igual).
        let p = a_star(&s, 2, 2, &w(), &ZeroHeuristic).unwrap().unwrap();
        assert_eq!(p.cost, 0.0);
        assert!(p.steps.is_empty());
        assert_eq!(p.nodes(), vec![2]);
    }

    // ════════════════════════════════════════════════════════════════
    //  Euclídea: mismo coste, MENOS expansiones (el punto del capítulo)
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn euclidea_mismo_coste_con_menos_expansiones() {
        // Cadena 0→...→9 sobre el eje X (tramos de 1) + una "trampa" de
        // aristas baratísimas (0.5) que se aleja del destino subiendo por el
        // eje Y. Dijkstra explora por g y cae en la trampa (los 0.5 son
        // tentadores: todos cuestan menos que el destino); A* ve que los
        // nodos de la trampa están LEJÍSIMOS del destino y no los toca.
        let mut s = MemoryStore::new();
        for i in 0..10 {
            s.put_node(ciudad(i, i as f64, 0.0)).unwrap(); // cadena en y=0
        }
        for i in 0..3 {
            s.put_node(ciudad(10 + i, 0.0, 100.0 * (i + 1) as f64))
                .unwrap(); // trampa
        }
        for i in 0..9 {
            s.put_edge(carretera(i, i, i + 1, 1.0)).unwrap();
        }
        s.put_edge(carretera(9, 0, 10, 0.5)).unwrap();
        s.put_edge(carretera(10, 10, 11, 0.5)).unwrap();
        s.put_edge(carretera(11, 11, 12, 0.5)).unwrap();

        let h = EuclideanHeuristic::new(&s, 9, "x", "y").unwrap();
        let pa = a_star(&s, 0, 9, &w(), &h).unwrap().unwrap();
        let pd = dijkstra_path(&s, 0, 9, &w()).unwrap().unwrap();

        // Mismo camino y coste (h admisible ⇒ óptimo):
        assert_eq!(pa.cost, pd.cost);
        assert_eq!(pa.cost, 9.0);
        assert_eq!(pa.nodes(), vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        // EL ahorro, medido: Dijkstra expande los 13 nodos (la cadena más
        // la trampa entera); A* sólo la cadena (10) — la trampa no le tienta.
        assert_eq!(pd.stats.expanded, 13);
        assert_eq!(pa.stats.expanded, 10);
        assert_camino_valido(&s, &pa, &w());
        // Y en esta red euclídea la heurística es además CONSISTENTE:
        assert!(check_consistency(&s, &w(), &h).is_ok());
    }

    #[test]
    fn hito_del_brief_rutas_sobre_una_red_de_ciudades() {
        // El hito del capítulo (§brief): calcular rutas sobre una red de
        // ciudades. Coordenadas estilizadas (km, Madrid en el origen) y
        // carreteras = distancia recta + margen (una carretera nunca es más
        // corta que la línea recta ⇒ euclídea admisible y consistente).
        let mut s = MemoryStore::new();
        let ciudades = [
            (0, "Madrid", 0.0, 0.0),
            (1, "Valladolid", -130.0, 140.0),
            (2, "Bilbao", -190.0, 200.0),
            (3, "Zaragoza", 190.0, 130.0),
            (4, "Barcelona", 380.0, 180.0),
            (5, "Valencia", 230.0, -170.0),
            (6, "Sevilla", -60.0, -330.0),
        ];
        for (id, nombre, x, y) in ciudades {
            s.put_node(ciudad(id, x, y).with_prop("name", Value::String(nombre.to_string())))
                .unwrap();
        }
        // (id, from, to, km) — bidireccionales: dos aristas por carretera.
        let carreteras = [
            (0, 0, 3, 240.0), // Madrid–Zaragoza
            (1, 3, 0, 240.0),
            (2, 3, 4, 200.0), // Zaragoza–Barcelona
            (3, 4, 3, 200.0),
            (4, 0, 4, 460.0), // Madrid–Barcelona (directo, caro)
            (5, 4, 0, 460.0),
            (6, 0, 1, 195.0), // Madrid–Valladolid
            (7, 1, 0, 195.0),
            (8, 1, 2, 90.0), // Valladolid–Bilbao
            (9, 2, 1, 90.0),
            (10, 0, 5, 290.0), // Madrid–Valencia
            (11, 5, 0, 290.0),
            (12, 5, 3, 310.0), // Valencia–Zaragoza
            (13, 3, 5, 310.0),
            (14, 5, 4, 390.0), // Valencia–Barcelona
            (15, 4, 5, 390.0),
            (16, 0, 6, 340.0), // Madrid–Sevilla
            (17, 6, 0, 340.0),
        ];
        for &(eid, from, to, km) in &carreteras {
            s.put_edge(carretera(eid, from, to, km)).unwrap();
        }

        let h = EuclideanHeuristic::new(&s, 4, "x", "y").unwrap();
        let pa = a_star(&s, 0, 4, &w(), &h).unwrap().unwrap();
        let pd = dijkstra_path(&s, 0, 4, &w()).unwrap().unwrap();

        // Ruta óptima: Madrid→Zaragoza→Barcelona (440) gana al directo (460)
        // y a las alternativas por Valencia (680/800).
        assert_eq!(pa.nodes(), vec![0, 3, 4]);
        assert_eq!(pa.cost, pd.cost);
        assert_eq!(pa.cost, 440.0);
        assert_camino_valido(&s, &pa, &w());
        // El ahorro: Dijkstra asienta las 7 ciudades (explora "en círculo"
        // hasta que Barcelona sale del heap); A* sólo Madrid, Zaragoza y
        // Barcelona — el resto del mapa ni se mira.
        assert_eq!(pd.stats.expanded, 7);
        assert_eq!(pa.stats.expanded, 3);
        // La red es "honesta" (carretera ≥ recta): heurística consistente.
        assert!(check_consistency(&s, &w(), &h).is_ok());
        // Y el constructor apunta al destino que se le dio:
        assert_eq!(h.destination(), 4);
    }

    #[test]
    fn unidades_mezcladas_km_vs_minutos_rompen_la_admisibilidad() {
        // El error clásico al reutilizar la euclídea: coordenadas en km pero
        // pesos en MINUTOS. La recta (km) deja de acotar el TIEMPO, h
        // sobre-estima y A* devuelve un camino subóptimo EN SILENCIO —
        // el mismo síntoma que la sobre-estimación deliberada del test
        // siguiente, pero nacido de un descuido de unidades.
        let mut s = MemoryStore::new();
        s.put_node(ciudad(0, 0.0, 0.0)).unwrap();
        s.put_node(ciudad(1, 150.0, 130.0)).unwrap(); // desviación al norte
        s.put_node(ciudad(2, 200.0, 0.0)).unwrap();
        // Autopista directa: 200 km con tráfico a 60 km/h = 200 min.
        s.put_edge(carretera(0, 0, 2, 200.0)).unwrap();
        // Ruta desviada pero rápida: ~197 km a 120 km/h ≈ 98 min y
        // ~134 km ≈ 67 min → total 165 min (el óptimo real).
        s.put_edge(carretera(1, 0, 1, 98.0)).unwrap();
        s.put_edge(carretera(2, 1, 2, 67.0)).unwrap();

        let h = EuclideanHeuristic::new(&s, 2, "x", "y").unwrap(); // h en km
        let pa = a_star(&s, 0, 2, &w(), &h).unwrap().unwrap();
        let pd = dijkstra_path(&s, 0, 2, &w()).unwrap().unwrap();

        assert_eq!(pd.cost, 165.0); // el óptimo: por la desviada rápida
        assert_eq!(pd.nodes(), vec![0, 1, 2]);
        // La heurística en km hunde a la desviada (h(1) = recta 1→2 ≈ 139
        // "km" que no son minutos) y premia la directa: A* se detiene en el
        // primer pop del destino y responde 200 min sin ningún error.
        assert_eq!(pa.cost, 200.0, "unidades mezcladas ⇒ subóptimo silencioso");
        assert_eq!(pa.nodes(), vec![0, 2]);
        // check_consistency delata la arista culpable (1→2: h(1) > 67 + 0):
        assert_eq!(
            check_consistency(&s, &w(), &h),
            Err(PathError::InconsistentHeuristic {
                edge: 2,
                h_from: (150.0 - 200.0f64).hypot(130.0), // recta 1→2 en km
                bound: 67.0,
            })
        );
    }

    // ════════════════════════════════════════════════════════════════
    //  Admisible pero INCONSISTENTE: re-expansión (y aun así óptimo)
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn admisible_no_consistente_reexpande_y_sigue_optimo() {
        // Grafo: 0→A(1) w4; 0→B(2) w2.6; B→A w1; A→T(3) w1; B→T w9.
        // h = [0, 0, 1.9, 0]: admisible (h(B)=1.9 ≤ d(B)=2) pero INCONSISTENTE
        // (h(B)=1.9 > w(B→A)+h(A) = 1). La inconsistencia hace que A se
        // expanda con g=4, luego MEJORE a 3.6 vía B, y se RE-EXPANDA.
        let s = grafo(
            4,
            &[
                (0, 0, 1, 4.0),
                (1, 0, 2, 2.6),
                (2, 2, 1, 1.0),
                (3, 1, 3, 1.0),
                (4, 2, 3, 9.0),
            ],
        );
        let h = Fija(vec![0.0, 0.0, 1.9, 0.0]);
        // check_consistency delata exactamente la arista culpable:
        assert_eq!(
            check_consistency(&s, &w(), &h),
            Err(PathError::InconsistentHeuristic {
                edge: 2, // B→A
                h_from: 1.9,
                bound: 1.0,
            })
        );

        let pa = a_star(&s, 0, 3, &w(), &h).unwrap().unwrap();
        let pd = dijkstra_path(&s, 0, 3, &w()).unwrap().unwrap();
        // A* con h admisible (aunque inconsistente) sigue encontrando el
        // óptimo — gracias a la RE-APERTURA:
        assert_eq!(pa.cost, pd.cost);
        assert!((pa.cost - 4.6).abs() < 1e-9, "coste {}", pa.cost);
        assert_eq!(pa.nodes(), vec![0, 2, 1, 3]); // por B, no por el directo a A
        assert_camino_valido(&s, &pa, &w());
        // El precio de la inconsistencia, medido: 5 expansiones para 4 nodos
        // (A se expande dos veces: con g=4 y con g=3.6).
        assert_eq!(pa.stats.expanded, 5, "A se re-expande");
        // (Con h ≡ 0 este grafo necesita 4 expansiones: la inconsistencia
        // ni siquiera ahorra trabajo aquí.)
        assert_eq!(
            a_star(&s, 0, 3, &w(), &ZeroHeuristic)
                .unwrap()
                .unwrap()
                .stats
                .expanded,
            4
        );
    }

    // ════════════════════════════════════════════════════════════════
    //  Sobre-estimación: el riesgo demostrado ("cuándo A* no ayuda")
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn sobre_estimacion_devuelve_suboptimo_demostrando_el_riesgo() {
        // h(1) = 10 sobre-estima (el coste real de 1 a 2 es 1): la
        // admisibilidad se rompe y A* responde SIN error un camino SUBÓPTIMO
        // (3.0 en vez de 2.0). Ni pánico ni error tipado: una respuesta
        // plausible pero mala — exactamente por lo que el contracto de
        // admisibilidad se DOCUMENTA y no se verifica (verificarlo = resolver
        // el problema).
        let s = grafo(3, &[(0, 0, 1, 1.0), (1, 1, 2, 1.0), (2, 0, 2, 3.0)]);
        let h = Fija(vec![0.0, 10.0, 0.0]);
        let pa = a_star(&s, 0, 2, &w(), &h).unwrap().unwrap();
        let pd = dijkstra_path(&s, 0, 2, &w()).unwrap().unwrap();
        assert_eq!(pd.cost, 2.0); // el óptimo: 0→1→2
        assert_eq!(pa.cost, 3.0, "sobre-estimar h ⇒ subóptimo en silencio");
        assert_eq!(pa.nodes(), vec![0, 2]);
        // El camino devuelto es VÁLIDO (existe, encadena, suma): A* no miente
        // sobre el camino que devuelve — sólo sobre su optimalidad.
        assert_camino_valido(&s, &pa, &w());
    }

    // ════════════════════════════════════════════════════════════════
    //  Herencia del cap 22: pesos estrictos, nodos, overflow, multigrafos
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn pesos_herencia_cap22_missing_invalid_negativo() {
        // Propiedad de peso ausente → MissingWeight (aunque la heurística
        // sea perfecta): el contracto de datos es el del cap 22.
        let mut s = MemoryStore::new();
        s.put_node(Node::new(0, "A")).unwrap();
        s.put_node(Node::new(1, "B")).unwrap();
        s.put_edge(Edge::new(0, 0, 1, "R")).unwrap(); // sin "weight"
        assert_eq!(
            a_star(&s, 0, 1, &w(), &ZeroHeuristic),
            Err(PathError::MissingWeight {
                edge: 0,
                prop: "weight".into()
            })
        );
        // Peso no numérico → InvalidWeight (store nuevo: MemoryStore no
        // reemplaza ids de arista existentes):
        let mut s1 = MemoryStore::new();
        s1.put_node(Node::new(0, "A")).unwrap();
        s1.put_node(Node::new(1, "B")).unwrap();
        s1.put_edge(Edge::new(0, 0, 1, "R").with_prop("weight", Value::String("lejos".into())))
            .unwrap();
        assert!(matches!(
            a_star(&s1, 0, 1, &w(), &ZeroHeuristic),
            Err(PathError::InvalidWeight { found, .. }) if found == "String"
        ));
        // Peso negativo → rechazo eager aunque esté en zona no tocada
        // (misma filosofía que dijkstra; la misión de A* también es codiciosa):
        let s2 = grafo(5, &[(0, 0, 1, 1.0), (1, 3, 4, -2.0)]);
        assert_eq!(
            a_star(&s2, 0, 1, &w(), &ZeroHeuristic),
            Err(PathError::NegativeWeight {
                edge: 1,
                weight: -2.0
            })
        );
    }

    #[test]
    fn nodos_desconocidos_y_coste_que_desborda() {
        let vacio = MemoryStore::new();
        assert_eq!(
            a_star(&vacio, 0, 1, &w(), &ZeroHeuristic),
            Err(PathError::UnknownNode(0))
        );
        let s = grafo(2, &[(0, 0, 1, 1.0)]);
        assert_eq!(
            a_star(&s, 1, 5, &w(), &ZeroHeuristic),
            Err(PathError::UnknownNode(5))
        );
        // g desborda a infinito → CostOverflow (para no confundirlo con el
        // centinela de inalcanzable), herencia cap 22:
        let s2 = grafo(3, &[(0, 0, 1, 1e308), (1, 1, 2, 1e308)]);
        assert_eq!(
            a_star(&s2, 0, 2, &w(), &ZeroHeuristic),
            Err(PathError::CostOverflow { edge: 1 })
        );
    }

    #[test]
    fn multigrafos_self_loops_y_fuente_constante_como_en_cap22() {
        // Aristas paralelas: gana la barata (e1, peso 3).
        let s = grafo(2, &[(0, 0, 1, 7.0), (1, 0, 1, 3.0)]);
        let p = a_star(&s, 0, 1, &w(), &ZeroHeuristic).unwrap().unwrap();
        assert_eq!(p.cost, 3.0);
        assert_eq!(p.steps[0].edge, 1);
        // Self-loop positivo: nunca ayuda (coste 0 en quedarse).
        let s2 = grafo(
            3,
            &[
                (0, 0, 0, 5.0),
                (1, 0, 1, 1.0),
                (2, 1, 1, 2.0),
                (3, 1, 2, 1.0),
            ],
        );
        let p2 = a_star(&s2, 0, 2, &w(), &ZeroHeuristic).unwrap().unwrap();
        assert_eq!(p2.cost, 2.0);
        assert_eq!(p2.nodes(), vec![0, 1, 2]);
        // Y la fuente Constant(1.0) cuenta saltos (Default del cap 22):
        let p3 = a_star(&s, 0, 1, &WeightSource::default(), &ZeroHeuristic)
            .unwrap()
            .unwrap();
        assert_eq!(p3.cost, 1.0);
    }

    // ════════════════════════════════════════════════════════════════
    //  Errores tipados nuevos: coordenadas y heurística
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn coordenadas_ausentes_invalidas_o_no_finitas() {
        let mut s = MemoryStore::new();
        s.put_node(Node::new(0, "A")).unwrap(); // SIN x/y
        s.put_node(ciudad(1, 3.0, 4.0)).unwrap();
        s.put_edge(Edge::new(0, 0, 1, "R").with_prop("weight", Value::Float(1.0)))
            .unwrap();
        // La construcción valida el destino eager (2 props, barato):
        assert_eq!(
            EuclideanHeuristic::new(&s, 0, "x", "y"),
            Err(PathError::MissingCoordinate {
                node: 0,
                prop: "x".into()
            })
        );
        // ...y el resto on-demand: 0 sólo se delata cuando la búsqueda lo pisa.
        let h = EuclideanHeuristic::new(&s, 1, "x", "y").unwrap();
        assert_eq!(
            a_star(&s, 0, 1, &w(), &h),
            Err(PathError::MissingCoordinate {
                node: 0,
                prop: "x".into()
            })
        );

        // Coordenada no numérica → InvalidCoordinate:
        let mut s2 = MemoryStore::new();
        s2.put_node(
            Node::new(0, "A")
                .with_prop("x", Value::String("oeste".into()))
                .with_prop("y", Value::Float(0.0)),
        )
        .unwrap();
        s2.put_node(ciudad(1, 1.0, 0.0)).unwrap();
        assert!(matches!(
            EuclideanHeuristic::new(&s2, 0, "x", "y"),
            Err(PathError::InvalidCoordinate { found, .. }) if found == "String"
        ));
        // Coordenada NaN → InvalidCoordinate ("non-finite Float"):
        let mut s3 = MemoryStore::new();
        s3.put_node(
            Node::new(0, "A")
                .with_prop("x", Value::Float(f64::NAN))
                .with_prop("y", Value::Float(0.0)),
        )
        .unwrap();
        s3.put_node(ciudad(1, 1.0, 0.0)).unwrap();
        assert!(matches!(
            EuclideanHeuristic::new(&s3, 0, "x", "y"),
            Err(PathError::InvalidCoordinate { node: 0, .. })
        ));
        // Coordenada Int se promociona (misma regla que los pesos):
        let mut s4 = MemoryStore::new();
        s4.put_node(
            Node::new(0, "A")
                .with_prop("x", Value::Int(3))
                .with_prop("y", Value::Int(4)),
        )
        .unwrap();
        s4.put_node(ciudad(1, 0.0, 0.0)).unwrap();
        let h4 = EuclideanHeuristic::new(&s4, 0, "x", "y").unwrap();
        assert_eq!(h4.estimate(&s4, 1).unwrap(), 5.0); // triángulo 3-4-5
    }

    #[test]
    fn heuristica_negativa_o_no_finita_es_rechazada() {
        // Negativa: admisible "en teoría", bug en la práctica (y rompe el
        // criterio de parada) → error tipado.
        let s = grafo(2, &[(0, 0, 1, 1.0)]);
        let neg = Fija(vec![-1.0, 0.0]);
        assert_eq!(
            a_star(&s, 0, 1, &w(), &neg),
            Err(PathError::NegativeHeuristic {
                node: 0,
                value: -1.0
            })
        );
        // NaN: sin este control el heap haría PANIC dentro de Cost::cmp
        // (f64 no implementa Ord precisamente por esto).
        let nan = Fija(vec![f64::NAN, 0.0]);
        assert!(matches!(
            a_star(&s, 0, 1, &w(), &nan),
            Err(PathError::NonFiniteHeuristic { node: 0, .. })
        ));
    }

    #[test]
    fn errores_nuevos_display_y_std_error() {
        let errs = vec![
            PathError::MissingCoordinate {
                node: 4,
                prop: "x".into(),
            },
            PathError::InvalidCoordinate {
                node: 4,
                prop: "y".into(),
                found: "String".into(),
            },
            PathError::NonFiniteHeuristic {
                node: 2,
                value: f64::NAN,
            },
            PathError::NegativeHeuristic {
                node: 2,
                value: -0.5,
            },
            PathError::InconsistentHeuristic {
                edge: 7,
                h_from: 3.0,
                bound: 2.0,
            },
        ];
        for e in &errs {
            assert!(!e.to_string().is_empty());
            let _: &dyn std::error::Error = e; // trait implementado (herencia cap 22)
        }
        assert!(errs[0].to_string().contains("x"));
        assert!(errs[4].to_string().contains("7"));
    }

    // ════════════════════════════════════════════════════════════════
    //  Coherencia de stats y Display del camino (interfaz cap 22)
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn stats_coherentes_y_display_heredado() {
        // stats: expanded ≤ popped (los obsoletos se saltan), updates ≤
        // attempts — los invariantes del cap 22 siguen valiendo.
        let mut s = MemoryStore::new();
        for i in 0..10 {
            s.put_node(ciudad(i, i as f64, 0.0)).unwrap();
        }
        for i in 0..9 {
            s.put_edge(carretera(i, i, i + 1, 1.0)).unwrap();
        }
        // Un "atajo" caro paralelo deja entradas obsoletas en el heap:
        s.put_edge(carretera(9, 0, 9, 50.0)).unwrap();
        let h = EuclideanHeuristic::new(&s, 9, "x", "y").unwrap();
        let p = a_star(&s, 0, 9, &w(), &h).unwrap().unwrap();
        assert!(p.stats.expanded >= 1);
        assert!(p.stats.expanded <= p.stats.popped);
        assert!(p.stats.relax_updates <= p.stats.relax_attempts);
        // Path es el del cap 22: Display estilo Cypher, hops(), nodes().
        assert_eq!(p.hops(), 9);
        assert_eq!(
            p.to_string(),
            "(n0)-[e0 w=1]->(n1)-[e1 w=1]->(n2)-[e2 w=1]->(n3)-[e3 w=1]->(n4)-[e4 w=1]->(n5)-[e5 w=1]->(n6)-[e6 w=1]->(n7)-[e7 w=1]->(n8)-[e8 w=1]->(n9) cost=9"
        );
    }
}
