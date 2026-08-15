use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::Path;

use crate::cap07_modelo::{EdgeId, Element, NodeId};
use crate::cap08_graph_store::{GraphStore, StoreError};
use crate::cap27_transacciones::{EntradaAcid, GarantiaAcid, NivelGarantia, Operacion};
use crate::cap28_wal::{CuerpoWal, Lsn, TxId, Wal, WalRecord, aplicar_para_redo};

// ─────────────────── Cap 29: recuperación después de un fallo ───────────────────
//
// El cap. 28 dejó la regla «el cambio se escribe en el WAL antes que en la
// página de datos» hecha protocolo, pero con el arranque A MANO: tras un
// corte de luz, ALGUIEN tenía que ejecutar `replay_wal`. Este capítulo
// construye el arranque AUTOMÁTICO — la recuperación — con el esqueleto de
// ARIES (Mohan et al. 1992, «ARIES: A Transaction Recovery Method…»), el
// algoritmo que casi todas las bases de datos relacionales usan para
// recuperarse de un fallo: **Analysis-Redo-Undo**.
//
// ARIES en tres fases (aquí, simplificado a un Property Graph):
//
// ```text
// 1. ANÁLISIS  — recorrer el log hacia delante para reconstruir:
//                * la tabla de transacciones (quién confirmó, quién no);
//                * los contadores next_lsn / next_tx_id (reabrir = escanear);
//                * la tabla de elementos sucios (primer LSN que tocó cada
//                  nodo/arista) — el germen de la dirty page table.
// 2. REDO      — re-aplicar TODAS las operaciones (confirmadas Y no
//                confirmadas) en orden de LSN, de forma idempotente: el
//                store queda en el estado exacto del instante del fallo.
// 3. UNDO      — deshacer, en orden INVERSO de LSN, las operaciones de las
//                transacciones PERDEDORAS (sin Commit): sus efectos
//                desaparecen y el store queda como si nunca hubieran
//                existido.
// ```
//
// La pieza NUEVA respecto al cap. 28 es el UNDO. Allí la decisión fue
// «commit-marker-ANTES-del-apply»: un apply a medias de una transacción
// CONFIRMADA se completa con redo (roll-forward), y una transacción SIN
// confirmar NO había tocado el store (el staging vive en `WalTransaccion`
// y sólo aplica tras el Commit). Bajo esa política (no-steal), el undo es
// trivialmente vacío: las perdedoras no dejaron huella.
//
// Pero un motor real con buffer pool ROBA («steal»): cuando se queda sin
// páginas, evacúa a disco páginas sucias de transacciones AÚN NO
// confirmadas. Entonces sí hay que DESHACER lo que una perdedora escribió.
// Éste capítulo implementa el undo general y DEMUESTRA con un test-tesis
// (un store al que una perdedora «robó» escrituras) que el undo la
// elimina. De paso, documenta la única frontera que un log de SOLO
// after-image (el del cap. 28) no puede cruzar: deshacer un BORRADO
// robado exige la imagen ANTERIOR del elemento, que el log no guarda —
// es exactamente el hueco que ARIES completo cierra con registros de
// compensación (CLR) y before-images.
//
// Además de la recuperación, el capítulo cierra dos deudas del cap. 28:
//
//   1. EL FICHERO — `guardar_wal` / `cargar_wal` ponen los bytes del log
//      en un fichero real (el `sync` del cap. 28 era un CONTADOR; aquí el
//      fichero ES el almacenamiento estable). `cargar_wal` reconstruye el
//      `Wal` escaneando el log: reabrir = leer fichero + `Wal::reconstruir`.
//
//   2. EL CHECKPOINT Y EL TRUNCADO SEGURO — `Checkpoint` registra «todo lo
//      anterior a este LSN ya es durable»; `truncar_seguro` lo usa para
//      truncar SIN romper el contrato que el cap. 28 dejaba firmado a mano.
//      La ROTACIÓN por tamaño (`rotar_si_excede`) es ese checkpoint
//      disparado por un umbral de bytes: checkpoint + truncado.
//
// Qué NO es todavía (honesto, como en el cap. 28):
//   * El store de DATOS sigue en RAM: la recuperación reconstruye el grafo
//     re-ejecutando el log (replay). El checkpoint de DATOS (persistir el
//     store y truncar el log a vacío sin perder nada) es la integración del
//     cap. 36/37; aquí el checkpoint persiste los CONTADORES y habilita el
//     truncado del prefijo ya durable.
//   * El undo de un borrado robado necesita la imagen anterior, que un log
//     de solo after-image no lleva: `InformeUndo::operaciones_sin_before_image`
//     lo CUENTA y lo dice, no lo calla. ARIES completo lo cierra con CLRs.
//   * Sin concurrencia: `recuperar` es un único escritor (préstamo
//     exclusivo &mut), igual que los caps. 27-28. El aislamiento es cap. 30.

// ─────────────────── Tipos base ───────────────────

/// El estado de una transacción según el log, reconstruido en la fase de
/// análisis de ARIES.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstadoTx {
    /// Se vio su Begin (o una operación) pero NO su Commit: está activa,
    /// es decir, PERDEDORA — el fallo la dejó a medias.
    Activa,
    /// Se vio su Commit: GANADORA — sus efectos deben sobrevivir.
    Confirmada,
    /// Se vio su Rollback: perdedora deliberada — sus efectos (si los
    /// hubiera, por steal) deben deshacerse.
    Abortada,
}

impl EstadoTx {
    /// ¿Es una transacción ganadora (confirmada)?
    pub fn es_confirmada(self) -> bool {
        matches!(self, EstadoTx::Confirmada)
    }
}

impl fmt::Display for EstadoTx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EstadoTx::Activa => write!(f, "activa (perdedora, sin commit)"),
            EstadoTx::Confirmada => write!(f, "confirmada (ganadora)"),
            EstadoTx::Abortada => write!(f, "abortada (perdedora)"),
        }
    }
}

/// Un elemento del grafo (nodo o arista) como clave de la «dirty element
/// table»: el análogo a la dirty page table de ARIES, pero a nivel de
/// elementos en lugar de páginas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElementoId {
    /// Un nodo, identificado por su [`NodeId`].
    Nodo(NodeId),
    /// Una arista, identificada por su [`EdgeId`].
    Arista(EdgeId),
}

impl ElementoId {
    /// Los elementos que una operación toca directamente.
    ///
    /// El borrado de un nodo arrastra sus aristas por cascada (cap. 8),
    /// pero el análisis no las conoce sin leer el store: se registra el
    /// elemento DIRECTAMENTE referenciado. La dirty table existe para
    /// decidir dónde empieza el redo conservador, no para reconstruir la
    /// cascada — la cascada la rehace el redo idempotente.
    pub fn de_operacion(op: &Operacion) -> Vec<ElementoId> {
        match op {
            Operacion::PutNode(n) => vec![ElementoId::Nodo(n.id)],
            Operacion::PutEdge(e) => vec![ElementoId::Arista(e.id)],
            Operacion::DeleteNode(id) => vec![ElementoId::Nodo(*id)],
            Operacion::DeleteEdge(id) => vec![ElementoId::Arista(*id)],
        }
    }
}

impl fmt::Display for ElementoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ElementoId::Nodo(id) => write!(f, "nodo {id}"),
            ElementoId::Arista(id) => write!(f, "arista {id}"),
        }
    }
}

