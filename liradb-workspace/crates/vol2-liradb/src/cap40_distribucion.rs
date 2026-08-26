//! Vol.II — Cap.40: Distribuir una base de datos de grafos.
//!
//! Cuarto y ÚLTIMO capítulo de la Parte VIII y CIERRE del Vol.II. COBRADOR
//! de cuatro deudas explícitas: el hexágono del cap. 36 («¿qué partes del
//! mapa sobreviven a repartir el motor entre máquinas?»), el frente abierto
//! de `informe_produccion()` (cap. 37: distribución/consenso), el gancho
//! saliente del cap. 39 («ya sabes QUÉ calcular y CÓMO leerlo rápido EN UNA
//! máquina; ¿y cuando el grafo no cabe en una?») y el anzuelo del cap. 30
//! ([`GrafoEspera`] construido SIN usar: aquí se enchufa por fin para el
//! deadlock ENTRE particiones).
//!
//! El modelo mental único: **localidad = combustible**. Todo lo que los caps.
//! 7-39 construyen sigue valiendo DENTRO de cada máquina; lo nuevo es la
//! contabilidad de la frontera: cada arista que cruza particiones es UN
//! MENSAJE que pagarás mañana (PowerGraph, Gonzalez et al., OSDI '12).
//!
//! Qué entrega este módulo (contrato §2):
//!
//! 1. **Tres estrategias de particionado** sobre el mismo dataset:
//!    [`particionar_hash`] (FNV-1a del cap. 15 módulo k — reutilizado tal
//!    cual), [`particionar_por_comunidad`] (Louvain del cap. 25 + fusión
//!    determinista de comunidades pequeñas) y
//!    [`particionar_balanceo_codicioso`] (grado descendente, cada nodo a la
//!    partición que menos cortes incrementales paga).
//! 2. **Métricas de corte EXACTAS** ([`metricas_corte`] → [`MetricasCorte`]):
//!    edge cut, nodos-frontera y factor de replicación por DEFINICIÓN
//!    literal, verificables a mano en fixtures de ≤8 nodos.
//! 3. **Vertex cut demostrable** ([`replicar_hub`] → [`InformeVertexCut`]):
//!    la estrella del hub pasa de m cortes a 0 pagando réplicas del centro.
//! 4. **BFS entre particiones contada** ([`bfs_entre_particiones`]): el
//!    coste de red es CONTADOR exacto de mensajes y saltos (misma disciplina
//!    que los pasos WCOJ del cap. 39), jamás latencia inventada.
//! 5. **Hotspots y rebalanceo** ([`carga_por_particion`] +
//!    [`rebalancear`] → [`InformeRebalanceo`]): mover una partición tiene
//!    precio, y se cuenta antes/después.
//! 6. **Raft mínimo DETERMINISTA** ([`EnjambreRaft`]): elecciones con tics
//!    lógicos y timeouts ESCALONADOS FIJOS (sin RNG, sin sleeps, sin hilos),
//!    AppendEntries con retroceso de `siguiente_indice`, compromiso por
//!    mayoría 2-de-3, caída y reconexión (Ongaro-Ousterhout, USENIX ATC '14,
//!    pp. 305-319). El trade-off está DOCUMENTADO: los timeouts escalonados
//!    sacrifican el anti-split-vote aleatorio del paper a cambio de
//!    determinismo total en CI.
//! 7. **COBRO de la deuda cap. 30**: [`fusionar_grafos_espera`] ensambla las
//!    esperas LOCALES de cada partición en un grafo-espera GLOBAL usando
//!    sólo la API pública del cap. 30 (`agregar_espera`/`detectar_ciclo`) —
//!    sin duplicar una línea de su DFS.
//! 8. **Informe reproducible**
//!    ([`informe_distribucion_reproducible_sobre_mini`]): tabla estrategias ×
//!    métricas SIN tiempos (decisión #11 del contrato: sin bench criterion
//!    nuevo — lo medible aquí son enteros exactos).
//!
//! Frontera declarada: capa EDUCATIVA fuera del motor — ni `Executor` ni
//! `GraphStore` se tocan; sin red TCP/RPC real ni serialización; sin runtime
//! asíncrono; sin cambios de pertenencia (joint consensus), compaction ni
//! snapshots; sin tolerancia bizantina; 2PC entre particiones SOLO diseño en
//! prosa (Petrov, *Database Internals*, O'Reilly 2019) anclado al registro
//! Commit del WAL (cap. 28). Atribución según ADR-001: Kùzu/LadybugDB son
//! motores EMBEBIDOS mononodo (Jin et al., CIDR 2023, CC-BY 4.0) — el
//! contraste honesto que cierra el volumen.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::cap08_graph_store::{GraphStore, MemoryStore};
use crate::cap15_indices::fnv1a_64;
use crate::cap22_caminos_minimos::WeightSource;
use crate::cap25_comunidades::louvain;
use crate::cap30_mvcc::GrafoEspera;

// ─────────────────── La asignación de particiones ───────────────────

/// Dueño de cada nodo: `dueno[node_id]` = índice de partición (0..k).
///
/// Es el contrato mínimo del sharding por NODOS: cada vértice vive en
/// EXACTAMENTE una máquina (edge cut — PowerGraph, OSDI '12, §2). Las aristas
/// cuyos extremos tienen dueños distintos son los CORTES, y todo el coste del
/// capítulo se deriva de contarlos.
///
/// Los ids de nodo son densos (0..n) como en todo LiraDB, así que un
/// `Vec<u32>` basta: sin maps, sin hashing secundario, una lectura por acceso.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsignacionParticion {
    /// Número de particiones declaradas (k).
    pub num_particiones: usize,
    /// Dueño de cada nodo, indexado por node id. Valores siempre `< num_particiones`.
    pub dueno: Vec<u32>,
}

impl AsignacionParticion {
    /// Asignación trivial: todos los nodos en la partición 0 (una sola
    /// máquina — el punto de partida del volumen: cortes = 0).
    pub fn monolito(n: usize) -> Self {
        AsignacionParticion {
            num_particiones: 1,
            dueno: vec![0; n],
        }
    }

    /// Dueño del nodo `id`, o `None` si el id está fuera de rango.
    pub fn dueno_de(&self, id: u32) -> Option<u32> {
        self.dueno.get(id as usize).copied()
    }

    /// Nodos asignados a la partición `p`, en orden ascendente de id.
    pub fn nodos_de(&self, p: usize) -> Vec<u32> {
        self.dueno
            .iter()
            .enumerate()
            .filter(|&(_, d)| d == &(p as u32))
            .map(|(id, _)| id as u32)
            .collect()
    }

    /// Tamaño de cada partición (longitud `num_particiones`; las vacías cuentan 0).
    pub fn tamanos(&self) -> Vec<usize> {
        let mut t = vec![0usize; self.num_particiones];
        for &d in &self.dueno {
            t[d as usize] += 1;
        }
        t
    }
}

// ─────────────────── Helpers de terreno ───────────────────

/// Extrae las aristas del store como pares `(source, target)`, ORDENADAS para
/// determinismo total (`iter_edges` no garantiza orden — ley de la casa).
pub fn aristas_de_store(store: &MemoryStore) -> Vec<(u32, u32)> {
    let mut aristas: Vec<(u32, u32)> = store
        .iter_edges()
        .map(|e| (e.source as u32, e.target as u32))
        .collect();
    aristas.sort_unstable();
    aristas
}

/// Adyacencia SIMÉTRICA (no dirigida) densa: `adj[u]` = vecinos de u, en
/// orden de llegada de las aristas.
///
/// Para métricas de corte y travesías el sentido de la arista es ruido: lo
/// que importa es QUIÉN es vecino de QUIÉN. Duplicados conservados (dos
/// aristas paralelas son DOS cortes potenciales).
pub fn adyacencia_simetrica(aristas: &[(u32, u32)], n: usize) -> Vec<Vec<u32>> {
    let mut adj: Vec<Vec<u32>> = vec![Vec::new(); n];
    for &(a, b) in aristas {
        adj[a as usize].push(b);
        adj[b as usize].push(a);
    }
    adj
}

/// Grado NO dirigido de cada nodo según las aristas dadas.
pub fn grados_desde_aristas(aristas: &[(u32, u32)], n: usize) -> Vec<u32> {
    let mut grados = vec![0u32; n];
    for &(a, b) in aristas {
        grados[a as usize] += 1;
        grados[b as usize] += 1;
    }
    grados
}

/// Vecinos (no dirigidos) de un nodo leídos del store, o `None` si no existe.
fn vecinos_de_nodo(store: &dyn GraphStore, id: usize) -> Option<Vec<usize>> {
    store.get_node(id)?;
    let mut v = Vec::new();
    for eid in store.out_edges(id) {
        if let Some(e) = store.get_edge(eid) {
            v.push(e.target);
        }
    }
    for eid in store.in_edges(id) {
        if let Some(e) = store.get_edge(eid) {
            v.push(e.source);
        }
    }
    Some(v)
}

// ─────────────────── Estrategia 1: hash módulo k ───────────────────

/// Particionado por HASH: `dueño(nodo) = fnv1a_64(bytes_le(id)) % k`.
///
/// Reutiliza el FNV-1a del cap. 15 TAL CUAL (la misma función que reparte
/// claves en buckets del HashIndex) — sharding con cero código criptográfico
/// nuevo. Balance O(1) por construcción estadística… y la factura que abre el
/// resto del capítulo: el hash ignora POR COMPLETO la topología, así que cada
/// vecindario queda desparramado y casi toda travesía cruza la frontera.
///
/// Elección empírica documentada: sobre ids secuenciales, FNV-1a módulo 8
/// reparte 400 nodos en buckets de EXACTAMENTE 50 (medido en
/// `hash_modulo_k_asigna_todo_y_balancea`, no prometido).
///
/// `k` debe ser ≥ 1; panic si no (contrato firmado, no error silencioso).
pub fn particionar_hash(n: usize, k: u32) -> AsignacionParticion {
    assert!(k >= 1, "k debe ser >= 1");
    let mut dueno = Vec::with_capacity(n);
    for id in 0..n as u32 {
        let h = fnv1a_64(&id.to_le_bytes());
        dueno.push((h % k as u64) as u32);
    }
    AsignacionParticion {
        num_particiones: k as usize,
        dueno,
    }
}

