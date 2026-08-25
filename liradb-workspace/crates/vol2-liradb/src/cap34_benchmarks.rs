// ─────────────────── Cap 34: benchmarks y perfilado ───────────────────
//
// El cap. 33 dejó la torre de pruebas («primero correcto») y deliberadamente
// SIN criterion. Este módulo añade el CALIBRADO del capítulo de medición
// («luego rápido»): las piezas que benches (`benches/`) y prosa comparten
// como único punto de verdad.
//
//   * EL PRNG A MANO ([`Xorshift64Star`]) — xorshift64* de Marsaglia (JSS
//     2003) en ~15 líneas con std. La regla «primero a mano» del workspace
//     aplica también aquí: `rand` sería una dependencia de RUNTIME para
//     generar datos de TEST. Semilla 0 prohibida (degenera a todo-ceros).
//
//   * EL DATASET DE REFERENCIA ([`dataset_referencia`]) — 100.000 nodos /
//     500.000 aristas / 10 etiquetas / 20 claves de propiedades, DETERMINISTA
//     bajo [`SEMILLA_REFERENCIA`]. Sin dataset fijo no hay baseline
//     comparable: si la entrada cambia entre runs, comparar es comparar
//     peras con manzanas (Mytkowicz et al., ASPLOS 2009 — control de
//     variables). Distribución HEAVY-TAIL LIGERA estilo Barabási-Albert
//     simplificado: mayoría de nodos con grado bajo + ~128 hubs que
//     concentran una parte de las aristas. La UNIFORME sería la mentira
//     cómoda: nadie despliega grafos uniformes y ocultaría el coste de los
//     hubs, que es justo lo que un motor de grafos tiene que enseñar.
//
//   * EL HARNESS DE PERCENTILES ([`percentiles`]) — p50/p90/p99/p99.9
//     calculados A MANO sobre muestras crudas con nearest-rank (enteros,
//     sin interpolación ni floats: determinista y auditable). Criterion
//     expone media/mediana/desv., pero el lenguaje de los SLO reales es la
//     COLA: una consulta lenta de cada cien no se ve en la media (G. Tene,
//     «How NOT to Measure Latency», 2015).
//
// Además vive aquí el PIPELINE DE CONSULTA desenrollado que comparten los
// benches de `benches/` y los tests de este módulo: parse → lower →
// Executor capturando `ExecMetrics` (los contadores del cap. 20 se REPORTAN
// junto a los tiempos; instrumentarlos es el cap. 35).
//
// HALLAZGO MEDIDO que delimita el capítulo (honestidad ante todo):
// `Catalog::collect` del cap. 21 reconstruye el índice de igualdad con
// búsqueda LINEAL por entrada (`eq_push` hace `find` sobre el vector), así
// que su coste crece cuadráticamente con los valores DISTINTOS. Con el email
// único de este dataset (100.000 valores × 2 entradas label/comodín) medimos
// ~224 s para construir el catálogo en release — inviable dentro de un bench
// y señal honesta de trabajo futuro del optimizador. Por eso los benches de
// consulta miden EJECUCIÓN sobre planes semi-ligados: exactamente el plan
// `IndexSeek` que la regla R4 produce (demostrado por
// `optimizador_real_produce_index_seek_en_mini` contra un dataset mini donde
// el catálogo sí es barato), resuelto UNA vez fuera de la región medida. El
// frío/caliente de CONSULTAS completas sigue sin existir: no hay DiskStore
// detrás del puerto (verificado caps. 8/12-15) y fingirlo está prohibido.

use crate::cap07_modelo::{Edge, Node, NodeId, Value};
use crate::cap08_graph_store::{GraphStore, MemoryStore};
use crate::cap17_liraql_ast::RelDirection;
use crate::cap19_plan_logico::{LogicalPlan, Projection, ScalarExpr};
use crate::cap20_volcano::{ExecError, ExecMetrics, Executor, ResultSet};
use crate::cap22_caminos_minimos::{Path, dijkstra_path};

// ─────────────────── PRNG determinista a mano ───────────────────

/// Semilla PÚBLICA del dataset de referencia y de las muestras de los tests.
///
/// Cualquier valor ≠ 0 sirve; fijarla en el libro convierte cada cifra del
/// capítulo en reproducible byte a byte. Fracción áurea como convención
/// (buena dispersión binaria, nada mágico).
pub const SEMILLA_REFERENCIA: u64 = 0x9E37_79B9_7F4A_7C15;

/// PRNG xorshift64* (Marsaglia, JSS 2003) — el generador «primero a mano».
///
/// Dos razones para escribirlo aquí en vez de depender de `rand`:
/// 1. La regla del workspace: dependencias de runtime para datos de TEST son
///    peso y superficie de supply-chain gratuitos.
/// 2. La REPRODUCIBILIDAD es requisito de un benchmark: una docena de líneas
///    de enteros puros dan la misma secuencia en cualquier máquina y
///    versión, cosa que ninguna crate promete entre majors.
///
/// Estado interno nunca 0: la semilla 0 es punto fijo del xorshift
/// (generaría todo-ceros) y se rechaza en construcción.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Xorshift64Star {
    estado: u64,
}

impl Xorshift64Star {
    /// Crea el generador. Entra en pánico con semilla 0 (punto fijo).
    pub fn new(semilla: u64) -> Self {
        assert!(
            semilla != 0,
            "xorshift64*: la semilla 0 degenera a todo-ceros"
        );
        Self { estado: semilla }
    }

