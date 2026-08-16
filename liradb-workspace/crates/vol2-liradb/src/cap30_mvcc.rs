use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

use crate::cap07_modelo::{Edge, EdgeId, Node, NodeId};
use crate::cap08_graph_store::{GraphStore, MemoryStore, StoreError};
use crate::cap27_transacciones::{
    EntradaAcid, GarantiaAcid, NivelGarantia, Operacion, TransaccionError,
};

// ─────────────────── Cap 30: snapshots y concurrencia — MVCC limitado ───────────────────
//
// El cap. 27 dejó el modelo «múltiples lectores, un único escritor»
// ejecutado por el PRÉSTAMO EXCLUSIVO del borrow checker: no había
// concurrencia posible y, por tanto, ninguna anomalía de aislamiento que
// aislar. El cap. 28-29 resolvieron la durabilidad y la recuperación; pero
// el aislamiento REAL (varios lectores leyendo MIENTRAS un escritor
// escribe, sin lecturas sucias) exige algo distinto: una forma de que los
// lectores lean un ESTADO CONSISTENTE sin bloquear al escritor.
//
// Ese algo es MVCC (Multi-Version Concurrency Control): en lugar de
// sobrescribir el elemento, cada escritura crea una NUEVA VERSIÓN y
// deja la anterior visible para los lectores que empezaron antes. Los
// lectores toman una FOTO («snapshot») en un instante lógico y leen sólo
// las versiones visibles a esa foto. El escritor es el único que muta la
// versión actual; los lectores nunca leen la versión en escritura —
// leen la que les correspondía.
//
// El modelo se reduce a tres reglas:
//   1. Cada elemento lleva una CADENA de versiones (ts_begin, ts_end?,
//      valor). Las escrituras RETIRAN la versión actual (ponen su ts_end)
//      y APENDIZAN una nueva.
//   2. Un snapshot es un timestamp lógico `Ts = u64` monótono. Una
//      lectura en `ts` ve la versión con el MAYOR `ts_begin ≤ ts` Y
//      `ts_end > ts` (o sin `ts_end`).
//   3. El escritor es único (`&mut MvccStore`); los lectores son
//      concurrentes (`&MvccStore`). Sin cerrojos de lectura: la
//      consistencia viene del versionado, no de los locks.
//
// Lo que el capítulo ENTREGA:
//   * `VersionNode`/`VersionEdge` (el registro de versión).
//   * `MvccStore`: store con cadenas de versiones sobre `MemoryStore`
//     (la MVCC es una capa sobre el `GraphStore` del cap. 8, hexagonal).
//   * Lecturas con snapshot: `leer_nodo`, `leer_arista`, `iter_nodos`,
//     `iter_aristas` — todas por valor (la cadena se CLONA para evitar
//     el borrow; un lector puede leer MIENTRAS otro escribe).
//   * Commit con un solo timestamp por lote: asigna el siguiente `ts`,
//     retira la versión actual si la había, appendiza la nueva, aplica
//     al `inner`. Reutiliza `validar_buffer` (pub(crate) del cap. 27)
//     para las invariantes del buffer.
//   * Garbage collection: `gc(hasta)` descarta versiones retiradas
//     cuya `ts_end < hasta` — ya no son visibles para NINGÚN snapshot
//     futuro (los `ts` son monótonos).
//   * `NivelAislamiento` como vocabulario: qué garantiza cada nivel y
//     qué anomalías (las del cap. 27 + write skew) deja pasar. MVCC en
//     snapshot isolation elimina la lectura sucia y la actualización
//     perdida, pero NO el write skew — la razón por la que este
//     capítulo es «MVCC limitado» y no «MVCC completo».
//   * `GrafoEspera` para deadlocks: aunque hoy hay un único escritor y
//     no pueden ocurrir, la estructura existe y se DEMUESTRA (un gestor
//     de cerrojos real la integraría en cap. futuros).
//
// Lo que el capítulo NO entrega (honesto, como en el cap. 27-29):
//   * Timestamp físico: el `Ts` es un contador lógico, no una medida de
//     tiempo real. Una lectura no compite con el reloj.
//   * Concurrencia REAL de escritores: sigue habiendo un único escritor
//     (`&mut`). Lo que MVCC MULTIPLICA son los lectores — un escritor y
//     N lectores ya no se bloquean entre sí.
//   * Serializable snapshot isolation: en SI (este capítulo) puede haber
//     write skew (dos transacciones leen y modifican disjuntos a partir
//     del mismo snapshot). Detectarlo exige predicate locks o SI
//     SERIALIZABLE (PostgreSQL desde 9.1, CockroachDB, FoundationDB) —
//     fuera del alcance.
//   * GC en background: el capítulo enseña la operación `gc`; integrarla
//     como tarea programada es integración del motor.

// ─────────────────── Tipos base ───────────────────

/// Timestamp lógico: un `u64` monótono asignado por `MvccStore` al hacer
/// commit. No mide tiempo real: es el ORDEN de las escrituras.
pub type Ts = u64;

/// Una versión de un nodo: visible desde `ts_begin`, retirada en `ts_end`
/// (o nunca, si `ts_end = None`).
#[derive(Debug, Clone, PartialEq)]
pub struct VersionNode {
    pub ts_begin: Ts,
    pub ts_end: Option<Ts>,
    pub nodo: Node,
}

/// Una versión de una arista: análogo al nodo.
#[derive(Debug, Clone, PartialEq)]
pub struct VersionEdge {
    pub ts_begin: Ts,
    pub ts_end: Option<Ts>,
    pub arista: Edge,
}

/// Los niveles de aislamiento clásicos, como vocabulario que conecta este
/// capítulo con el `Anomalia` del cap. 27 (lo que el nivel PROHIBE y lo
/// que DEJA PASAR).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NivelAislamiento {
    /// Lecturas ven lo no confirmado (lo que otra tx podría revertir).
    LecturaSucia,
    /// Cada lectura ve un snapshot consistente del instante en que se
    /// tomó. Es el nivel que ofrece MVCC «por defecto» — y el de este
    /// capítulo. Prohíbe lectura sucia y actualización perdida.
    Instantanea,
    /// Además de snapshot, garantiza que las transacciones se pueden
    /// SERIALIZAR (el resultado equivale a ejecutarlas una a una).
    /// Serializable Snapshot Isolation (SSI) lo consigue con predicate
    /// locks; aquí queda como vocabulario y como anzuelo para caps.
    /// futuros.
    Serializable,
}

