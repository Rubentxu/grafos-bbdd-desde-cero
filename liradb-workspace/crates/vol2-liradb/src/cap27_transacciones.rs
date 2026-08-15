use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::cap07_modelo::{Edge, EdgeId, Node, NodeId};
use crate::cap08_graph_store::{GraphStore, StoreError};

// ─────────────────── Cap 27: transacciones (ACID) ───────────────────
//
// ABRE LA PARTE VI (Fiabilidad). Hasta aquí, LiraDB era un motor de
// consultas y algoritmos sobre un grafo que SÓLO crece bien: cada
// `put_node`/`put_edge`/`delete_*` del `GraphStore` (cap 8) es su
// PROPIA transacción — lo que una base de datos real llama AUTOCOMMIT.
// Si la operación 5 de un lote de 10 falla, las 4 anteriores ya están
// aplicadas y el grafo quedó a medias. Este capítulo introduce el
// vocabulario y la primera maquinaria para evitarlo.
//
// Qué añade el capítulo:
//   1. VOCABULARIO ACID TIPODO (`GarantiaAcid`, `NivelGarantia`,
//      `InformeAcid`, `Anomalia`): qué significa cada letra PARA LiraDB
//      en este punto del libro, con una valoración HONESTA del estado
//      actual — qué garantiza hoy `MemoryStore` y qué no (ver
//      `informe_acid()`). Los caps. 28-30 irán llenando este esqueleto.
//   2. LA TRANSACCIÓN COMO OBJETO (`Transaccion`): begin → staging →
//      commit/rollback. Las operaciones se ACUMULAN en un buffer y sólo
//      se aplican al commit, tras validar TODO el buffer de golpe:
//      atomicidad NAIVE por acumulación — o se aplican todas o ninguna
//      (frente a errores de validación).
//
// Qué NO garantiza todavía (y se documenta en el código y en los tests):
//   * Si el APPLY real falla a mitad (error o crash entre escrituras),
//     el store QUEDA A MEDIOS: sin log no hay vuelta atrás. Es el gancho
//     al WAL del cap. 28 (el append-only log del cap. 10 es su germen) y
//     a la recuperación del cap. 29. Dos tests lo DEMUESTRAN.
//   * Commit en RAM no es durable: un corte de luz lo borra. El camino a
//     disco ya existe (`Pager::sync` cap 12, `BufferPool::flush` cap 13),
//     pero falta el PROTOCOLO write-ahead (cap. 28). Sólo documentado.
//   * No hay aislamiento que construir: el modelo del brief — «múltiples
//     lectores, un único escritor» — está FORZADO por el préstamo
//     exclusivo `&mut dyn GraphStore`: mientras vive una `Transaccion`,
//     NI otro escritor NI ningún lector puede tocar el store (el borrow
//     checker es el cerrojo). Las anomalías clásicas (lectura sucia,
//     lost update) se definen como vocabulario; MVCC/2PL llegan en el
//     cap. 30.
//
// Nota de alcance (fiel al guion): este capítulo es CONCEPTUAL + primera
// maquinaria. El WAL real es cap. 28, la recuperación cap. 29 y la
// concurrencia cap. 30. Aquí se sientan las palabras y el esqueleto.

// ─────────────────── Vocabulario ACID tipado ───────────────────

/// Las cuatro garantías de una transacción, como tipo.
///
/// Cada variante sabe su letra, su nombre y qué significa PARA LiraDB
/// (no la definición de un manual genérico): los caps. 28-30 las irán
/// convirtiendo de vocabulario en maquinaria.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GarantiaAcid {
    /// «O todo o nada»: el grafo nunca queda a medias.
    Atomicidad,
    /// Las invariantes se cumplen antes y después de cada transacción.
    Consistencia,
    /// Transacciones concurrentes no se ven unas a otras a medias.
    Aislamiento,
    /// Lo confirmado sobrevive al corte de luz.
    Durabilidad,
}

impl GarantiaAcid {
    /// La letra del acrónimo (para `InformeAcid` y el explain humano).
    pub fn letra(self) -> char {
        match self {
            GarantiaAcid::Atomicidad => 'A',
            GarantiaAcid::Consistencia => 'C',
            GarantiaAcid::Aislamiento => 'I',
            GarantiaAcid::Durabilidad => 'D',
        }
    }

    /// Nombre largo en español.
    pub fn nombre(self) -> &'static str {
        match self {
            GarantiaAcid::Atomicidad => "Atomicidad",
            GarantiaAcid::Consistencia => "Consistencia",
            GarantiaAcid::Aislamiento => "Aislamiento",
            GarantiaAcid::Durabilidad => "Durabilidad",
        }
    }

    /// Qué significa esta garantía PARA LiraDB.
    pub fn definicion(self) -> &'static str {
        match self {
            GarantiaAcid::Atomicidad => {
                "o se aplican TODAS las operaciones de la transacción o NINGUNA: \
                 el grafo nunca queda a medias"
            }
            GarantiaAcid::Consistencia => {
                "las invariantes del grafo se cumplen antes y después de cada \
                 transacción (extremos de arista existentes, ids sin duplicados)"
            }
            GarantiaAcid::Aislamiento => {
                "transacciones concurrentes no se ven unas a otras a medias: \
                 sin lecturas sucias ni actualizaciones perdidas"
            }
            GarantiaAcid::Durabilidad => {
                "una vez confirmado el commit, el corte de luz no lo borra: \
                 lo confirmado está en disco"
            }
        }
    }
}

impl fmt::Display for GarantiaAcid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.nombre(), self.letra())
    }
}

/// Valoración honesta de hasta dónde llega HOY una garantía.
///
/// Es deliberadamente un vocabulario de tres niveles, no un bool: la
/// gracia del capítulo es que NINGUNA letra está completa todavía.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NivelGarantia {
    /// No existe ninguna maquinaria que la proporcione.
    Ninguna,
    /// Hay una versión ingenua/parcial, con límites documentados.
    Parcial,
    /// La garantía se cumple (ninguna llega a este nivel en el cap. 27).
    Completa,
}

impl NivelGarantia {
    /// Etiqueta legible.
    pub fn nombre(self) -> &'static str {
        match self {
            NivelGarantia::Ninguna => "ninguna (todavía)",
            NivelGarantia::Parcial => "parcial / naive",
            NivelGarantia::Completa => "completa",
        }
    }
}

impl fmt::Display for NivelGarantia {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.nombre())
    }
}

/// Una fila del informe ACID: la garantía, su nivel hoy, la explicación
/// honesta y el capítulo que cerrará la brecha.
#[derive(Debug, Clone)]
pub struct EntradaAcid {
    /// La letra.
    pub garantia: GarantiaAcid,
    /// Hasta dónde llega en el cap. 27.
    pub nivel: NivelGarantia,
    /// Qué garantiza HOY `MemoryStore` + `Transaccion`, sin maquillaje.
    pub como_esta_hoy: &'static str,
    /// El capítulo del Vol.II que construye lo que falta.
    pub capitulo_que_la_cierra: u8,
}