/// Imágenes ANTERIORES de elementos, para el UNDO de borrados robados.
///
/// Un log de solo after-image (el del cap. 28) sabe CÓMO quedó un elemento
/// tras cada operación, pero no cómo estaba ANTES. Para deshacer un
/// `DeleteNode`/`DeleteEdge` robado hay que saber qué había antes: esa
/// imagen la aporta el llamador desde un snapshot de datos (o, en ARIES
/// completo, el propio log mediante before-images/CLR). `capturar_antes`
/// la construye a partir del store en un instante dado.
pub type AntesImagenes = HashMap<ElementoId, Element>;

/// Captura las imágenes anteriores de TODO el store (nodos y aristas).
///
/// Es un snapshot completo — honesto pero no óptimo; un sistema real
/// captura sólo lo necesario. El matiz importante para usarla bien: hay que
/// capturarla ANTES de que la transacción que se va a deshacer toque el
/// store. En un escenario steal (el store ya contiene el efecto de la
/// perdedora), `capturar_antes` sobre ese store captura el estado DE
/// DESPUÉS, no el de antes — por eso el test del borrado robado construye
/// el snapshot manualmente desde un clon pre-crash.
pub fn capturar_antes(store: &dyn GraphStore) -> AntesImagenes {
    let mut antes = HashMap::new();
    for n in store.iter_nodes() {
        antes.insert(ElementoId::Nodo(n.id), Element::Node(n.clone()));
    }
    for e in store.iter_edges() {
        antes.insert(ElementoId::Arista(e.id), Element::Edge(e.clone()));
    }
    antes
}

// ─────────────────── Errores ───────────────────

/// Errores tipados del mundo de la recuperación.
#[derive(Debug)]
pub enum RecoveryError {
    /// La lectura/escritura del fichero del WAL falló (reabrir/guardar).
    Io(std::io::Error),
    /// El REDO falló en el registro con ese LSN: normalmente un log
    /// truncado rompiendo el contrato de durabilidad (una arista referencia
    /// nodos que el prefijo legible ya no contiene).
    Redo { lsn: Lsn, causa: StoreError },
}

impl fmt::Display for RecoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecoveryError::Io(e) => write!(f, "recuperación: E/S del WAL: {e}"),
            RecoveryError::Redo { lsn, causa } => write!(
                f,
                "recuperación: redo falló en lsn {lsn} ({causa}) — ¿se truncó el log \
                 rompiendo el contrato de durabilidad?"
            ),
        }
    }
}

impl std::error::Error for RecoveryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RecoveryError::Io(e) => Some(e),
            RecoveryError::Redo { causa, .. } => Some(causa),
        }
    }
}

// ─────────────────── Fase 1: análisis ───────────────────

/// El resultado medible de la fase de análisis de ARIES.
#[derive(Debug, Clone)]
pub struct Analisis {
    /// Tabla de transacciones: `tx_id → estado`.
    transacciones: HashMap<TxId, EstadoTx>,
    /// Transacciones ganadoras (confirmadas), en orden de primer registro.
    ganadoras: Vec<TxId>,
    /// Transacciones perdedoras (activas o abortadas).
    perdedoras: Vec<TxId>,
    /// Contador de LSN reconstruido: `máximo LSN + 1`.
    next_lsn: Lsn,
    /// Contador de TxId reconstruido: `máximo TxId + 1`.
    next_tx_id: TxId,
    /// Dirty element table: `elemento → primer LSN que lo tocó`.
    sucias: HashMap<ElementoId, Lsn>,
    /// El mínimo de la tabla de sucias (donde un redo CON checkpoint
    /// empezaría; aquí el redo es conservador y lo recorre todo).
    primer_lsn_sucio: Option<Lsn>,
    /// Los registros legibles del log, en orden (parada limpia ante
    /// corrupción, como [`Wal::iter`]).
    registros: Vec<WalRecord>,
}

impl Analisis {
    /// Tabla de transacciones (sólo lectura).
    pub fn transacciones(&self) -> &HashMap<TxId, EstadoTx> {
        &self.transacciones
    }

    /// El estado de una transacción concreta (None si no aparece).
    pub fn estado(&self, tx_id: TxId) -> Option<EstadoTx> {
        self.transacciones.get(&tx_id).copied()
    }

    /// Las transacciones ganadoras.
    pub fn ganadoras(&self) -> &[TxId] {
        &self.ganadoras
    }

    /// Las transacciones perdedoras.
    pub fn perdedoras(&self) -> &[TxId] {
        &self.perdedoras
    }

    /// Contador de LSN reconstruido.
    pub fn next_lsn(&self) -> Lsn {
        self.next_lsn
    }

    /// Contador de TxId reconstruido.
    pub fn next_tx_id(&self) -> TxId {
        self.next_tx_id
    }

    /// Dirty element table.
    pub fn sucias(&self) -> &HashMap<ElementoId, Lsn> {
        &self.sucias
    }

    /// Primer LSN que ensució algún elemento.
    pub fn primer_lsn_sucio(&self) -> Option<Lsn> {
        self.primer_lsn_sucio
    }

    /// Los registros legibles del log.
    pub fn registros(&self) -> &[WalRecord] {
        &self.registros
    }
}

impl fmt::Display for Analisis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "análisis: {} ganadoras, {} perdedoras, {} elementos sucios, \
             next_lsn={}, next_tx_id={}",
            self.ganadoras.len(),
            self.perdedoras.len(),
            self.sucias.len(),
            self.next_lsn,
            self.next_tx_id
        )
    }
}

/// FASE ANÁLISIS: recorre el log hacia delante y reconstruye la tabla de
/// transacciones, los contadores y la dirty element table.
///
/// La lectura usa [`Wal::iter`], que PARA LIMPIO en la primera cola
/// corrupta/truncada: se confía en el prefijo íntegro y se descarta el
/// resto — exactamente la semántica de recuperación de un log append-only.
///
/// Reglas del análisis:
///   * `Begin` → la transacción nace Activa (si no estaba).
///   * `Operacion` → la transacción queda Activa (si aún no se vio su
///     Begin — p.ej. porque el prefijo arranca a mitad de un log truncado)
///     y sus elementos entran en la dirty table con su primer LSN.
///   * `Commit` → la transacción pasa a Confirmada (ganadora).
///   * `Rollback` → la transacción pasa a Abortada (perdedora).
pub fn analizar(wal: &Wal) -> Analisis {
    let mut transacciones: HashMap<TxId, EstadoTx> = HashMap::new();
    let mut sucias: HashMap<ElementoId, Lsn> = HashMap::new();
    let mut registros: Vec<WalRecord> = Vec::new();
    let mut next_lsn = 1u64;
    let mut next_tx_id = 1u64;

    for rec in wal.iter() {
        next_lsn = next_lsn.max(rec.lsn + 1);
        next_tx_id = next_tx_id.max(rec.tx_id + 1);
        match &rec.cuerpo {
            CuerpoWal::Begin => {
                transacciones.entry(rec.tx_id).or_insert(EstadoTx::Activa);
            }
            CuerpoWal::Operacion(op) => {
                transacciones.entry(rec.tx_id).or_insert(EstadoTx::Activa);
                for elem in ElementoId::de_operacion(op) {
                    sucias.entry(elem).or_insert(rec.lsn);
                }
            }
            CuerpoWal::Commit => {
                transacciones.insert(rec.tx_id, EstadoTx::Confirmada);
            }
            CuerpoWal::Rollback => {
                transacciones.insert(rec.tx_id, EstadoTx::Abortada);
            }
        }
        registros.push(rec);
    }

    // El orden de ganadoras/perdedoras es determinista (por primer LSN,
    // que es el orden natural del log). Se deduplican por si un Commit
    // aparece varias veces (idempotente).
    let mut ganadoras: Vec<TxId> = Vec::new();
    let mut perdedoras: Vec<TxId> = Vec::new();
    let mut vistos: HashSet<TxId> = HashSet::new();
    for rec in &registros {
        if !vistos.insert(rec.tx_id) {
            continue;
        }
        match transacciones.get(&rec.tx_id) {
            Some(EstadoTx::Confirmada) => ganadoras.push(rec.tx_id),
            _ => perdedoras.push(rec.tx_id),
        }
    }

    let primer_lsn_sucio = sucias.values().copied().min();
    Analisis {
        transacciones,
        ganadoras,
        perdedoras,
        next_lsn,
        next_tx_id,
        sucias,
        primer_lsn_sucio,
        registros,
    }
}