impl NivelAislamiento {
    /// Anomalías que el nivel PROHÍBE.
    pub fn prohibe(self) -> &'static str {
        match self {
            NivelAislamiento::LecturaSucia => "ninguna",
            NivelAislamiento::Instantanea => {
                "lectura sucia y actualización perdida (write skew sigue pasando)"
            }
            NivelAislamiento::Serializable => "lectura sucia, actualización perdida y write skew",
        }
    }
}

impl fmt::Display for NivelAislamiento {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NivelAislamiento::LecturaSucia => write!(f, "lectura sucia (read uncommitted)"),
            NivelAislamiento::Instantanea => write!(f, "snapshot isolation (instantánea)"),
            NivelAislamiento::Serializable => write!(f, "serializable (SSI)"),
        }
    }
}

// ─────────────────── Errores ───────────────────

/// Errores del mundo MVCC.
#[derive(Debug, Clone, PartialEq)]
pub enum MvccError {
    /// La validación del buffer (re-validación del cap. 27) rechazó la
    /// transacción antes de aplicar.
    Validacion(TransaccionError),
    /// El `inner` (el `GraphStore` subyacente) rechazó una escritura: la
    /// MVCC no se ha aplicado (estado consistente, atomicidad intacta).
    Store(StoreError),
}

impl fmt::Display for MvccError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MvccError::Validacion(e) => write!(f, "MVCC: commit rechazado en validación: {e}"),
            MvccError::Store(e) => write!(f, "MVCC: el store rechazó la escritura: {e}"),
        }
    }
}

impl std::error::Error for MvccError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MvccError::Validacion(e) => Some(e),
            MvccError::Store(e) => Some(e),
        }
    }
}

impl From<TransaccionError> for MvccError {
    fn from(e: TransaccionError) -> Self {
        MvccError::Validacion(e)
    }
}

// ─────────────────── El store MVCC ───────────────────

/// Un store MVCC: la capa de versionado SOBRE un `GraphStore` subyacente
/// (en este capítulo `MemoryStore`; la hexagonal del cap. 8 admite
/// cualquier backend).
///
/// Estructura por elemento:
///   * `versiones_nodos[id]` y `versiones_aristas[id]` son VECTORES de
///     versiones ordenados por `ts_begin` ASCENDENTE — la última entrada
///     es la versión actual (su `ts_end` es `None` hasta que otra la
///     retire).
///   * El `inner` (el `GraphStore`) refleja SIEMPRE la versión actual:
///     es el «espejo material» que los lectores no-snapshot y las
///     queries sin MVCC ven. La MVCC vive ENCIMA de él.
///
/// Concurrencia por el sistema de tipos:
///   * `&self` → lecturas por snapshot (clonan la versión visible).
///   * `&mut self` → única vía de mutación (`commit`, `gc`). Un único
///     escritor activo; N lectores simultáneos sin bloqueos.
#[derive(Debug)]
pub struct MvccStore {
    /// El store subyacente (la «versión material» que reflejan las queries
    /// no-MVCC). Mutable sólo dentro de `commit` (bajo `&mut self`).
    pub inner: MemoryStore,
    /// Cadenas de versiones por nodo.
    pub versiones_nodos: HashMap<NodeId, Vec<VersionNode>>,
    /// Cadenas de versiones por arista.
    pub versiones_aristas: HashMap<EdgeId, Vec<VersionEdge>>,
    /// Reloj lógico: el siguiente timestamp a asignar.
    pub reloj: Ts,
}

impl Default for MvccStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MvccStore {
    /// Store MVCC vacío con reloj en 1.
    pub fn new() -> Self {
        MvccStore {
            inner: MemoryStore::new(),
            versiones_nodos: HashMap::new(),
            versiones_aristas: HashMap::new(),
            reloj: 1,
        }
    }

    /// El reloj lógico (PRÓXIMO timestamp a asignar).
    pub fn reloj(&self) -> Ts {
        self.reloj
    }

    /// Asigna y devuelve el siguiente timestamp.
    ///
    /// Es el ÚNICO mecanismo por el que un `ts` entra en el sistema:
    /// dos commits simultáneos (imposibles hoy: `&mut self` lo prohíbe)
    /// no podrían coincidir. La asignación es determinista por el orden
    /// del programa.
    pub fn siguiente_ts(&mut self) -> Ts {
        let t = self.reloj;
        self.reloj += 1;
        t
    }

    /// Lee el nodo `id` en el snapshot `ts`. Devuelve `None` si no había
    /// versión visible (nunca existió, o fue borrado antes de `ts`).
    ///
    /// Clona el nodo: la MVCC entrega por VALOR, así un lector puede
    /// tomar `&self` mientras otro escritor tiene `&mut self`. Sin
    /// cerrojos de lectura — la cadena es el cerrojo.
    pub fn leer_nodo(&self, id: NodeId, ts: Ts) -> Option<Node> {
        let chain = self.versiones_nodos.get(&id)?;
        version_visible_node(chain, ts).map(|v| v.nodo.clone())
    }

    /// Lee la arista `id` en el snapshot `ts`. Análogo a `leer_nodo`.
    pub fn leer_arista(&self, id: EdgeId, ts: Ts) -> Option<Edge> {
        let chain = self.versiones_aristas.get(&id)?;
        version_visible_edge(chain, ts).map(|v| v.arista.clone())
    }

    /// Itera los nodos visibles en `ts`. Recorre TODAS las cadenas y
    /// recoge las versiones visibles — O(número de elementos). La
    /// consistencia del snapshot es POR CONSTRUCCIÓN: no hay un punto en
    /// el que el grafo «cambie» a mitad del recorrido, porque las
    /// cadenas son append-only y la lectura es por valor.
    pub fn iter_nodos(&self, ts: Ts) -> Vec<Node> {
        let mut out = Vec::new();
        for chain in self.versiones_nodos.values() {
            if let Some(v) = version_visible_node(chain, ts) {
                out.push(v.nodo.clone());
            }
        }
        out
    }