/// El informe ACID completo de LiraDB tal y como está en el cap. 27.
///
/// Es un artefacto EJECUTABLE: los tests lo verifican para que la
/// documentación no prometa más de lo que el código cumple.
#[derive(Debug, Clone)]
pub struct InformeAcid {
    entradas: Vec<EntradaAcid>,
}

impl InformeAcid {
    /// Las entradas en orden A, C, I, D.
    pub fn entradas(&self) -> &[EntradaAcid] {
        &self.entradas
    }

    /// Busca la entrada de una garantía concreta.
    pub fn por_garantia(&self, garantia: GarantiaAcid) -> Option<&EntradaAcid> {
        self.entradas.iter().find(|e| e.garantia == garantia)
    }

    /// Las letras del informe, en orden («ACID»).
    pub fn letras(&self) -> String {
        self.entradas.iter().map(|e| e.garantia.letra()).collect()
    }
}

impl fmt::Display for InformeAcid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Informe ACID de LiraDB (cap. 27):")?;
        for e in &self.entradas {
            writeln!(
                f,
                "  {} — {}: {}. {} (cap. {} lo cierra)",
                e.garantia.letra(),
                e.garantia.nombre(),
                e.nivel.nombre(),
                e.como_esta_hoy,
                e.capitulo_que_la_cierra
            )?;
        }
        write!(f, "  (W: nada de esto es durable sin WAL — cap. 28)")
    }
}

/// El estado honesto de las cuatro garantías al cerrar este capítulo.
///
/// ```text
/// A — Atomicidad:      PARCIAL. El staging de `Transaccion` da «o todo o
///                      nada» frente a ERRORES DE VALIDACIÓN (se valida el
///                      buffer completo antes de tocar el store). Pero si el
///                      APPLY real falla a mitad (error o crash entre
///                      escrituras), el store queda a medias: sin log no hay
///                      vuelta atrás. El WAL (cap. 28) + recuperación
///                      (cap. 29) la cierran de verdad.
/// C — Consistencia:    PARCIAL y trivial: las únicas invariantes son las
///                      estructurales del store (sin duplicados, extremos de
///                      arista existentes), validadas por operación. No hay
///                      restricciones declarativas ni esquema; en una BD real
///                      la C es un contrato COMPARTIDO entre motor y
///                      aplicación, no algo que el motor pueda dar solo.
/// I — Aislamiento:     PARCIAL por diseño, no por motor: no hay concurrencia.
///                      El modelo del brief («múltiples lectores, un único
///                      escritor») lo fuerza el préstamo exclusivo &mut: mientras
///                      vive una Transaccion, nadie más toca el store. Con un
///                      solo hilo no hay lecturas sucias ni lost updates que
///                      aislar; MVCC/2PL son el cap. 30.
/// D — Durabilidad:     NINGUNA. commit() sólo muta RAM: un corte de luz borra
///                      todo lo confirmado. Las piezas existen (Pager::sync del
///                      cap. 12, BufferPool::flush del cap. 13) pero falta el
///                      protocolo write-ahead: el WAL del cap. 28.
/// ```
pub fn informe_acid() -> InformeAcid {
    InformeAcid {
        entradas: vec![
            EntradaAcid {
                garantia: GarantiaAcid::Atomicidad,
                nivel: NivelGarantia::Parcial,
                como_esta_hoy: "el staging da «o todo o nada» frente a errores de \
                                validación, pero un fallo durante el APPLY real deja \
                                el store a medias (demostrado en tests): sin log no \
                                hay vuelta atrás",
                capitulo_que_la_cierra: 28,
            },
            EntradaAcid {
                garantia: GarantiaAcid::Consistencia,
                nivel: NivelGarantia::Parcial,
                como_esta_hoy: "sólo las invariantes estructurales del store (ids sin \
                                duplicados, extremos de arista existentes), validadas \
                                por operación; no hay restricciones declarativas — la C \
                                es un contrato compartido con la aplicación",
                capitulo_que_la_cierra: 30,
            },
            EntradaAcid {
                garantia: GarantiaAcid::Aislamiento,
                nivel: NivelGarantia::Parcial,
                como_esta_hoy: "no hay concurrencia: el préstamo exclusivo &mut del \
                                store («un único escritor» del brief) lo garantiza el \
                                borrow checker, no un motor de aislamiento; con un hilo \
                                no hay anomalías que aislar",
                capitulo_que_la_cierra: 30,
            },
            EntradaAcid {
                garantia: GarantiaAcid::Durabilidad,
                nivel: NivelGarantia::Ninguna,
                como_esta_hoy: "commit() sólo muta RAM: un corte de luz borra todo lo \
                                confirmado; el camino a disco existe (sync cap. 12, \
                                flush cap. 13) pero falta el protocolo WAL",
                capitulo_que_la_cierra: 28,
            },
        ],
    }
}

/// Las anomalías clásicas del aislamiento, como vocabulario tipado.
///
/// Hoy NO pueden ocurrir (un solo hilo + préstamo exclusivo), pero el
/// cap. 30 las hará posibles y las combatirá con MVCC/2PL: primero se
/// nombra el enemigo, luego se construye la defensa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Anomalia {
    /// Leer lo que otra transacción aún no ha confirmado (y quizás
    /// revierta): ver un futuro que puede deshacerse.
    LecturaSucia,
    /// Dos transacciones escriben la misma pieza y una pisa a la otra
    /// sin enterarse: el update del perdedor se pierde.
    ActualizacionPerdida,
}

impl Anomalia {
    /// Nombre largo en español.
    pub fn nombre(self) -> &'static str {
        match self {
            Anomalia::LecturaSucia => "lectura sucia (dirty read)",
            Anomalia::ActualizacionPerdida => "actualización perdida (lost update)",
        }
    }

    /// Definición de la anomalía.
    pub fn definicion(self) -> &'static str {
        match self {
            Anomalia::LecturaSucia => {
                "una transacción lee cambios de otra que aún no ha hecho commit: \
                 si la otra hace rollback, se leyó algo que nunca existió"
            }
            Anomalia::ActualizacionPerdida => {
                "dos transacciones modifican el mismo elemento a la vez y la última \
                 en escribir pisa a la primera: su actualización se pierde en silencio"
            }
        }
    }

    /// Por qué no puede ocurrir en LiraDB hoy (cap. 27).
    pub fn por_que_no_pasa_hoy(self) -> &'static str {
        "no hay concurrencia: mientras vive una Transaccion, el préstamo \
         exclusivo &mut del store impide que cualquier otro lector o escritor \
         lo toque (modelo «múltiples lectores, un único escritor» del brief, \
         ejecutado por el borrow checker)"
    }
}

impl fmt::Display for Anomalia {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.nombre())
    }
}

// ─────────────────── La operación como dato ───────────────────

/// Una operación de escritura sobre el grafo, como DATO.
///
/// Es la pieza clave del staging: mientras las operaciones son valores
/// que se acumulan en un buffer, la transacción puede validarlas TODAS
/// juntas antes de tocar el store — y descartarlas limpio en rollback.
/// El cap. 28 serializará exactamente esto al WAL (el `RecordKind` del
/// append-only log del cap. 10 ya anticipa el formato).
#[derive(Debug, Clone, PartialEq)]
pub enum Operacion {
    /// Insertar un nodo (id nuevo).
    PutNode(Node),
    /// Insertar una arista (id nuevo, extremos existentes).
    PutEdge(Edge),
    /// Eliminar un nodo (con su cascada de aristas).
    DeleteNode(NodeId),
    /// Eliminar una arista.
    DeleteEdge(EdgeId),
}