// ─────────────────── Estrategia 2: comunidades de Louvain ───────────────────

/// Particionado por COMUNIDAD: Louvain del cap. 25 detecta los grupos y cada
/// comunidad entera se convierte en partición.
///
/// REUTILIZA [`louvain`] tal cual (spacing puro: la detección ya es maquinaria
/// probada — aquí sólo se USA como política de colocación). Cuando Louvain
/// encuentra MÁS comunidades que `k`, las pequeñas se FUNDEN hasta bajar a k:
///
/// - Orden de fusión: primero la comunidad más PEQUEÑA (empate → menor
///   miembro), repetidamente.
/// - Destino: la comunidad vecina con la que comparte MÁS ARISTAS (empate →
///   la de menor miembro). Fundirse con quien ya te hablas minimiza los
///   cortes nuevos — la intuición greedy de siempre, aplicada a colocación.
///
/// Si Louvain encuentra MENOS de k comunidades, se devuelven las que haya
/// (particiones vacías honestas mejor que partir una comunidad por la mitad,
/// que violaría «la misma comunidad nunca acaba en particiones distintas»).
/// Numeración final: particiones renumeradas 0..C-1 por menor miembro, la
/// MISMA convención de renumeración de [`crate::Particion`].
///
/// Parámetros de Louvain fijados: pesos constantes 1.0 (grafo no ponderado),
/// γ = 1.0 (resolución clásica), 30 pasadas máximo por nivel.
pub fn particionar_por_comunidad(store: &dyn GraphStore, k: usize) -> AsignacionParticion {
    assert!(k >= 1, "k debe ser >= 1");
    let resultado = louvain(store, &WeightSource::Constant(1.0), 1.0, 30)
        .expect("parámetros fijos de Louvain válidos por construcción");
    let n = store.node_count();
    let mut grupos = resultado.comunidades();

    // Fusión determinista mientras haya más grupos que particiones.
    while grupos.len() > k {
        // El más pequeño primero (empate → menor miembro: los miembros de
        // cada grupo vienen en orden ascendente, basta comparar el primero).
        let origen = (0..grupos.len())
            .min_by_key(|&i| (grupos[i].len(), grupos[i][0]))
            .expect("hay al menos un grupo");
        let pequena = grupos.remove(origen);

        // Peso de aristas del grupo pequeño hacia CADA grupo restante.
        let grupo_de: HashMap<usize, usize> = grupos
            .iter()
            .enumerate()
            .flat_map(|(gi, miembros)| miembros.iter().map(move |&m| (m, gi)))
            .collect();
        let mut peso: HashMap<usize, usize> = HashMap::new();
        for &miembro in &pequena {
            if let Some(vecinos) = vecinos_de_nodo(store, miembro) {
                for v in vecinos {
                    if let Some(&gi) = grupo_de.get(&v) {
                        *peso.entry(gi).or_insert(0) += 1;
                    }
                }
            }
        }
        // Destino: más aristas compartidas; empate → menor miembro del grupo.
        let destino = (0..grupos.len())
            .min_by_key(|&gi| {
                (
                    std::cmp::Reverse(peso.get(&gi).copied().unwrap_or(0)),
                    grupos[gi][0],
                )
            })
            .expect("queda al menos un grupo");
        grupos[destino].extend_from_slice(&pequena);
        grupos[destino].sort_unstable();
    }

    // Renumerar particiones por menor miembro (convención del cap. 25).
    grupos.sort_by_key(|g| g[0]);
    let mut dueno = vec![0u32; n];
    for (pi, miembros) in grupos.iter().enumerate() {
        for &m in miembros {
            dueno[m] = pi as u32;
        }
    }
    AsignacionParticion {
        num_particiones: k,
        dueno,
    }
}

// ─────────────────── Estrategia 3: balanceo codicioso ───────────────────

/// Particionado GREEDY por balanceo: nodos por GRADO DESCENDENTE (empate →
/// menor id), cada uno aterriza en la partición donde paga MENOS cortes
/// incrementales (# de vecinos YA COLOCADOS fuera de esa partición; empate →
/// índice menor).
///
/// Es el greedy clásico de particionado de grafos: los hubs se colocan
/// PRIMERO (cuando aún pueden elegir dónde vivir baratos) y sus hojas después
/// tienden a seguirlos. Intermedio medido entre hash (balance perfecto,
/// localidad nula) y comunidad (localidad máxima, balance regalado).
///
/// El universo de nodos son los EXTREMOS de las aristas dadas: un nodo
/// aislado no aporta cortes ni carga y queda fuera (documentado, no oculto).
pub fn particionar_balanceo_codicioso(aristas: &[(u32, u32)], k: usize) -> AsignacionParticion {
    assert!(k >= 1, "k debe ser >= 1");
    let mut max_id = 0usize;
    for &(a, b) in aristas {
        max_id = max_id.max(a as usize).max(b as usize);
    }
    let n = max_id + 1;
    let grados = grados_desde_aristas(aristas, n);
    let adj = adyacencia_simetrica(aristas, n);

    let mut orden: Vec<u32> = (0..n as u32).filter(|&v| grados[v as usize] > 0).collect();
    orden.sort_unstable_by(|&x, &y| grados[y as usize].cmp(&grados[x as usize]).then(x.cmp(&y)));

    let mut dueno = vec![0u32; n];
    let mut colocado = vec![false; n];
    for &v in &orden {
        let mut costes = vec![0u32; k];
        for &w in &adj[v as usize] {
            if colocado[w as usize] {
                costes[dueno[w as usize] as usize] += 1;
            }
        }
        let mejor = (0..k).min_by_key(|&p| (costes[p], p)).expect("k >= 1");
        dueno[v as usize] = mejor as u32;
        colocado[v as usize] = true;
    }

    AsignacionParticion {
        num_particiones: k,
        dueno,
    }
}

// ─────────────────── Métricas de corte (definiciones PowerGraph) ───────────────────

/// Las tres facturas de un particionado + el balance, por DEFINICIÓN literal.
///
/// Definiciones (Gonzalez et al., OSDI '12, §2):
/// - `cortes_arista`: aristas cuyos extremos tienen dueños distintos. Cada
///   una es UN MENSAJE por cada travesía que la cruce.
/// - `nodos_frontera`: nodos incidentes a al menos un corte. Su estado debe
///   sincronizarse entre máquinas.
/// - `factor_replicacion`: Σ réplicas / n. Con colocación edge-cut pura cada
///   nodo vive UNA vez (el factor bruto sería 1.0 siempre — trivial), así que
///   aquí se cuenta el factor OPERATIVO del gather de PowerGraph: cuántas
///   copias del nodo harían falta para evaluarlo LOCALMENTE en cada máquina
///   que aloja un vecino suyo. réplica(v) = 1 + nº de particiones VECINAS
///   distintas a la propia. Los nodos interiores pagan 1; la frontera paga
///   por cada máquina con la que habla.
/// - `tam_max` / `tam_min`: mayor y menor partición (las vacías cuentan 0 —
///   el desbalance de Louvain se MUESTRA, no se esconde).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricasCorte {
    /// Aristas con extremos en particiones distintas.
    pub cortes_arista: u64,
    /// Nodos incidentes a al menos un corte.
    pub nodos_frontera: u64,
    /// Σ réplicas operativas / n (ver doc del struct).
    pub factor_replicacion: f64,
    /// Mayor partición.
    pub tam_max: usize,
    /// Menor partición (vacía cuenta 0).
    pub tam_min: usize,
}

/// Calcula [`MetricasCorte`] de una asignación sobre unas aristas dadas.
///
/// Las aristas se tratan como NO dirigidas para cortes, frontera y réplicas
/// (cortar b–c afecta a ambos igual que c–b). Coste O(n + m).
pub fn metricas_corte(asignacion: &AsignacionParticion, aristas: &[(u32, u32)]) -> MetricasCorte {
    let n = asignacion.dueno.len();
    let adj = adyacencia_simetrica(aristas, n);

    let mut cortes_dobles = 0u64;
    let mut frontera = vec![false; n];
    let mut replicas = 0u64;
    for v in 0..n {
        let propia = asignacion.dueno[v];
        let mut ajenas: HashSet<u32> = HashSet::new();
        for &w in &adj[v] {
            let dueno_w = asignacion.dueno[w as usize];
            if dueno_w != propia {
                cortes_dobles += 1;
                frontera[v] = true;
                frontera[w as usize] = true;
                ajenas.insert(dueno_w);
            }
        }
        replicas += 1 + ajenas.len() as u64;
    }

    let tamanos = asignacion.tamanos();
    MetricasCorte {
        // Cada corte no dirigido se conta DOS veces (una por extremo).
        cortes_arista: cortes_dobles / 2,
        nodos_frontera: frontera.iter().filter(|&&f| f).count() as u64,
        factor_replicacion: replicas as f64 / n as f64,
        tam_max: *tamanos.iter().max().unwrap_or(&0),
        tam_min: *tamanos.iter().min().unwrap_or(&0),
    }
}

// ─────────────────── Vertex cut: la estrella del hub ───────────────────