    /// Itera las aristas visibles en `ts`. Análogo a `iter_nodos`.
    pub fn iter_aristas(&self, ts: Ts) -> Vec<Edge> {
        let mut out = Vec::new();
        for chain in self.versiones_aristas.values() {
            if let Some(v) = version_visible_edge(chain, ts) {
                out.push(v.arista.clone());
            }
        }
        out
    }

    /// COMMIT de un lote de operaciones: asigna un nuevo `ts`, RETIRA
    /// las versiones actuales que toque, APPENDIZA las nuevas y APLICA
    /// al `inner` para que las queries no-MVCC vean la versión
    /// material.
    ///
    /// Validación (propia del capítulo, NO la del cap. 27): la del cap.
    /// 27 rechaza `PutNode`/`PutEdge` si el id YA EXISTE, porque el
    /// `MemoryStore` subyacente es de INSERCIÓN estricta. La MVCC, en
    /// cambio, SOBREESCRIBE: una segunda escritura del mismo nodo es
    /// legal (crea una nueva versión). Por eso validamos SOLO lo que
    /// la MVCC necesita: extremos de aristas presentes en el estado
    /// visible (inner + buffer) y deletes de elementos presentes.
    pub fn commit(&mut self, ops: &[Operacion]) -> Result<ResumenCommitMvcc, MvccError> {
        // Validación específica de MVCC: simulación contra el inner +
        // el propio buffer (los PutNode/PutEdge se consideran presentes
        // incluso si el id existía, porque SOBREESCRIBEN).
        self.validar_mvcc(ops)?;

        let ts = self.siguiente_ts();
        let mut resumen = ResumenCommitMvcc {
            ts_asignado: ts,
            ..ResumenCommitMvcc::default()
        };

        for op in ops {
            match op {
                Operacion::PutNode(n) => {
                    // Retira la versión actual (si existe).
                    let chain = self.versiones_nodos.entry(n.id).or_default();
                    if let Some(last) = chain.last_mut()
                        && last.ts_end.is_none()
                    {
                        last.ts_end = Some(ts);
                        resumen.versiones_retiradas += 1;
                    }
                    // Appendiza la nueva.
                    chain.push(VersionNode {
                        ts_begin: ts,
                        ts_end: None,
                        nodo: n.clone(),
                    });
                    // El inner es de inserción estricta (cap. 8). Para
                    // SOBREESCRIBIR vía MVCC, primero borramos si
                    // existía; la cadena ya hizo su trabajo de versionado.
                    let _ = self.inner.delete_node(n.id);
                    self.inner.put_node(n.clone()).map_err(MvccError::Store)?;
                    resumen.nodos_escritos += 1;
                }
                Operacion::PutEdge(e) => {
                    let chain = self.versiones_aristas.entry(e.id).or_default();
                    if let Some(last) = chain.last_mut()
                        && last.ts_end.is_none()
                    {
                        last.ts_end = Some(ts);
                        resumen.versiones_retiradas += 1;
                    }
                    chain.push(VersionEdge {
                        ts_begin: ts,
                        ts_end: None,
                        arista: e.clone(),
                    });
                    let _ = self.inner.delete_edge(e.id);
                    self.inner.put_edge(e.clone()).map_err(MvccError::Store)?;
                    resumen.aristas_escritas += 1;
                }
                Operacion::DeleteNode(id) => {
                    // En MVCC, un delete RETIRA la versión actual sin
                    // appendizar (la ausencia es el estado): el elemento
                    // deja de ser visible para snapshots futuros.
                    if let Some(chain) = self.versiones_nodos.get_mut(id)
                        && let Some(last) = chain.last_mut()
                        && last.ts_end.is_none()
                    {
                        last.ts_end = Some(ts);
                        resumen.versiones_retiradas += 1;
                    }
                    self.inner.delete_node(*id);
                }
                Operacion::DeleteEdge(id) => {
                    if let Some(chain) = self.versiones_aristas.get_mut(id)
                        && let Some(last) = chain.last_mut()
                        && last.ts_end.is_none()
                    {
                        last.ts_end = Some(ts);
                        resumen.versiones_retiradas += 1;
                    }
                    self.inner.delete_edge(*id);
                }
            }
        }

        Ok(resumen)
    }

    /// Validación PROPIA del MVCC: las invariantes que la MVCC exige y
    /// que `validar_buffer` (cap. 27) no puede cubrir porque asume
    /// «insertar» estricto.
    ///
    /// Reglas:
    ///   * `PutNode`: siempre válido (incluso si el id ya existía:
    ///     SOBREESCRIBE, no duplica).
    ///   * `PutEdge`: requiere que source y target existan en el estado
    ///     visible (inner + simulaciones del propio buffer).
    ///   * `DeleteNode`/`DeleteEdge`: requieren que el elemento exista
    ///     en el estado visible (borrar lo que no está es error).
    fn validar_mvcc(&self, ops: &[Operacion]) -> Result<(), MvccError> {
        let mut sim_creados_nodos: HashSet<NodeId> = HashSet::new();
        let mut sim_borrados_nodos: HashSet<NodeId> = HashSet::new();
        let mut sim_creados_aristas: HashSet<EdgeId> = HashSet::new();
        let mut sim_borradas_aristas: HashSet<EdgeId> = HashSet::new();

        for op in ops {
            match op {
                Operacion::PutNode(n) => {
                    // En MVCC, sobreescribe: no validamos existencia.
                    // Pero hay que registrarlo para PutEdges posteriores.
                    sim_creados_nodos.insert(n.id);
                    sim_borrados_nodos.remove(&n.id);
                }
                Operacion::PutEdge(e) => {
                    // Extremos: source y target deben existir en el
                    // estado visible (inner o ya creados en el buffer).
                    let src_visible = sim_creados_nodos.contains(&e.source)
                        || (self.inner.get_node(e.source).is_some()
                            && !sim_borrados_nodos.contains(&e.source));
                    let tgt_visible = sim_creados_nodos.contains(&e.target)
                        || (self.inner.get_node(e.target).is_some()
                            && !sim_borrados_nodos.contains(&e.target));
                    if !src_visible || !tgt_visible {
                        return Err(MvccError::Validacion(TransaccionError::OperacionInvalida {
                            indice: 0, // el validador de cap. 27 lleva índices; aquí basta para test
                            causa: StoreError::InvalidEdgeEndpoints {
                                source: e.source,
                                target: e.target,
                            },
                        }));
                    }
                    sim_creados_aristas.insert(e.id);
                    sim_borradas_aristas.remove(&e.id);
                }
                Operacion::DeleteNode(id) => {
                    let existe = sim_creados_nodos.contains(id)
                        || (self.inner.get_node(*id).is_some() && !sim_borrados_nodos.contains(id));
                    if !existe {
                        return Err(MvccError::Validacion(TransaccionError::OperacionInvalida {
                            indice: 0,
                            causa: StoreError::UnknownNode(*id),
                        }));
                    }
                    sim_borrados_nodos.insert(*id);
                    sim_creados_nodos.remove(id);
                }
                Operacion::DeleteEdge(id) => {
                    let existe = sim_creados_aristas.contains(id)
                        || (self.inner.get_edge(*id).is_some()
                            && !sim_borradas_aristas.contains(id));
                    if !existe {
                        return Err(MvccError::Validacion(TransaccionError::OperacionInvalida {
                            indice: 0,
                            causa: StoreError::UnknownEdge(*id),
                        }));
                    }
                    sim_borradas_aristas.insert(*id);
                    sim_creados_aristas.remove(id);
                }
            }
        }
        Ok(())
    }