    /// Siguiente u64 de la secuencia (periodo 2^64 − 1).
    ///
    /// Truco de Marsaglia: xorshift con desplazamientos (12, 25, 27) y
    /// multiplicación por constante impar al final — el `*` de «64star» —
    /// que arregla los bits bajos del xorshift pelado.
    #[allow(clippy::should_implement_trait)]
    pub fn siguiente(&mut self) -> u64 {
        let mut x = self.estado;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.estado = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// u64 uniforme en `[0, techo)` (sesgo de módulo documentado: para datos
    /// de prueba es irrelevante y evita rechazo-y-re-muestreo).
    pub fn debajo_de(&mut self, techo: u64) -> u64 {
        self.siguiente() % techo.max(1)
    }
}

// ─────────────────── Harness de percentiles ───────────────────

/// Percentiles de una distribución de latencias (o de lo que sea) en u64.
///
/// Campos públicos a propósito: la prosa los imprime tal cual y ningún
/// consumidor necesita métodos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Percentiles {
    /// Mediana: la mitad de las muestras vive por debajo.
    pub p50: u64,
    /// El 10 % más lento empieza aquí.
    pub p90: u64,
    /// La cola: 1 de cada 100 supera esto.
    pub p99: u64,
    /// Cola extrema (el SLO de la guardia nocturna): 1 de cada 1.000.
    pub p999: u64,
}

/// Calcula p50/p90/p99/p99.9 por NEAREST-RANK sobre muestras crudas.
///
/// Por qué nearest-rank y no interpolación: el rango es `k = ⌈p·n⌉` en
/// ENTEROS (sin floats ni ambigüedades de redondeo), el índice existe
/// siempre dentro de la muestra y el resultado es auditable a mano en un
/// examen. Con muestras suficientes, la diferencia con la interpolada es
/// menor que el ruido del propio bench.
///
/// Entra en pánico con `muestras` vacía: no hay percentil de nada, y
/// devolver ceros sería exactamente el tipo de «valor de pared» que este
/// harness existe para evitar.
pub fn percentiles(muestras: &[u64]) -> Percentiles {
    assert!(
        !muestras.is_empty(),
        "percentiles: no hay percentiles de una muestra vacía"
    );
    let mut ordenadas = muestras.to_vec();
    ordenadas.sort_unstable();
    // Rango nearest-rank exacto en enteros: k = ⌈num·n/den⌉, índice k−1.
    let rango = |num: u64, den: u64| -> u64 {
        let n = ordenadas.len() as u128;
        let k = (num as u128 * n).div_ceil(den as u128);
        let idx = (k.max(1) - 1) as usize;
        ordenadas[idx.min(ordenadas.len() - 1)]
    };
    Percentiles {
        p50: rango(50, 100),
        p90: rango(90, 100),
        p99: rango(99, 100),
        p999: rango(999, 1000),
    }
}

// ─────────────────── El dataset de referencia ───────────────────

/// Nodos del dataset de referencia (la «100k personas» del corpus).
pub const NODOS_DATASET: usize = 100_000;
/// Aristas del dataset de referencia (las «500k relaciones» del corpus).
pub const ARISTAS_DATASET: usize = 500_000;
/// Etiquetas DISTINCTAS del esquema: base + secundarias.
pub const ETIQUETAS_DATASET: usize = 10;
/// Claves de propiedades del esquema (los nodos son SPARSE sobre ellas).
pub const PROPS_ESQUEMA: usize = 20;
/// Cuántos hubs concentra el heavy-tail ligero (~0,1 % de los nodos).
pub const HUBS_CONTEO: usize = 128;
/// Muestras deterministas expuestas para consultas reproducibles.
pub const MUESTRAS_CONTEO: usize = 16;

/// Etiqueta base: TODOS los nodos la llevan (las «100k personas» del corpus);
/// las otras 9 etiquetas son secundarias por nodo. Con ello el esquema tiene
/// exactamente [`ETIQUETAS_DATASET`] etiquetas distintas.
pub const ETIQUETA_BASE: &str = "Persona";
/// Único tipo de relación del dataset (denominadores de throughput limpios).
pub const TIPO_ARISTA: &str = "CONOCE";

/// Las 9 etiquetas secundarias (con la base suman las 10 del esquema).
pub const ETIQUETAS_SECUNDARIAS: [&str; 9] = [
    "Estudiante",
    "Docente",
    "Inversor",
    "Autor",
    "Atleta",
    "Artista",
    "Voluntario",
    "Cliente",
    "Investigador",
];

/// Las 20 claves del esquema. `email` (índice 0) es ÚNICA por nodo y alimenta
/// la ruta IndexSeek del cap. 21; el resto son dominios pequeños realistas.
pub const CLAVES_ESQUEMA: [&str; PROPS_ESQUEMA] = [
    "email",
    "nombre",
    "edad",
    "ciudad",
    "pais",
    "activo",
    "saldo",
    "nivel",
    "categoria",
    "telefono",
    "antiguedad",
    "sector",
    "idioma",
    "estado",
    "prioridad",
    "canal",
    "dispositivo",
    "suscriptor",
    "verificado",
    "puntuacion",
];

const CIUDADES: [&str; 32] = [
    "Madrid",
    "Barcelona",
    "Valencia",
    "Sevilla",
    "Zaragoza",
    "Malaga",
    "Murcia",
    "Bilbao",
    "Alicante",
    "Valladolid",
    "Vigo",
    "Gijon",
    "Granada",
    "Coruna",
    "Pamplona",
    "Santander",
    "Burgos",
    "Salamanca",
    "Albacete",
    "Castellon",
    "Huelva",
    "Logrono",
    "Caceres",
    "Oviedo",
    "Cadiz",
    "Teruel",
    "Soria",
    "Avila",
    "Leon",
    "Toledo",
    "Cuenca",
    "Lugo",
];
const PAISES: [&str; 8] = [
    "Espana", "Portugal", "Francia", "Italia", "Alemania", "Mexico", "Colombia", "Chile",
];
const SECTORES: [&str; 8] = [
    "tecnologia",
    "salud",
    "educacion",
    "finanzas",
    "retail",
    "energia",
    "transporte",
    "turismo",
];
const IDIOMAS: [&str; 6] = ["es", "en", "fr", "pt", "de", "it"];
const ESTADOS: [&str; 4] = ["nuevo", "activo", "suspendido", "baja"];
const CANALES: [&str; 6] = ["web", "movil", "tienda", "telefono", "partner", "email"];
const DISPOSITIVOS: [&str; 4] = ["pc", "movil", "tablet", "smart-tv"];
const CATEGORIAS: [&str; 9] = [
    "basico",
    "estandar",
    "premium",
    "corporativo",
    "educativo",
    "familiar",
    "profesional",
    "temporal",
    "vitalicio",
];