// ─────────────────── Fase 2: redo ───────────────────

/// FASE REDO: re-aplica TODAS las operaciones (ganadoras Y perdedoras) en
/// orden de LSN, con semántica idempotente (reutiliza `aplicar_para_redo`
/// del cap. 28).
///
/// El objetivo del redo en ARIES es llevar el store al estado EXACTO del
/// instante del fallo: incluye lo que las perdedoras «robaron» (steal) a
/// las páginas. El redo de las perdedoras es necesario para que el undo
/// posterior parta de una base consistente (y porque re-aplicar lo ya
/// aplicado es un no-op gracias a la idempotencia, no un error).
///
/// Aquí el redo es CONSERVADOR: recorre todos los registros. Un ARIES con
/// checkpoint arrancaría en `Analisis::primer_lsn_sucio`; sin checkpoint,
/// todo es potencialmente sucio y se rehace todo — correcto, sólo más
/// trabajo.
pub fn redo(store: &mut dyn GraphStore, analisis: &Analisis) -> Result<usize, RecoveryError> {
    let mut reaplicadas = 0usize;
    for rec in &analisis.registros {
        if let CuerpoWal::Operacion(op) = &rec.cuerpo {
            aplicar_para_redo(store, op).map_err(|causa| RecoveryError::Redo {
                lsn: rec.lsn,
                causa,
            })?;
            reaplicadas += 1;
        }
    }
    Ok(reaplicadas)
}

// ─────────────────── Fase 3: undo ───────────────────

/// El resultado medible del undo.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InformeUndo {
    /// Operaciones deshechas (compensadas) de transacciones perdedoras.
    pub operaciones_deshechas: usize,
    /// Borrados de perdedoras que NO se pudieron deshacer por falta de
    /// imagen anterior (el hueco que ARIES completo cierra con CLR).
    pub operaciones_sin_before_image: usize,
}

impl fmt::Display for InformeUndo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "undo: {} operaciones deshechas, {} sin imagen anterior",
            self.operaciones_deshechas, self.operaciones_sin_before_image
        )
    }
}

/// FASE UNDO: deshace, en orden INVERSO de LSN, las operaciones de las
/// transacciones perdedoras (activas o abortadas).
///
/// La compensación es lógica e IDEMPOTENTE (la recuperación puede correrse
/// varias veces):
///   * `PutNode(n)` → si el nodo está, borrarlo (el inserto se deshace con
///     un borrado; la cascada del store arrastra las aristas de ESTA
///     transacción, que el orden inverso ya habrá borrado antes).
///   * `PutEdge(e)` → si la arista está, borrarla.
///   * `DeleteNode(id)` → si el nodo NO está (el robo lo borró), restaurar
///     la imagen anterior de `antes`; si no hay imagen anterior, se
///     CUENTA en `operaciones_sin_before_image` y se sigue (no se falla:
///     la recuperación reporta el hueco, no lo calla).
///   * `DeleteEdge(id)` → análogo con la arista.
///
/// El orden inverso es EL punto: deshacer primero la arista y después sus
/// nodos evita que un `delete_node` arrastre aristas ajenas; y deshacer
/// «de atrás hacia delante» es lo que ARIES prescribe para que las
/// compensaciones no interfieran entre sí.
pub fn deshacer(
    store: &mut dyn GraphStore,
    analisis: &Analisis,
    antes: &AntesImagenes,
) -> InformeUndo {
    let perdedoras: HashSet<TxId> = analisis.perdedoras.iter().copied().collect();
    let mut informe = InformeUndo::default();

    for rec in analisis.registros.iter().rev() {
        if !perdedoras.contains(&rec.tx_id) {
            continue;
        }
        if let CuerpoWal::Operacion(op) = &rec.cuerpo {
            match op {
                Operacion::PutNode(n) => {
                    if store.get_node(n.id).is_some() {
                        store.delete_node(n.id);
                    }
                    informe.operaciones_deshechas += 1;
                }
                Operacion::PutEdge(e) => {
                    if store.get_edge(e.id).is_some() {
                        store.delete_edge(e.id);
                    }
                    informe.operaciones_deshechas += 1;
                }
                Operacion::DeleteNode(id) => {
                    if store.get_node(*id).is_some() {
                        // Ya restaurado (undo idempotente).
                        informe.operaciones_deshechas += 1;
                    } else {
                        match antes.get(&ElementoId::Nodo(*id)) {
                            Some(Element::Node(n)) => {
                                let _ = store.put_node(n.clone());
                                informe.operaciones_deshechas += 1;
                            }
                            _ => informe.operaciones_sin_before_image += 1,
                        }
                    }
                }
                Operacion::DeleteEdge(id) => {
                    if store.get_edge(*id).is_some() {
                        informe.operaciones_deshechas += 1;
                    } else {
                        match antes.get(&ElementoId::Arista(*id)) {
                            Some(Element::Edge(e)) => {
                                let _ = store.put_edge(e.clone());
                                informe.operaciones_deshechas += 1;
                            }
                            _ => informe.operaciones_sin_before_image += 1,
                        }
                    }
                }
            }
        }
    }

    informe
}

// ─────────────────── La recuperación completa ───────────────────

/// El resultado medible de la recuperación completa (análisis + redo +
/// undo) — el «arranque automático» que el cap. 28 dejó pendiente.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InformeRecuperacion {
    /// Transacciones ganadoras (confirmadas) encontradas en el análisis.
    pub transacciones_ganadoras: usize,
    /// Transacciones perdedoras (sin commit o abortadas).
    pub transacciones_perdedoras: usize,
    /// Operaciones re-aplicadas en la fase redo.
    pub operaciones_redo: usize,
    /// Operaciones deshechas en la fase undo.
    pub operaciones_undo: usize,
    /// Borrados de perdedoras que no se pudieron deshacer (sin before-image).
    pub operaciones_sin_before_image: usize,
    /// Contador de LSN reconstruido.
    pub next_lsn: Lsn,
    /// Contador de TxId reconstruido.
    pub next_tx_id: TxId,
}