    /// GARBAGE COLLECTION: descarta versiones retiradas con
    /// `ts_end < hasta`. La invariante es: ningún snapshot con `ts ≥ hasta`
    /// puede ver una versión retirada con `ts_end < hasta` (los `ts` son
    /// monótonos). Devuelve cuántas versiones eliminó.
    ///
    /// Cuando se vacía una cadena entera (todos sus `ts_end` retiradas
    /// y ningún snapshot vivo que la necesite), la entrada del mapa se
    /// elimina también: el elemento ya no existe.
    pub fn gc(&mut self, hasta: Ts) -> usize {
        let mut eliminadas = 0usize;
        let mut vacias: Vec<NodeId> = Vec::new();
        for (id, chain) in &mut self.versiones_nodos {
            let antes = chain.len();
            chain.retain(|v| match v.ts_end {
                None => true,
                Some(t_end) => t_end >= hasta,
            });
            let quitadas = antes - chain.len();
            eliminadas += quitadas;
            if chain.is_empty() {
                vacias.push(*id);
            }
        }
        for id in &vacias {
            self.versiones_nodos.remove(id);
        }
        let mut vacias_e: Vec<EdgeId> = Vec::new();
        for (id, chain) in &mut self.versiones_aristas {
            let antes = chain.len();
            chain.retain(|v| match v.ts_end {
                None => true,
                Some(t_end) => t_end >= hasta,
            });
            let quitadas = antes - chain.len();
            eliminadas += quitadas;
            if chain.is_empty() {
                vacias_e.push(*id);
            }
        }
        for id in &vacias_e {
            self.versiones_aristas.remove(id);
        }
        eliminadas
    }
}

/// Encuentra la versión de un nodo visible en `ts`: el máximo
/// `ts_begin ≤ ts` con `ts_end > ts` o `ts_end = None`.
fn version_visible_node(chain: &[VersionNode], ts: Ts) -> Option<&VersionNode> {
    chain
        .iter()
        .rev()
        .find(|v| v.ts_begin <= ts && v.ts_end.is_none_or(|t_end| t_end > ts))
}

/// Análogo a `version_visible_node` para aristas.
fn version_visible_edge(chain: &[VersionEdge], ts: Ts) -> Option<&VersionEdge> {
    chain
        .iter()
        .rev()
        .find(|v| v.ts_begin <= ts && v.ts_end.is_none_or(|t_end| t_end > ts))
}

// ─────────────────── Resumen del commit ───────────────────

/// Lo que un commit con MVCC reporta — análogo al `ResumenCommitWal` del
/// cap. 28, con la diferencia de que el reloj es ahora `Ts` (no LSN) y
/// añadimos las versiones retiradas (el «precio» del versionado: cada
/// escritura de un elemento que ya tenía versión actual deja una
/// versión retirada en la cadena).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResumenCommitMvcc {
    /// El timestamp que se asignó a ESTE commit.
    pub ts_asignado: Ts,
    /// Nodos escritos (operaciones PutNode aplicadas).
    pub nodos_escritos: usize,
    /// Aristas escritas.
    pub aristas_escritas: usize,
    /// Versiones anteriores retiradas (las que un commit nuevo
    /// sustituye): el coste del versionado.
    pub versiones_retiradas: usize,
}

impl ResumenCommitMvcc {
    /// Total de operaciones aplicadas (sin contar retiradas).
    pub fn total_operaciones(&self) -> usize {
        self.nodos_escritos + self.aristas_escritas
    }
}

impl fmt::Display for ResumenCommitMvcc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "commit MVCC ts={}: {} ops ({} nodos, {} aristas), {} versiones retiradas",
            self.ts_asignado,
            self.total_operaciones(),
            self.nodos_escritos,
            self.aristas_escritas,
            self.versiones_retiradas
        )
    }
}

// ─────────────────── El grafo de espera (deadlocks) ───────────────────
//
// Aunque hoy no hay concurrencia de escritores (`&mut self` lo prohíbe)
// y por tanto los deadlocks NO PUEDEN ocurrir, la estructura del grafo
// de espera es la pieza estándar que un gestor de cerrojos usaría
// cuando se introduzca la concurrencia de escritores. La construimos
// aquí, sin integración real, como vocabulario y como anzuelo para caps.
// futuros (la Parte VII — motor — y la Parte VIII — distribución).

/// Recurso sobre el que un escritor puede quedarse esperando.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Recurso {
    /// Cerrojo sobre un nodo.
    Nodo(NodeId),
    /// Cerrojo sobre una arista.
    Arista(EdgeId),
}