/// Resultado del vertex cut del hub sobre su estrella.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InformeVertexCut {
    /// Cortes de arista ANTES de replicar (colocación edge-cut inicial).
    pub cortes_antes: u64,
    /// Cortes DESPUÉS (0 por construcción: cada hoja guarda su arista local).
    pub cortes_despues: u64,
    /// Réplicas del hub: una por cada partición que aloje al menos una hoja.
    pub replicas_hub: usize,
}

/// Vertex cut del HUB sobre su estrella: el contraejemplo que motivó PowerGraph.
///
/// Situación INICIAL (edge cut, determinista y dibujable): el hub vive SOLO en
/// la partición 0 y cada hoja `j` cae en la partición `j % k` (round-robin).
/// Toda hoja fuera de la partición 0 deja su arista CORTADA.
///
/// La cura (vertex cut): se invierte el cuchillo — el hub se REPLICA en todas
/// las particiones que guardan hojas y cada hoja conserva su arista local.
/// Los m cortes se vuelven 0… pagando réplicas del centro: el skew no
/// desaparece al repartir, se muda de RAM a RED (eco del ×521 del cap. 39).
///
/// Devuelve las cifras ANTES/DESPUÉS para comparar facturas, no promesas.
/// `vecinos` son las hojas (los duplicados cuentan: son aristas también).
pub fn replicar_hub(hub: u32, vecinos: &[u32], k: usize) -> InformeVertexCut {
    let _ = hub; // el centro vive en P0 por definición del baseline
    assert!(k >= 1, "k debe ser >= 1");
    let cortes_antes = vecinos
        .iter()
        .filter(|&&j| !(j as usize).is_multiple_of(k))
        .count() as u64;
    let particiones_con_hojas: HashSet<usize> = vecinos.iter().map(|&j| j as usize % k).collect();
    InformeVertexCut {
        cortes_antes,
        cortes_despues: 0,
        replicas_hub: particiones_con_hojas.len(),
    }
}

// ─────────────────── BFS entre particiones ───────────────────

/// Resultado de una BFS que CRUZA particiones: alcanzables + factura de red.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultadoBfsDistribuido {
    /// Nodos alcanzables, ascendentes. IDÉNTICOS a los del BFS local: la
    /// distribución cambia el COSTE, jamás el RESULTADO (test-tesis).
    pub visitados: Vec<u32>,
    /// Mensajes de red: una transferencia por cada descubrimiento cuya víctima
    /// vive en OTRA partición. Contador exacto de trabajo.
    pub mensajes_red: u64,
    /// Saltos de red: niveles BFS (supersteps — vocabulario Pregel, SIGMOD
    /// '10) en los que cruzó AL MENOS un mensaje.
    pub saltos_red: u64,
}

/// BFS síncrona por supersteps sobre particiones: en cada nivel, cada
/// partición procesa SU lote local y cuando descubre un nodo ajeno envía UN
/// mensaje a su dueña, que lo procesará en el nivel SIGUIENTE.
///
/// Modelo BSP a lo Pregel (SIGMOD '10) con doble buffer de colas: los
/// mensajes enviados durante el nivel L llegan para el nivel L+1 — jamás se
/// procesan en el mismo nivel en que viajaron. `saltos_red` cuenta los
/// niveles con tráfico cruzado: la versión honesta de «latencia», sin
/// inventar milisegundos (decisión #8 del contrato: contar es reproducible,
/// cronometrar simulado es humo).
///
/// `adj` es la adyacencia simétrica densa ([`adyacencia_simetrica`]).
pub fn bfs_entre_particiones(
    adj: &[Vec<u32>],
    asignacion: &AsignacionParticion,
    origen: u32,
) -> ResultadoBfsDistribuido {
    let n = adj.len();
    let mut visitado = vec![false; n];
    let mut actual: Vec<VecDeque<u32>> = vec![VecDeque::new(); asignacion.num_particiones];
    let mut siguiente: Vec<VecDeque<u32>> = vec![VecDeque::new(); asignacion.num_particiones];
    let o = origen as usize;
    visitado[o] = true;
    actual[asignacion.dueno[o] as usize].push_back(origen);

    let mut mensajes = 0u64;
    let mut saltos = 0u64;
    loop {
        let hay_trabajo = actual.iter().any(|c| !c.is_empty());
        if !hay_trabajo {
            break;
        }
        let mut cruzo_red = false;
        for p in 0..asignacion.num_particiones {
            let lote: Vec<u32> = actual[p].drain(..).collect();
            for u in lote {
                for &w in &adj[u as usize] {
                    let wi = w as usize;
                    if visitado[wi] {
                        continue;
                    }
                    visitado[wi] = true;
                    let dueno_w = asignacion.dueno[wi] as usize;
                    if dueno_w == p {
                        siguiente[p].push_back(w);
                    } else {
                        mensajes += 1;
                        cruzo_red = true;
                        siguiente[dueno_w].push_back(w);
                    }
                }
            }
        }
        if cruzo_red {
            saltos += 1;
        }
        std::mem::swap(&mut actual, &mut siguiente);
    }

    let mut visitados: Vec<u32> = (0..n).filter(|&i| visitado[i]).map(|i| i as u32).collect();
    visitados.sort_unstable();
    ResultadoBfsDistribuido {
        visitados,
        mensajes_red: mensajes,
        saltos_red: saltos,
    }
}

// ─────────────────── Hotspots y rebalanceo ───────────────────

/// Carga por partición: suma de grados de SUS nodos.
///
/// La métrica del hotspot: en grafos power-law el hub puede concentrar solo
/// una fracción enorme de la carga de SU partición (conexión directa con la
/// centralidad del cap. 24 y los hubs del cap. 34).
pub fn carga_por_particion(asignacion: &AsignacionParticion, grados: &[u32]) -> Vec<u64> {
    let mut carga = vec![0u64; asignacion.num_particiones];
    for (id, &d) in asignacion.dueno.iter().enumerate() {
        carga[d as usize] += grados[id] as u64;
    }
    carga
}

/// Informe de un rebalanceo: qué se movió y cuánto costó.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InformeRebalanceo {
    /// Nodos trasladados (todo el contenido de la partición movida).
    pub nodos_movidos: usize,
    /// Aristas con UN extremo en la partición movida: todas cambian de
    /// encaminamiento aunque algunas dejen de ser cortes.
    pub aristas_retocadas: usize,
    /// Cortes antes del movimiento.
    pub cortes_antes: u64,
    /// Cortes después. Mover P hacia Q SÓLO cambia las aristas P–Q (dejan de
    /// cortarse); las P–X siguen cortándose hacia otro destino. Por eso
    /// `cortes_antes − cortes_despues == aristas entre P y Q`.
    pub cortes_despues: u64,
}

/// Rebalanceo: mover la partición COMPLETA `particion_a_mover` sobre otra
/// (consolidación: su host desaparece y sus nodos aterrizan en el host menos
/// cargado; empate → índice menor). MUTA la asignación y devuelve el informe.
///
/// Determinista por construcción: destino = partición distinta de menor carga
/// (grados). El coste se cuenta ANTES/DESPUÉS con la MISMA regla
/// ([`metricas_corte`]) — nada de estimaciones.
pub fn rebalancear(
    asignacion: &mut AsignacionParticion,
    aristas: &[(u32, u32)],
    particion_a_mover: usize,
) -> InformeRebalanceo {
    assert!(
        particion_a_mover < asignacion.num_particiones,
        "partición a mover fuera de rango"
    );
    assert!(asignacion.num_particiones >= 2, "no hay a dónde mover");

    let n = asignacion.dueno.len();
    let grados = grados_desde_aristas(aristas, n);
    let cortes_antes = metricas_corte(asignacion, aristas).cortes_arista;

    let carga = carga_por_particion(asignacion, &grados);
    let destino = (0..asignacion.num_particiones)
        .filter(|&p| p != particion_a_mover)
        .min_by_key(|&p| (carga[p], p))
        .expect("hay otra partición");

    let movidos = asignacion.nodos_de(particion_a_mover);
    let retocadas = aristas
        .iter()
        .filter(|&&(a, b)| {
            let da = asignacion.dueno[a as usize];
            let db = asignacion.dueno[b as usize];
            (da as usize == particion_a_mover) != (db as usize == particion_a_mover)
        })
        .count();

    for m in movidos.iter() {
        asignacion.dueno[*m as usize] = destino as u32;
    }
    let cortes_despues = metricas_corte(asignacion, aristas).cortes_arista;

    InformeRebalanceo {
        nodos_movidos: movidos.len(),
        aristas_retocadas: retocadas,
        cortes_antes,
        cortes_despues,
    }
}

// ─────────────────── Raft mínimo determinista ───────────────────

/// Rol de un nodo Raft (ATC '14, fig. 1): Seguidor pasivo, Candidato en
/// elección, o Líder sirviendo el log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RolRaft {
    /// Sin líder reconocido: acepta propuestas de otros, vota.
    Seguidor,
    /// Ha pedido votos para su término actual.
    Candidato,
    /// Ganó la mayoría: replica el log y responde a clientes.
    Lider,
}

/// Una entrada del log replicado: término en que fue creada + comando.
///
/// `comando` es un entero (id de operación) para poder comparar logs BYTE A
/// BYTE en los tests — el contenido real viajaría serializado (fuera de
/// alcance: sin red ni serialización en este capítulo).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntradaLog {
    /// Término en que el líder creó la entrada.
    pub termino: u64,
    /// Comando opaco (id de operación).
    pub comando: u64,
}