impl fmt::Display for InformeRecuperacion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "recuperación: {} ganadoras y {} perdedoras; redo={}, undo={}, \
             sin_before_image={}; next_lsn={}, next_tx_id={}",
            self.transacciones_ganadoras,
            self.transacciones_perdedoras,
            self.operaciones_redo,
            self.operaciones_undo,
            self.operaciones_sin_before_image,
            self.next_lsn,
            self.next_tx_id
        )
    }
}

/// RECUPERACIÓN COMPLETA (ARIES simplificado): análisis → redo → undo.
///
/// Es el punto de entrada del arranque: dado un `Wal` (recién reabierto o
/// no) y un store en cualquier estado (incluso vacío, o a medias por un
/// crash), deja el store con EXACTAMENTE los efectos de las transacciones
/// confirmadas y nada de las perdedoras.
///
/// `antes` aporta las imágenes anteriores para deshacer borrados robados;
/// en el caso más común (sólo insertos) puede ser un mapa vacío.
///
/// ```
/// use vol2_liradb::{AntesImagenes, Edge, GraphStore, MemoryStore, Node, Wal,
///                   WalTransaccion, recuperar};
///
/// // Antes del crash: dos transacciones, una confirmada y una no.
/// let mut store = MemoryStore::new();
/// let mut wal = Wal::new();
/// {
///     let mut tx = WalTransaccion::begin(&mut store, &mut wal);
///     tx.put_node(Node::new(0, "Person")).unwrap();
///     tx.put_node(Node::new(1, "Person")).unwrap();
///     tx.put_edge(Edge::new(0, 0, 1, "KNOWS")).unwrap();
///     tx.commit().unwrap();
/// }
/// {
///     let mut tx = WalTransaccion::begin(&mut store, &mut wal);
///     tx.put_node(Node::new(2, "Fantasma")).unwrap();
///     // Sin commit: el proceso muere con la transacción abierta.
///     drop(tx);
/// }
///
/// // El «reinicio»: store vacío + recuperación = lo confirmado vuelve,
/// // lo no confirmado no existe.
/// let mut renacido = MemoryStore::new();
/// let informe = recuperar(&mut renacido, &wal, &AntesImagenes::new()).unwrap();
/// assert_eq!(informe.transacciones_ganadoras, 1);
/// assert_eq!(informe.transacciones_perdedoras, 1);
/// assert_eq!(renacido.node_count(), 2);
/// assert_eq!(renacido.edge_count(), 1);
/// assert!(renacido.get_node(2).is_none());
/// ```
pub fn recuperar(
    store: &mut dyn GraphStore,
    wal: &Wal,
    antes: &AntesImagenes,
) -> Result<InformeRecuperacion, RecoveryError> {
    let analisis = analizar(wal);
    let operaciones_redo = redo(store, &analisis)?;
    let undo = deshacer(store, &analisis, antes);
    Ok(InformeRecuperacion {
        transacciones_ganadoras: analisis.ganadoras.len(),
        transacciones_perdedoras: analisis.perdedoras.len(),
        operaciones_redo,
        operaciones_undo: undo.operaciones_deshechas,
        operaciones_sin_before_image: undo.operaciones_sin_before_image,
        next_lsn: analisis.next_lsn,
        next_tx_id: analisis.next_tx_id,
    })
}

// ─────────────────── El fichero del WAL (reopen) ───────────────────

/// Persiste los bytes del WAL a un fichero (el almacenamiento estable real;
/// el `sync` del cap. 28 era un contador, aquí el fichero ES la promesa).
///
/// No hace `fsync` explícito: `std::fs::write` cierra el fichero y el
/// sistema lo vuelca. Para un `fsync` garantizado antes de devolver el
/// control (durabilidad estricta), un motor real usaría `File::sync_all`
/// como `FilePager::sync` (cap. 12) — el patrón es idéntico.
pub fn guardar_wal(wal: &Wal, path: impl AsRef<Path>) -> std::io::Result<()> {
    std::fs::write(path, wal.as_bytes())
}

/// Lee el fichero del WAL y reconstruye el objeto [`Wal`] escaneando el
/// log (reabrir = leer fichero + [`Wal::reconstruir`]).
///
/// Los contadores `next_lsn`/`next_tx_id` se recuperan del contenido: los
/// LSN y TxId no se reutilizan aunque el proceso haya muerto.
pub fn cargar_wal(path: impl AsRef<Path>) -> std::io::Result<Wal> {
    let bytes = std::fs::read(path)?;
    Ok(Wal::reconstruir(&bytes))
}

/// REABRIR: lee el WAL del fichero y ejecuta la recuperación completa.
///
/// Es el flujo de arranque de una base de datos real, de principio a fin:
/// leer el log persistido → reconstruir el `Wal` (contadores incluidos) →
/// análisis → redo → undo. Devuelve el informe de recuperación.
///
/// ```
/// use vol2_liradb::{AntesImagenes, Edge, GraphStore, MemoryStore, Node, Wal,
///                   WalTransaccion, guardar_wal, reabrir};
///
/// let dir = tempfile::tempdir().unwrap();
/// let path = dir.path().join("liradb.wal");
///
/// // Sesión 1: se escribe, se confirma y se guarda el log a fichero.
/// {
///     let mut store = MemoryStore::new();
///     let mut wal = Wal::new();
///     let mut tx = WalTransaccion::begin(&mut store, &mut wal);
///     tx.put_node(Node::new(0, "Person")).unwrap();
///     tx.put_node(Node::new(1, "City")).unwrap();
///     tx.put_edge(Edge::new(0, 0, 1, "LIVES_IN")).unwrap();
///     tx.commit().unwrap();
///     guardar_wal(&wal, &path).unwrap();
/// }
///
/// // Sesión 2 (tras el corte de luz): store nuevo, se reabre el fichero.
/// let mut store = MemoryStore::new();
/// let informe = reabrir(&mut store, &path, &AntesImagenes::new()).unwrap();
/// assert_eq!(informe.transacciones_ganadoras, 1);
/// assert_eq!(store.node_count(), 2);
/// assert_eq!(store.edge_count(), 1);
/// ```
pub fn reabrir(
    store: &mut dyn GraphStore,
    path: impl AsRef<Path>,
    antes: &AntesImagenes,
) -> Result<InformeRecuperacion, RecoveryError> {
    let bytes = std::fs::read(path).map_err(RecoveryError::Io)?;
    let wal = Wal::reconstruir(&bytes);
    recuperar(store, &wal, antes)
}

// ─────────────────── Checkpoint y truncado seguro ───────────────────

/// Un checkpoint: el registro de «todo lo anterior a este LSN ya es durable
/// en el store, y los contadores a reanudar tras un reinicio».
///
/// En ARIES el checkpoint es un REGISTRO del propio WAL (para que la fase
/// de análisis lo encuentre al despertar); aquí se modela como estructura
/// aparte — su PAPEL es idéntico, su serialización dentro del log queda
/// como simplificación documentada. Guarda también los contadores porque
/// tras truncar el log a vacío, `Wal::reconstruir` no podría recuperarlos
/// (no queda nada que escanear).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Checkpoint {
    /// Último LSN cuyo efecto es durable en el store (todo lo ≤ a él se
    /// puede truncar sin perder nada).
    pub hasta_lsn: Lsn,
    /// Contador de LSN a reanudar (el checkpoint lo congela).
    pub next_lsn: Lsn,
    /// Contador de TxId a reanudar (el checkpoint lo congela).
    pub next_tx_id: TxId,
}