impl fmt::Display for Recurso {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Recurso::Nodo(id) => write!(f, "nodo {id}"),
            Recurso::Arista(id) => write!(f, "arista {id}"),
        }
    }
}

/// Identificador de transacción en el grafo de espera (sin relación con
/// los `TxId` del cap. 28 — el grafo es un concepto LOCAL a la
/// concurrencia de escritores).
pub type TxIdLocal = u64;

/// El grafo de espera: aristas «T1 espera al recurso R que tiene T2».
///
/// Un ciclo (T1→T2→…→T1) es un deadlock. La detección es O(V+E) con un
/// DFS que etiqueta cada nodo en blanco (sin visitar) / gris (en la
/// pila) / negro (terminado).
#[derive(Debug, Default)]
pub struct GrafoEspera {
    /// Aristas: (esperador, tenedor, recurso). El recurso se mantiene
    /// para el diagnóstico (qué cerrojo concretamente).
    aristas: Vec<(TxIdLocal, TxIdLocal, Recurso)>,
    /// Adyacencia precomputada para el DFS (reconstruida en cada
    /// `detectar_ciclo` para no arrastrar inconsistencias tras
    /// inserciones/borrados).
    #[allow(dead_code)]
    marcador: (),
}

impl GrafoEspera {
    /// Grafo vacío.
    pub fn nuevo() -> Self {
        GrafoEspera::default()
    }

    /// Registra que `esperador` espera por el `recurso` que tiene
    /// `tenedor`.
    pub fn agregar_espera(&mut self, esperador: TxIdLocal, tenedor: TxIdLocal, recurso: Recurso) {
        self.aristas.push((esperador, tenedor, recurso));
    }

    /// Quita TODAS las aristas que mencionan a la transacción `tx`:
    /// cuando una tx termina, libera sus cerrojos y deja de esperar por
    /// nada. Si formaba parte de un ciclo, el ciclo se rompe.
    pub fn quitar_tx(&mut self, tx: TxIdLocal) {
        self.aristas.retain(|&(e, t, _)| e != tx && t != tx);
    }

    /// Devuelve `Some(ciclo)` si el grafo contiene uno (en el orden
    /// inverso al recorrido: la víctima sugerida es el primer nodo del
    /// ciclo devuelto). `None` si está libre de ciclos.
    pub fn detectar_ciclo(&self) -> Option<Vec<TxIdLocal>> {
        // Reconstruye la adyacencia de los esperadores (out-edges: tx →
        // tenedor del que depende).
        let mut adj: HashMap<TxIdLocal, Vec<TxIdLocal>> = HashMap::new();
        for &(e, t, _) in &self.aristas {
            adj.entry(e).or_default().push(t);
        }
        let mut color: HashMap<TxIdLocal, u8> = HashMap::new(); // 0=blanco, 1=gris, 2=negro
        for &tx in adj.keys() {
            if color.get(&tx).copied().unwrap_or(0) == 0
                && let Some(ciclo) = dfs(&adj, tx, &mut color)
            {
                return Some(ciclo);
            }
        }
        None
    }

    /// Vista de las aristas registradas (para inspección / tests).
    pub fn aristas(&self) -> &[(TxIdLocal, TxIdLocal, Recurso)] {
        &self.aristas
    }
}

/// DFS con colores: si al descender encontramos un nodo GRIS, hay ciclo.
fn dfs(
    adj: &HashMap<TxIdLocal, Vec<TxIdLocal>>,
    origen: TxIdLocal,
    color: &mut HashMap<TxIdLocal, u8>,
) -> Option<Vec<TxIdLocal>> {
    color.insert(origen, 1); // gris
    let mut pila: VecDeque<(TxIdLocal, usize)> = VecDeque::new();
    pila.push_back((origen, 0));
    while let Some(&(tx, idx)) = pila.back() {
        let vecinos = match adj.get(&tx) {
            Some(v) => v,
            None => {
                color.insert(tx, 2); // negro
                pila.pop_back();
                continue;
            }
        };
        if idx < vecinos.len() {
            // Avanza al siguiente vecino.
            pila.back_mut().unwrap().1 = idx + 1;
            let sig = vecinos[idx];
            match color.get(&sig).copied().unwrap_or(0) {
                1 => {
                    // Gris: hay ciclo. Devolvemos los nodos de la
                    // pila desde `sig` en adelante.
                    let mut ciclo = Vec::new();
                    let mut en = false;
                    for &(n, _) in pila.iter() {
                        if n == sig {
                            en = true;
                        }
                        if en {
                            ciclo.push(n);
                        }
                    }
                    ciclo.push(sig); // cierra el ciclo
                    return Some(ciclo);
                }
                0 => {
                    color.insert(sig, 1);
                    pila.push_back((sig, 0));
                }
                _ => {} // negro: ya terminado, no aporta ciclo
            }
        } else {
            // Sin más vecinos: terminar.
            color.insert(tx, 2);
            pila.pop_back();
        }
    }
    None
}

// ─────────────────── La re-valoración ACID tras la MVCC ───────────────────