/// Mensajes del protocolo: petición y concesión de voto, AppendEntries
/// ([`MensajeRaft::Entradas`]) y su acuse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MensajeRaft {
    /// Candidato → todos: «vota por mí para este término».
    PideVoto {
        /// Término de la candidatura.
        termino: u64,
        /// Id del candidato.
        candidato: u32,
    },
    /// Respuesta de voto: `candidato: Some(c)` concede, `None` rechaza.
    Voto {
        /// Término del votante (puede ser MAYOR que el pedido: «estás anticuado»).
        termino: u64,
        /// A quién concede (si concede).
        candidato: Option<u32>,
    },
    /// Líder → seguidor: AppendEntries (con `entradas` vacías es un LATIDO).
    Entradas {
        /// Término del líder.
        termino: u64,
        /// Id del líder.
        lider: u32,
        /// Índice (1-based) de la entrada ANTERIOR a las que viajan.
        prev_indice: usize,
        /// Término de esa entrada anterior (0 si `prev_indice == 0`).
        prev_termino: u64,
        /// Entradas que siguen a `prev_indice` (vacío = latido puro).
        entradas: Vec<EntradaLog>,
        /// Índice de compromiso del líder (los seguidores lo adoptan).
        compromiso_lider: usize,
    },
    /// Seguidor → líder: acuse de recibo de AppendEntries.
    Acuse {
        /// Término del seguidor.
        termino: u64,
        /// Id del seguidor.
        seguidor: u32,
        /// ¿Coincidía el prefijo (`prev_indice`, `prev_termino`)?
        exito: bool,
        /// Si `exito`: último índice coincidente tras aplicar. Si falla: 0.
        coincide_hasta: usize,
    },
}

/// Un nodo Raft con su ESTADO PERSISTENTE (término, voto, log — lo que
/// sobrevive a una caída) y su estado volátil (rol, temporizadores, índices).
#[derive(Debug, Clone)]
pub struct NodoRaft {
    /// Id del nodo en el clúster.
    pub id: u32,
    /// Rol actual.
    pub rol: RolRaft,
    /// Término actual (persistente: monótono creciente).
    pub termino: u64,
    /// A quién votó en ESTE término (persistente; `None` = nadie aún).
    pub voto_de: Option<u32>,
    /// Log replicado (persistente): `log[i]` es la entrada de índice i+1.
    pub log: Vec<EntradaLog>,
    /// Índice de la última entrada COMPROMETIDA (replicada en mayoría).
    pub indice_compromiso: usize,
    /// Timeout de ELECCIÓN en tics lógicos — ESCALONADO FIJO por nodo
    /// (decisión #9: sacrificio DOCUMENTADO del anti-split-vote aleatorio
    /// del paper a cambio de determinismo total).
    pub timeout_tics: u64,
    /// Tics desde el último latido/líder visto (volátil).
    pub tics_sin_lider: u64,
    /// ¿Está CAÍDO? Los caídos no tican ni reciben ni envían; conservan su
    /// estado persistente (semántica de disco de Raft).
    pub vivo: bool,
    /// (Candidato) votos conseguidos en la candidatura actual.
    votos_recibidos: u32,
    /// (Líder) próximo índice a ENVIAR a cada seguidor: 1-based, arranca en
    /// `log.len()+1` y RETROCEDE ante desacuerdo de prefijo.
    siguiente_indice: HashMap<u32, usize>,
    /// (Líder) último índice confirmado por acuse de cada seguidor.
    confirmado_hasta: HashMap<u32, usize>,
}

/// Huella de «log al día» de las elecciones (ATC '14 §5.4.1): el par
/// (último término, longitud) se compara lexicográficamente — gana el voto
/// quien no va retrasado.
fn huella_log(log: &[EntradaLog]) -> (u64, usize) {
    (log.last().map(|e| e.termino).unwrap_or(0), log.len())
}

/// Un clúster Raft completo sobre un BUS FIFO determinista.
///
/// Nada de hilos, sockets ni RNG: la red es un `VecDeque` y el tiempo son
/// TICS lógicos. Un [`EnjambreRaft::tic`] hace, EN ORDEN:
///
/// 1. Avanza los relojes de los nodos VIVOS (los caídos quedan congelados).
/// 2. El líder vivo emite LATIDOS periódicos (AppendEntries desde su
///    `siguiente_indice` — vacíos si el seguidor está al día) cada
///    [`EnjambreRaft::latido_cada_tics`] tics.
/// 3. Todo seguidor/candidato cuyo `tics_sin_lider` alcanza su `timeout_tics`
///    escalonado inicia ELECCIÓN (término+1, voto propio, PideVoto).
/// 4. Drena el bus en orden FIFO; los mensajes hacia muertos se pierden (la
///    red no filtra: el receptor no existe). Las respuestas encoladas durante
///    el drenaje se entregan en el MISMO tic — el protocolo responde A LO SUMO
///    una vez por mensaje recibido, así que el drenaje TERMINA.
///
/// Mayoría = sobre el TAMAÑO DEL CLÚSTER (2-de-3), no sobre los vivos: por
/// eso una minoría caída no bloquea y una mayoría caída SÍ (test-tesis).
pub struct EnjambreRaft {
    /// Nodos del clúster, en el orden de ids dado en [`EnjambreRaft::nuevo`].
    pub nodos: Vec<NodoRaft>,
    /// Tics entre latidos del líder (fijo, determinista).
    pub latido_cada_tics: u64,
    /// Tics transcurridos desde el último latido emitido.
    pub tics_desde_latido: u64,
    /// Bus FIFO: (destino, origen, mensaje).
    bus: VecDeque<(u32, u32, MensajeRaft)>,
    /// Reloj lógico global (total de tics ejecutados).
    pub tics_totales: u64,
}

impl EnjambreRaft {
    /// Clúster con timeouts ESCALONADOS FIJOS: el nodo en posición i recibe
    /// `base_tics + i * escalon_tics` — el primero en impacientarse es SIEMPRE
    /// el de posición menor (determinismo: primera elección predecible).
    pub fn nuevo(ids: &[u32], base_tics: u64, escalon_tics: u64) -> Self {
        let nodos = ids
            .iter()
            .enumerate()
            .map(|(i, &id)| NodoRaft {
                id,
                rol: RolRaft::Seguidor,
                termino: 0,
                voto_de: None,
                log: Vec::new(),
                indice_compromiso: 0,
                timeout_tics: base_tics + i as u64 * escalon_tics,
                tics_sin_lider: 0,
                vivo: true,
                votos_recibidos: 0,
                siguiente_indice: HashMap::new(),
                confirmado_hasta: HashMap::new(),
            })
            .collect();
        EnjambreRaft {
            nodos,
            latido_cada_tics: 5,
            tics_desde_latido: 0,
            bus: VecDeque::new(),
            tics_totales: 0,
        }
    }

    /// Posición del nodo `id` en [`Self::nodos`].
    fn pos(&self, id: u32) -> usize {
        self.nodos
            .iter()
            .position(|n| n.id == id)
            .expect("id existe")
    }

    /// Vista inmutable del nodo `id`.
    pub fn nodo(&self, id: u32) -> &NodoRaft {
        &self.nodos[self.pos(id)]
    }

    /// Id del líder vivo, si lo hay.
    pub fn lider(&self) -> Option<u32> {
        self.nodos
            .iter()
            .find(|n| n.rol == RolRaft::Lider && n.vivo)
            .map(|n| n.id)
    }

    /// Tamaño del clúster (la mayoría se calcula sobre ÉL).
    pub fn tamano_cluster(&self) -> u32 {
        self.nodos.len() as u32
    }

    /// CAÍDA de un nodo: deja de ticar, recibir y enviar; conserva término,
    /// voto y log (el disco sobrevive aunque la máquina no).
    pub fn caer(&mut self, id: u32) {
        let p = self.pos(id);
        self.nodos[p].vivo = false;
    }

    /// RECONEXIÓN: vuelve como SEGUIDOR con su estado persistente intacto y
    /// el temporizador a cero. Se pondrá al día con AppendEntries SOLOS.
    pub fn revivir(&mut self, id: u32) {
        let p = self.pos(id);
        let n = &mut self.nodos[p];
        n.vivo = true;
        n.rol = RolRaft::Seguidor;
        n.tics_sin_lider = 0;
        n.votos_recibidos = 0;
    }

    /// Encola un mensaje en el bus (FIFO).
    fn encolar(&mut self, destino: u32, origen: u32, mensaje: MensajeRaft) {
        self.bus.push_back((destino, origen, mensaje));
    }

    /// Ids de los demás nodos (todos: el bus filtra a los muertos al entregar).
    fn ids_otros(&self, yo: u32) -> Vec<u32> {
        self.nodos
            .iter()
            .map(|n| n.id)
            .filter(|&i| i != yo)
            .collect()
    }

    /// Avanza un tic lógico: relojes → latidos → elecciones → drenaje.
    pub fn tic(&mut self) {
        self.tics_totales += 1;

        // 1) Relojes de los vivos que no son líderes.
        for n in &mut self.nodos {
            if n.vivo && n.rol != RolRaft::Lider {
                n.tics_sin_lider += 1;
            }
        }

        // 2) Latidos periódicos del líder vivo.
        if self.lider().is_some() {
            self.tics_desde_latido += 1;
            if self.tics_desde_latido >= self.latido_cada_tics {
                self.tics_desde_latido = 0;
                self.latidos();
            }
        }

        // 3) Elecciones: snapshot de impacientados antes de mutar nada.
        let impacientados: Vec<u32> = self
            .nodos
            .iter()
            .filter(|n| n.vivo && n.rol != RolRaft::Lider && n.tics_sin_lider >= n.timeout_tics)
            .map(|n| n.id)
            .collect();
        for id in impacientados {
            self.iniciar_eleccion(id);
        }

        // 4) Drenar el bus (respuestas incluidas, hasta vaciar).
        self.drenar_bus();
    }

    /// Muchos tics de golpe (comodidad de tests).
    pub fn tics(&mut self, n: u64) {
        for _ in 0..n {
            self.tic();
        }
    }