impl Checkpoint {
    /// Toma un checkpoint del estado actual del [`Wal`].
    ///
    /// CONTRATO (el mismo que `truncar_hasta_lsn` del cap. 28, ahora
    /// automatizado): el llamador asegura que TODO lo escrito hasta
    /// `hasta_lsn` ya está sincronizado en el store. En el modelo en
    /// memoria de este capítulo, `tomar` asume que el store acompaña al
    /// log registro a registro (no hay página sucia descolgada).
    pub fn tomar(wal: &Wal) -> Self {
        Checkpoint {
            hasta_lsn: wal.lsn_siguiente().saturating_sub(1),
            next_lsn: wal.lsn_siguiente(),
            next_tx_id: wal.next_tx_id(),
        }
    }
}

impl fmt::Display for Checkpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "checkpoint: durable hasta lsn {} (resume next_lsn={}, next_tx_id={})",
            self.hasta_lsn, self.next_lsn, self.next_tx_id
        )
    }
}

/// Trunca el log hasta el LSN del checkpoint (el truncado SEGURO que el
/// cap. 28 dejaba firmado a mano). Devuelve cuántos registros eliminó.
pub fn truncar_seguro(wal: &mut Wal, cp: &Checkpoint) -> usize {
    wal.truncar_hasta_lsn(cp.hasta_lsn)
}

/// ROTACIÓN por tamaño: si el log supera `umbral_bytes`, toma un checkpoint
/// y trunca el prefijo durable. Devuelve `Some(checkpoint)` si rotó, `None`
/// si aún no hacía falta.
///
/// Es la deuda «rotación por tamaño» del cap. 28 resuelta como política de
/// una línea: checkpoint disparado por tamaño, no por tiempo. El umbral es
/// decisión del llamador (en producción, cientos de MB).
pub fn rotar_si_excede(wal: &mut Wal, umbral_bytes: usize) -> Option<Checkpoint> {
    if wal.as_bytes().len() <= umbral_bytes {
        return None;
    }
    let cp = Checkpoint::tomar(wal);
    let _ = truncar_seguro(wal, &cp);
    Some(cp)
}

// ─────────────────── La re-valoración ACID tras la recuperación ───────────────────

/// La re-valoración honesta de las garantías ACID DESPUÉS del cap. 29
/// (mismos tipos que el cap. 27; el informe del cap. 28 queda intacto).
///
/// ```text
/// A — Atomicidad:   PARCIAL. El arranque automático (análisis + redo +
///                   undo) repara un apply a medias Y deshace lo no
///                   confirmado (incluso robado por steal). Queda el
///                   before-image: deshacer un BORRADO robado exige la
///                   imagen anterior, que un log de solo after-image no
///                   lleva (ARIES completo: CLR).
/// C — Consistencia: PARCIAL, sin cambios (cap. 30).
/// I — Aislamiento:  PARCIAL, sin cambios (cap. 30).
/// D — Durabilidad:  PARCIAL. El WAL ahora persiste a fichero y se reabre
///                   con recuperación: lo confirmado sobrevive al reinicio
///                   vía replay. El store de datos no tiene checkpoint
///                   independiente (persistencia end-to-end: cap. 37).
/// ```
pub fn informe_acid_post_recovery() -> Vec<EntradaAcid> {
    vec![
        EntradaAcid {
            garantia: GarantiaAcid::Atomicidad,
            nivel: NivelGarantia::Parcial,
            como_esta_hoy: "el arranque automático (análisis + redo + undo) repara un \
                            apply a medias y deshace lo no confirmado, incluso robado \
                            por steal; queda el before-image para deshacer borrados \
                            robados (ARIES completo lo cierra con registros de \
                            compensación)",
            capitulo_que_la_cierra: 30,
        },
        EntradaAcid {
            garantia: GarantiaAcid::Consistencia,
            nivel: NivelGarantia::Parcial,
            como_esta_hoy: "sin cambios: sólo invariantes estructurales del store; la \
                            recuperación no añade restricciones declarativas",
            capitulo_que_la_cierra: 30,
        },
        EntradaAcid {
            garantia: GarantiaAcid::Aislamiento,
            nivel: NivelGarantia::Parcial,
            como_esta_hoy: "sin cambios: un único escritor por préstamo exclusivo &mut; \
                            el aislamiento real (MVCC/2PL) es cap. 30",
            capitulo_que_la_cierra: 30,
        },
        EntradaAcid {
            garantia: GarantiaAcid::Durabilidad,
            nivel: NivelGarantia::Parcial,
            como_esta_hoy: "el WAL persiste a fichero y se reabre con recuperación: lo \
                            confirmado sobrevive al reinicio vía replay; el store de \
                            datos no tiene checkpoint independiente (persistencia \
                            end-to-end: cap. 37)",
            capitulo_que_la_cierra: 37,
        },
    ]
}