impl fmt::Display for Operacion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operacion::PutNode(n) => write!(f, "PUT nodo {} {:?}", n.id, n.labels),
            Operacion::PutEdge(e) => {
                write!(f, "PUT arista {} ({} -> {})", e.id, e.source, e.target)
            }
            Operacion::DeleteNode(id) => write!(f, "DELETE nodo {id}"),
            Operacion::DeleteEdge(id) => write!(f, "DELETE arista {id}"),
        }
    }
}

// ─────────────────── Errores ───────────────────

/// Errores tipados del mundo transaccional.
#[derive(Debug, Clone, PartialEq)]
pub enum TransaccionError {
    /// La operación nº `indice` del buffer no es válida (contra el store
    /// o contra las operaciones anteriores del PROPIO buffer). Nada se
    /// ha aplicado: la transacción sigue usable sin esa operación.
    OperacionInvalida {
        /// Posición de la operación en el buffer (desde 0).
        indice: usize,
        /// Qué regla se rompió.
        causa: StoreError,
    },
    /// El APPLY real falló en la operación nº `indice` cuando `aplicadas`
    /// operaciones ya estaban escritas en el store. El store QUEDÓ A
    /// MEDIAS y no hay log para deshacerlo: el gancho al WAL (cap. 28)
    /// y a la recuperación (cap. 29). En teoría no puede ocurrir (el
    /// buffer se validó entero antes de aplicar); en la práctica, un
    /// store que diga «no» a lo que la simulación aprobó —o un proceso
    /// que muera a mitad— deja exactamente esto.
    ApplyFallido {
        /// Posición de la operación que falló.
        indice: usize,
        /// Cuántas operaciones ya estaban aplicadas cuando falló.
        aplicadas: usize,
        /// El error del store.
        causa: StoreError,
    },
}

impl fmt::Display for TransaccionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransaccionError::OperacionInvalida { indice, causa } => write!(
                f,
                "la operación #{} no es válida en esta transacción: {} \
                 (nada se ha aplicado)",
                indice, causa
            ),
            TransaccionError::ApplyFallido {
                indice,
                aplicadas,
                causa,
            } => write!(
                f,
                "fallo durante el APPLY de la operación #{} ({}): {} operaciones \
                 ya estaban aplicadas y sin log no se pueden deshacer — el store \
                 quedó a medias (cap. 28: WAL)",
                indice, causa, aplicadas
            ),
        }
    }
}

impl std::error::Error for TransaccionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TransaccionError::OperacionInvalida { causa, .. }
            | TransaccionError::ApplyFallido { causa, .. } => Some(causa),
        }
    }
}

// ─────────────────── La transacción como objeto ───────────────────

/// Resultado de un commit con éxito.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResumenCommit {
    /// Nodos escritos (`PutNode` aplicados).
    pub nodos_escritos: usize,
    /// Aristas escritas (`PutEdge` aplicadas).
    pub aristas_escritas: usize,
    /// Nodos borrados explícitamente (`DeleteNode` aplicados; las
    /// aristas que arrastra la cascada del store NO cuentan aquí).
    pub nodos_borrados: usize,
    /// Aristas borradas explícitamente (`DeleteEdge` aplicadas).
    pub aristas_borradas: usize,
}

impl ResumenCommit {
    /// Total de operaciones aplicadas.
    pub fn total_operaciones(&self) -> usize {
        self.nodos_escritos + self.aristas_escritas + self.nodos_borrados + self.aristas_borradas
    }
}

impl fmt::Display for ResumenCommit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "commit: {} operaciones ({} nodos y {} aristas escritos, {} nodos y {} \
             aristas borrados) — en RAM, no durable (cap. 28)",
            self.total_operaciones(),
            self.nodos_escritos,
            self.aristas_escritas,
            self.nodos_borrados,
            self.aristas_borradas
        )
    }
}

/// Resultado de un rollback: el buffer se descarta y el store no se tocó.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResumenRollback {
    /// Operaciones que había acumuladas (y se descartaron).
    pub operaciones_descartadas: usize,
}

impl fmt::Display for ResumenRollback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "rollback: {} operaciones descartadas (el store no se tocó)",
            self.operaciones_descartadas
        )
    }
}

/// Una transacción explícita sobre un `GraphStore`: begin → staging →
/// commit/rollback.
///
/// Las operaciones se acumulan en un buffer privado y el store NO se
/// toca hasta el commit. El commit valida el buffer ENTERO contra el
/// store y, sólo si todo es válido, lo aplica operación a operación:
/// atomicidad naive por acumulación («o todas o ninguna» frente a
/// errores de validación).
///
/// # El ciclo de vida vive en los tipos
///
/// `commit` y `rollback` consumen `self`: no existe el objeto
/// «transacción cerrada» y usarlo tras cerrarla es error de COMPILACIÓN,
/// no un error en runtime. Anidar transacciones sobre el mismo store
/// también lo rechaza el compilador: `begin` pide `&mut dyn GraphStore`
/// y mientras viva la transacción ese préstamo es exclusivo — el modelo
/// «un único escritor» del brief, gratis, ejecutado por el borrow
/// checker.
///
/// # Límites documentados (el gancho a los caps. 28-29)
///
/// * Si el APPLY real falla a mitad, el store queda a medias
///   ([`TransaccionError::ApplyFallido`] lleva la cuenta de lo aplicado;
///   dos tests lo demuestran). Sin log no hay vuelta atrás: eso es el
///   WAL del cap. 28.
/// * Commit en RAM no es durable: ver [`informe_acid()`], letra D.
/// * Abandonar una transacción activa (drop) es un rollback implícito
///   SEGURO por construcción: como nada se aplica fuera del commit, no
///   hay nada que deshacer.
///
/// ```
/// use vol2_liradb::{Edge, GraphStore, MemoryStore, Node, Transaccion};
///
/// let mut store = MemoryStore::new();
/// let mut tx = Transaccion::begin(&mut store);
/// tx.put_node(Node::new(0, "Person")).unwrap();
/// tx.put_node(Node::new(1, "Person")).unwrap();
/// // La arista referencia un nodo creado EN LA MISMA transacción:
/// // válido, porque el buffer se valida como un todo.
/// tx.put_edge(Edge::new(0, 0, 1, "KNOWS")).unwrap();
/// let resumen = tx.commit().unwrap();
///
/// assert_eq!(resumen.total_operaciones(), 3);
/// assert_eq!(store.node_count(), 2);
/// assert_eq!(store.edge_count(), 1);
/// ```
pub struct Transaccion<'a> {
    store: &'a mut dyn GraphStore,
    buffer: Vec<Operacion>,
}

impl<'a> Transaccion<'a> {
    /// Abre una transacción sobre el store (BEGIN).
    ///
    /// El préstamo `&mut` dura lo que la transacción: mientras viva,
    /// nadie más puede leer ni escribir el store.
    pub fn begin(store: &'a mut dyn GraphStore) -> Self {
        Transaccion {
            store,
            buffer: Vec::new(),
        }
    }