    /// El líder propone un comando: lo añade a SU log y dispara AppendEntries.
    /// Devuelve `false` si no hay líder vivo — sin mayoría no hay servicio
    /// (Raft garantiza consistencia, no disponibilidad: el test-tesis lo usa).
    pub fn proponer(&mut self, comando: u64) -> bool {
        let lider = match self.lider() {
            Some(l) => l,
            None => return false,
        };
        let p = self.pos(lider);
        let termino = self.nodos[p].termino;
        self.nodos[p].log.push(EntradaLog { termino, comando });
        let ids = self.ids_otros(lider);
        for destino in ids {
            self.enviar_entradas_a(lider, destino);
        }
        true
    }

    /// Construye y encola un AppendEntries del líder `lider` al seguidor
    /// `destino`: prefijo en `siguiente_indice - 1` + TODO el sufijo restante
    /// del log. Con el sufijo completo, el mismo mecanismo sirve de latido
    /// (seguidor al día ⇒ entradas vacías), de réplica (entrada nueva) y de
    /// puesta al día (rezagado que recibe lo comprometido ya).
    fn enviar_entradas_a(&mut self, lider: u32, destino: u32) {
        let pl = self.pos(lider);
        let n = &self.nodos[pl];
        let termino = n.termino;
        let compromiso = n.indice_compromiso;
        let siguiente = n
            .siguiente_indice
            .get(&destino)
            .copied()
            .unwrap_or(n.log.len() + 1);
        let prev_indice = siguiente - 1;
        let prev_termino = if prev_indice == 0 {
            0
        } else {
            n.log[prev_indice - 1].termino
        };
        let entradas: Vec<EntradaLog> = n.log[prev_indice..].to_vec();
        let mensaje = MensajeRaft::Entradas {
            termino,
            lider,
            prev_indice,
            prev_termino,
            entradas,
            compromiso_lider: compromiso,
        };
        self.encolar(destino, lider, mensaje);
    }

    /// Latido del líder: AppendEntries hacia cada seguidor (ver
    /// [`Self::enviar_entradas_a`]: si está al día viajan cero entradas).
    fn latidos(&mut self) {
        let lider = match self.lider() {
            Some(l) => l,
            None => return,
        };
        let ids = self.ids_otros(lider);
        for destino in ids {
            self.enviar_entradas_a(lider, destino);
        }
    }

    /// Elección iniciada por `id`: Candidato, término+1, voto propio, PideVoto.
    fn iniciar_eleccion(&mut self, id: u32) {
        let p = self.pos(id);
        let n = &mut self.nodos[p];
        n.rol = RolRaft::Candidato;
        n.termino += 1;
        n.voto_de = Some(id);
        n.votos_recibidos = 1;
        n.tics_sin_lider = 0;
        let termino = n.termino;
        let ids = self.ids_otros(id);
        for destino in ids {
            self.encolar(
                destino,
                id,
                MensajeRaft::PideVoto {
                    termino,
                    candidato: id,
                },
            );
        }
    }

    /// Entrega TODA la cola del bus en orden FIFO; los mensajes dirigidos a
    /// nodos caídos se pierden.
    fn drenar_bus(&mut self) {
        while let Some((destino, origen, mensaje)) = self.bus.pop_front() {
            let pd = self.pos(destino);
            if !self.nodos[pd].vivo {
                continue;
            }
            self.entregar(destino, origen, mensaje);
        }
    }

    /// Procesa un mensaje entregado. Cada rama genera A LO SUMO una respuesta:
    /// por eso el drenaje termina (sin bucles infinitos de mensajes).
    fn entregar(&mut self, destino: u32, origen: u32, mensaje: MensajeRaft) {
        match mensaje {
            MensajeRaft::PideVoto { termino, candidato } => {
                self.recibir_pide_voto(destino, termino, candidato);
            }
            MensajeRaft::Voto { termino, candidato } => {
                self.recibir_voto(destino, termino, candidato);
            }
            MensajeRaft::Entradas {
                termino,
                lider,
                prev_indice,
                prev_termino,
                entradas,
                compromiso_lider,
            } => {
                self.recibir_entradas(
                    destino,
                    termino,
                    lider,
                    prev_indice,
                    prev_termino,
                    entradas,
                    compromiso_lider,
                );
            }
            MensajeRaft::Acuse {
                termino,
                seguidor,
                exito,
                coincide_hasta,
            } => {
                let _ = origen;
                self.recibir_acuse(destino, termino, seguidor, exito, coincide_hasta);
            }
        }
    }

    /// Regla de concesión de voto (ATC '14 §5.2 + §5.4.1): término correcto,
    /// voto libre (o ya concedido a este candidato) y log del candidato AL
    /// MENOS tan actualizado como el nuestro.
    ///
    /// Simplificación DOCUMENTADA del modelo educativo: el PideVoto del paper
    /// transporta la huella del candidato; aquí el votante la consulta en el
    /// estado del clúster (mismo proceso, mismo instante). La SEMÁNTICA de la
    /// regla —«no gana quien va retrasado»— es idéntica.
    fn recibir_pide_voto(&mut self, yo: u32, termino: u64, candidato: u32) {
        let p = self.pos(yo);
        if termino > self.nodos[p].termino {
            let n = &mut self.nodos[p];
            n.rol = RolRaft::Seguidor;
            n.termino = termino;
            n.voto_de = None;
            n.tics_sin_lider = 0;
        }
        let pc = self.pos(candidato);
        let su_huella = huella_log(&self.nodos[pc].log);
        let n = &self.nodos[p];
        let mi_huella = huella_log(&n.log);
        let voto_libre = n.voto_de.is_none() || n.voto_de == Some(candidato);
        let concede = termino == n.termino && voto_libre && su_huella >= mi_huella;
        if concede {
            let n = &mut self.nodos[p];
            n.voto_de = Some(candidato);
            n.tics_sin_lider = 0;
            let termino = n.termino;
            self.encolar(
                candidato,
                yo,
                MensajeRaft::Voto {
                    termino,
                    candidato: Some(candidato),
                },
            );
        } else {
            let termino = self.nodos[p].termino;
            self.encolar(
                candidato,
                yo,
                MensajeRaft::Voto {
                    termino,
                    candidato: None,
                },
            );
        }
    }

    /// Candidato recibe voto: con mayoría del CLÚSTER se proclama líder,
    /// inicializa `siguiente_indice` y dispara latidos inmediatos.
    fn recibir_voto(&mut self, yo: u32, termino: u64, candidato: Option<u32>) {
        let p = self.pos(yo);
        if termino > self.nodos[p].termino {
            // Alguien nos dice que estamos anticuados: paso atrás.
            let n = &mut self.nodos[p];
            n.rol = RolRaft::Seguidor;
            n.termino = termino;
            n.voto_de = None;
            return;
        }
        if self.nodos[p].rol != RolRaft::Candidato
            || termino != self.nodos[p].termino
            || candidato != Some(yo)
        {
            return;
        }
        self.nodos[p].votos_recibidos += 1;
        if self.nodos[p].votos_recibidos * 2 > self.tamano_cluster() {
            let siguiente_inicial = self.nodos[p].log.len() + 1;
            let ids = self.ids_otros(yo);
            let n = &mut self.nodos[p];
            n.rol = RolRaft::Lider;
            n.tics_sin_lider = 0;
            for destino in ids {
                n.siguiente_indice.insert(destino, siguiente_inicial);
                n.confirmado_hasta.insert(destino, 0);
            }
            self.tics_desde_latido = 0;
            self.latidos();
        }
    }

    /// AppendEntries: comprobar prefijo, resolver conflictos (truncate),
    /// adoptar el compromiso del líder y acusar.
    // Los campos SON el mensaje Entradas del protocolo: la firma espeja al
    // paper (figura 2 de ATC '14), no una API que convenga comprimir.
    #[allow(clippy::too_many_arguments)]
    fn recibir_entradas(
        &mut self,
        yo: u32,
        termino: u64,
        lider: u32,
        prev_indice: usize,
        prev_termino: u64,
        entradas: Vec<EntradaLog>,
        compromiso_lider: usize,
    ) {
        let p = self.pos(yo);
        if termino < self.nodos[p].termino {
            // Líder anticuado: acuse de fallo con nuestro término (que el
            // líder se dé cuenta y dé un paso atrás).
            let mi_termino = self.nodos[p].termino;
            self.encolar(
                lider,
                yo,
                MensajeRaft::Acuse {
                    termino: mi_termino,
                    seguidor: yo,
                    exito: false,
                    coincide_hasta: 0,
                },
            );
            return;
        }
        // Término válido (≥ el nuestro): reconocemos al líder.
        {
            let n = &mut self.nodos[p];
            if n.termino != termino {
                n.termino = termino;
                n.voto_de = None;
            }
            n.rol = RolRaft::Seguidor;
            n.tics_sin_lider = 0;
        }

        // Coincidencia de prefijo: sin ella, NO tocamos el log.
        let log = &mut self.nodos[p].log;
        let prefijo_ok = prev_indice == 0
            || (log.len() >= prev_indice && log[prev_indice - 1].termino == prev_termino);
        if !prefijo_ok {
            self.encolar(
                lider,
                yo,
                MensajeRaft::Acuse {
                    termino,
                    seguidor: yo,
                    exito: false,
                    coincide_hasta: 0,
                },
            );
            return;
        }

        // Aplicar entradas con resolución de conflictos por término.
        // La primera entrada que viaja tiene índice prev_indice + 1 (el
        // prefijo termina EN prev_indice).
        for (i, e) in entradas.iter().enumerate() {
            let idx = prev_indice + 1 + i; // índice 1-based de e
            if idx <= log.len() {
                if log[idx - 1].termino != e.termino {
                    // Conflicto: el log del líder manda — truncate y sustituir.
                    log.truncate(idx - 1);
                    log.push(*e);
                }
                // Mismo término: la entrada YA está — idempotencia de Raft.
            } else {
                log.push(*e);
            }
        }
        let n = &mut self.nodos[p];
        n.indice_compromiso = n.indice_compromiso.max(compromiso_lider.min(n.log.len()));
        self.encolar(
            lider,
            yo,
            MensajeRaft::Acuse {
                termino,
                seguidor: yo,
                exito: true,
                coincide_hasta: prev_indice + entradas.len(),
            },
        );
    }