// ─────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests_recuperacion {
    use super::*;
    use crate::cap07_modelo::{Edge, Node};
    use crate::cap08_graph_store::MemoryStore;
    use crate::cap28_wal::{PoliticaFlush, WalTransaccion, informe_acid_post_wal};
    use std::error::Error;

    /// Store de pruebas que delega en `MemoryStore` y cuenta cuántas
    /// operaciones de redo intenta el recovery (el «voltímetro» para
    /// verificar que el redo recorre lo esperado). No falla: es para
    /// medir, no para romper.
    struct ContandoStore {
        inner: MemoryStore,
        redo_intentos: usize,
    }

    impl ContandoStore {
        fn new() -> Self {
            ContandoStore {
                inner: MemoryStore::new(),
                redo_intentos: 0,
            }
        }
    }

    impl GraphStore for ContandoStore {
        fn put_node(&mut self, node: Node) -> Result<(), StoreError> {
            self.redo_intentos += 1;
            self.inner.put_node(node)
        }
        fn put_edge(&mut self, edge: Edge) -> Result<(), StoreError> {
            self.redo_intentos += 1;
            self.inner.put_edge(edge)
        }
        fn get_node(&self, id: NodeId) -> Option<&Node> {
            self.inner.get_node(id)
        }
        fn get_edge(&self, id: EdgeId) -> Option<&Edge> {
            self.inner.get_edge(id)
        }
        fn out_edges(&self, u: NodeId) -> Vec<EdgeId> {
            self.inner.out_edges(u)
        }
        fn in_edges(&self, u: NodeId) -> Vec<EdgeId> {
            self.inner.in_edges(u)
        }
        fn node_count(&self) -> usize {
            self.inner.node_count()
        }
        fn edge_count(&self) -> usize {
            self.inner.edge_count()
        }
        fn delete_node(&mut self, id: NodeId) -> bool {
            self.inner.delete_node(id)
        }
        fn delete_edge(&mut self, id: EdgeId) -> bool {
            self.inner.delete_edge(id)
        }
        fn iter_nodes(&self) -> Box<dyn Iterator<Item = &Node> + '_> {
            self.inner.iter_nodes()
        }
        fn iter_edges(&self) -> Box<dyn Iterator<Item = &Edge> + '_> {
            self.inner.iter_edges()
        }
    }

    // ── Análisis ─────────────────────────────────────────────────

    #[test]
    fn analisis_reconstruye_tabla_de_transacciones_y_contadores() {
        let mut store = MemoryStore::new();
        let mut wal = Wal::new();
        // Tx1 confirmada (Begin 1, Op 2, Commit 3).
        {
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_node(Node::new(0, "A")).unwrap();
            tx.commit().unwrap();
        }
        // Tx2 abortada (Begin 4, Op 5, Rollback 6).
        {
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_node(Node::new(1, "B")).unwrap();
            tx.rollback();
        }
        // Tx3 abandonada (Begin 7, Op 8, sin cierre).
        let (tx3, _) = wal.begin_tx();
        wal.log_write(
            tx3,
            CuerpoWal::Operacion(Operacion::PutNode(Node::new(2, "C"))),
        );

        let analisis = analizar(&wal);
        assert_eq!(analisis.estado(1), Some(EstadoTx::Confirmada));
        assert_eq!(analisis.estado(2), Some(EstadoTx::Abortada));
        assert_eq!(analisis.estado(3), Some(EstadoTx::Activa));
        assert_eq!(analisis.ganadoras(), &[1]);
        assert_eq!(analisis.perdedoras(), &[2, 3]);
        assert_eq!(analisis.next_lsn(), 8);
        assert_eq!(analisis.next_tx_id(), 4);
        assert!(analisis.to_string().contains("1 ganadoras"));
        assert!(analisis.to_string().contains("2 perdedoras"));
    }

    #[test]
    fn analisis_tabla_sucias_registra_primer_lsn_de_cada_elemento() {
        let mut wal = Wal::new();
        let (tx, _) = wal.begin_tx();
        // El nodo 0 se toca por primera vez en lsn 2.
        wal.log_write(
            tx,
            CuerpoWal::Operacion(Operacion::PutNode(Node::new(0, "A"))),
        );
        // La arista 0 toca por primera vez en lsn 3.
        wal.log_write(
            tx,
            CuerpoWal::Operacion(Operacion::PutEdge(Edge::new(0, 0, 0, "SELF"))),
        );
        // El nodo 0 se vuelve a tocar en lsn 4 (delete): NO cambia su primer LSN.
        wal.log_write(tx, CuerpoWal::Operacion(Operacion::DeleteNode(0)));
        wal.log_write(tx, CuerpoWal::Commit);

        let analisis = analizar(&wal);
        assert_eq!(analisis.sucias().get(&ElementoId::Nodo(0)), Some(&2));
        assert_eq!(analisis.sucias().get(&ElementoId::Arista(0)), Some(&3));
        assert_eq!(analisis.primer_lsn_sucio(), Some(2));
        assert_eq!(analisis.sucias().len(), 2);
    }

    #[test]
    fn analisis_de_log_vacio_es_neutro() {
        let wal = Wal::new();
        let analisis = analizar(&wal);
        assert!(analisis.ganadoras().is_empty());
        assert!(analisis.perdedoras().is_empty());
        assert_eq!(analisis.next_lsn(), 1);
        assert_eq!(analisis.next_tx_id(), 1);
        assert!(analisis.primer_lsn_sucio().is_none());
        assert!(analisis.sucias().is_empty());
    }

    // ── Recuperación (sin steal) ─────────────────────────────────

    #[test]
    fn recuperar_reenvia_confirmadas_y_descarta_no_confirmadas() {
        let mut store = MemoryStore::new();
        let mut wal = Wal::new();
        {
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_node(Node::new(0, "Person")).unwrap();
            tx.put_node(Node::new(1, "Person")).unwrap();
            tx.put_edge(Edge::new(0, 0, 1, "KNOWS")).unwrap();
            tx.commit().unwrap();
        }
        {
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_node(Node::new(2, "Fantasma")).unwrap();
            tx.rollback();
        }

        let mut renacido = MemoryStore::new();
        let informe = recuperar(&mut renacido, &wal, &AntesImagenes::new()).unwrap();
        assert_eq!(informe.transacciones_ganadoras, 1);
        assert_eq!(informe.transacciones_perdedoras, 1);
        // El rollback NO loguea sus operaciones (sólo el marker Rollback):
        // el redo sólo re-aplica las 3 operaciones del commit, y el undo de
        // la perdedora es vacío (nada suyo llegó al log).
        assert_eq!(informe.operaciones_redo, 3);
        assert_eq!(informe.operaciones_undo, 0);
        assert_eq!(informe.operaciones_sin_before_image, 0);
        assert_eq!(renacido.node_count(), 2);
        assert_eq!(renacido.edge_count(), 1);
        assert!(renacido.get_node(2).is_none());
    }

    #[test]
    fn recuperar_es_idempotente() {
        let mut store = MemoryStore::new();
        let mut wal = Wal::new();
        {
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_node(Node::new(0, "A")).unwrap();
            tx.put_node(Node::new(1, "B")).unwrap();
            tx.put_edge(Edge::new(0, 0, 1, "KNOWS")).unwrap();
            tx.commit().unwrap();
        }
        let primero = recuperar(&mut store, &wal, &AntesImagenes::new()).unwrap();
        let segundo = recuperar(&mut store, &wal, &AntesImagenes::new()).unwrap();
        assert_eq!(primero, segundo);
        assert_eq!(store.node_count(), 2);
        assert_eq!(store.edge_count(), 1);
        assert_eq!(store.out_edges(0), vec![0]);
    }

    // ── El test-tesis: undo de escrituras robadas (steal) ────────

    #[test]
    fn undo_elimina_las_escrituras_robadas_de_una_perdedora() {
        // Escenario steal: una perdedora ESCRIBIÓ al store antes de morir
        // (como un buffer pool que evacúa páginas sucias de una tx no
        // confirmada). El cap. 28 no sabía deshacer esto (no había undo);
        // aquí el undo lo elimina.
        let mut wal = Wal::new();
        let (tx, _) = wal.begin_tx();
        wal.log_write(
            tx,
            CuerpoWal::Operacion(Operacion::PutNode(Node::new(0, "Robado"))),
        );
        wal.log_write(
            tx,
            CuerpoWal::Operacion(Operacion::PutNode(Node::new(1, "Robado"))),
        );
        wal.log_write(
            tx,
            CuerpoWal::Operacion(Operacion::PutEdge(Edge::new(0, 0, 1, "KNOWS"))),
        );
        // SIN Commit: perdedora. Pero el store YA tiene sus escrituras
        // (steal simulado aplicándolas a mano).
        let mut store = MemoryStore::new();
        store.put_node(Node::new(0, "Robado")).unwrap();
        store.put_node(Node::new(1, "Robado")).unwrap();
        store.put_edge(Edge::new(0, 0, 1, "KNOWS")).unwrap();
        assert_eq!(store.node_count(), 2);
        assert_eq!(store.edge_count(), 1);

        let informe = recuperar(&mut store, &wal, &AntesImagenes::new()).unwrap();
        assert_eq!(informe.transacciones_perdedoras, 1);
        assert_eq!(informe.operaciones_undo, 3);
        assert_eq!(informe.operaciones_sin_before_image, 0);
        // El store queda LIMPIO: como si la perdedora nunca hubiera existido.
        assert_eq!(store.node_count(), 0);
        assert_eq!(store.edge_count(), 0);
    }

    #[test]
    fn undo_restaura_un_borrado_robado_con_before_image() {
        // Un borrado robado SÍ se puede deshacer si hay imagen anterior.
        let mut wal = Wal::new();
        let (tx, _) = wal.begin_tx();
        wal.log_write(tx, CuerpoWal::Operacion(Operacion::DeleteNode(0)));

        // El store pre-crash tenía el nodo 0; la perdedora lo borró (steal).
        let mut store = MemoryStore::new();
        store.put_node(Node::new(0, "Original")).unwrap();
        let antes = capturar_antes(&store); // snapshot ANTES del robo
        store.delete_node(0); // el robo
        assert_eq!(store.node_count(), 0);

        let informe = recuperar(&mut store, &wal, &antes).unwrap();
        assert_eq!(informe.operaciones_undo, 1);
        assert_eq!(informe.operaciones_sin_before_image, 0);
        assert_eq!(store.node_count(), 1);
        assert_eq!(
            store.get_node(0).unwrap().labels,
            vec!["Original".to_string()]
        );
    }

    #[test]
    fn borrado_robado_sin_before_image_se_reporta_no_se_calla() {
        // La frontera honesta: sin imagen anterior, el undo NO puede
        // reconstruir el borrado robado, y lo CUENTA en vez de fingir.
        let mut wal = Wal::new();
        let (tx, _) = wal.begin_tx();
        wal.log_write(tx, CuerpoWal::Operacion(Operacion::DeleteNode(0)));

        let mut store = MemoryStore::new();
        store.put_node(Node::new(0, "Original")).unwrap();
        store.delete_node(0); // robo; y NO tenemos snapshot

        let informe = recuperar(&mut store, &wal, &AntesImagenes::new()).unwrap();
        assert_eq!(informe.operaciones_undo, 0);
        assert_eq!(informe.operaciones_sin_before_image, 1);
        assert_eq!(store.node_count(), 0); // el hueco queda documentado, no inventado
        assert!(informe.to_string().contains("sin_before_image=1"));
    }

    #[test]
    fn redo_recorre_todas_las_operaciones_con_contador() {
        // El redo es CONSERVADOR (sin checkpoint rehace todo): el voltímetro
        // cuenta las 3 operaciones del log aunque ya estén en el store.
        let mut wal = Wal::new();
        let (tx, _) = wal.begin_tx();
        wal.log_write(
            tx,
            CuerpoWal::Operacion(Operacion::PutNode(Node::new(0, "A"))),
        );
        wal.log_write(
            tx,
            CuerpoWal::Operacion(Operacion::PutNode(Node::new(1, "B"))),
        );
        wal.log_write(
            tx,
            CuerpoWal::Operacion(Operacion::PutEdge(Edge::new(0, 0, 1, "KNOWS"))),
        );
        wal.log_write(tx, CuerpoWal::Commit);

        let analisis = analizar(&wal);
        let mut contando = ContandoStore::new();
        let reaplicadas = redo(&mut contando, &analisis).unwrap();
        assert_eq!(reaplicadas, 3);
        assert_eq!(contando.redo_intentos, 3);
        assert_eq!(contando.node_count(), 2);
        assert_eq!(contando.edge_count(), 1);
    }

    // ── Checkpoint, truncado y rotación ───────────────────────────

    #[test]
    fn checkpoint_y_truncar_seguro_no_reutiliza_lsns() {
        let mut store = MemoryStore::new();
        let mut wal = Wal::new();
        {
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_node(Node::new(0, "A")).unwrap();
            tx.put_node(Node::new(1, "B")).unwrap();
            tx.commit().unwrap(); // lsns 1..4
        }
        let cp = Checkpoint::tomar(&wal);
        assert_eq!(cp.hasta_lsn, 4);
        assert_eq!(cp.next_lsn, 5);
        assert_eq!(cp.next_tx_id, 2);

        assert_eq!(truncar_seguro(&mut wal, &cp), 4);
        assert_eq!(wal.record_count(), 0);

        // Los contadores del checkpoint permiten reanudar sin reutilizar.
        {
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_node(Node::new(2, "C")).unwrap();
            tx.commit().unwrap(); // lsns 5..7, NO 1..3
        }
        let lsns: Vec<Lsn> = wal.iter().map(|r| r.lsn).collect();
        assert_eq!(lsns.first(), Some(&5));
        assert!(cp.to_string().contains("durable hasta lsn 4"));
    }

    #[test]
    fn rotar_si_excede_trunca_solo_cuando_hace_falta() {
        let mut store = MemoryStore::new();
        let mut wal = Wal::new();
        {
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_node(Node::new(0, "A")).unwrap();
            tx.commit().unwrap();
        }
        // Por debajo del umbral no rota.
        let umbral = wal.as_bytes().len();
        assert!(rotar_si_excede(&mut wal, umbral).is_none());
        // Justo al alcanzar el umbral (ya no «≤») rota y trunca.
        let cp = rotar_si_excede(&mut wal, umbral - 1).unwrap();
        assert_eq!(cp.hasta_lsn, 3);
        assert_eq!(wal.record_count(), 0);
    }

    // ── El fichero del WAL (reopen) ───────────────────────────────

    #[test]
    fn guardar_y_cargar_wal_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("liradb.wal");

        let mut store = MemoryStore::new();
        let mut wal = Wal::new();
        {
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_node(Node::new(0, "Person")).unwrap();
            tx.put_edge(Edge::new(0, 0, 0, "SELF")).unwrap();
            tx.commit().unwrap();
        }
        guardar_wal(&wal, &path).unwrap();

        let cargado = cargar_wal(&path).unwrap();
        assert_eq!(cargado.as_bytes(), wal.as_bytes());
        assert_eq!(cargado.lsn_siguiente(), wal.lsn_siguiente());
        assert_eq!(cargado.next_tx_id(), wal.next_tx_id());
    }

    #[test]
    fn reabrir_recupera_lo_confirmado_tras_corte_de_luz() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("liradb.wal");

        {
            let mut store = MemoryStore::new();
            let mut wal = Wal::new();
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_node(Node::new(0, "Person")).unwrap();
            tx.put_node(Node::new(1, "City")).unwrap();
            tx.put_edge(Edge::new(0, 0, 1, "LIVES_IN")).unwrap();
            tx.commit().unwrap();
            guardar_wal(&wal, &path).unwrap();
        }

        let mut renacido = MemoryStore::new();
        let informe = reabrir(&mut renacido, &path, &AntesImagenes::new()).unwrap();
        assert_eq!(informe.transacciones_ganadoras, 1);
        assert_eq!(renacido.node_count(), 2);
        assert_eq!(renacido.edge_count(), 1);
        assert_eq!(
            renacido.get_node(0).unwrap().labels,
            vec!["Person".to_string()]
        );
    }

    #[test]
    fn cargar_wal_reconstruye_contadores_sin_reutilizar() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("liradb.wal");

        let mut store = MemoryStore::new();
        let mut wal = Wal::new();
        for i in 0..3 {
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_node(Node::new(i, "Person")).unwrap();
            tx.commit().unwrap();
        }
        guardar_wal(&wal, &path).unwrap();

        let cargado = cargar_wal(&path).unwrap();
        // 3 txs → 3 Begin + 3 ops + 3 Commit = 9 registros, next_lsn = 10.
        assert_eq!(cargado.lsn_siguiente(), 10);
        assert_eq!(cargado.next_tx_id(), 4);
    }

    // ── Errores y re-valoración ACID ──────────────────────────────

    #[test]
    fn redo_falla_ruidosamente_si_el_truncado_rompio_dependencias() {
        // El mismo escenario del cap. 28: truncar rompiendo el contrato
        // deja una arista huérfana; la recuperación lo grita, no lo calla.
        let mut wal = Wal::new();
        let (tx1, _) = wal.begin_tx();
        wal.log_write(
            tx1,
            CuerpoWal::Operacion(Operacion::PutNode(Node::new(0, "A"))),
        );
        wal.log_write(tx1, CuerpoWal::Commit);
        let (tx2, _) = wal.begin_tx();
        wal.log_write(
            tx2,
            CuerpoWal::Operacion(Operacion::PutEdge(Edge::new(0, 0, 1, "KNOWS"))),
        );
        wal.log_write(tx2, CuerpoWal::Commit);
        // Truncar el nodo 0 (y el nodo 1, que ni existía) rompe la arista.
        let _ = wal.truncar_hasta_lsn(3);

        let mut store = MemoryStore::new();
        let err = recuperar(&mut store, &wal, &AntesImagenes::new()).unwrap_err();
        assert!(err.to_string().contains("redo falló"));
        match err {
            RecoveryError::Redo { lsn, causa } => {
                assert_eq!(lsn, 5);
                assert!(matches!(
                    causa,
                    StoreError::InvalidEdgeEndpoints {
                        source: 0,
                        target: 1
                    }
                ));
            }
            otra => panic!("esperaba Redo, llegó {otra:?}"),
        }
    }

    #[test]
    fn errores_display_y_std_error() {
        let io = RecoveryError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "no wal"));
        assert!(io.to_string().contains("no wal"));
        assert!(io.source().is_some());

        let redo = RecoveryError::Redo {
            lsn: 9,
            causa: StoreError::UnknownNode(3),
        };
        assert!(redo.to_string().contains("lsn 9"));
        assert!(redo.to_string().contains("contrato de durabilidad"));
        assert_eq!(
            redo.source().unwrap().to_string(),
            StoreError::UnknownNode(3).to_string()
        );
    }

    #[test]
    fn informe_post_recovery_actualiza_a_y_d() {
        let antes = informe_acid_post_wal();
        let despues = informe_acid_post_recovery();
        assert_eq!(despues.len(), 4);

        // A: sigue Parcial, pero su cierre pasa de 29 a 30 (falta el
        // before-image para borrados robados).
        assert_eq!(antes[0].capitulo_que_la_cierra, 29);
        assert_eq!(despues[0].capitulo_que_la_cierra, 30);
        assert_eq!(despues[0].nivel, NivelGarantia::Parcial);
        assert!(despues[0].como_esta_hoy.contains("arranque automático"));

        // D: sigue Parcial, pero su cierre pasa de 29 a 37 (persistencia
        // end-to-end).
        assert_eq!(antes[3].capitulo_que_la_cierra, 29);
        assert_eq!(despues[3].capitulo_que_la_cierra, 37);
        assert!(despues[3].como_esta_hoy.contains("persiste a fichero"));

        // C e I: sin cambios, cerradas por el 30.
        assert_eq!(despues[1].capitulo_que_la_cierra, 30);
        assert_eq!(despues[2].capitulo_que_la_cierra, 30);
        assert_eq!(despues[1].nivel, NivelGarantia::Parcial);
        assert_eq!(despues[2].nivel, NivelGarantia::Parcial);
    }

    // ── Intercalación y parada limpia ────────────────────────────

    #[test]
    fn analisis_maneja_transacciones_intercaladas() {
        // Dos tx abiertas a la vez, intercaladas en el log: lo que decide
        // es el Commit, igual que en el cap. 28.
        let mut wal = Wal::new();
        let (tx1, _) = wal.begin_tx();
        wal.log_write(
            tx1,
            CuerpoWal::Operacion(Operacion::PutNode(Node::new(0, "Abandonada"))),
        );
        let (tx2, _) = wal.begin_tx();
        wal.log_write(
            tx2,
            CuerpoWal::Operacion(Operacion::PutNode(Node::new(1, "Confirmada"))),
        );
        wal.log_write(tx2, CuerpoWal::Commit);
        wal.log_write(
            tx1,
            CuerpoWal::Operacion(Operacion::PutNode(Node::new(2, "Tardía"))),
        );
        wal.log_write(tx1, CuerpoWal::Rollback);

        let analisis = analizar(&wal);
        assert_eq!(analisis.ganadoras(), &[2]);
        assert_eq!(analisis.perdedoras(), &[1]);

        // La confirmada vuelve, la abandonada no (rollback la descarta).
        let mut store = MemoryStore::new();
        let informe = recuperar(&mut store, &wal, &AntesImagenes::new()).unwrap();
        assert_eq!(informe.transacciones_ganadoras, 1);
        assert_eq!(store.node_count(), 1);
        assert!(store.get_node(1).is_some());
        assert!(store.get_node(0).is_none());
        assert!(store.get_node(2).is_none());
    }

    #[test]
    fn cola_truncada_el_analisis_para_limpio_en_el_prefijo_integro() {
        // Un Commit record a medias: el iterador para limpio y el análisis
        // no ve el Commit → la transacción es perdedora (no confirmada).
        let mut store = MemoryStore::new();
        let mut wal = Wal::new();
        {
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_node(Node::new(0, "A")).unwrap();
            tx.commit().unwrap(); // Begin(1) Op(2) Commit(3)
        }
        let len = wal.as_bytes().len();
        wal.truncar_a_bytes(len - 2); // el Commit queda a medias

        let analisis = analizar(&wal);
        assert!(analisis.ganadoras().is_empty());
        assert_eq!(analisis.perdedoras(), &[1]);

        let mut renacido = MemoryStore::new();
        let informe = recuperar(&mut renacido, &wal, &AntesImagenes::new()).unwrap();
        assert_eq!(informe.transacciones_ganadoras, 0);
        assert_eq!(renacido.node_count(), 0);
    }

    #[test]
    fn politica_flush_no_cambia_la_recuperacion() {
        // La optimización SoloCommit produce el MISMO log y la MISMA
        // recuperación que la regla de oro CadaEscritura.
        let mut store_a = MemoryStore::new();
        let mut wal_a = Wal::con_politica(PoliticaFlush::SoloCommit);
        {
            let mut tx = WalTransaccion::begin(&mut store_a, &mut wal_a);
            tx.put_node(Node::new(0, "A")).unwrap();
            tx.put_node(Node::new(1, "B")).unwrap();
            tx.commit().unwrap();
        }
        let mut store_b = MemoryStore::new();
        let mut wal_b = Wal::new();
        {
            let mut tx = WalTransaccion::begin(&mut store_b, &mut wal_b);
            tx.put_node(Node::new(0, "A")).unwrap();
            tx.put_node(Node::new(1, "B")).unwrap();
            tx.commit().unwrap();
        }

        let mut ra = MemoryStore::new();
        let mut rb = MemoryStore::new();
        recuperar(&mut ra, &wal_a, &AntesImagenes::new()).unwrap();
        recuperar(&mut rb, &wal_b, &AntesImagenes::new()).unwrap();
        assert_eq!(ra.node_count(), rb.node_count());
        assert_eq!(ra.get_node(0), rb.get_node(0));
    }
}