    /// Añade una operación al buffer, validándola contra el store y
    /// contra las operaciones anteriores.
    ///
    /// Si no es válida, se rechaza CON el índice que habría ocupado y
    /// la transacción queda exactamente como estaba: el prefijo válido
    /// sobrevive y puede seguir usándose (o hacerse rollback).
    pub fn stage(&mut self, operacion: Operacion) -> Result<(), TransaccionError> {
        self.buffer.push(operacion);
        match validar_buffer(self.store, &self.buffer) {
            Ok(()) => Ok(()),
            Err(e) => {
                // El prefijo era válido (inducción): sólo puede haber
                // fallado la última. Se expulsa y la tx continúa viva.
                self.buffer.pop();
                Err(e)
            }
        }
    }

    /// Insertar un nodo (staging; ver [`Transaccion::stage`]).
    pub fn put_node(&mut self, node: Node) -> Result<(), TransaccionError> {
        self.stage(Operacion::PutNode(node))
    }

    /// Insertar una arista (staging; ver [`Transaccion::stage`]).
    pub fn put_edge(&mut self, edge: Edge) -> Result<(), TransaccionError> {
        self.stage(Operacion::PutEdge(edge))
    }

    /// Eliminar un nodo (staging; ver [`Transaccion::stage`]).
    pub fn delete_node(&mut self, id: NodeId) -> Result<(), TransaccionError> {
        self.stage(Operacion::DeleteNode(id))
    }

    /// Eliminar una arista (staging; ver [`Transaccion::stage`]).
    pub fn delete_edge(&mut self, id: EdgeId) -> Result<(), TransaccionError> {
        self.stage(Operacion::DeleteEdge(id))
    }

    /// Vista de sólo lectura del buffer (para explain/depuración).
    pub fn operaciones(&self) -> &[Operacion] {
        &self.buffer
    }

    /// Operaciones acumuladas.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// ¿No hay operaciones acumuladas? (commit vacío = no-op válido).
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// COMMIT: valida el buffer entero una última vez (el punto de no
    /// retorno) y, si todo es válido, lo aplica al store.
    ///
    /// El resultado resumen cuenta OPERACIONES, no efectos en cascada:
    /// borrar un nodo con aristas es 1 operación aunque el store
    /// elimine sus aristas al aplicarla.
    ///
    /// Lo que NO hace (documentado en [`informe_acid()`]): no escribe
    /// ningún log y no toca disco. Es un commit en RAM.
    pub fn commit(self) -> Result<ResumenCommit, TransaccionError> {
        // Punto de no retorno: re-validación completa. Es redundante por
        // inducción (cada stage validó su prefijo y nada externo puede
        // haber cambiado el store mientras lo teníamos prestado), pero es
        // barata (O(n)) y hace de `commit` la única puerta responsable de
        // la decisión todo-o-nada.
        validar_buffer(self.store, &self.buffer)?;

        let mut resumen = ResumenCommit::default();
        for (indice, op) in self.buffer.iter().enumerate() {
            let ya = resumen.total_operaciones();
            match op {
                Operacion::PutNode(n) => match self.store.put_node(n.clone()) {
                    Ok(()) => resumen.nodos_escritos += 1,
                    Err(causa) => {
                        return Err(TransaccionError::ApplyFallido {
                            indice,
                            aplicadas: ya,
                            causa,
                        });
                    }
                },
                Operacion::PutEdge(e) => match self.store.put_edge(e.clone()) {
                    Ok(()) => resumen.aristas_escritas += 1,
                    Err(causa) => {
                        return Err(TransaccionError::ApplyFallido {
                            indice,
                            aplicadas: ya,
                            causa,
                        });
                    }
                },
                Operacion::DeleteNode(id) => {
                    if self.store.delete_node(*id) {
                        resumen.nodos_borrados += 1;
                    } else {
                        // Imposible si la validación es fiel (vigilado
                        // igualmente): divergencia simulación/store.
                        return Err(TransaccionError::ApplyFallido {
                            indice,
                            aplicadas: ya,
                            causa: StoreError::UnknownNode(*id),
                        });
                    }
                }
                Operacion::DeleteEdge(id) => {
                    if self.store.delete_edge(*id) {
                        resumen.aristas_borradas += 1;
                    } else {
                        return Err(TransaccionError::ApplyFallido {
                            indice,
                            aplicadas: ya,
                            causa: StoreError::UnknownEdge(*id),
                        });
                    }
                }
            }
        }
        Ok(resumen)
    }

    /// ROLLBACK: descarta el buffer. El store no se ha tocado NUNCA
    /// (nada se aplica fuera del commit), así que el descarte es limpio
    /// por construcción.
    ///
    /// Ésa es la lección del staging: deshacer es trivial ANTES de
    /// aplicar. Deshacer DESPUÉS de aplicar (un rollback de verdad, a
    /// mitad de escrituras) exigiría un log — cap. 28.
    pub fn rollback(self) -> ResumenRollback {
        ResumenRollback {
            operaciones_descartadas: self.buffer.len(),
        }
    }
}

/// El modo por defecto de LiraDB hasta este capítulo: cada operación es
/// su PROPIA transacción (begin implícito + commit inmediato).
///
/// Es lo que hace `store.put_node(n)` a pelo — y sigue siendo legítimo
/// para escrituras sueltas. Esta función lo hace visible y ejecutable:
/// equivalente exacto a abrir una transacción, staged una operación y
/// confirmarla.
pub fn autocommit(
    store: &mut dyn GraphStore,
    operacion: Operacion,
) -> Result<ResumenCommit, TransaccionError> {
    let mut tx = Transaccion::begin(store);
    tx.stage(operacion)?;
    tx.commit()
}

// ─────────────────── Validación del buffer (la simulación) ───────────────────
//
// El corazón de la atomicidad naive: REPLAY del buffer sobre una vista
// simulada del store (qué nodos/aristas existirían si se aplicara todo).
// Si alguna operación rompe una invariante, se rechaza ANTES de tocar el
// store de verdad — y como nada se había aplicado, el «rollback» es gratis.
//
// Coste: O(n·(n+E)) para un buffer de n operaciones — cada stage revalida
// el buffer entero. Elección NAIVE y documentada: aquí manda la claridad
// (una única función que valida todo) sobre la eficiencia (un WAL real
// valida incrementalmente).

/// Estado simulado del store a mitad de replay del buffer.
#[derive(Debug, Default)]
struct Simulacion {
    /// Nodos creados por el buffer (no están en el store... todavía).
    nodos_creados: HashSet<NodeId>,
    /// Nodos del store que el buffer ya ha borrado.
    nodos_borrados: HashSet<NodeId>,
    /// Aristas creadas por el buffer, con sus extremos (para cascadas).
    aristas_creadas: HashMap<EdgeId, (NodeId, NodeId)>,
    /// Aristas del store que el buffer ya ha borrado (explícita o por
    /// cascada de un delete_node).
    aristas_borradas: HashSet<EdgeId>,
}

impl Simulacion {
    fn nodo_existe(&self, store: &dyn GraphStore, id: NodeId) -> bool {
        self.nodos_creados.contains(&id)
            || (store.get_node(id).is_some() && !self.nodos_borrados.contains(&id))
    }