    /// Líder recibe acuse: avanza `siguiente_indice`/`confirmado_hasta` o
    /// RETROCEDE uno (backoff) reenviando al momento; luego recalcula el
    /// compromiso por MAYORÍA de huellas de log.
    fn recibir_acuse(
        &mut self,
        yo: u32,
        termino: u64,
        seguidor: u32,
        exito: bool,
        coincide_hasta: usize,
    ) {
        let p = self.pos(yo);
        if termino > self.nodos[p].termino {
            let n = &mut self.nodos[p];
            n.rol = RolRaft::Seguidor;
            n.termino = termino;
            n.voto_de = None;
            return;
        }
        if self.nodos[p].rol != RolRaft::Lider || termino != self.nodos[p].termino {
            return; // acuse viejo o ya no somos líderes: ignorar
        }
        if exito {
            let n = &mut self.nodos[p];
            n.siguiente_indice.insert(seguidor, coincide_hasta + 1);
            n.confirmado_hasta.insert(seguidor, coincide_hasta);
        } else {
            let nueva = self.nodos[p]
                .siguiente_indice
                .get(&seguidor)
                .copied()
                .unwrap_or(1)
                .saturating_sub(1)
                .max(1);
            self.nodos[p].siguiente_indice.insert(seguidor, nueva);
            // Reintento INMEDIATO con la ventana corregida (determinismo:
            // sin esperar al próximo latido).
            self.enviar_entradas_a(yo, seguidor);
        }

        // Compromiso por mayoría: ordenando las huellas confirmadas (líder
        // incluido, él tiene SU log entero) la mediana es el índice presente
        // en la mayoría. Sólo avanza si esa entrada es del término ACTUAL del
        // líder — regla de seguridad §5.4.2 del paper.
        let pl = self.pos(yo);
        let mut huellas: Vec<usize> = vec![self.nodos[pl].log.len()];
        for &h in self.nodos[pl].confirmado_hasta.values() {
            huellas.push(h);
        }
        huellas.sort_unstable();
        huellas.reverse();
        let posicion_quorum = self.tamano_cluster() as usize / 2;
        if let Some(&q) = huellas.get(posicion_quorum) {
            let n = &mut self.nodos[pl];
            let termino_en_q = n.log.get(q.wrapping_sub(1)).map(|e| e.termino);
            if q > n.indice_compromiso && termino_en_q == Some(n.termino) {
                n.indice_compromiso = q;
            }
        }
    }
}

// ───────────── Deuda del cap. 30: deadlock ENTRE particiones ─────────────

/// Fusiona los grafos de espera LOCALES de cada partición en uno GLOBAL,
/// usando SÓLO la API pública del cap. 30 (`aristas()` + `agregar_espera`) —
/// la pieza que el cap. 30 dejó «enchufada» para la concurrencia multi-máquina.
///
/// Por qué funciona: cada partición sólo ve SUS esperas, así que un ciclo que
/// cruce particiones es INVISIBLE para cualquier grafo local (test-tesis) y
/// salta a la vista en la fusión. Requisito operativo: los ids de transacción
/// deben ser únicos EN TODO EL CLÚSTER (p.ej. prefijo por partición) — si dos
/// particiones numeran sus tx desde 1, la fusión mezclaría identidades.
pub fn fusionar_grafos_espera(locales: &[&GrafoEspera]) -> GrafoEspera {
    let mut global = GrafoEspera::nuevo();
    for local in locales {
        for &(esperador, tenedor, recurso) in local.aristas() {
            global.agregar_espera(esperador, tenedor, recurso);
        }
    }
    global
}

// ─────────────────── Informe reproducible ───────────────────

/// Tabla estrategias × métricas sobre el dataset MINI — SIN TIEMPOS.
///
/// Ejecuta las TRES estrategias sobre el MISMO grafo, mide cortes/frontera/
/// factor/tamaños, corre la BFS distribuida DESDE EL HUB con cada estrategia
/// y reporta la carga por partición. Todos los valores son enteros EXACTOS y
/// reproducibles (dataset determinista del cap. 34): esta salida es la que la
/// prosa pega LITERALMENTE.
///
/// Sin bench criterion (decisión #11): ninguna afirmación del capítulo es
/// sobre tiempo — aquí los enteros SON la física del sistema.
pub fn informe_distribucion_reproducible_sobre_mini(store: &MemoryStore) -> String {
    let aristas = aristas_de_store(store);
    let n = store.node_count();
    let k = 8usize;

    // Hub = nodo de mayor grado no dirigido (empate → menor id): la MISMA
    // regla de los metadatos del cap. 34, derivada del store sin magia.
    let grados = grados_desde_aristas(&aristas, n);
    let hub = (0..n as u32)
        .max_by_key(|&v| (grados[v as usize], std::cmp::Reverse(v)))
        .expect("el mini dataset tiene nodos");
    let adj = adyacencia_simetrica(&aristas, n);

    let estrategias: [(&str, AsignacionParticion); 3] = [
        ("hash", particionar_hash(n, k as u32)),
        ("comunidad", particionar_por_comunidad(store, k)),
        ("codicioso", particionar_balanceo_codicioso(&aristas, k)),
    ];

    let mut lineas: Vec<String> = Vec::new();
    lineas.push("=== Informe de distribución (cap. 40) ===".to_string());
    lineas.push(format!(
        "dataset: {n} nodos, {} aristas dirigidas | k={k} particiones | hub: nodo {hub} (grado {})",
        aristas.len(),
        grados[hub as usize]
    ));
    lineas.push("-- estrategias x metricas (contadores exactos; BFS desde el hub) --".to_string());
    lineas.push(format!(
        "{:<11} {:>7} {:>9} {:>8} {:>7} {:>8} {:>12} {:>11}",
        "estrategia",
        "cortes",
        "frontera",
        "factor",
        "tam-max",
        "tam-min",
        "bfs-mensajes",
        "bfs-saltos"
    ));
    for (nombre, asignacion) in &estrategias {
        let m = metricas_corte(asignacion, &aristas);
        let bfs = bfs_entre_particiones(&adj, asignacion, hub);
        lineas.push(format!(
            "{nombre:<11} {:>7} {:>9} {:>8.3} {:>7} {:>8} {:>12} {:>11}",
            m.cortes_arista,
            m.nodos_frontera,
            m.factor_replicacion,
            m.tam_max,
            m.tam_min,
            bfs.mensajes_red,
            bfs.saltos_red,
        ));
    }
    lineas.push("-- carga por partición (suma de grados de sus nodos) --".to_string());
    for (nombre, asignacion) in &estrategias {
        let carga = carga_por_particion(asignacion, &grados);
        let lista: Vec<String> = carga.iter().map(|c| c.to_string()).collect();
        lineas.push(format!("{nombre:<11} [{}]", lista.join(", ")));
    }
    let (nombre_hub, asignacion_hub) = &estrategias[0];
    let carga_hub = carga_por_particion(asignacion_hub, &grados);
    let particion_hub = asignacion_hub.dueno_de(hub).unwrap_or(0) as usize;
    let total: u64 = carga_hub.iter().sum();
    lineas.push(format!(
        "hotspot ({nombre_hub}): el hub {hub} vive en la partición {particion_hub}, que \
         concentra {} de {total} de carga ({:.1}%)",
        carga_hub[particion_hub],
        100.0 * carga_hub[particion_hub] as f64 / total as f64
    ));
    lineas.push(
        "(sin tiempos: los enteros SON la física del sistema — regla del cap. 34, decisión #11)"
            .to_string(),
    );

    let mut informe = lineas.join("\n");
    informe.push('\n');
    informe
}

// ─────────────────── Los tests de honestidad ───────────────────

#[cfg(test)]
mod tests_distribucion {
    use super::*;
    use crate::cap30_mvcc::Recurso;
    use crate::cap34_benchmarks::{SEMILLA_REFERENCIA, dataset_referencia_mini};

    #[test]
    fn hash_modulo_k_asigna_todo_y_balancea() {
        let n = 400usize;
        let k = 8u32;
        let a = particionar_hash(n, k);
        assert_eq!(a.dueno.len(), n);
        assert!(a.dueno.iter().all(|&d| d < k), "todo dueño < k");
        // Medido (no prometido): FNV-1a módulo 8 sobre ids 0..399 reparte
        // EXACTAMENTE 50 nodos por bucket — dentro de ±1 con margen sobrado.
        let tamanos = a.tamanos();
        for t in &tamanos {
            assert!(
                (*t as i64 - (n as i64 / k as i64)).abs() <= 1,
                "bucket desbalanceado: {tamanos:?}"
            );
        }
        assert_eq!(tamanos, vec![50; 8]);
    }

    #[test]
    fn particiones_comunidad_respectan_grupos_de_louvain() {
        let ds = dataset_referencia_mini(SEMILLA_REFERENCIA);
        let k = 8usize;
        let asignacion = particionar_por_comunidad(&ds.store, k);

        // La verdad del cap. 25: mismas comunidades Louvain ⇒ misma partición.
        let r = louvain(&ds.store, &WeightSource::Constant(1.0), 1.0, 30)
            .expect("louvain con parámetros fijos");
        assert!(r.num_comunidades() > 0);
        for grupo in r.comunidades() {
            let particiones: HashSet<u32> = grupo.iter().map(|&m| asignacion.dueno[m]).collect();
            assert_eq!(
                particiones.len(),
                1,
                "una comunidad Louvain quedó partida: {particiones:?}"
            );
        }
        // Todo nodo asignado y no se usan más particiones que k.
        assert_eq!(asignacion.dueno.len(), ds.store.node_count());
        let usadas: HashSet<u32> = asignacion.dueno.iter().copied().collect();
        assert!(usadas.len() <= k);
        assert!(usadas.iter().all(|&p| (p as usize) < k));
    }