/// Email canónico del nodo `id`: ÚNICO por construcción (el `id` va
/// incrustado). Función PURA: el test de unicidad lo verifica contra un
/// `HashSet` y los benches resuelven id↔email sin buscar en el store.
pub fn email_de(id: NodeId) -> String {
    format!("usuario{id:06}@ejemplo.es")
}

/// Mezcla entera pura de (id, clave) — sustituto determinista de un hash.
///
/// Multiplicadores tipo splitmix: misma entrada, mismo valor, en cualquier
/// máquina. De aquí salen la DISPERSIÓN sparse de las props y sus valores.
fn mezcla(id: usize, clave_idx: usize) -> u64 {
    (id as u64 ^ (clave_idx as u64).wrapping_mul(0xD1B5_4A32_D192_ED03))
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

/// Valor de la clave `clave_idx` para el nodo `id` (la clave 0=`email` va
/// aparte porque es única). `None` = prop AUSENTE: el sparse del esquema
/// (~80 % de presencia por clave, decidido con la mezcla, no con `rand`).
fn valor_de_clave(id: usize, clave_idx: usize) -> Option<Value> {
    if mezcla(id, clave_idx).is_multiple_of(5) {
        return None;
    }
    let h = mezcla(id, clave_idx ^ 0xFFFF);
    let valor = match clave_idx {
        1 => Value::String(format!("Nombre{}", h % 4096)),
        2 => Value::Int(18 + (h % 53) as i64),
        3 => Value::String(CIUDADES[(h % 32) as usize].to_string()),
        4 => Value::String(PAISES[(h % 8) as usize].to_string()),
        5 => Value::Bool(h.is_multiple_of(2)),
        6 => Value::Float(((h % 100_000) as f64) / 100.0),
        7 => Value::Int(1 + (h % 10) as i64),
        8 => Value::String(CATEGORIAS[(h % 9) as usize].to_string()),
        9 => Value::String(format!("+34-6{:08}", h % 100_000_000)),
        10 => Value::Int((h % 25) as i64),
        11 => Value::String(SECTORES[(h % 8) as usize].to_string()),
        12 => Value::String(IDIOMAS[(h % 6) as usize].to_string()),
        13 => Value::String(ESTADOS[(h % 4) as usize].to_string()),
        14 => Value::Int(1 + (h % 3) as i64),
        15 => Value::String(CANALES[(h % 6) as usize].to_string()),
        16 => Value::String(DISPOSITIVOS[(h % 4) as usize].to_string()),
        17 => Value::Bool(!h.is_multiple_of(3)),
        18 => Value::Bool(!h.is_multiple_of(7)),
        19 => Value::Float(((h % 50) as f64) / 10.0),
        _ => unreachable!("el esquema tiene exactamente {PROPS_ESQUEMA} claves"),
    };
    Some(valor)
}

/// Ventana de nodos semilla: entre los primeros VENTANA_SEMILLA ids se
/// concentran las aristas de arranque — su ventaja temprana ES el mecanismo
/// Barabási-Albert («the rich get richer») en versión barata.
const VENTANA_SEMILLA: usize = 256;
/// Probabilidad (permilaje) de adjunción PREFERENCIAL por extremo.
///
/// 850‰ preferencial + 150‰ uniforme = heavy-tail LIGERO: hubs claros pero
/// sin la cola extrema de un BA puro, que con 500k aristas produciría grados
/// de decenas de miles y dominaría todos los benches.
const ALFA_PREFERENCIAL_PERMIL: u64 = 850;

/// El dataset de referencia + metadatos para consultas deterministas.
///
/// Los metadatos existen para que benches y tests NO busquen nada en tiempo
/// de ejecución («dame un hub», «dame un email válido»): todo viene resuelto
/// de la generación, otra pieza del calibrado.
#[derive(Debug, Clone)]
pub struct DatasetReferencia {
    /// El grafo completo sobre el store de producción (cap. 8).
    pub store: MemoryStore,
    /// Los `HUBS_CONTEO` nodos de mayor grado (empates → id menor):
    /// origen del contraste hub vs. grado bajo de los benches.
    pub hubs: Vec<NodeId>,
    /// Emails de muestra (paralelo a [`Self::ids_muestra`]): entradas
    /// válidas garantizadas para el point-lookup por igualdad.
    pub emails_muestra: Vec<String>,
    /// Ids de muestra, uniformemente espaciados (paralelo a `emails_muestra`).
    pub ids_muestra: Vec<NodeId>,
    /// Un nodo con grado SALIENTE 1..=2 elegido de forma determinista: el
    /// otro extremo del contraste de expansión (expandir desde aquí es casi
    /// gratis sin degenerar en cero filas).
    pub nodo_grado_bajo: NodeId,
    /// Par (origen, destino) CON camino garantizado para Q5: destino =
    /// `hubs[0]`, origen = el nodo más lejano alcanzable (empates → id
    /// menor), calculado por BFS en la generación. Sin él, un origen
    /// cualquiera puede caer en otra componente y no hay camino.
    pub par_camino_minimo: (NodeId, NodeId),
}

/// Genera el dataset COMPLETO (100k nodos / 500k aristas) con `seed`.
///
/// Presupuesto medido: ~0,3 s en release en laptop moderna — margen sobrado
/// frente a los <10 s del contrato. Se insertan TODOS los nodos antes que
/// las aristas (el puerto exige extremos existentes) y luego las aristas en
/// dos fases:
///
/// 1. **Semilla**: aristas densas dentro de la ventana inicial — la ventaja
///    temprana que Barabási-Albert convierte en grado.
/// 2. **Crecimiento preferencial**: cada extremo nuevo sale, con prob.
///    `ALFA_PREFERENCIAL_PERMIL/1000`, del extremo de una arista existente
///    elegida al azar. Muestrear EXTREMOS DE ARISTAS es proporcional al
///    grado: adjunción preferencial O(1) sin llevar cuentas ponderadas (el
///    BA literal costaría O(n) por arista y rompería el presupuesto).
pub fn dataset_referencia(seed: u64) -> DatasetReferencia {
    generar_dataset_con_tamanos(seed, NODOS_DATASET, ARISTAS_DATASET)
}

/// Versión MINI (400 nodos / 1.200 aristas) del MISMO generador.
///
/// Misma estructura (etiquetas, esquema sparse, email único, hubs, heavy
/// tail) a escala de test: permite correr el OPTIMIZADOR REAL —incluido el
/// `Catalog::collect`, cuadrático en valores distintos— en milisegundos y
/// demostrar ahí que las consultas de los benches disparan IndexSeek.
pub fn dataset_referencia_mini(seed: u64) -> DatasetReferencia {
    generar_dataset_con_tamanos(seed, 400, 1_200)
}

/// Núcleo paramétrico del generador (los tamaños públicos llaman a esto).
pub fn generar_dataset_con_tamanos(seed: u64, nodos: usize, aristas: usize) -> DatasetReferencia {
    let mut rng = Xorshift64Star::new(seed);
    let mut store = MemoryStore::new();

    // Fase de nodos: etiqueta base SIEMPRE («personas») + secundaria por
    // rotación modular; props sparse sobre el esquema, email único primero.
    for id in 0..nodos {
        let mut nodo = Node::new(id, ETIQUETA_BASE);
        nodo.labels
            .push(ETIQUETAS_SECUNDARIAS[id % ETIQUETAS_SECUNDARIAS.len()].to_string());
        let mut props = std::collections::HashMap::with_capacity(PROPS_ESQUEMA);
        props.insert(CLAVES_ESQUEMA[0].to_string(), Value::String(email_de(id)));
        for (clave_idx, clave) in CLAVES_ESQUEMA.iter().enumerate().skip(1) {
            if let Some(valor) = valor_de_clave(id, clave_idx) {
                props.insert((*clave).to_string(), valor);
            }
        }
        nodo.props = props;
        store
            .put_node(nodo)
            .expect("ids secuenciales: sin duplicados");
    }

    // Fase de aristas. `extremos` guarda (source, target) de lo emitido para
    // el muestreo preferencial O(1); `grados` alimenta los metadatos.
    let ventana = VENTANA_SEMILLA.min(nodos);
    let aristas_semilla = (VENTANA_SEMILLA * 4).min(aristas / 4).max(1);
    let mut extremos: Vec<(u32, u32)> = Vec::with_capacity(aristas);
    let mut grados = vec![0u32; nodos];
    let mut grados_out = vec![0u32; nodos];

    let mut eid = 0usize;
    // (1) Semilla densa en la ventana: conectividad garantizada + head start.
    while eid < aristas_semilla {
        let u = rng.debajo_de(ventana as u64) as usize;
        let mut v = (u + 1 + rng.debajo_de((ventana - 1) as u64) as usize) % ventana;
        if v == u {
            v = (u + 1) % ventana;
        }
        store
            .put_edge(Edge::new(eid, u, v, TIPO_ARISTA))
            .expect("todos los extremos existen");
        extremos.push((u as u32, v as u32));
        grados[u] += 1;
        grados_out[u] += 1;
        grados[v] += 1;
        eid += 1;
    }

    // (2) Crecimiento con adjunción preferencial: extremo de arista al azar
    // (∝ grado) o nodo uniforme — la mezcla da el tail LIGERO. Sin
    // self-loops: contaminarían expand y Dijkstra.
    while eid < aristas {
        let u = extremo_preferencial(&mut rng, &extremos, nodos);
        let mut v = extremo_preferencial(&mut rng, &extremos, nodos);
        while v == u {
            v = (v + 1) % nodos;
        }
        store
            .put_edge(Edge::new(eid, u, v, TIPO_ARISTA))
            .expect("todos los extremos existen");
        extremos.push((u as u32, v as u32));
        grados[u] += 1;
        grados_out[u] += 1;
        grados[v] += 1;
        eid += 1;
    }

    // Metadatos: hubs por grado total (empate → id menor, determinista) y un
    // nodo de grado ≤ 2 para el contraste barato.
    let mut orden: Vec<NodeId> = (0..nodos).collect();
    orden.sort_unstable_by(|&a, &b| grados[b].cmp(&grados[a]).then(a.cmp(&b)));
    orden.truncate(HUBS_CONTEO.min(nodos));

    // Nodo de grado SALIENTE bajo pero no aislado (1..=2): el contraste de
    // expansión debe ser «casi gratis» sin degenerar en cero filas.
    let nodo_grado_bajo = (ventana..nodos)
        .find(|&id| (1..=2).contains(&grados_out[id]))
        .unwrap_or_else(|| {
            (0..nodos)
                .min_by_key(|&id| grados_out[id])
                .expect("hay nodos")
        });

    let ids_muestra: Vec<NodeId> = (0..MUESTRAS_CONTEO)
        .map(|i| i * nodos / MUESTRAS_CONTEO)
        .collect();
    let emails_muestra = ids_muestra.iter().map(|&id| email_de(id)).collect();

    // Par garantizado para Q5: desde hubs[0] siguiendo SOLO aristas
    // salientes (la dirección de Dijkstra), el alcanzable MÁS LEJANO por
    // BFS (empate → id menor). Camino dirigido seguro por construcción.
    let origen_q5 = orden[0];
    let par_camino_minimo = (origen_q5, mas_lejano_alcanzable(&store, origen_q5));

    DatasetReferencia {
        store,
        hubs: orden,
        emails_muestra,
        ids_muestra,
        nodo_grado_bajo,
        par_camino_minimo,
    }
}

/// BFS DIRIGIDO desde `desde` (sólo `out_edges`, por el puerto): devuelve el
/// id alcanzable de distancia máxima, empates → id menor. Determinista.
///
/// Dirigido a propósito: es la alcanzabilidad que respeta `dijkstra_path`
/// (cap. 22), así el camino del par existe por construcción.
fn mas_lejano_alcanzable(store: &MemoryStore, desde: NodeId) -> NodeId {
    use std::collections::VecDeque;
    let n = store.node_count();
    let mut dist = vec![u32::MAX; n];
    let mut cola = VecDeque::new();
    dist[desde] = 0;
    cola.push_back(desde);
    while let Some(u) = cola.pop_front() {
        for eid in store.out_edges(u) {
            let v = store.get_edge(eid).expect("adyacencia viva").target;
            if dist[v] == u32::MAX {
                dist[v] = dist[u] + 1;
                cola.push_back(v);
            }
        }
    }
    let max_dist = dist
        .iter()
        .copied()
        .filter(|d| *d != u32::MAX)
        .max()
        .unwrap_or(0);
    (0..n)
        .find(|&id| dist[id] == max_dist)
        .expect("el propio `desde` está a distancia 0")
}

/// Extremo de nueva arista: preferencial (extremo de arista existente ∝
/// grado) con prob. ALFA; uniforme si no. Núcleo O(1) del heavy-tail ligero.
fn extremo_preferencial(rng: &mut Xorshift64Star, extremos: &[(u32, u32)], nodos: usize) -> usize {
    if rng.debajo_de(1000) < ALFA_PREFERENCIAL_PERMIL {
        let j = rng.debajo_de(extremos.len() as u64) as usize;
        let (a, b) = extremos[j];
        if rng.siguiente().is_multiple_of(2) {
            a as usize
        } else {
            b as usize
        }
    } else {
        rng.debajo_de(nodos as u64) as usize
    }
}

/// Grado total (out + in) de un nodo vía el PUERTO (cap. 8), sin bajar a
/// campos internos: el mismo respeto a la abstracción del verificador
/// del cap. 33.
pub fn grado_total(store: &dyn GraphStore, id: NodeId) -> usize {
    store.out_edges(id).len() + store.in_edges(id).len()
}

// ─────────────────── Pipeline de consulta desenrollado ───────────────────

/// Ejecuta texto LiraQL por el pipeline de la CLI (cap. 31): parse → lower →
/// Executor, devolviendo filas Y contadores del cap. 20.
///
/// Sin `optimize` NI catálogo: es la ruta de `pipeline_con_detalle` de hoy.
/// Las consultas de TEXTO de los benches (scan+filtro, proyección amplia)
/// no necesitan optimizador para ser representativas — y el hallazgo del
/// catálogo cuadrático (doc del módulo) delimita por qué no va aquí.
pub fn ejecutar_texto(
    store: &dyn GraphStore,
    src: &str,
) -> Result<(ResultSet, ExecMetrics), ExecError> {
    let query = crate::cap18_lexer_parser::parse(src)?;
    let plan = query.lower()?;
    ejecutar_plan(store, &plan)
}

/// Ejecuta un PLAN ya construido (semi-ligado o salida de lower) y devuelve
/// filas + `ExecMetrics`. Es el paso que cronometran los benches: una
/// ejecución real compila su plan UNA vez, así que `Executor::new` va
/// DENTRO de la región medida.
pub fn ejecutar_plan(
    store: &dyn GraphStore,
    plan: &LogicalPlan,
) -> Result<(ResultSet, ExecMetrics), ExecError> {
    let mut executor = Executor::new(plan, store)?;
    let rs = executor.execute()?;
    Ok((rs, executor.metrics()))
}

/// Q1 — point-lookup por igualdad: el plan EXACTO que produce la regla R4
/// del cap. 21 para `WHERE p.email = "<email>"`.
///
/// `IndexSeek(ids=[id])` semi-ligado —los ids se resuelven contra el
/// catálogo UNA vez, FUERA de la región medida (hallazgo del doc de
/// módulo)— más proyección de la propiedad buscada. Filas esperadas: 1.
/// `muestra_idx` elige la entrada determinista de `ids_muestra`.
pub fn plan_q1_point_lookup(ds: &DatasetReferencia, muestra_idx: usize) -> LogicalPlan {
    let id = ds.ids_muestra[muestra_idx % ds.ids_muestra.len()];
    LogicalPlan::Project {
        input: Box::new(LogicalPlan::IndexSeek {
            variable: "p".to_string(),
            label: Some(ETIQUETA_BASE.to_string()),
            property: CLAVES_ESQUEMA[0].to_string(),
            value: Value::String(email_de(id)),
            ids: vec![id],
        }),
        items: vec![Projection {
            expr: ScalarExpr::Property {
                variable: "p".to_string(),
                property: CLAVES_ESQUEMA[0].to_string(),
            },
            alias: None,
        }],
    }
}

/// Q2 — expansión 1-hop desde `origen`: el plan post-R4
/// `IndexSeek(origen) → Expand[:CONOCE]->(f) → Project(f.email)`.
///
/// Con `ds.hubs[k]` como origen mide el fanout caro; con
/// `ds.nodo_grado_bajo`, el caso trivial — el CONTRASTE que enseña qué
/// domina el coste de una expansión.
pub fn plan_q2_expand_desde(origen: NodeId) -> LogicalPlan {
    LogicalPlan::Project {
        input: Box::new(LogicalPlan::Expand {
            input: Box::new(LogicalPlan::IndexSeek {
                variable: "p".to_string(),
                label: Some(ETIQUETA_BASE.to_string()),
                property: CLAVES_ESQUEMA[0].to_string(),
                value: Value::String(email_de(origen)),
                ids: vec![origen],
            }),
            from: "p".to_string(),
            rel_variable: None,
            rel_type: Some(TIPO_ARISTA.to_string()),
            direction: RelDirection::Outgoing,
            to: "f".to_string(),
        }),
        items: vec![Projection {
            expr: ScalarExpr::Property {
                variable: "f".to_string(),
                property: CLAVES_ESQUEMA[0].to_string(),
            },
            alias: None,
        }],
    }
}

/// Q3 — scan + filtro SIN índice (predicado de RANGO: la igualdad es lo que
/// dispara R4; un `>` recorre los 100k nodos sí o sí). Pipeline de TEXTO
/// completo, menos optimizador.
pub const TEXTO_Q3_SCAN_FILTRO: &str = r#"MATCH (p:Persona) WHERE p.edad > 60 RETURN p.email"#;

/// Q4 — proyección AMPLIA: 5 columnas (4 lookups de props por fila) sobre
/// los 100k nodos: el coste ancho real de materializar filas.
///
/// NOTA de divergencia documentada: el contrato hablaba de `LIMIT`, pero la
/// gramática LiraQL (cap. 17) aún no tiene LIMIT ni agregación — el plan
/// lógico no puede expresarlo (`LimitOp` del cap. 20 es inalcanzable desde
/// texto). Se mide la proyección amplia completa, término dominante del
/// coste de cualquier LIMIT pequeño.
pub const TEXTO_Q4_PROYECCION_AMPLIA: &str = r#"MATCH (p:Persona) RETURN p.nombre AS nombre, p.edad AS edad, p.ciudad AS ciudad, p.saldo AS saldo, p.activo AS activo"#;

/// Q5 — camino mínimo vía API `dijkstra_path` (cap. 22). LiraQL aún no tiene
/// shortest-path: DELIMITADO por contrato, no fingido.
///
/// Origen y destino son `ds.par_camino_minimo` (metadato con camino
/// GARANTIZADO por BFS en la generación): mismo par siempre ⇒ coste idéntico
/// entre runs, y sin la sorpresa de caer en otra componente conexa.
pub fn camino_minimo_q5(ds: &DatasetReferencia) -> Option<Path> {
    let (origen, destino) = ds.par_camino_minimo;
    dijkstra_path(&ds.store, origen, destino, &Default::default()).ok()?
}

/// ¿Contiene el plan un nodo `IndexSeek`? Para afirmar EN TESTS que una
/// consulta pasa por la ruta de índice (el `Display` del plan lo pintaría;
/// esto lo comprueba estructuralmente).
pub fn plan_contiene_index_seek(plan: &LogicalPlan) -> bool {
    match plan {
        LogicalPlan::IndexSeek { .. } => true,
        LogicalPlan::Expand { input, .. }
        | LogicalPlan::Filter { input, .. }
        | LogicalPlan::Project { input, .. } => plan_contiene_index_seek(input),
        LogicalPlan::CartesianProduct { left, right } => {
            plan_contiene_index_seek(left) || plan_contiene_index_seek(right)
        }
        LogicalPlan::NodeScan { .. } => false,
    }
}

// ─────────────────── Tests-tesis del capítulo ───────────────────

#[cfg(test)]
mod tests_cap34 {
    use std::collections::HashSet;

    use super::*;
    use crate::PAGE_SIZE;
    use crate::cap12_pager::{FilePager, Pager};
    use crate::cap13_buffer_pool::{BufferPool, Metrics};
    use crate::cap21_optimizador::{Catalog, optimize};
    use crate::cap33_pruebas::verificar_invariantes;

    /// Dataset mini compartido: construirlo cuesta milisegundos y todos los
    /// tests de lógica (invariantes, métricas, optimizador) lo reutilizan.
    fn mini() -> DatasetReferencia {
        dataset_referencia_mini(SEMILLA_REFERENCIA)
    }

    // ─── PRNG ───

    #[test]
    fn prng_determinista_misma_semilla_misma_secuencia() {
        let mut a = Xorshift64Star::new(SEMILLA_REFERENCIA);
        let mut b = Xorshift64Star::new(SEMILLA_REFERENCIA);
        for _ in 0..1000 {
            assert_eq!(a.siguiente(), b.siguiente());
        }
        // Otra semilla → otra secuencia (sanidad mínima de dispersión).
        let mut c = Xorshift64Star::new(SEMILLA_REFERENCIA ^ 1);
        let distintos = (0..100).filter(|_| a.siguiente() != c.siguiente()).count();
        assert!(
            distintos > 90,
            "secuencias de semillas distintas deben diferir"
        );
    }

    #[test]
    #[should_panic(expected = "la semilla 0 degenera")]
    fn prng_rechaza_semilla_cero() {
        let _ = Xorshift64Star::new(0);
    }

    // ─── Percentiles ───

    #[test]
    fn percentiles_casos_conocidos_exactos() {
        // 1..=100: nearest-rank da el valor literal del rango.
        let m: Vec<u64> = (1..=100).collect();
        let p = percentiles(&m);
        assert_eq!((p.p50, p.p90, p.p99, p.p999), (50, 90, 99, 100));
        // Tres muestras: la mediana nearest-rank es la segunda.
        let p3 = percentiles(&[30, 10, 20]);
        assert_eq!((p3.p50, p3.p90, p3.p99, p3.p999), (20, 30, 30, 30));
        // Una sola muestra: todo el reparto cae en ella.
        let p1 = percentiles(&[7]);
        assert_eq!((p1.p50, p1.p90, p1.p99, p1.p999), (7, 7, 7, 7));
    }

    #[test]
    fn percentiles_monotonos_y_completos() {
        // Muestras pseudoaleatorias del PRNG del capítulo: CI determinista,
        // sin valores de pared (ni 0 gratuito ni u64::MAX centinela).
        let mut rng = Xorshift64Star::new(SEMILLA_REFERENCIA);
        let muestras: Vec<u64> = (0..1000).map(|_| 1 + rng.debajo_de(10_000)).collect();
        let p = percentiles(&muestras);
        assert!(p.p50 <= p.p90);
        assert!(p.p90 <= p.p99);
        assert!(p.p99 <= p.p999);
        assert!(p.p50 >= 1, "jamás valores de pared");
        assert!(p.p999 <= 10_000, "dentro del rango de las muestras");
    }

    #[test]
    #[should_panic(expected = "muestra vacía")]
    fn percentiles_rechaza_vacio() {
        let _ = percentiles(&[]);
    }

    // ─── Dataset completo (los tests pesados comparten UNA instancia) ───

    fn dataset_completo() -> &'static DatasetReferencia {
        static DATASET: std::sync::OnceLock<DatasetReferencia> = std::sync::OnceLock::new();
        DATASET.get_or_init(|| dataset_referencia(SEMILLA_REFERENCIA))
    }

    #[test]
    fn dataset_cuenta_exacta_100k_nodos_500k_aristas() {
        let ds = dataset_completo();
        assert_eq!(ds.store.node_count(), NODOS_DATASET);
        assert_eq!(ds.store.edge_count(), ARISTAS_DATASET);
        // Metadatos coherentes con el tamaño.
        assert_eq!(ds.hubs.len(), HUBS_CONTEO);
        assert_eq!(ds.ids_muestra.len(), MUESTRAS_CONTEO);
        assert_eq!(ds.emails_muestra.len(), MUESTRAS_CONTEO);
    }

    #[test]
    fn dataset_determinista_misma_semilla_grafo_identico() {
        // DOS construcciones independientes: mismos conteos Y mismo contenido
        // muestreado (props, adyacencias de hubs, metadatos completos). Es la
        // propiedad que convierte un bench en baseline comparable.
        let a = dataset_referencia_mini(SEMILLA_REFERENCIA);
        let b = dataset_referencia_mini(SEMILLA_REFERENCIA);
        assert_eq!(a.store.node_count(), b.store.node_count());
        assert_eq!(a.store.edge_count(), b.store.edge_count());
        assert_eq!(a.hubs, b.hubs);
        assert_eq!(a.nodo_grado_bajo, b.nodo_grado_bajo);
        assert_eq!(a.par_camino_minimo, b.par_camino_minimo);
        assert_eq!(a.emails_muestra, b.emails_muestra);

        // Contenido: props de los nodos muestra y vecinos out de hubs[0].
        for &id in &a.ids_muestra {
            let na = a.store.get_node(id).expect("muestra existe");
            let nb = b.store.get_node(id).expect("muestra existe");
            assert_eq!(na.labels, nb.labels);
            assert_eq!(na.props.get("email"), nb.props.get("email"));
            assert_eq!(na.props.get("edad"), nb.props.get("edad"));
            assert_eq!(na.props.get("ciudad"), nb.props.get("ciudad"));
        }
        let vecinos_a = a.store.out_edges(a.hubs[0]);
        let vecinos_b = b.store.out_edges(b.hubs[0]);
        assert_eq!(vecinos_a, vecinos_b);
        // Y otra semilla produce OTRO grafo (que no nos hemos colado).
        let c = dataset_referencia_mini(SEMILLA_REFERENCIA + 1);
        assert_ne!(a.store.out_edges(a.hubs[0]), c.store.out_edges(c.hubs[0]));
    }

    #[test]
    fn dataset_hubs_concentran_grado_maximo() {
        let ds = dataset_completo();
        // Grados totales recalculados por el puerto (una pasada de aristas).
        let mut grados = vec![0u64; NODOS_DATASET];
        for e in ds.store.iter_edges() {
            grados[e.source] += 1;
            grados[e.target] += 1;
        }
        let total: u64 = grados.iter().sum();
        let concentracion: u64 = ds.hubs.iter().map(|&h| grados[h]).sum();
        // Los 128 hubs (0,128 % de los nodos) sostienen >25 % de los
        // extremos: eso es heavy-tail LIGERO — concentración visible sin
        // cola extrema. (Umbral medido con la semilla de referencia.)
        let fraccion = concentracion as f64 / total as f64;
        assert!(
            fraccion > 0.25,
            "los hubs concentran {fraccion:.3}: menos que eso sería uniforme"
        );
        // Y ningún no-hub supera al último hub (son el TOP por definición:
        // el 129º tiene grado igual o menor que el 128º).
        let min_hub = ds.hubs.iter().map(|&h| grados[h]).min().expect("hay hubs");
        let max_fuera = (0..NODOS_DATASET)
            .filter(|id| !ds.hubs.contains(id))
            .map(|id| grados[id])
            .max()
            .expect("hay nodos");
        assert!(min_hub >= max_fuera);
        // El nodo de grado bajo del contraste es, en efecto, de grado ≤ 2.
        // El nodo de grado bajo del contraste tiene grado SALIENTE ≤ 2
        // (visto por el puerto: es el grado que usa la expansión OUTGOING).
        assert!(ds.store.out_edges(ds.nodo_grado_bajo).len() <= 2);
        assert!(!ds.store.out_edges(ds.nodo_grado_bajo).is_empty());
    }

    #[test]
    fn dataset_esquema_10_etiquetas_20_claves_email_unico() {
        let ds = dataset_completo();
        // Etiquetas distintas == 10 (base + 9 secundarias).
        let mut etiquetas: HashSet<&str> = HashSet::new();
        // Claves presentes en el store (subconjunto del esquema, con huecos
        // sparse, pero TODAS las 20 deben aparecer alguna vez en 100k nodos).
        let mut claves_vistas: HashSet<&str> = HashSet::new();
        for n in ds.store.iter_nodes() {
            for l in &n.labels {
                etiquetas.insert(l.as_str());
            }
            for k in n.props.keys() {
                claves_vistas.insert(k.as_str());
            }
        }
        assert_eq!(etiquetas.len(), ETIQUETAS_DATASET);
        assert!(etiquetas.contains(ETIQUETA_BASE));
        assert_eq!(claves_vistas.len(), PROPS_ESQUEMA);
        for clave in CLAVES_ESQUEMA {
            assert!(claves_vistas.contains(clave), "falta la clave {clave}");
        }
        // Email ÚNICO por nodo (la ruta IndexSeek exige unicidad).
        let mut emails = HashSet::with_capacity(NODOS_DATASET);
        for n in ds.store.iter_nodes() {
            match n.props.get("email") {
                Some(Value::String(e)) => {
                    assert!(emails.insert(e.clone()), "email repetido: {e}");
                }
                otro => panic!("todo nodo lleva email String, hubo {otro:?}"),
            }
        }
        assert_eq!(emails.len(), NODOS_DATASET);
    }

    #[test]
    fn dataset_invariantes_del_puerto_se_cumplen() {
        // El ORÁCULO del cap. 33 sobre la versión MINI: verificar_invariantes
        // es O(V·E) por diseño honesto del verificador, así que corre en mini
        // (misma maquinaria de generación, escala de test).
        let ds = mini();
        assert_eq!(verificar_invariantes(&ds.store), Ok(()));
    }

    // ─── Consultas: contadores visibles y ruta IndexSeek REAL ───

    #[test]
    fn consultas_reportan_exec_metrics() {
        // Sobre el MINI (rápido en debug): para Q1..Q5, per_operator no vacío
        // y rows_returned > 0 — los contadores que la prosa pegará JUNTO a
        // los tiempos de los benches.
        let ds = mini();

        let (_, m1) = ejecutar_plan(&ds.store, &plan_q1_point_lookup(&ds, 7)).expect("Q1 ejecuta");
        assert!(!m1.per_operator.is_empty() && m1.rows_returned > 0);

        let (_, m2h) =
            ejecutar_plan(&ds.store, &plan_q2_expand_desde(ds.hubs[0])).expect("Q2 hub ejecuta");
        assert!(!m2h.per_operator.is_empty() && m2h.rows_returned > 0);

        let (_, m2b) = ejecutar_plan(&ds.store, &plan_q2_expand_desde(ds.nodo_grado_bajo))
            .expect("Q2 grado bajo ejecuta");
        assert!(!m2b.per_operator.is_empty() && m2b.rows_returned > 0);
        // El CONTRASTE hub vs. grado bajo es visible hasta en mini: el hub
        // devuelve al menos tantas filas como el nodo pobre.
        assert!(m2h.rows_returned >= m2b.rows_returned);

        let (_, m3) = ejecutar_texto(&ds.store, TEXTO_Q3_SCAN_FILTRO).expect("Q3 ejecuta");
        assert!(!m3.per_operator.is_empty() && m3.rows_returned > 0);

        let (_, m4) = ejecutar_texto(&ds.store, TEXTO_Q4_PROYECCION_AMPLIA).expect("Q4 ejecuta");
        assert!(!m4.per_operator.is_empty() && m4.rows_returned > 0);

        let camino = camino_minimo_q5(&ds).expect("Q5 encuentra camino (mini conexo)");
        assert!(camino.hops() > 0);
    }

    #[test]
    fn optimizador_real_produce_index_seek_en_mini() {
        // La prueba de honestidad de los benches: sobre el dataset MINI (donde
        // Catalog::collect es barato), el OPTIMIZADOR COMPLETO del cap. 21
        // convierte la consulta de igualdad por email en EXACTAMENTE el plan
        // que `plan_q1_point_lookup` construye a mano — y ambos devuelven lo
        // mismo.
        let ds = mini();
        let email = &ds.emails_muestra[7];
        let src = format!(r#"MATCH (p:Persona) WHERE p.email = "{email}" RETURN p.email"#,);
        let query = crate::cap18_lexer_parser::parse(&src).expect("parsea");
        let plano = query.lower().expect("liga");
        let catalog = Catalog::collect(&ds.store);
        let optimizado = optimize(&plano, &catalog);
        assert!(
            plan_contiene_index_seek(&optimizado),
            "R4 debía reescribir la igualdad por email a IndexSeek"
        );

        let (rs_opt, m_opt) = ejecutar_plan(&ds.store, &optimizado).expect("ejecuta optimizado");
        let (rs_manual, m_manual) =
            ejecutar_plan(&ds.store, &plan_q1_point_lookup(&ds, 7)).expect("ejecuta manual");
        assert_eq!(rs_opt, rs_manual, "mismo resultado por ambas rutas");
        assert_eq!(m_opt.rows_returned, 1);
        assert_eq!(m_manual.per_operator, m_opt.per_operator);
    }

    // ─── Frío/caliente HONESTO a nivel componente (caps. 12+13) ───

    #[test]
    fn buffer_pool_frio_caliente_hit_ratio_sube() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("pool_frio_caliente.bin");

        // Páginas escritas de verdad en disco (con contenido distinguible).
        const PAGINAS: u32 = 32;
        {
            let mut pager = FilePager::create(&path).expect("pager nuevo");
            for _ in 0..PAGINAS {
                pager.allocate().expect("asigna página");
            }
            for id in 0..PAGINAS {
                let mut buf = vec![0u8; PAGE_SIZE];
                buf[0] = id as u8;
                pager.write(id, &buf).expect("escribe página");
            }
            pager.sync().expect("sync");
        }

        let abre_pool = |capacidad: usize| -> BufferPool<FilePager> {
            let pager = FilePager::open(&path).expect("reabre");
            BufferPool::new(pager, capacidad)
        };

        // PASADA FRÍA: capacidad 8 < 32 páginas ⇒ toda barrido es miss.
        let mut frio = abre_pool(8);
        let antes = frio.metrics();
        assert_eq!(antes.hit_ratio(), 0.0, "arranca sin accesos");
        for id in 0..PAGINAS {
            let pagina = frio.get_page(id).expect("frío lee");
            assert_eq!(pagina[0], id as u8);
            frio.unpin(id, false).expect("unpin");
        }
        let tras_fria: Metrics = frio.metrics();
        assert_eq!(
            tras_fria.buffer_misses - antes.buffer_misses,
            PAGINAS as u64,
            "primera pasada: TODO miss (pool de 8 frames, 32 páginas)"
        );
        assert_eq!(tras_fria.buffer_hits, 0);

        // PASADA CALIENTE: capacidad ≥ páginas ⇒ toda barrido es hit.
        let mut caliente = abre_pool((PAGINAS + 1) as usize);
        for id in 0..PAGINAS {
            // Calentamiento fuera del interés del test.
            caliente.get_page(id).expect("precarga");
            caliente.unpin(id, false).expect("unpin");
        }
        let precargada = caliente.metrics();
        for _ in 0..3 {
            for id in 0..PAGINAS {
                let pagina = caliente.get_page(id).expect("caliente lee");
                assert_eq!(pagina[0], id as u8);
                caliente.unpin(id, false).expect("unpin");
            }
        }
        let final_metrics = caliente.metrics();
        assert_eq!(
            final_metrics.buffer_hits - precargada.buffer_hits,
            3 * PAGINAS as u64,
            "pasadas calientes: TODO hit"
        );
        assert!(
            final_metrics.hit_ratio() > tras_fria.hit_ratio(),
            "hit ratio global sube de {} a {}",
            tras_fria.hit_ratio(),
            final_metrics.hit_ratio()
        );
    }
}