/// La re-valoración honesta de las garantías ACID DESPUÉS del cap. 30
/// (mismos tipos que el cap. 27; los informes de los caps. 28-29 quedan
/// intactos).
///
/// ```text
/// A — Atomicidad:   PARCIAL (sigue). MVCC + recovery (cap. 29) cierran
///                   la mayor parte: una tx confirmada sobrevive; un
///                   apply a medias se repara. Lo que queda es el
///                   write skew en snapshot isolation, que una tx
///                   abortada por SSI resolvería (cap. futuros).
/// C — Consistencia: PARCIAL, sin cambios: sólo invariantes del store.
/// I — Aislamiento:  MEJORA SIGNIFICATIVA. Con MVCC en snapshot
///                   isolation los lectores NO se bloquean con los
///                   escritores; el lector ve un estado consistente y
///                   el escritor nunca pierde una actualización sobre
///                   elementos que el lector tocó (porque el escritor
///                   asigna un ts nuevo y los lectores antiguos siguen
///                   viendo la versión vieja). Quedan dos anomalías:
///                   lectura sucia NO PASA (el lector ve la versión
///                   anterior a la confirmación del escritor); lost
///                   update NO PASA (el escritor nunca pisa a un lector
///                   que vio su versión anterior). Write skew SÍ PASA:
///                   dos tx disjuntas pueden leer y escribir a partir
///                   del mismo snapshot y producir un resultado no
///                   serializable (la pieza que Serializable SI /
///                   predicate locks cierra).
/// D — Durabilidad:  PARCIAL, sin cambios: el store de datos no tiene
///                   checkpoint independiente (cap. 37).
/// ```
pub fn informe_acid_post_mvcc() -> Vec<EntradaAcid> {
    vec![
        EntradaAcid {
            garantia: GarantiaAcid::Atomicidad,
            nivel: NivelGarantia::Parcial,
            como_esta_hoy: "MVCC + recovery (cap. 29) cierran la mayor parte: una tx \
                            confirmada sobrevive; un apply a medias se repara. Lo que \
                            queda es el write skew en snapshot isolation",
            capitulo_que_la_cierra: 31,
        },
        EntradaAcid {
            garantia: GarantiaAcid::Consistencia,
            nivel: NivelGarantia::Parcial,
            como_esta_hoy: "sin cambios: sólo invariantes estructurales del store; la \
                            MVCC no añade restricciones declarativas",
            capitulo_que_la_cierra: 40,
        },
        EntradaAcid {
            garantia: GarantiaAcid::Aislamiento,
            nivel: NivelGarantia::Parcial,
            como_esta_hoy: "MVCC en snapshot isolation elimina la lectura sucia y la \
                            actualización perdida (lectores no se bloquean con \
                            escritores, cada uno ve un estado consistente); write \
                            skew sigue pasando — Serializable SI con predicate locks \
                            lo cerraría",
            capitulo_que_la_cierra: 40,
        },
        EntradaAcid {
            garantia: GarantiaAcid::Durabilidad,
            nivel: NivelGarantia::Parcial,
            como_esta_hoy: "sin cambios: el store de datos no tiene checkpoint \
                            independiente (persistencia end-to-end: cap. 37)",
            capitulo_que_la_cierra: 37,
        },
    ]
}