    #[test]
    fn cortes_hash_vs_comunidad_en_mini_cuentas_exactas() {
        let ds = dataset_referencia_mini(SEMILLA_REFERENCIA);
        let aristas = aristas_de_store(&ds.store);
        let n = ds.store.node_count();
        let k = 8usize;

        let hash = particionar_hash(n, k as u32);
        let comunidad = particionar_por_comunidad(&ds.store, k);
        let codicioso = particionar_balanceo_codicioso(&aristas, k);

        let mh = metricas_corte(&hash, &aristas);
        let mc = metricas_corte(&comunidad, &aristas);
        let mg = metricas_corte(&codicioso, &aristas);

        // TESIS del capítulo (medida, no declamada): la comunidad respeta
        // vecindarios ⇒ MUCHOS menos cortes que el hash. Si algún día la
        // dirección cambiara, el test DEBE fallar: prohibido inflar (§8).
        assert!(
            mc.cortes_arista < mh.cortes_arista,
            "comunidad ({}) debería cortar menos que hash ({})",
            mc.cortes_arista,
            mh.cortes_arista
        );
        // Consistencia interna de TODAS las métricas (cuentas exactas):
        for m in [&mh, &mc, &mg] {
            assert!(m.factor_replicacion >= 1.0, "réplicas >= 1 por nodo");
            assert!(m.tam_max >= m.tam_min);
        }
        // La suma de tamaños cubre todos los nodos en las tres estrategias.
        for a in [&hash, &comunidad, &codicioso] {
            let total: usize = a.tamanos().iter().sum();
            assert_eq!(total, n);
        }
        println!("mini: hash={mh:?}");
        println!("mini: comunidad={mc:?}");
        println!("mini: codicioso={mg:?}");
    }

    #[test]
    fn corte_de_arista_frontera_y_factor_replicacion_contados_a_mano() {
        // Fixture dibujado en la prosa (≤8 nodos):
        //
        //   P0 = {0, 1}    P1 = {2, 3}    P2 = {4}
        //   aristas: 0─1, 1─2, 2─3, 3─4, 0─3
        //
        // CORTES (extremos en particiones distintas): 1─2, 3─4, 0─3 ⇒ 3.
        // FRONTERA (incidentes a algún corte): {0, 1, 2, 3, 4} ⇒ 5.
        // RÉPLICAS por nodo (1 + nº de particiones vecinas distintas):
        //   0: vecinos {1:P0, 3:P2} ⇒ 1+1=2     3: vecinos {2:P1, 4:P2, 0:P0} ⇒ 1+2=3
        //   1: vecinos {0:P0, 2:P1} ⇒ 2         4: vecinos {3:P1} ⇒ 2
        //   2: vecinos {1:P0, 3:P1} ⇒ 2         Σ réplicas = 11 ⇒ 11/5 = 2.2
        // TAMAÑOS: {2, 2, 1} ⇒ máx 2, mín 1.
        let aristas = vec![(0, 1), (1, 2), (2, 3), (3, 4), (0, 3)];
        let asignacion = AsignacionParticion {
            num_particiones: 3,
            dueno: vec![0, 0, 1, 1, 2],
        };
        let m = metricas_corte(&asignacion, &aristas);
        assert_eq!(m.cortes_arista, 3);
        assert_eq!(m.nodos_frontera, 5);
        assert!((m.factor_replicacion - 2.2).abs() < 1e-12);
        assert_eq!(m.tam_max, 2);
        assert_eq!(m.tam_min, 1);

        // Contraste: TODO local ⇒ cero de todo (menos factor = 1.0).
        let mono = AsignacionParticion::monolito(5);
        let m0 = metricas_corte(&mono, &aristas);
        assert_eq!(m0.cortes_arista, 0);
        assert_eq!(m0.nodos_frontera, 0);
        assert!((m0.factor_replicacion - 1.0).abs() < 1e-12);
    }

    #[test]
    fn corte_de_vertice_del_hub_elimina_cortes_pagando_replicas() {
        // Estrella de 6 hojas, k=3. Baseline documentada: hub→P0; hoja j→j%3
        // ⇒ hojas 3 y 6 viven en P0 (2 aristas locales), las otras 4 cortadas.
        let aristas: Vec<(u32, u32)> = (1..=6).map(|j| (0, j)).collect();
        let baseline = AsignacionParticion {
            num_particiones: 3,
            dueno: vec![0, 1, 2, 0, 1, 2, 0], // nodo 0=hub, hoja j en j%3
        };
        let antes = metricas_corte(&baseline, &aristas);
        assert_eq!(antes.cortes_arista, 4, "6 hojas − 2 locales = 4 cortes");

        // Vertex cut: el hub se replica donde hacen falta sus hojas.
        let hojas: Vec<u32> = (1..=6).collect();
        let info = replicar_hub(0, &hojas, 3);
        assert_eq!(info.cortes_antes, 4);
        assert_eq!(info.cortes_despues, 0);
        assert_eq!(info.replicas_hub, 3, "una réplica por partición con hojas");

        // Y con m < k: tantas réplicas como hojas (cada una en su partición).
        let info_chica = replicar_hub(0, &[1, 2], 5);
        assert_eq!(info_chica.replicas_hub, 2);
        assert_eq!(info_chica.cortes_antes, 2); // ninguna hoja cae en P0 (1%5=1, 2%5=2)
        assert_eq!(info_chica.cortes_despues, 0);
    }

    #[test]
    fn bfs_entre_particiones_cuenta_mensajes_y_coincide_con_bfs_local() {
        // Camino 0─1─2─3─4 con dueños alternos P0,P1,P0,P1,P0: CADA paso cruza.
        let adj = vec![vec![1], vec![0, 2], vec![1, 3], vec![2, 4], vec![3]];
        let asignacion = AsignacionParticion {
            num_particiones: 2,
            dueno: vec![0, 1, 0, 1, 0],
        };
        let r = bfs_entre_particiones(&adj, &asignacion, 0);
        assert_eq!(r.visitados, vec![0, 1, 2, 3, 4]);
        assert_eq!(
            r.mensajes_red, 4,
            "nodos 1..4 transferidos una vez cada uno"
        );
        assert_eq!(r.saltos_red, 4, "los cuatro niveles cruzaron red");

        // Todo local ⇒ cero mensajes, cero saltos, mismos alcanzables.
        let mono = AsignacionParticion::monolito(5);
        let r0 = bfs_entre_particiones(&adj, &mono, 0);
        assert_eq!(r0.visitados, vec![0, 1, 2, 3, 4]);
        assert_eq!(r0.mensajes_red, 0);
        assert_eq!(r0.saltos_red, 0);

        // Mini dataset bajo hash: mismos alcanzables que el BFS local y red pagada.
        let ds = dataset_referencia_mini(SEMILLA_REFERENCIA);
        let aristas = aristas_de_store(&ds.store);
        let n = ds.store.node_count();
        let adj_mini = adyacencia_simetrica(&aristas, n);
        let hash = particionar_hash(n, 8);
        let rd = bfs_entre_particiones(&adj_mini, &hash, ds.hubs[0] as u32);

        // BFS local de referencia (cola simple, mismo terreno).
        let mut vista = vec![false; n];
        let mut cola: VecDeque<usize> = VecDeque::new();
        vista[ds.hubs[0]] = true;
        cola.push_back(ds.hubs[0]);
        while let Some(u) = cola.pop_front() {
            for &w in &adj_mini[u] {
                if !vista[w as usize] {
                    vista[w as usize] = true;
                    cola.push_back(w as usize);
                }
            }
        }
        let locales: Vec<u32> = (0..n).filter(|&i| vista[i]).map(|i| i as u32).collect();
        assert_eq!(
            rd.visitados, locales,
            "la distribución no cambia QUÉ se alcanza"
        );
        assert!(rd.mensajes_red > 0, "bajo hash la travesía paga red");
        println!(
            "bfs hash mini: mensajes={} saltos={}",
            rd.mensajes_red, rd.saltos_red
        );
    }

    #[test]
    fn hub_concentra_la_carga_en_su_particion() {
        let ds = dataset_referencia_mini(SEMILLA_REFERENCIA);
        let aristas = aristas_de_store(&ds.store);
        let n = ds.store.node_count();
        let grados = grados_desde_aristas(&aristas, n);
        let hub = ds.hubs[0];

        let hash = particionar_hash(n, 8);
        let carga = carga_por_particion(&hash, &grados);
        let p_hub = hash.dueno_de(hub as u32).unwrap() as usize;

        // El hub ES el máximo grado (misma regla que los metadatos del 34)…
        assert_eq!(grados[hub], grados.iter().copied().max().unwrap());
        // …y su partición es la MÁS cargada de todas bajo hash.
        let max_otra = carga
            .iter()
            .enumerate()
            .filter(|&(p, _)| p != p_hub)
            .map(|(_, &c)| c)
            .max()
            .unwrap();
        assert!(
            carga[p_hub] > max_otra,
            "la partición del hub ({}) debe superar a la segunda ({}): {carga:?}",
            carga[p_hub],
            max_otra
        );
        println!(
            "hub {}: grado {}, partición {p_hub}, carga {} / total {}",
            hub,
            grados[hub],
            carga[p_hub],
            carga.iter().sum::<u64>()
        );
    }