    fn arista_existe(&self, store: &dyn GraphStore, id: EdgeId) -> bool {
        self.aristas_creadas.contains_key(&id)
            || (store.get_edge(id).is_some() && !self.aristas_borradas.contains(&id))
    }
}

/// Valida que el buffer completo sea aplicable en orden: cada operación
/// contra el store REAL más el efecto de las anteriores del buffer.
///
/// Reglas (las invariantes del cap. 8, vistas a través del tiempo):
/// * `PutNode`/`PutEdge`: el id debe estar LIBRE (no existe en el store
///   salvo que el buffer lo haya borrado antes, ni lo ha creado el buffer
///   salvo que lo haya borrado después). Los extremos de una arista deben
///   existir — incluidos los creados por operaciones ANTERIORES del
///   buffer (el orden del buffer importa: la arista debe ir después de
///   sus nodos).
/// * `DeleteNode`/`DeleteEdge`: debe existir en la vista simulada. Borrar
///   un nodo arrastra sus aristas (las del store y las del buffer): una
///   operación posterior que toque una arista arrastrada es error.
fn validar_buffer(store: &dyn GraphStore, buffer: &[Operacion]) -> Result<(), TransaccionError> {
    let mut sim = Simulacion::default();

    for (indice, op) in buffer.iter().enumerate() {
        let causa = match op {
            Operacion::PutNode(n) => {
                if sim.nodo_existe(store, n.id) {
                    Some(StoreError::DuplicateNode(n.id))
                } else {
                    sim.nodos_creados.insert(n.id);
                    None
                }
            }
            Operacion::PutEdge(e) => {
                if sim.arista_existe(store, e.id) {
                    Some(StoreError::DuplicateEdge(e.id))
                } else if !sim.nodo_existe(store, e.source) || !sim.nodo_existe(store, e.target) {
                    Some(StoreError::InvalidEdgeEndpoints {
                        source: e.source,
                        target: e.target,
                    })
                } else {
                    sim.aristas_creadas.insert(e.id, (e.source, e.target));
                    None
                }
            }
            Operacion::DeleteNode(id) => {
                if !sim.nodo_existe(store, *id) {
                    Some(StoreError::UnknownNode(*id))
                } else {
                    // Cascada: las aristas del store adyacentes al nodo
                    // desaparecen (out ∪ in; el self-loop sale en ambas,
                    // el set lo deduplica).
                    let mut adyacentes = store.out_edges(*id);
                    adyacentes.extend(store.in_edges(*id));
                    for eid in adyacentes {
                        sim.aristas_creadas.remove(&eid);
                        sim.aristas_borradas.insert(eid);
                    }
                    // Y también las aristas nacidas en el buffer con un
                    // extremo en el nodo (nunca estuvieron en el store,
                    // así que no hace falta marcarlas en borradas).
                    let moribundas: Vec<EdgeId> = sim
                        .aristas_creadas
                        .iter()
                        .filter(|(_, (s, t))| *s == *id || *t == *id)
                        .map(|(&eid, _)| eid)
                        .collect();
                    for eid in moribundas {
                        sim.aristas_creadas.remove(&eid);
                    }
                    sim.nodos_creados.remove(id);
                    sim.nodos_borrados.insert(*id);
                    None
                }
            }
            Operacion::DeleteEdge(id) => {
                if !sim.arista_existe(store, *id) {
                    Some(StoreError::UnknownEdge(*id))
                } else {
                    if sim.aristas_creadas.remove(id).is_none() {
                        sim.aristas_borradas.insert(*id);
                    }
                    None
                }
            }
        };
        if let Some(causa) = causa {
            return Err(TransaccionError::OperacionInvalida { indice, causa });
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests_transacciones {
    use super::*;
    use crate::cap08_graph_store::MemoryStore;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    /// Grafo de 3 nodos y 2 aristas: 0 -> 1 -> 2.
    fn grafo_pequeno() -> MemoryStore {
        let mut s = MemoryStore::new();
        s.put_node(Node::new(0, "Person")).unwrap();
        s.put_node(Node::new(1, "Person")).unwrap();
        s.put_node(Node::new(2, "Person")).unwrap();
        s.put_edge(Edge::new(0, 0, 1, "KNOWS")).unwrap();
        s.put_edge(Edge::new(1, 1, 2, "KNOWS")).unwrap();
        s
    }

    // ── Vocabulario ACID ──────────────────────────────────────────

    #[test]
    fn informe_acid_tiene_las_cuatro_letras_en_orden() {
        let informe = informe_acid();
        assert_eq!(informe.letras(), "ACID");
        assert_eq!(informe.entradas().len(), 4);
        assert_eq!(informe.entradas()[0].garantia, GarantiaAcid::Atomicidad);
        assert_eq!(informe.entradas()[1].garantia, GarantiaAcid::Consistencia);
        assert_eq!(informe.entradas()[2].garantia, GarantiaAcid::Aislamiento);
        assert_eq!(informe.entradas()[3].garantia, GarantiaAcid::Durabilidad);
    }

    #[test]
    fn informe_acid_es_honesto_sobre_el_estado_actual() {
        let informe = informe_acid();
        // A: parcial (staging naive, no sobrevive a un fallo del apply).
        let a = informe.por_garantia(GarantiaAcid::Atomicidad).unwrap();
        assert_eq!(a.nivel, NivelGarantia::Parcial);
        assert!(a.como_esta_hoy.contains("APPLY") || a.como_esta_hoy.contains("apply"));
        // C: parcial y trivial (sólo invariantes estructurales).
        let c = informe.por_garantia(GarantiaAcid::Consistencia).unwrap();
        assert_eq!(c.nivel, NivelGarantia::Parcial);
        assert!(c.como_esta_hoy.contains("estructurales"));
        // I: no hay concurrencia — el borrow checker es el cerrojo.
        let i = informe.por_garantia(GarantiaAcid::Aislamiento).unwrap();
        assert_eq!(i.nivel, NivelGarantia::Parcial);
        assert!(i.como_esta_hoy.contains("concurrencia"));
        // D: NINGUNA — commit en RAM no es durable.
        let d = informe.por_garantia(GarantiaAcid::Durabilidad).unwrap();
        assert_eq!(d.nivel, NivelGarantia::Ninguna);
        assert!(d.como_esta_hoy.contains("RAM"));
        // Ninguna letra está completa: ésa es la tesis del capítulo.
        assert!(
            informe
                .entradas()
                .iter()
                .all(|e| e.nivel != NivelGarantia::Completa)
        );
    }

    #[test]
    fn garantia_acid_letra_nombre_definicion() {
        assert_eq!(GarantiaAcid::Atomicidad.letra(), 'A');
        assert_eq!(GarantiaAcid::Consistencia.letra(), 'C');
        assert_eq!(GarantiaAcid::Aislamiento.letra(), 'I');
        assert_eq!(GarantiaAcid::Durabilidad.letra(), 'D');
        for g in [
            GarantiaAcid::Atomicidad,
            GarantiaAcid::Consistencia,
            GarantiaAcid::Aislamiento,
            GarantiaAcid::Durabilidad,
        ] {
            assert!(!g.nombre().is_empty());
            assert!(g.definicion().len() > 30, "{g} con definición corta");
            assert_eq!(g.to_string(), format!("{} ({})", g.nombre(), g.letra()));
        }
    }

    #[test]
    fn informe_acid_display_muestra_niveles_y_caps() {
        let texto = informe_acid().to_string();
        assert!(texto.contains('A') && texto.contains('D'));
        assert!(texto.contains("ninguna (todavía)"));
        assert!(texto.contains("cap. 28"));
    }

    #[test]
    fn anomalias_de_aislamiento_definidas() {
        assert_eq!(
            Anomalia::LecturaSucia.nombre(),
            "lectura sucia (dirty read)"
        );
        assert_eq!(
            Anomalia::ActualizacionPerdida.nombre(),
            "actualización perdida (lost update)"
        );
        for a in [Anomalia::LecturaSucia, Anomalia::ActualizacionPerdida] {
            assert!(a.definicion().len() > 30);
            // Por qué no pasan hoy: el préstamo exclusivo / un solo hilo.
            let razon = a.por_que_no_pasa_hoy();
            assert!(razon.contains("préstamo") || razon.contains("hilo"));
            assert!(razon.contains("escritor"));
        }
    }

    // ── Commit / rollback básicos ─────────────────────────────────

    #[test]
    fn commit_aplica_todo_el_buffer() {
        let mut store = MemoryStore::new();
        let mut tx = Transaccion::begin(&mut store);
        tx.put_node(Node::new(0, "Person")).unwrap();
        tx.put_node(Node::new(1, "City")).unwrap();
        tx.put_node(Node::new(2, "City")).unwrap();
        tx.put_edge(Edge::new(0, 0, 1, "LIVES_IN")).unwrap();
        tx.put_edge(Edge::new(1, 0, 2, "LIVES_IN")).unwrap();
        assert_eq!(tx.len(), 5);
        let resumen = tx.commit().unwrap();

        assert_eq!(resumen.nodos_escritos, 3);
        assert_eq!(resumen.aristas_escritas, 2);
        assert_eq!(resumen.nodos_borrados, 0);
        assert_eq!(resumen.total_operaciones(), 5);
        assert_eq!(store.node_count(), 3);
        assert_eq!(store.edge_count(), 2);
        assert_eq!(store.out_edges(0), vec![0, 1]);
    }

    #[test]
    fn rollback_no_aplica_nada() {
        let mut store = grafo_pequeno();
        let mut tx = Transaccion::begin(&mut store);
        tx.put_node(Node::new(9, "Intruso")).unwrap();
        tx.delete_node(0).unwrap();
        let resumen = tx.rollback();
        assert_eq!(resumen.operaciones_descartadas, 2);
        // El store quedó EXACTAMENTE como estaba.
        assert_eq!(store.node_count(), 3);
        assert_eq!(store.edge_count(), 2);
        assert!(store.get_node(0).is_some());
    }

    #[test]
    fn drop_implicito_es_rollback_seguro() {
        let mut store = MemoryStore::new();
        {
            let mut tx = Transaccion::begin(&mut store);
            tx.put_node(Node::new(0, "Efímero")).unwrap();
            tx.put_edge(Edge::new(0, 0, 0, "SELF")).unwrap();
            // Sin commit ni rollback: la tx muere con el scope.
        }
        // Nada se aplicó: el staging no toca el store fuera del commit,
        // así que abandonar una tx activa es un rollback implícito SEGURO.
        assert_eq!(store.node_count(), 0);
        assert_eq!(store.edge_count(), 0);
    }

    #[test]
    fn commit_vacio_es_noop_valido() {
        let mut store = grafo_pequeno();
        let tx = Transaccion::begin(&mut store);
        assert!(tx.is_empty());
        let resumen = tx.commit().unwrap();
        assert_eq!(resumen.total_operaciones(), 0);
        assert_eq!(store.node_count(), 3);
        // Y el rollback vacío, simétrico.
        let tx = Transaccion::begin(&mut store);
        let resumen = tx.rollback();
        assert_eq!(resumen.operaciones_descartadas, 0);
    }

    // ── Atomicidad del staging: error en la op 3 de 5 → nada se aplica ──

    #[test]
    fn error_en_la_operacion_3_de_5_no_aplica_nada() {
        // Por la API pública, `stage` ya rechaza la operación inválida al
        // momento (ver `stage_rechaza_edge_a_nodo_inexistente`). Para
        // ejercitar la RE-VALIDACIÓN del commit —el punto de no retorno,
        // segunda cerradura por si un refactor rompe la inducción— este
        // test siembra el buffer a mano (los tests viven dentro del
        // módulo y pueden): 5 operaciones, la 3ª inválida.
        let mut store = MemoryStore::new();
        let mut tx = Transaccion::begin(&mut store);
        tx.buffer = vec![
            Operacion::PutNode(Node::new(0, "Person")),      // 1
            Operacion::PutNode(Node::new(1, "Person")),      // 2
            Operacion::PutEdge(Edge::new(0, 0, 7, "KNOWS")), // 3: nodo 7 NO existe
            Operacion::PutNode(Node::new(7, "Person")),      // 4 (llegó tarde)
            Operacion::PutEdge(Edge::new(1, 7, 0, "KNOWS")), // 5
        ];
        let err = tx.commit().unwrap_err();

        assert_eq!(
            err,
            TransaccionError::OperacionInvalida {
                indice: 2,
                causa: StoreError::InvalidEdgeEndpoints {
                    source: 0,
                    target: 7
                }
            }
        );
        // ATOMICIDAD NAIVE: el grafo no quedó a medias — quedó intacto.
        assert_eq!(store.node_count(), 0);
        assert_eq!(store.edge_count(), 0);
    }

    #[test]
    fn stage_rechaza_duplicado_dentro_del_buffer_y_la_tx_sigue_viva() {
        let mut store = MemoryStore::new();
        let mut tx = Transaccion::begin(&mut store);
        tx.put_node(Node::new(0, "A")).unwrap();
        let err = tx.put_node(Node::new(0, "B")).unwrap_err();
        assert_eq!(
            err,
            TransaccionError::OperacionInvalida {
                indice: 1,
                causa: StoreError::DuplicateNode(0)
            }
        );
        // La operación inválida se expulsa; el prefijo válido sobrevive.
        assert_eq!(tx.len(), 1);
        tx.put_node(Node::new(1, "B")).unwrap();
        let resumen = tx.commit().unwrap();
        assert_eq!(resumen.nodos_escritos, 2);
        assert_eq!(store.node_count(), 2);
    }

    #[test]
    fn stage_rechaza_edge_a_nodo_inexistente() {
        let mut store = MemoryStore::new();
        let mut tx = Transaccion::begin(&mut store);
        let err = tx.put_edge(Edge::new(0, 0, 1, "KNOWS")).unwrap_err();
        assert_eq!(
            err,
            TransaccionError::OperacionInvalida {
                indice: 0,
                causa: StoreError::InvalidEdgeEndpoints {
                    source: 0,
                    target: 1
                }
            }
        );
        assert!(tx.is_empty());
    }

    // ── El orden del buffer importa (la razón de ser del staging) ──

    #[test]
    fn edge_a_nodo_creado_en_la_misma_tx_es_valido() {
        let mut store = MemoryStore::new();
        let mut tx = Transaccion::begin(&mut store);
        tx.put_node(Node::new(0, "Person")).unwrap();
        tx.put_node(Node::new(1, "Person")).unwrap();
        tx.put_edge(Edge::new(0, 0, 1, "KNOWS")).unwrap(); // extremos del MISMO buffer
        tx.commit().unwrap();
        assert_eq!(store.edge_count(), 1);
    }

    #[test]
    fn el_orden_importa_edge_antes_de_sus_nodos_es_invalido() {
        let mut store = MemoryStore::new();
        let mut tx = Transaccion::begin(&mut store);
        // La arista ANTES que sus nodos: los extremos aún no existen en
        // la vista simulada → rechazada en el acto. El orden del buffer
        // es parte del contrato (como el orden de un log).
        let err = tx.put_edge(Edge::new(0, 0, 1, "KNOWS")).unwrap_err();
        assert_eq!(
            err,
            TransaccionError::OperacionInvalida {
                indice: 0,
                causa: StoreError::InvalidEdgeEndpoints {
                    source: 0,
                    target: 1
                }
            }
        );
        assert_eq!(store.node_count(), 0);
    }

    #[test]
    fn delete_de_nodo_creado_en_la_misma_tx() {
        let mut store = MemoryStore::new();
        let mut tx = Transaccion::begin(&mut store);
        tx.put_node(Node::new(0, "Person")).unwrap();
        tx.delete_node(0).unwrap(); // Nació y murió dentro de la tx.
        let resumen = tx.commit().unwrap();
        assert_eq!(resumen.nodos_escritos, 1);
        assert_eq!(resumen.nodos_borrados, 1);
        assert_eq!(store.node_count(), 0);
    }

    #[test]
    fn delete_node_inexistente_rechazado() {
        let mut store = MemoryStore::new();
        let mut tx = Transaccion::begin(&mut store);
        let err = tx.delete_node(42).unwrap_err();
        assert_eq!(
            err,
            TransaccionError::OperacionInvalida {
                indice: 0,
                causa: StoreError::UnknownNode(42)
            }
        );
    }

    #[test]
    fn delete_edge_tras_cascada_de_delete_node_rechazado() {
        let mut store = grafo_pequeno(); // arista 0: 0 -> 1
        let mut tx = Transaccion::begin(&mut store);
        tx.delete_node(0).unwrap(); // Arrastra la arista 0.
        let err = tx.delete_edge(0).unwrap_err(); // Ya no está: murió en la cascada.
        assert_eq!(
            err,
            TransaccionError::OperacionInvalida {
                indice: 1,
                causa: StoreError::UnknownEdge(0)
            }
        );
        // Commit del prefijo válido: sólo el delete del nodo.
        let resumen = tx.commit().unwrap();
        assert_eq!(resumen.nodos_borrados, 1);
        assert_eq!(resumen.aristas_borradas, 0); // La cascada no cuenta: era 1 operación.
        assert_eq!(store.node_count(), 2);
        assert_eq!(store.edge_count(), 1); // La 1 -> 2 sobrevive.
    }

    #[test]
    fn recrear_nodo_tras_borrarlo_en_la_misma_tx() {
        let mut store = grafo_pequeno();
        let mut tx = Transaccion::begin(&mut store);
        tx.delete_node(1).unwrap();
        tx.put_node(Node::new(1, "Renacido")).unwrap(); // El id queda libre.
        tx.commit().unwrap();
        assert_eq!(
            store.get_node(1).unwrap().labels,
            vec!["Renacido".to_string()]
        );
        // El nodo renació, pero sus aristas murieron con el delete:
        // re-crear el nodo NO re-crea la historia.
        assert_eq!(store.edge_count(), 0);
    }

    #[test]
    fn edge_arrastrada_por_cascada_de_nodo_del_buffer() {
        let mut store = MemoryStore::new();
        let mut tx = Transaccion::begin(&mut store);
        tx.put_node(Node::new(0, "A")).unwrap();
        tx.put_node(Node::new(1, "B")).unwrap();
        tx.put_edge(Edge::new(0, 0, 1, "KNOWS")).unwrap();
        tx.delete_node(0).unwrap(); // Arrastra la arista 0 del BUFFER.
        let err = tx.delete_edge(0).unwrap_err();
        assert_eq!(
            err,
            TransaccionError::OperacionInvalida {
                indice: 4,
                causa: StoreError::UnknownEdge(0)
            }
        );
    }

    // ── Un único escritor: el préstamo exclusivo ──────────────────

    #[test]
    fn transacciones_secuenciales_el_prestamo_se_libera() {
        // El modelo del brief («múltiples lectores, un único escritor»)
        // está forzado por el borrow checker: `begin` pide `&mut`, así
        // que dos transacciones SIMULTÁNEAS sobre el mismo store ni
        // compilan (el anidamiento se rechaza en tiempo de compilación,
        // que es mejor que rechazarlo en runtime). Lo que SÍ existe es
        // la secuencia: commit/rollback consumen la tx y liberan el
        // préstamo.
        let mut store = grafo_pequeno();
        {
            let mut tx = Transaccion::begin(&mut store);
            tx.put_node(Node::new(3, "Person")).unwrap();
            tx.commit().unwrap();
        } // El préstamo muere aquí…
        {
            let mut tx = Transaccion::begin(&mut store); // …y se puede volver a tomar.
            tx.delete_node(3).unwrap();
            tx.commit().unwrap();
        }
        assert_eq!(store.node_count(), 3);
        // Y con la tx viva, NI escribir NI leer el store es posible:
        // `store.node_count()` ahí no compilaría. Cerrojo gratis.
    }

    // ── Autocommit ────────────────────────────────────────────────

    #[test]
    fn autocommit_equivalente_a_la_operacion_directa() {
        let mut a = MemoryStore::new();
        let mut b = MemoryStore::new();
        // El modo por defecto de LiraDB hasta este capítulo:
        a.put_node(Node::new(0, "Person")).unwrap();
        // …expresado como transacción de una sola operación:
        let resumen = autocommit(&mut b, Operacion::PutNode(Node::new(0, "Person"))).unwrap();
        assert_eq!(resumen.total_operaciones(), 1);
        assert_eq!(a.node_count(), b.node_count());
        assert_eq!(a.edge_count(), b.edge_count());
    }

    #[test]
    fn autocommit_operacion_invalida_no_toca_el_store() {
        let mut store = grafo_pequeno();
        let err =
            autocommit(&mut store, Operacion::PutNode(Node::new(0, "Duplicado"))).unwrap_err();
        assert_eq!(
            err,
            TransaccionError::OperacionInvalida {
                indice: 0,
                causa: StoreError::DuplicateNode(0)
            }
        );
        assert_eq!(store.node_count(), 3);
    }

    // ── Los límites honestos: el gancho al cap. 28 ────────────────

    /// Store de pruebas que delega en un `MemoryStore` y FALLA en la
    /// N-ésima escritura: la simulación de «algo va mal a mitad del
    /// apply» que el staging NO puede evitar (y que motiva el WAL).
    struct StoreQueFalla {
        inner: MemoryStore,
        escrituras: u64,
        /// Nº de escritura que debe fallar (1-based).
        fallar_en: u64,
        /// Cómo fallar: devolviendo Err (error tipado) o con pánico
        /// (el «corte de luz»).
        con_panic: bool,
    }

    impl StoreQueFalla {
        fn cuenta(&mut self) -> Option<u64> {
            self.escrituras += 1;
            (self.escrituras == self.fallar_en).then_some(self.escrituras)
        }
    }

    impl GraphStore for StoreQueFalla {
        fn put_node(&mut self, node: Node) -> Result<(), StoreError> {
            if let Some(n) = self.cuenta() {
                if self.con_panic {
                    panic!("corte de luz simulado en la escritura #{n}");
                }
                return Err(StoreError::UnknownNode(usize::MAX));
            }
            self.inner.put_node(node)
        }
        fn put_edge(&mut self, edge: Edge) -> Result<(), StoreError> {
            if let Some(n) = self.cuenta() {
                if self.con_panic {
                    panic!("corte de luz simulado en la escritura #{n}");
                }
                return Err(StoreError::UnknownEdge(usize::MAX as EdgeId));
            }
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
            if let Some(n) = self.cuenta() {
                if self.con_panic {
                    panic!("corte de luz simulado en la escritura #{n}");
                }
                return false;
            }
            self.inner.delete_node(id)
        }
        fn delete_edge(&mut self, id: EdgeId) -> bool {
            if let Some(n) = self.cuenta() {
                if self.con_panic {
                    panic!("corte de luz simulado en la escritura #{n}");
                }
                return false;
            }
            self.inner.delete_edge(id)
        }
        fn iter_nodes(&self) -> Box<dyn Iterator<Item = &Node> + '_> {
            self.inner.iter_nodes()
        }
        fn iter_edges(&self) -> Box<dyn Iterator<Item = &Edge> + '_> {
            self.inner.iter_edges()
        }
    }

    #[test]
    fn apply_fallido_deja_el_store_a_medias_gancho_al_cap_28() {
        // La validación pasa (el buffer es impecable)… pero el store
        // dice «no» en la 3ª escritura. ESTO es lo que el staging NO
        // puede arreglar: sin log, las 2 primeras ya están aplicadas.
        let mut store = StoreQueFalla {
            inner: MemoryStore::new(),
            escrituras: 0,
            fallar_en: 3,
            con_panic: false,
        };
        let mut tx = Transaccion::begin(&mut store);
        tx.put_node(Node::new(0, "A")).unwrap();
        tx.put_node(Node::new(1, "B")).unwrap();
        tx.put_node(Node::new(2, "C")).unwrap(); // El store fallará aquí.
        tx.put_node(Node::new(3, "D")).unwrap();
        let err = tx.commit().unwrap_err();

        assert_eq!(
            err,
            TransaccionError::ApplyFallido {
                indice: 2,
                aplicadas: 2,
                causa: StoreError::UnknownNode(usize::MAX)
            }
        );
        // El store QUEDÓ A MEDIOS: atomicidad rota a este nivel.
        // Deshacerlo exigiría un log de lo aplicado → cap. 28 (WAL).
        assert_eq!(store.node_count(), 2);
    }

    #[test]
    fn panic_a_mitad_de_apply_deja_el_store_a_medias() {
        // El «corte de luz»: ni siquiera hay error que devolver — el
        // proceso muere entre dos escrituras. catch_unwind simula el
        // reinicio de la máquina: al volver, el grafo está a medias y
        // NADIE recuerda qué faltaba. Ésa es exactamente la pregunta
        // que el WAL del cap. 28 responde (y el cap. 29, al arrancar).
        let mut store = StoreQueFalla {
            inner: MemoryStore::new(),
            escrituras: 0,
            fallar_en: 2,
            con_panic: true,
        };
        let resultado = catch_unwind(AssertUnwindSafe(|| {
            let mut tx = Transaccion::begin(&mut store);
            tx.put_node(Node::new(0, "A")).unwrap();
            tx.put_node(Node::new(1, "B")).unwrap(); // Pánico AQUÍ.
            tx.put_node(Node::new(2, "C")).unwrap();
            tx.commit()
        }));
        let panico = resultado.unwrap_err();
        let mensaje = panico
            .downcast_ref::<String>()
            .expect("el pánico lleva un mensaje String");
        assert!(mensaje.contains("corte de luz"));
        // La primera escritura SÍ llegó al store; las dos últimas, no.
        assert_eq!(store.node_count(), 1);
        // Sin WAL no hay forma de saber si ese nodo era parte de una
        // transacción confirmada o de una que murió a medias. Cap. 28.
    }

    // ── Errores y resúmenes ───────────────────────────────────────

    #[test]
    fn errores_display_y_std_error() {
        let e1 = TransaccionError::OperacionInvalida {
            indice: 3,
            causa: StoreError::DuplicateEdge(7),
        };
        assert!(e1.to_string().contains("#3"));
        assert!(e1.to_string().contains("nada se ha aplicado"));

        let e2 = TransaccionError::ApplyFallido {
            indice: 1,
            aplicadas: 4,
            causa: StoreError::UnknownNode(9),
        };
        assert!(e2.to_string().contains("#1"));
        assert!(e2.to_string().contains("4 operaciones"));
        assert!(e2.to_string().contains("cap. 28"));

        use std::error::Error;
        assert_eq!(
            e1.source().unwrap().to_string(),
            StoreError::DuplicateEdge(7).to_string()
        );
        assert_eq!(
            e2.source().unwrap().to_string(),
            StoreError::UnknownNode(9).to_string()
        );
    }

    #[test]
    fn resumenes_display() {
        let r = ResumenCommit {
            nodos_escritos: 2,
            aristas_escritas: 1,
            nodos_borrados: 1,
            aristas_borradas: 0,
        };
        let texto = r.to_string();
        assert!(texto.contains("4 operaciones"));
        assert!(texto.contains("no durable"));

        let rr = ResumenRollback {
            operaciones_descartadas: 3,
        };
        assert!(rr.to_string().contains("3 operaciones descartadas"));
        assert!(rr.to_string().contains("no se tocó"));
    }

    #[test]
    fn operacion_display() {
        assert_eq!(Operacion::DeleteNode(4).to_string(), "DELETE nodo 4");
        assert_eq!(
            Operacion::PutEdge(Edge::new(2, 0, 1, "KNOWS")).to_string(),
            "PUT arista 2 (0 -> 1)"
        );
    }

    #[test]
    fn operaciones_vista_del_buffer() {
        let mut store = MemoryStore::new();
        let mut tx = Transaccion::begin(&mut store);
        tx.put_node(Node::new(5, "X")).unwrap();
        tx.delete_edge(3).unwrap_err(); // Inválida: se expulsa.
        tx.put_node(Node::new(6, "Y")).unwrap();
        assert_eq!(tx.len(), 2);
        assert_eq!(tx.operaciones()[0], Operacion::PutNode(Node::new(5, "X")));
        assert_eq!(tx.operaciones()[1], Operacion::PutNode(Node::new(6, "Y")));
    }
}