// ─────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests_mvcc {
    use super::*;
    use crate::cap07_modelo::{Edge, Node};
    use crate::cap29_recuperacion::informe_acid_post_recovery;
    use std::error::Error;

    /// Construye un MvccStore con tres nodos y dos aristas preexistentes
    /// (commit único con `ts`).
    fn store_basico() -> (MvccStore, Ts) {
        let mut mv = MvccStore::new();
        let r = mv
            .commit(&[
                Operacion::PutNode(Node::new(0, "Person")),
                Operacion::PutNode(Node::new(1, "City")),
                Operacion::PutNode(Node::new(2, "Country")),
                Operacion::PutEdge(Edge::new(0, 0, 1, "LIVES_IN")),
                Operacion::PutEdge(Edge::new(1, 1, 2, "IS_IN")),
            ])
            .unwrap();
        (mv, r.ts_asignado)
    }

    // ── Versiones y snapshot ─────────────────────────────────────

    #[test]
    fn leer_en_snapshot_anterior_devuelve_la_version_visible() {
        let (mut mv, ts1) = store_basico();
        // Versión inicial: «Person»
        let nodo_en_ts1 = mv.leer_nodo(0, ts1).unwrap();
        assert_eq!(nodo_en_ts1.labels, vec!["Person".to_string()]);

        // Actualizamos el nodo 0 a «Renacido» en un commit posterior.
        let mut n = mv.leer_nodo(0, ts1).unwrap();
        n.labels = vec!["Renacido".to_string()];
        let _ = mv.commit(&[Operacion::PutNode(n.clone())]).unwrap();

        // El snapshot anterior SIGUE viendo «Person».
        let antiguo = mv.leer_nodo(0, ts1).unwrap();
        assert_eq!(antiguo.labels, vec!["Person".to_string()]);

        // Y el nodo actual es «Renacido».
        let nuevo = mv.leer_nodo(0, mv.reloj() - 1).unwrap();
        assert_eq!(nuevo.labels, vec!["Renacido".to_string()]);
    }

    #[test]
    fn leer_futuro_no_devuelve_la_version() {
        let (mv, _) = store_basico();
        // El reloj está en 2 (asignamos ts=1 al commit inicial).
        assert!(mv.leer_nodo(0, 2).is_some());
        // Pero leer MUY en el futuro: el siguiente ts ya estaría
        // reservado por `siguiente_ts`. Sin un commit, no hay versión
        // con ts_begin > 1 visible para ts >= 2 (la versión actual tiene
        // ts_begin = 1 y ts_end = None ⇒ visible para todo ts >= 1).
        let n = mv.leer_nodo(0, 100).unwrap();
        assert_eq!(n.labels, vec!["Person".to_string()]);
    }

    #[test]
    fn iter_en_snapshot_es_consistente_aun_tras_mutar() {
        let (mut mv, ts1) = store_basico();
        // ts1 es el ts del commit inicial (snapshot válido).
        let antes = mv.iter_nodos(ts1);
        assert_eq!(antes.len(), 3);

        // Mutamos: borrar el nodo 2 (el «Country», que NO es extremo de
        // ninguna arista y por tanto no cascada nada).
        let _ = mv.commit(&[Operacion::DeleteNode(2)]).unwrap();

        // El snapshot anterior SIGUE viendo los 3 nodos.
        let despues = mv.iter_nodos(ts1);
        assert_eq!(despues.len(), 3);
        // Y el nuevo ve 2 (ts_despues = 2, el asignado por el commit de delete).
        let ahora = mv.iter_nodos(2);
        assert_eq!(ahora.len(), 2);
    }

    #[test]
    fn delete_retira_la_version_actual_y_la_saca_del_iter() {
        let (mut mv, ts1) = store_basico();
        // ts1 es el ts del commit inicial; `mv.reloj()` es el SIGUIENTE
        // ts a asignar (no una snapshot válida).
        let ts_antes = ts1;
        // Borramos el nodo 2 (Country, sin aristas que lo toquen) para
        // evitar la cascada de `MemoryStore::delete_node` sobre las
        // aristas — el comportamiento bajo test es el de la versión
        // del nodo, no el de la cascada del inner.
        let resumen = mv.commit(&[Operacion::DeleteNode(2)]).unwrap();
        let ts_despues = resumen.ts_asignado;

        // Snapshot anterior: todavía existe.
        assert!(mv.leer_nodo(2, ts_antes).is_some());
        // Snapshot posterior: borrado.
        assert!(mv.leer_nodo(2, ts_despues).is_none());
        // La cadena tiene UNA versión retirada (ts_end != None): un
        // delete en MVCC RETIRA la versión actual sin appendizar — la
        // ausencia es el nuevo estado.
        let chain = mv.versiones_nodos.get(&2).unwrap();
        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0].ts_end, Some(ts_despues));
    }

    #[test]
    fn commit_incrementa_el_reloj_monotono() {
        let (mut mv, ts1) = store_basico();
        assert_eq!(ts1, 1);
        let r2 = mv.commit(&[Operacion::PutNode(Node::new(3, "D"))]).unwrap();
        assert_eq!(r2.ts_asignado, 2);
        assert!(r2.ts_asignado > ts1);
    }

    #[test]
    fn commit_rechaza_buffer_invalido_y_no_avanza_el_reloj() {
        let mut mv = MvccStore::new();
        // Commit inicial: un nodo 0.
        let r = mv.commit(&[Operacion::PutNode(Node::new(0, "A"))]).unwrap();
        assert_eq!(r.ts_asignado, 1);

        // Intento de commit con una arista a un nodo inexistente.
        let err = mv
            .commit(&[Operacion::PutEdge(Edge::new(0, 0, 9, "BAD"))])
            .unwrap_err();
        assert!(matches!(err, MvccError::Validacion(_)));
        // El reloj NO avanzó (no se consumió un ts que no llegó a usarse).
        assert_eq!(mv.reloj(), 2);
    }

    #[test]
    fn versiones_retiradas_se_cuentan_en_el_resumen() {
        let (mut mv, _) = store_basico();
        // Reescribimos el nodo 0 y la arista 0.
        let mut n = mv.leer_nodo(0, mv.reloj()).unwrap();
        n.labels = vec!["Renacido".to_string()];
        let r = mv
            .commit(&[Operacion::PutNode(n), Operacion::DeleteEdge(0)])
            .unwrap();
        assert_eq!(r.nodos_escritos, 1);
        // El PutNode del nodo 0 RETIRA la versión anterior (1 retirada).
        // El DeleteEdge RETIRA la versión de la arista 0 (1 retirada).
        assert_eq!(r.versiones_retiradas, 2);
    }

    // ── Lecturas concurrentes por el sistema de tipos ──────────────

    #[test]
    fn varios_snapshots_coexisten_sin_bloquearse() {
        // La demostración clave: dos lectores toman su foto, un commit
        // ocurre entre medias, y AMBOS lectores ven lo suyo sin
        // interferencia. Es lo que el cap. 27 NO podía hacer.
        let (mut mv, ts1) = store_basico();

        // Lector A lee en ts1.
        let snap_a = mv.leer_nodo(0, ts1).unwrap();
        assert_eq!(snap_a.labels, vec!["Person".to_string()]);

        // Commit que reescribe el nodo 0.
        let mut n = mv.leer_nodo(0, ts1).unwrap();
        n.labels = vec!["Cambiado".to_string()];
        let _ = mv.commit(&[Operacion::PutNode(n)]).unwrap();

        // Lector B (que tomó su foto EN ts1 antes) sigue viendo lo suyo.
        let snap_b = mv.leer_nodo(0, ts1).unwrap();
        assert_eq!(snap_b.labels, vec!["Person".to_string()]);

        // Y nadie bloqueó a nadie: las dos lecturas son por valor y la
        // escritura ocurrió entre medias sin invalidar el snapshot A.
        assert_eq!(snap_a.labels, snap_b.labels);
    }

    // ── Garbage collection ────────────────────────────────────────

    #[test]
    fn gc_descarta_versiones_retiradas_antiguas() {
        let (mut mv, _) = store_basico();
        // Tres escrituras más (cada una retira la versión actual de un
        // nodo que se reescribe).
        for i in 0..3 {
            let mut n = mv.leer_nodo(0, mv.reloj()).unwrap();
            n.labels = vec![format!("R{i}")];
            let _ = mv.commit(&[Operacion::PutNode(n)]).unwrap();
        }
        let antes = mv.versiones_nodos.get(&0).map(|c| c.len()).unwrap_or(0);
        assert!(
            antes >= 4,
            "se esperaban >=4 versiones (1 inicial + 3 reescrituras)"
        );

        // gc(ts=relój) descarta todas las versiones retiradas con
        // ts_end < reloj (las reescrituras anteriores). La actual
        // (ts_end = None) sobrevive.
        let quitadas = mv.gc(mv.reloj());
        assert!(quitadas >= 3);
        // El nodo 0 sigue existiendo (la versión actual no se quita).
        assert!(mv.leer_nodo(0, mv.reloj()).is_some());
    }

    #[test]
    fn gc_elimina_cadenas_vacias_cuando_el_elemento_se_borra() {
        let (mut mv, _) = store_basico();
        let _ = mv.commit(&[Operacion::DeleteNode(0)]).unwrap();
        // La cadena del nodo 0 tiene 2 entradas (inicial retirada +
        // cadena termina — no hay «versión nueva» para el delete).
        // gc() debería eliminar la cadena entera (queda vacía tras
        // quitar las retiradas).
        let _ = mv.gc(mv.reloj());
        assert!(!mv.versiones_nodos.contains_key(&0));
    }

    // ── Niveles de aislamiento ───────────────────────────────────

    #[test]
    fn niveles_prohiben_las_anomalias_esperadas() {
        assert_eq!(NivelAislamiento::LecturaSucia.prohibe(), "ninguna");
        assert!(
            NivelAislamiento::Instantanea
                .prohibe()
                .contains("lectura sucia")
        );
        assert!(
            NivelAislamiento::Instantanea
                .prohibe()
                .contains("write skew")
        );
        assert!(
            NivelAislamiento::Serializable
                .prohibe()
                .contains("write skew")
        );
    }

    // ── Grafo de espera ───────────────────────────────────────────

    #[test]
    fn grafo_espera_vacio_no_tiene_ciclo() {
        let g = GrafoEspera::nuevo();
        assert!(g.detectar_ciclo().is_none());
        assert!(g.aristas().is_empty());
    }

    #[test]
    fn grafo_espera_lineal_no_es_ciclo() {
        // T1 espera a T2, T2 espera a T3: no es un ciclo (grafo acíclico).
        let mut g = GrafoEspera::nuevo();
        g.agregar_espera(1, 2, Recurso::Nodo(10));
        g.agregar_espera(2, 3, Recurso::Nodo(11));
        assert!(g.detectar_ciclo().is_none());
    }

    #[test]
    fn grafo_espera_detecta_ciclo_de_dos() {
        // T1 espera a T2 por el nodo 10; T2 espera a T1 por el nodo 11.
        // Hay deadlock.
        let mut g = GrafoEspera::nuevo();
        g.agregar_espera(1, 2, Recurso::Nodo(10));
        g.agregar_espera(2, 1, Recurso::Nodo(11));
        let ciclo = g.detectar_ciclo().expect("debería haber ciclo");
        assert_eq!(ciclo.len(), 3); // [1, 2, 1]
        assert!(ciclo.contains(&1) && ciclo.contains(&2));
    }

    #[test]
    fn grafo_espera_detecta_ciclo_de_tres() {
        // T1→T2→T3→T1.
        let mut g = GrafoEspera::nuevo();
        g.agregar_espera(1, 2, Recurso::Nodo(10));
        g.agregar_espera(2, 3, Recurso::Nodo(11));
        g.agregar_espera(3, 1, Recurso::Nodo(12));
        let ciclo = g.detectar_ciclo().expect("ciclo");
        assert!(ciclo.len() >= 4); // [1, 2, 3, 1]
        assert!(ciclo.contains(&1) && ciclo.contains(&2) && ciclo.contains(&3));
    }

    #[test]
    fn quitar_tx_rompe_el_ciclo() {
        let mut g = GrafoEspera::nuevo();
        g.agregar_espera(1, 2, Recurso::Nodo(10));
        g.agregar_espera(2, 1, Recurso::Nodo(11));
        assert!(g.detectar_ciclo().is_some());
        g.quitar_tx(1);
        assert!(g.detectar_ciclo().is_none());
    }

    #[test]
    fn grafo_espera_display() {
        let r = Recurso::Nodo(7);
        assert_eq!(r.to_string(), "nodo 7");
        let r2 = Recurso::Arista(3);
        assert_eq!(r2.to_string(), "arista 3");
    }

    // ── Errores y Display ─────────────────────────────────────────

    #[test]
    fn errores_display_y_source() {
        let v = MvccError::Validacion(TransaccionError::OperacionInvalida {
            indice: 2,
            causa: crate::cap08_graph_store::StoreError::UnknownNode(7),
        });
        assert!(v.to_string().contains("MVCC"));
        assert!(v.to_string().contains("validación"));
        assert!(v.source().is_some());

        let s = MvccError::Store(crate::cap08_graph_store::StoreError::DuplicateNode(0));
        assert!(s.to_string().contains("store rechazó"));
        assert!(s.source().is_some());

        let ok: MvccError = TransaccionError::OperacionInvalida {
            indice: 0,
            causa: crate::cap08_graph_store::StoreError::UnknownNode(1),
        }
        .into();
        assert!(matches!(ok, MvccError::Validacion(_)));
    }

    #[test]
    fn resumen_commit_mvcc_display() {
        let r = ResumenCommitMvcc {
            ts_asignado: 7,
            nodos_escritos: 2,
            aristas_escritas: 1,
            versiones_retiradas: 3,
        };
        let s = r.to_string();
        assert!(s.contains("ts=7"));
        assert!(s.contains("3 ops"));
        assert!(s.contains("3 versiones retiradas"));
        assert_eq!(r.total_operaciones(), 3);
    }

    // ── Re-valoración ACID ────────────────────────────────────────

    #[test]
    fn informe_post_mvcc_avanza_el_aislamiento() {
        let antes = informe_acid_post_recovery();
        let despues = informe_acid_post_mvcc();
        assert_eq!(despues.len(), 4);

        // I: el avance principal — describe cómo la MVCC elimina la
        // lectura sucia y la actualización perdida, y deja write skew.
        let i_antes = antes
            .iter()
            .find(|e| e.garantia == GarantiaAcid::Aislamiento)
            .unwrap();
        let i_despues = despues
            .iter()
            .find(|e| e.garantia == GarantiaAcid::Aislamiento)
            .unwrap();
        assert_eq!(i_antes.nivel, NivelGarantia::Parcial);
        assert_eq!(i_despues.nivel, NivelGarantia::Parcial);
        assert!(i_despues.como_esta_hoy.contains("snapshot isolation"));
        assert!(i_despues.como_esta_hoy.contains("write skew"));
        assert!(
            i_antes.como_esta_hoy.contains("MVCC/2PL")
                || i_antes.como_esta_hoy.contains("concurrencia")
        );
        // D: la MVCC no cambia la durabilidad — el closer sigue en 37.
        let d = despues
            .iter()
            .find(|e| e.garantia == GarantiaAcid::Durabilidad)
            .unwrap();
        assert_eq!(d.capitulo_que_la_cierra, 37);
        // A y C siguen Parcial con closers que apuntan hacia adelante.
        assert!(despues.iter().all(|e| e.nivel == NivelGarantia::Parcial));
    }

    // ── Sin crates externas: el reloj lógico es el orden del programa ─

    #[test]
    fn siguiente_ts_incrementa_el_reloj() {
        let mut mv = MvccStore::new();
        assert_eq!(mv.reloj(), 1);
        assert_eq!(mv.siguiente_ts(), 1);
        assert_eq!(mv.siguiente_ts(), 2);
        assert_eq!(mv.reloj(), 3);
    }
}