    #[test]
    fn rebalanceo_mueve_una_particion_y_recuenta_cortes() {
        // Fixture dibujado: P0={0,1}, P1={2,3}, P2={4,5}
        //   aristas: 0─1 (local P0), 2─3 (local P1), 4─5 (local P2),
        //            0─2 (corte P0-P1), 1─4 (corte P0-P2)
        // ANTES: 2 cortes. Movemos P0 → destino menos cargado (todas cargan
        // igual ⇒ índice menor ≠ P0 ⇒ P1). Tras la fusión P0→P1:
        //   0─2 queda LOCAL, 1─4 SIGUE cortando (ahora P1-P2) ⇒ 1 corte.
        //   nodos_movidos = 2; aristas_retocadas = 2 (0─2 y 1─4).
        let aristas = vec![(0, 1), (2, 3), (4, 5), (0, 2), (1, 4)];
        let mut asignacion = AsignacionParticion {
            num_particiones: 3,
            dueno: vec![0, 0, 1, 1, 2, 2],
        };
        let informe = rebalancear(&mut asignacion, &aristas, 0);
        assert_eq!(informe.cortes_antes, 2);
        assert_eq!(informe.cortes_despues, 1);
        assert_eq!(informe.nodos_movidos, 2);
        assert_eq!(informe.aristas_retocadas, 2);
        // La asignación MUTADA refleja la fusión.
        assert_eq!(asignacion.dueno, vec![1, 1, 1, 1, 2, 2]);
        // Y el recuento independiente coincide con el del informe.
        assert_eq!(
            metricas_corte(&asignacion, &aristas).cortes_arista,
            informe.cortes_despues
        );
    }

    // ───────── Raft: tics lógicos, cero flakiness ─────────

    #[test]
    fn raft_eleccion_por_tics_elige_lider_con_mayoria() {
        // Timeouts escalonados 10/15/20: el nodo 0 impaciente primero GANA
        // (sus PideVoto llegan antes de que nadie más se impaciente).
        let mut cluster = EnjambreRaft::nuevo(&[0, 1, 2], 10, 5);
        let mut tics = 0u64;
        while cluster.lider().is_none() {
            cluster.tic();
            tics += 1;
            assert!(tics <= 100, "elección no converge: bug de protocolo");
        }
        assert_eq!(cluster.lider(), Some(0));
        assert_eq!(cluster.nodo(0).rol, RolRaft::Lider);
        assert_eq!(cluster.nodo(0).termino, 1);
        for id in [1u32, 2] {
            assert_eq!(cluster.nodo(id).rol, RolRaft::Seguidor);
            assert_eq!(cluster.nodo(id).termino, 1, "todos ven el mismo término");
        }
        // Cerca de su timeout escalonado (10): ni antes ni muy después.
        assert!((10..=15).contains(&tics), "primera elección en tic {tics}");
        // Estabilidad: muchos tics más, mismo líder y mismo término
        // (los latidos mantienen a los seguidores bajo su timeout).
        cluster.tics(60);
        assert_eq!(cluster.lider(), Some(0));
        assert_eq!(cluster.nodo(0).termino, 1);
    }

    #[test]
    fn raft_appendentries_replica_el_log_en_los_seguidores() {
        let mut cluster = EnjambreRaft::nuevo(&[0, 1, 2], 10, 5);
        while cluster.lider().is_none() {
            cluster.tic();
        }
        assert!(cluster.proponer(100));
        assert!(cluster.proponer(200));
        cluster.tics(10);

        // Logs IDÉNTICOS byte a byte en los tres nodos…
        let log_lider = cluster.nodo(0).log.clone();
        assert_eq!(log_lider.len(), 2);
        assert_eq!(cluster.nodo(1).log, log_lider);
        assert_eq!(cluster.nodo(2).log, log_lider);
        assert_eq!(
            log_lider[0],
            EntradaLog {
                termino: 1,
                comando: 100
            }
        );
        assert_eq!(
            log_lider[1],
            EntradaLog {
                termino: 1,
                comando: 200
            }
        );
        // …comprometidos por mayoría y adoptado por TODOS (el índice de
        // compromiso del líder viaja en cada AppendEntries).
        for id in [0u32, 1, 2] {
            assert_eq!(cluster.nodo(id).indice_compromiso, 2);
        }
    }

    #[test]
    fn raft_no_compromete_sin_mayoria_de_acuses() {
        let mut cluster = EnjambreRaft::nuevo(&[0, 1, 2], 10, 5);
        while cluster.lider().is_none() {
            cluster.tic();
        }
        // Mayoría CAÍDA: sólo queda el líder (1 de 3 < 2).
        cluster.caer(1);
        cluster.caer(2);
        assert!(cluster.proponer(77));
        cluster.tics(40);

        // La entrada VIVE en el log del líder pero NADIE la compromete.
        assert_eq!(cluster.nodo(0).log.len(), 1);
        assert_eq!(cluster.nodo(0).indice_compromiso, 0);
        assert_eq!(cluster.lider(), Some(0), "el líder sigue siendo líder");
        // Sin mayoría no hay servicio: nuevas propuestas apuntan al log
        // LOCAL pero el compromiso NO avanza nunca.
        assert!(cluster.proponer(78));
        cluster.tics(40);
        assert_eq!(cluster.nodo(0).indice_compromiso, 0);
        assert_eq!(cluster.nodo(0).log.len(), 2);
    }

    #[test]
    fn raft_seguidor_reconectado_alcanza_el_log_del_lider() {
        let mut cluster = EnjambreRaft::nuevo(&[0, 1, 2], 10, 5);
        cluster.caer(2); // caído ANTES de la elección: revive anticuadísimo
        while cluster.lider().is_none() {
            cluster.tic();
        }
        assert_eq!(cluster.lider(), Some(0));
        for cmd in 1..=3 {
            assert!(cluster.proponer(cmd));
        }
        cluster.tics(15);
        // Con mayoría (0+1) el log avanza y SE COMPROMITE.
        assert_eq!(cluster.nodo(0).indice_compromiso, 3);
        assert_eq!(cluster.nodo(1).indice_compromiso, 3);
        assert_eq!(cluster.nodo(2).log.len(), 0, "estuvo caído todo el rato");

        // Reconexión: los latidos llevan el sufijo pendiente — el rezagado
        // se pone al día SOLO, sin propuestas nuevas.
        cluster.revivir(2);
        let mut tics = 0u64;
        while cluster.nodo(2).log.len() < 3 {
            cluster.tic();
            tics += 1;
            assert!(
                tics <= 200,
                "el reconectado no alcanza al líder: bug de protocolo"
            );
        }
        assert_eq!(cluster.nodo(2).log, cluster.nodo(0).log, "logs idénticos");
        assert_eq!(cluster.nodo(2).indice_compromiso, 3);
        assert_eq!(cluster.nodo(2).termino, 1);
    }

    // ───────── Deuda cap. 30 ─────────

    #[test]
    fn grafo_espera_global_detecta_deadlock_entre_particiones() {
        // T1 retiene el recurso nodo 7 en la partición A y espera el nodo 9
        // que retiene T2 en la partición B; T2 espera a su vez el nodo 7 de A.
        // Cada grafo LOCAL ve UNA sola arista ⇒ ningún ciclo local.
        let mut espera_a = GrafoEspera::nuevo();
        let mut espera_b = GrafoEspera::nuevo();
        // Ids de tx ÚNICOS en el clúster (prefijo por partición: A=100+, B=200+).
        let t1 = 101u64; // tx de la partición A
        let t2 = 202u64; // tx de la partición B
        espera_a.agregar_espera(t2, t1, Recurso::Nodo(7)); // en A: T2 espera nodo 7 (de T1)
        espera_b.agregar_espera(t1, t2, Recurso::Nodo(9)); // en B: T1 espera nodo 9 (de T2)

        assert!(
            espera_a.detectar_ciclo().is_none(),
            "localmente A no ve el ciclo"
        );
        assert!(
            espera_b.detectar_ciclo().is_none(),
            "localmente B no ve el ciclo"
        );

        // La pieza del cap. 30 enchufada por fin: fusión global + detectar_ciclo.
        let global = fusionar_grafos_espera(&[&espera_a, &espera_b]);
        let ciclo = global
            .detectar_ciclo()
            .expect("el deadlock ENTRE particiones debe verse en el grafo global");
        // El DFS del cap. 30 devuelve el ciclo CERRADO ([a, b, a], en orden
        // de recorrido que depende del HashMap interno): se compara por nodos
        // únicos + cierre, nunca por orden exacto.
        let unicos: HashSet<u64> = ciclo.iter().copied().collect();
        assert_eq!(unicos.len(), 2, "ciclo entre DOS transacciones: {ciclo:?}");
        assert_eq!(ciclo.first(), ciclo.last(), "el ciclo viene cerrado");
        assert!(unicos.contains(&t1) && unicos.contains(&t2));

        // Y si T1 termina (quitar_tx), el ciclo MUERE — API pública completa.
        let mut global2 = fusionar_grafos_espera(&[&espera_a, &espera_b]);
        global2.quitar_tx(t1);
        assert!(global2.detectar_ciclo().is_none());
    }

    // ───────── Informe reproducible ─────────

    #[test]
    fn informe_distribucion_reproducible_sobre_mini() {
        let ds = dataset_referencia_mini(SEMILLA_REFERENCIA);
        let a = super::informe_distribucion_reproducible_sobre_mini(&ds.store);
        let b = super::informe_distribucion_reproducible_sobre_mini(&ds.store);
        assert_eq!(a, b, "dos ejecuciones: byte a byte iguales");

        // Marcadores de contenido (la prosa pega esta salida LITERALMENTE).
        for marcador in [
            "=== Informe de distribución (cap. 40) ===",
            "400 nodos",
            "hash",
            "comunidad",
            "codicioso",
            "carga por partición",
            "sin tiempos",
        ] {
            assert!(a.contains(marcador), "falta el marcador `{marcador}`");
        }
        println!("{a}");
    }
}
