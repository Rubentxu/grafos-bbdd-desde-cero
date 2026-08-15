use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::cap07_modelo::{Edge, EdgeId, Node, NodeId, Value};
use crate::cap08_graph_store::{GraphStore, StoreError};
use crate::cap09_encoding::{
    decode_string, decode_u32_le, decode_value, encode_string, encode_u32_le, encode_value,
};
use crate::cap10_append_only::crc32_simple;
use crate::cap27_transacciones::{
    EntradaAcid, GarantiaAcid, NivelGarantia, Operacion, TransaccionError, validar_buffer,
};

// ─────────────────── Cap 28: write-ahead log ───────────────────
//
// Regla central del capítulo (brief §cap 28):
//
// ```text
// El cambio se escribe en el WAL antes que en la página de datos.
// ```
//
// El cap. 27 dejó dos agujeros DEMOSTRADOS en tests: un apply a medias
// dejaba el store inconsistente (ApplyFallido / «corte de luz»), y el
// commit en RAM no era durable. Este capítulo los cierra con la pieza
// que faltaba: un LOG que se escribe ANTES que los datos y que permite
// RE-APLICAR (redo) lo confirmado.
//
// Qué añade el capítulo:
//   1. EL REGISTRO DEL WAL (`WalRecord`): LSN (u64 monótono), id de
//      transacción y cuerpo (`CuerpoWal`: Begin / Operacion / Commit /
//      Rollback — el guion manda begin, commit y rollback como registros).
//      La `Operacion` es la MISMA del cap. 27 (que ya tenía el shape del
//      `RecordKind` del cap. 10: la semilla era deliberada) y se serializa
//      con el encoding del cap. 9 (strings/values) bajo el framing del
//      cap. 10 (length-prefix + CRC32 con `crc32_simple`).
//   2. EL PROTOCOLO WRITE-AHEAD (`WalTransaccion`): stage → commit en
//      dos fases. El COMMIT escribe TODAS las operaciones al log (una a
//      una, con flush según política), escribe el registro Commit, hace
//      sync — EL punto de durabilidad — y SÓLO ENTONCES aplica al store.
//      DECISIÓN (documentada y testeada): commit-marker-ANTES-del-apply.
//      Así un apply a medias (el StoreQueFalla del cap. 27) se completa
//      con replay: roll-forward. La alternativa (marker al final)
//      exigiría UNDO para rescatar el apply a medias — eso es ARIES,
//      cap. 29.
//   3. REDO (`replay_wal`): re-aplica las operaciones de las
//      transacciones CON commit marker, en orden de LSN, con semántica
//      IDEMPOTENTE (re-replay no duplica). Es el germen de la
//      recuperación del cap. 29; aquí se ejecuta a mano (el arranque
//      automático — reopen + replay — es aquel capítulo).
//   4. TRUNCADO (`Wal::truncar_hasta_lsn`): descartar el prefijo del log
//      cuyo efecto YA es durable en el store. El CONTRATO lo firma el
//      llamador: truncar lo no-durable pierde datos (testeado como la
//      deuda documentada que es). El checkpoint que lo automatiza es
//      cap. 29; la ROTACIÓN por tamaño queda como deuda documentada.
//   5. FLUSH Y GROUP COMMIT (`PoliticaFlush`): CadaEscritura (la regla
//      de oro, literal: sync tras cada log_write) vs SoloCommit (un solo
//      sync por transacción — correcto porque las páginas de datos no se
//      llevan a disco antes del commit). El group commit REAL (varias
//      transacciones concurrentes compartiendo un fsync) exige
//      concurrencia: cap. 30. Deuda documentada, semilla plantada.
//
// Qué NO es todavía (honesto, como en el cap. 27):
//   * El «disco» del WAL es un Vec<u8> en RAM: `sync()` es un CONTADOR
//     (la promesa que los tests verifican). El fsync de verdad ya existe
//     (`FilePager::sync`, cap. 12) y el BufferPool ya sabe flush→sync
//     (cap. 13): el protocolo que los conecta es éste; el fichero es
//     cap. 29.
//   * Tras reabrir, los contadores (next_lsn, next_tx_id) se pierden:
//     reabrir = escanear el log. Cap. 29.
//   * No hay UNDO: una transacción SIN commit marker no se deshace del
//     store (no tocó nada: el staging del cap. 27 sigue vivo en
//     `WalTransaccion`); una CON commit marker se completa. El caso
//     intermedio (aplicada y deshecha) es ARIES: cap. 29.

// ─────────────────── Tipos base: LSN y TxId ───────────────────

/// Log Sequence Number: la posición de un registro en el WAL.
///
/// Es un u64 MONÓTODO (asignado por el `Wal` al escribir, nunca por el
/// llamador) y NUNCA se reutiliza — ni después de truncar. Dos registros
/// no pueden compartir LSN: es la dirección física del log y la base del
/// «¿hasta dónde he recuperado?» del cap. 29.
pub type Lsn = u64;

/// Identificador de transacción dentro del WAL.
///
/// Lo asigna el `Wal` en el Begin; agrupa los registros de una misma
/// transacción (Begin, operaciones, Commit/Rollback) aunque estén
/// INTERCALADOS con los de otras en el log.
pub type TxId = u64;

// ─────────────────── El registro del WAL ───────────────────

/// El cuerpo de un registro del WAL: marcadores de ciclo de vida más la
/// operación misma.
///
/// Begin/Commit/Rollback son marcadores (payload vacío); `Operacion`
/// lleva el REDO: la operación completa, lista para re-aplicarse.
#[derive(Debug, Clone, PartialEq)]
pub enum CuerpoWal {
    /// Apertura de transacción (begin).
    Begin,
    /// Operación de escritura — el registro redo.
    Operacion(Operacion),
    /// Confirmación: TODO lo anterior de esta tx_id se vuelve durable.
    Commit,
    /// Abort explícito: el replay debe ignorar esta transacción.
    Rollback,
}

impl fmt::Display for CuerpoWal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CuerpoWal::Begin => write!(f, "BEGIN"),
            CuerpoWal::Operacion(op) => write!(f, "{op}"),
            CuerpoWal::Commit => write!(f, "COMMIT"),
            CuerpoWal::Rollback => write!(f, "ROLLBACK"),
        }
    }
}

/// Un registro del WAL: (LSN, tx, cuerpo).
///
/// El layout en bytes hereda el framing del append-only log del cap. 10
/// (length-prefix + CRC32) y lo extiende con la pareja (lsn, tx_id):
///
/// ```text
/// [record_len: u32 LE] [lsn: u64 LE] [tx_id: u64 LE] [tag: u8]
/// [payload...] [crc32: u32 LE]
/// ```
///
/// El CRC cubre `lsn || tx_id || tag || payload`: cualquier byte tocado
/// por una escritura a medias se detecta al releer.
#[derive(Debug, Clone, PartialEq)]
pub struct WalRecord {
    /// Posición monótona en el log.
    pub lsn: Lsn,
    /// Transacción a la que pertenece.
    pub tx_id: TxId,
    /// Begin / Operacion / Commit / Rollback.
    pub cuerpo: CuerpoWal,
}

impl fmt::Display for WalRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "lsn={} tx={} {}", self.lsn, self.tx_id, self.cuerpo)
    }
}

// ─────────────────── Errores ───────────────────

/// Errores tipados del mundo WAL.
#[derive(Debug, Clone, PartialEq)]
pub enum WalError {
    /// El payload de un registro no decodifica (formato roto).
    Serializacion(String),
    /// El CRC32 almacenado no cuadra con el calculado: el registro (y
    /// todo lo que viene detrás) es inservible. En un log append-only se
    /// LEE HASTA AQUÍ y se descarta la cola: el iterador para limpio,
    /// `decodificar_wal` lo grita.
    CrcInvalido {
        /// El LSN que DECLARA el registro dañado (best-effort: si el
        /// cuerpo está ilegible, `None`).
        lsn: Option<Lsn>,
        /// CRC recalculado sobre el cuerpo.
        esperado: u32,
        /// CRC almacenado en el registro.
        leido: u32,
    },
    /// Faltan bytes: el registro (normalmente el ÚLTIMO) quedó a medias
    /// por un corte de luz durante la escritura del log.
    RegistroTruncado {
        /// Bytes disponibles en el log desde el registro.
        disponibles: usize,
        /// Bytes que el length-prefix reclama.
        necesitados: usize,
    },
    /// Dos registros consecutivos no tienen LSNs consecutivos: hay un
    /// hueco físico en el log (bytes quitados de en medio).
    LsnInvalido {
        /// LSN leído en el registro.
        leido: Lsn,
        /// LSN que la cadena exigía.
        esperado: Lsn,
    },
    /// La re-validación del buffer en `commit` rechazó la transacción
    /// (envuelve el error del cap. 27). Nada se aplicó NI se logueó el
    /// commit: la transacción no ocurrió.
    Validacion(TransaccionError),
    /// El APPLY real falló en la operación `indice` con `aplicadas` ya
    /// escritas. En el cap. 27 esto era un final sin salida («sin log no
    /// hay vuelta atrás»); aquí el log YA CONTIENE el commit: la
    /// transacción ES durable y `replay_wal` la COMPLETA (roll-forward).
    ApplyFallido {
        /// Posición de la operación en el buffer.
        indice: usize,
        /// Operaciones ya aplicadas cuando falló.
        aplicadas: usize,
        /// El error del store.
        causa: StoreError,
    },
    /// El redo de `replay_wal` falló en el registro con ese LSN.
    /// Normalmente significa que se truncó el log rompiendo el contrato
    /// («sólo se trunca lo YA durable en el store») y ahora una arista
    /// referencia nodos que el replay ya no conoce.
    RedoFallido {
        /// LSN del registro cuyo redo falló.
        lsn: Lsn,
        /// El error del store.
        causa: StoreError,
    },
}

impl fmt::Display for WalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WalError::Serializacion(s) => write!(f, "WAL: payload no decodifica: {s}"),
            WalError::CrcInvalido {
                lsn,
                esperado,
                leido,
            } => {
                let donde = match lsn {
                    Some(l) => format!("del registro con lsn {l}"),
                    None => "del primer registro legible".to_string(),
                };
                write!(
                    f,
                    "WAL: CRC inválido {donde}: almacenado {leido:#x}, calculado {esperado:#x} \
                     — el log se lee hasta aquí y la cola se descarta"
                )
            }
            WalError::RegistroTruncado {
                disponibles,
                necesitados,
            } => write!(
                f,
                "WAL: registro truncado (hay {disponibles} bytes, el length-prefix reclama \
                 {necesitados}) — ¿corte de luz a mitad de escritura del log?"
            ),
            WalError::LsnInvalido { leido, esperado } => write!(
                f,
                "WAL: LSN no consecutivo (leído {leido}, esperado {esperado}): el log tiene \
                 un hueco físico"
            ),
            WalError::Validacion(e) => write!(f, "WAL: commit rechazado en validación: {e}"),
            WalError::ApplyFallido {
                indice,
                aplicadas,
                causa,
            } => write!(
                f,
                "WAL: fallo en el APPLY de la operación #{indice} ({causa}) con {aplicadas} \
                 ya aplicadas — pero el log YA contiene el commit: replay_wal COMPLETA la \
                 transacción (arranque automático: cap. 29)"
            ),
            WalError::RedoFallido { lsn, causa } => write!(
                f,
                "WAL: redo falló en lsn {lsn} ({causa}) — ¿se truncó el log rompiendo el \
                 contrato de durabilidad? (sólo se trunca lo YA durable en el store)"
            ),
        }
    }
}

impl std::error::Error for WalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WalError::Validacion(e) => Some(e),
            WalError::ApplyFallido { causa, .. } | WalError::RedoFallido { causa, .. } => {
                Some(causa)
            }
            _ => None,
        }
    }
}

// ─────────────────── Serialización (cap. 9 + framing cap. 10) ───────────────────
//
// Reutilización deliberada, sin duplicar nada:
//   * strings y Values: `encode_string`/`encode_value` del cap. 9;
//   * framing: length-prefix u32 + CRC32 (`crc32_simple`) del cap. 10;
//   * la Operacion es la del cap. 27, tal cual.
// Sólo se añade lo que ningún capítulo tenía: u64 LE (para LSN/tx/ids) y
// el formato de Node/Edge completo (el cap. 9 codificaba Values sueltos).

/// u64 little-endian — el patrón del cap. 9 extendido a 8 bytes.
fn encode_u64_le(value: u64) -> [u8; 8] {
    value.to_le_bytes()
}

/// Contraparte de [`encode_u64_le`].
fn decode_u64_le(bytes: &[u8; 8]) -> u64 {
    u64::from_le_bytes(*bytes)
}

/// Codifica propiedades ORDENADAS por clave.
///
/// La iteración de un `HashMap` no es determinista y un log debe
/// codificar SIEMPRE los mismos bytes para el mismo valor (mismo registro
/// ⇒ mismo CRC): se ordenan las claves antes de serializar.
fn encode_props(props: &HashMap<String, Value>) -> Vec<u8> {
    let mut claves: Vec<&String> = props.keys().collect();
    claves.sort();
    let mut out = Vec::with_capacity(4);
    out.extend_from_slice(&encode_u32_le(claves.len() as u32));
    for clave in claves {
        out.extend_from_slice(&encode_string(clave));
        out.extend_from_slice(&encode_value(&props[clave]));
    }
    out
}

/// Decodifica las propiedades escritas por [`encode_props`].
fn decode_props(mut rest: &[u8]) -> Result<(HashMap<String, Value>, &[u8]), String> {
    if rest.len() < 4 {
        return Err("props: faltan bytes del contador".into());
    }
    let n = decode_u32_le(rest[..4].try_into().unwrap()) as usize;
    rest = &rest[4..];
    let mut props = HashMap::with_capacity(n);
    for _ in 0..n {
        let (clave, r) = decode_string(rest)?;
        rest = r;
        let (valor, r) = decode_value(rest)?;
        rest = r;
        props.insert(clave, valor);
    }
    Ok((props, rest))
}

/// Codifica un nodo: `[id u64][n_labels u32][labels...][props]`.
fn encode_node(n: &Node) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&encode_u64_le(n.id as u64));
    out.extend_from_slice(&encode_u32_le(n.labels.len() as u32));
    for label in &n.labels {
        out.extend_from_slice(&encode_string(label));
    }
    out.extend_from_slice(&encode_props(&n.props));
    out
}

/// Decodifica un nodo escrito por [`encode_node`].
fn decode_node(mut rest: &[u8]) -> Result<(Node, &[u8]), String> {
    if rest.len() < 12 {
        return Err("nodo: faltan id y contador de labels".into());
    }
    let id = decode_u64_le(rest[..8].try_into().unwrap());
    rest = &rest[8..];
    let n_labels = decode_u32_le(rest[..4].try_into().unwrap()) as usize;
    rest = &rest[4..];
    let mut labels = Vec::with_capacity(n_labels);
    for _ in 0..n_labels {
        let (label, r) = decode_string(rest)?;
        rest = r;
        labels.push(label);
    }
    let (props, rest) = decode_props(rest)?;
    let node = Node {
        id: id as NodeId,
        labels,
        props,
    };
    Ok((node, rest))
}

/// Codifica una arista: `[id u64][source u64][target u64][label][props]`.
fn encode_edge(e: &Edge) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&encode_u64_le(e.id as u64));
    out.extend_from_slice(&encode_u64_le(e.source as u64));
    out.extend_from_slice(&encode_u64_le(e.target as u64));
    out.extend_from_slice(&encode_string(&e.label));
    out.extend_from_slice(&encode_props(&e.props));
    out
}

/// Decodifica una arista escrita por [`encode_edge`].
fn decode_edge(mut rest: &[u8]) -> Result<(Edge, &[u8]), String> {
    if rest.len() < 28 {
        return Err("arista: faltan ids y label".into());
    }
    let id = decode_u64_le(rest[..8].try_into().unwrap());
    let source = decode_u64_le(rest[8..16].try_into().unwrap());
    let target = decode_u64_le(rest[16..24].try_into().unwrap());
    rest = &rest[24..];
    let (label, rest) = decode_string(rest)?;
    let (props, rest) = decode_props(rest)?;
    let edge = Edge {
        id: id as EdgeId,
        source: source as NodeId,
        target: target as NodeId,
        label,
        props,
    };
    Ok((edge, rest))
}

/// Codifica una `Operacion` (cap. 27) a bytes.
///
/// Los tags 1-4 replican el orden del `RecordKind` del cap. 10 — la
/// semilla era deliberada y aquí florece.
fn encode_operacion(op: &Operacion) -> Vec<u8> {
    let mut out = Vec::new();
    match op {
        Operacion::PutNode(n) => {
            out.push(1);
            out.extend_from_slice(&encode_node(n));
        }
        Operacion::PutEdge(e) => {
            out.push(2);
            out.extend_from_slice(&encode_edge(e));
        }
        Operacion::DeleteNode(id) => {
            out.push(3);
            out.extend_from_slice(&encode_u64_le(*id as u64));
        }
        Operacion::DeleteEdge(id) => {
            out.push(4);
            out.extend_from_slice(&encode_u64_le(*id as u64));
        }
    }
    out
}

/// Decodifica una `Operacion` escrita por [`encode_operacion`].
fn decode_operacion(bytes: &[u8]) -> Result<(Operacion, &[u8]), String> {
    let Some((&tag, rest)) = bytes.split_first() else {
        return Err("operación: vacía".into());
    };
    match tag {
        1 => {
            let (nodo, rest) = decode_node(rest)?;
            Ok((Operacion::PutNode(nodo), rest))
        }
        2 => {
            let (arista, rest) = decode_edge(rest)?;
            Ok((Operacion::PutEdge(arista), rest))
        }
        3 => {
            if rest.len() < 8 {
                return Err("delete_node: faltan bytes del id".into());
            }
            let id = decode_u64_le(rest[..8].try_into().unwrap());
            Ok((Operacion::DeleteNode(id as NodeId), &rest[8..]))
        }
        4 => {
            if rest.len() < 8 {
                return Err("delete_edge: faltan bytes del id".into());
            }
            let id = decode_u64_le(rest[..8].try_into().unwrap());
            Ok((Operacion::DeleteEdge(id as EdgeId), &rest[8..]))
        }
        other => Err(format!("operación: tag desconocido {other}")),
    }
}

/// Codifica un `WalRecord` a bytes con el framing del cap. 10:
/// length-prefix u32 + cuerpo + CRC32.
pub fn encode_wal_record(rec: &WalRecord) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&encode_u64_le(rec.lsn));
    body.extend_from_slice(&encode_u64_le(rec.tx_id));
    match &rec.cuerpo {
        CuerpoWal::Begin => body.push(1),
        CuerpoWal::Operacion(op) => {
            body.push(2);
            body.extend_from_slice(&encode_operacion(op));
        }
        CuerpoWal::Commit => body.push(3),
        CuerpoWal::Rollback => body.push(4),
    }

    let crc = crc32_simple(&body);
    let mut inner = body;
    inner.extend_from_slice(&encode_u32_le(crc));

    // Length prefix: longitud de todo menos el propio prefix.
    let mut out = Vec::with_capacity(4 + inner.len());
    out.extend_from_slice(&encode_u32_le(inner.len() as u32));
    out.extend_from_slice(&inner);
    out
}

/// Decodifica UN registro desde bytes (verifica framing y CRC32).
///
/// Devuelve el registro y el resto de bytes. El orden de comprobaciones
/// es el del cap. 10: length → CRC → parse. El CRC se valida ANTES de
/// interpretar el contenido: un cuerpo corrupto nunca se parsea.
pub fn decode_wal_record(bytes: &[u8]) -> Result<(WalRecord, &[u8]), WalError> {
    // Mínimo: len(4) + lsn(8) + tx(8) + tag(1) + crc(4) = 25.
    const MINIMO: usize = 25;
    if bytes.len() < MINIMO {
        return Err(WalError::RegistroTruncado {
            disponibles: bytes.len(),
            necesitados: MINIMO,
        });
    }
    let inner_len = decode_u32_le(bytes[..4].try_into().unwrap()) as usize;
    if bytes.len() < 4 + inner_len {
        return Err(WalError::RegistroTruncado {
            disponibles: bytes.len(),
            necesitados: 4 + inner_len,
        });
    }
    let inner = &bytes[4..4 + inner_len];
    let body = &inner[..inner_len - 4];
    let crc_leido = decode_u32_le(inner[inner_len - 4..].try_into().unwrap());
    let crc_calculado = crc32_simple(body);
    // LSN «aparente» best-effort para el diagnóstico (si el cuerpo está
    // intacto salvo el CRC, señala el registro dañado).
    let lsn_aparente = (body.len() >= 8).then(|| decode_u64_le(body[..8].try_into().unwrap()));
    if crc_leido != crc_calculado {
        return Err(WalError::CrcInvalido {
            lsn: lsn_aparente,
            esperado: crc_calculado,
            leido: crc_leido,
        });
    }
    if body.len() < 17 {
        return Err(WalError::Serializacion(
            "cuerpo más corto que lsn+tx+tag".into(),
        ));
    }
    let lsn = decode_u64_le(body[..8].try_into().unwrap());
    let tx_id = decode_u64_le(body[8..16].try_into().unwrap());
    let cuerpo = match body[16] {
        1 => CuerpoWal::Begin,
        2 => {
            let (op, sobra) = decode_operacion(&body[17..]).map_err(WalError::Serializacion)?;
            if !sobra.is_empty() {
                return Err(WalError::Serializacion(format!(
                    "bytes sobrantes en la operación de lsn {lsn}"
                )));
            }
            CuerpoWal::Operacion(op)
        }
        3 => CuerpoWal::Commit,
        4 => CuerpoWal::Rollback,
        other => {
            return Err(WalError::Serializacion(format!(
                "tag de cuerpo desconocido: {other}"
            )));
        }
    };
    Ok((WalRecord { lsn, tx_id, cuerpo }, &bytes[4 + inner_len..]))
}

/// Decodifica un log ENTERO en modo estricto: cualquier registro
/// truncado, corrupto o con LSN no consecutivo es Err.
///
/// Es el «voltímetro» de los tests de corrupción; el iterador del `Wal`
/// (que PARA LIMPIO en el primer problema) es el modo recuperación.
pub fn decodificar_wal(bytes: &[u8]) -> Result<Vec<WalRecord>, WalError> {
    let mut out = Vec::new();
    let mut rest = bytes;
    let mut anterior: Option<Lsn> = None;
    while !rest.is_empty() {
        let (rec, cola) = decode_wal_record(rest)?;
        if let Some(previo) = anterior
            && rec.lsn != previo + 1
        {
            return Err(WalError::LsnInvalido {
                leido: rec.lsn,
                esperado: previo + 1,
            });
        }
        anterior = Some(rec.lsn);
        out.push(rec);
        rest = cola;
    }
    Ok(out)
}

// ─────────────────── El WAL ───────────────────

/// Cuándo se lleva el log a almacenamiento estable (flush/fsync).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoliticaFlush {
    /// Sync tras CADA `log_write`: la regla de oro del capítulo, literal
    /// — cada registro es durable antes de tocar el dato. Es la política
    /// por defecto (aquí se enseña la regla antes que la optimización).
    CadaEscritura,
    /// Sync SÓLO al escribir el registro Commit: correcto porque las
    /// páginas de DATOS no se llevan a disco antes del commit (lo que el
    /// write-ahead prohíbe), y es la semilla del group commit — la
    /// variante con varias transacciones concurrentes compartiendo un
    /// fsync exige concurrencia (cap. 30).
    SoloCommit,
}

impl fmt::Display for PoliticaFlush {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PoliticaFlush::CadaEscritura => {
                write!(f, "flush por escritura (la regla de oro, literal)")
            }
            PoliticaFlush::SoloCommit => write!(
                f,
                "flush sólo en commit (un sync por transacción; group commit real: cap. 30)"
            ),
        }
    }
}

/// El write-ahead log: el «disco» del capítulo.
///
/// En memoria (como el `AppendOnlyLog` del cap. 10, su germen directo):
/// un `Vec<u8>` de registros encadenados por length-prefix. Un WAL de
/// fichero real sería un `File` con `O_APPEND` y `sync_all` — el mismo
/// `FilePager::sync` del cap. 12; el PROTOCOLO (qué se escribe, cuándo,
/// y quién se lee al despertar) es lo que este capítulo construye.
///
/// Invariantes:
/// * Los LSN son monótonos y consecutivos desde el 1, y NUNCA se
///   reutilizan (ni tras `truncar_hasta_lsn`).
/// * Los `TxId` también son monótonos desde el 1.
/// * `sync()` cuenta las «fsync»: en RAM no hay nada que sincronizar,
///   pero la PROMESA es verificable y los tests la cuentan.
#[derive(Debug)]
pub struct Wal {
    /// Bytes del log (concatenación de registros encodificados).
    bytes: Vec<u8>,
    /// Próximo LSN a asignar (el último asignado + 1).
    next_lsn: Lsn,
    /// Próximo TxId a asignar.
    next_tx_id: TxId,
    /// Cuántas veces se ha llevado el log a almacenamiento estable.
    syncs: u64,
    /// Cuándo se sincroniza.
    politica: PoliticaFlush,
}

impl Default for Wal {
    /// WAL vacío con la política por defecto: CadaEscritura (se enseña
    /// la regla antes que la optimización).
    fn default() -> Self {
        Wal {
            bytes: Vec::new(),
            next_lsn: 1,
            next_tx_id: 1,
            syncs: 0,
            politica: PoliticaFlush::CadaEscritura,
        }
    }
}

impl Wal {
    /// WAL vacío con la política por defecto (CadaEscritura).
    pub fn new() -> Self {
        Self::default()
    }

    /// WAL vacío con una política de flush explícita.
    pub fn con_politica(politica: PoliticaFlush) -> Self {
        Wal {
            politica,
            ..Wal::default()
        }
    }

    /// REABRE un WAL a partir de sus bytes persistidos (el «escanear el
    /// log al despertar» del cap. 29).
    ///
    /// Recorre los registros LEGIBLES (parada limpia ante corrupción, igual
    /// que [`Wal::iter`]) y reconstruye los contadores: `next_lsn` y
    /// `next_tx_id` vuelven a ser `máximo + 1` para que los LSN/TxId no se
    /// reutilicen jamás. La política de flush es una elección de RUNTIME
    /// (no se persiste): se reabre con la regla de oro [`PoliticaFlush::CadaEscritura`].
    ///
    /// ADVERTENCIA honesta (la misma que [`Wal::truncar_hasta_lsn`]): si el
    /// log fue truncado sin guardar el contador (p.ej. a vacío), `next_tx_id`
    /// se reanuda en 1 y un TxId YA usado podría reutilizarse. El checkpoint
    /// del cap. 29 persiste esos contadores precisamente para cerrar ese
    /// hueco: reabrir tras un checkpoint arranca de los valores guardados.
    pub fn reconstruir(bytes: &[u8]) -> Self {
        let mut next_lsn = 1u64;
        let mut next_tx_id = 1u64;
        for rec in (WalIterator {
            bytes,
            anterior: None,
        }) {
            next_lsn = next_lsn.max(rec.lsn + 1);
            next_tx_id = next_tx_id.max(rec.tx_id + 1);
        }
        Wal {
            bytes: bytes.to_vec(),
            next_lsn,
            next_tx_id,
            syncs: 0,
            politica: PoliticaFlush::CadaEscritura,
        }
    }

    /// La política de flush activa.
    pub fn politica(&self) -> PoliticaFlush {
        self.politica
    }

    /// Cuántos syncs (fsync) se han hecho: la parte medible de la
    /// durabilidad.
    pub fn syncs(&self) -> u64 {
        self.syncs
    }

    /// El PRÓXIMO LSN que se asignará (el log tiene `lsn_siguiente() - 1`
    /// registros con LSN… salvo truncados, que ya no están pero SÍ
    /// cuentan: los LSN no se reutilizan).
    pub fn lsn_siguiente(&self) -> Lsn {
        self.next_lsn
    }

    /// El PRÓXIMO TxId que se asignará.
    ///
    /// Igual que el LSN, es un contador monótono que NUNCA se reutiliza
    /// (ni tras truncar). El checkpoint del cap. 29 lo persiste para poder
    /// reanudar el contador tras un reinicio sin reutilizar identificadores.
    pub fn next_tx_id(&self) -> TxId {
        self.next_tx_id
    }

    /// Bytes crudos del log (para inspección y tests de corrupción).
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Registros LEGIBLES (el iterador para limpio en la cola dañada).
    pub fn record_count(&self) -> usize {
        self.iter().count()
    }

    /// Abre una transacción: asigna TxId y escribe el registro Begin.
    ///
    /// El Begin no se sincroniza: un begin sin commit no significa nada
    /// tras un corte de luz (la ausencia de Commit YA es la respuesta).
    pub fn begin_tx(&mut self) -> (TxId, Lsn) {
        let tx_id = self.next_tx_id;
        self.next_tx_id += 1;
        let lsn = self.append(tx_id, CuerpoWal::Begin);
        (tx_id, lsn)
    }

    /// Escribe un registro y devuelve su LSN recién asignado.
    ///
    /// Éste es el corazón del write-AHEAD: se llama ANTES de aplicar el
    /// cambio en el store. No sincroniza por sí solo: la política decide
    /// (ver [`PoliticaFlush`]).
    pub fn log_write(&mut self, tx_id: TxId, cuerpo: CuerpoWal) -> Lsn {
        self.append(tx_id, cuerpo)
    }

    /// Lleva el log a almacenamiento estable (fsync).
    ///
    /// En este WAL en memoria es la PROMESA hecha contador: no hay disco
    /// bajo el `Vec<u8>`. En un WAL de fichero sería `sync_all` — el
    /// `FilePager::sync` del cap. 12. Lo que los tests verifican es que
    /// se LLAME cuando el protocolo lo exige (el commit, sobre todo).
    pub fn sync(&mut self) {
        self.syncs += 1;
    }

    /// Append interno: asigna LSN y encadena los bytes.
    fn append(&mut self, tx_id: TxId, cuerpo: CuerpoWal) -> Lsn {
        let lsn = self.next_lsn;
        self.next_lsn += 1;
        let rec = WalRecord { lsn, tx_id, cuerpo };
        self.bytes.extend_from_slice(&encode_wal_record(&rec));
        lsn
    }

    /// Itera los registros legibles.
    ///
    /// PARADA LIMPIA: ante el primer registro truncado, corrupto (CRC) o
    /// con LSN no consecutivo, el iterador termina — ésa es la semántica
    /// de recuperación de un log append-only: se confía en el prefijo
    /// íntegro y se descarta la cola. Para gritar el error en vez de
    /// callarlo, usar [`decodificar_wal`].
    pub fn iter(&self) -> WalIterator<'_> {
        WalIterator {
            bytes: &self.bytes,
            anterior: None,
        }
    }

    /// Trunca el log a `len` bytes (simulador de cortes de luz en tests;
    /// herencia directa del `truncate_to` del cap. 10).
    pub fn truncar_a_bytes(&mut self, len: usize) {
        self.bytes.truncate(len);
    }

    /// Descarta del log todos los registros con LSN ≤ `lsn` y devuelve
    /// cuántos eliminó.
    ///
    /// CONTRATO (lo firma el llamador): los cambios de esos registros ya
    /// son DURABLES en el store. Truncar lo no-durable PIERDE datos: el
    /// replay sólo ve lo que queda. El checkpoint que decide «hasta dónde
    /// es seguro» de forma automática es el cap. 29; la rotación por
    /// tamaño del fichero queda como deuda documentada.
    ///
    /// Lo que NUNCA se reinicia: `next_lsn`/`next_tx_id` — los
    /// identificadores no se reutilizan ni después de truncar.
    pub fn truncar_hasta_lsn(&mut self, lsn: Lsn) -> usize {
        let mut pos = 0usize;
        let mut eliminados = 0usize;
        let mut corte: Option<usize> = None;
        while pos < self.bytes.len() {
            match decode_wal_record(&self.bytes[pos..]) {
                Ok((rec, cola)) => {
                    let tamano = self.bytes.len() - cola.len() - pos;
                    if rec.lsn <= lsn {
                        eliminados += 1;
                        pos += tamano;
                    } else {
                        corte = Some(pos);
                        break;
                    }
                }
                // Cola ilegible: no sabemos su LSN, así que se CONSERVA.
                Err(_) => break,
            }
        }
        let corte = corte.unwrap_or(pos);
        self.bytes.drain(..corte);
        eliminados
    }
}

/// Iterador de registros legibles de un [`Wal`] (con parada limpia).
pub struct WalIterator<'a> {
    bytes: &'a [u8],
    anterior: Option<Lsn>,
}

impl<'a> Iterator for WalIterator<'a> {
    type Item = WalRecord;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bytes.is_empty() {
            return None;
        }
        match decode_wal_record(self.bytes) {
            Ok((rec, cola)) => {
                // Monotonía verificada también al iterar: un hueco físico
                // (bytes quitados de en medio) corta la lectura igual que
                // un CRC roto.
                if let Some(previo) = self.anterior
                    && rec.lsn != previo + 1
                {
                    return None;
                }
                self.anterior = Some(rec.lsn);
                self.bytes = cola;
                Some(rec)
            }
            Err(_) => None,
        }
    }
}

// ─────────────────── Redo: replay idempotente ───────────────────

/// Aplica una operación en modo REDO: semántica idempotente.
///
/// La diferencia con el apply del commit (estricto) es la tolerancia:
/// re-aplicar lo ya aplicado debe ser un no-op con error NINGUNO, porque
/// el replay no sabe qué sobrevivió al fallo en el store. La
/// idempotencia es EL contrato de un registro redo:
/// * Put que ya está (idéntico) → no-op; put distinto → se sobreescribe
///   (el log es la verdad);
/// * Delete de lo ya borrado → no-op silencioso.
pub(crate) fn aplicar_para_redo(
    store: &mut dyn GraphStore,
    op: &Operacion,
) -> Result<(), StoreError> {
    match op {
        Operacion::PutNode(n) => match store.get_node(n.id) {
            Some(actual) if actual == n => Ok(()),
            Some(_) => {
                // El log manda: sobreescribir el valor divergente.
                store.delete_node(n.id);
                store.put_node(n.clone())
            }
            None => store.put_node(n.clone()),
        },
        Operacion::PutEdge(e) => match store.get_edge(e.id) {
            Some(actual) if actual == e => Ok(()),
            Some(_) => {
                store.delete_edge(e.id);
                store.put_edge(e.clone())
            }
            None => store.put_edge(e.clone()),
        },
        Operacion::DeleteNode(id) => {
            let _ = store.delete_node(*id);
            Ok(())
        }
        Operacion::DeleteEdge(id) => {
            let _ = store.delete_edge(*id);
            Ok(())
        }
    }
}

/// El resultado medible de un replay.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InformeReplay {
    /// Transacciones con registro Commit en el log legible.
    pub transacciones_confirmadas: usize,
    /// Transacciones con Begin (o marker Rollback) SIN commit: el replay
    /// las ignora — nunca ocurrieron.
    pub transacciones_descartadas: usize,
    /// Operaciones redo procesadas (de transacciones confirmadas).
    pub operaciones_reaplicadas: usize,
}

impl fmt::Display for InformeReplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "replay: {} transacciones confirmadas ({} operaciones redo), {} descartadas \
             sin commit",
            self.transacciones_confirmadas,
            self.operaciones_reaplicadas,
            self.transacciones_descartadas
        )
    }
}

/// REDO: re-aplica al store las operaciones de las transacciones
/// CONFIRMADAS (con registro Commit), en orden de LSN, con semántica
/// idempotente.
///
/// Es el germen de la recuperación del cap. 29 — aquí se invoca A MANO
/// (la llamada en el arranque, tras reabrir disco y log, es aquel
/// capítulo). Qué hace con cada caso:
/// * Transacción con Commit → sus operaciones se re-aplican todas (las
///   ya presentes son no-ops): el apply a medias del cap. 27 se
///   COMPLETA. Ésta es la atomicidad que faltaba.
/// * Transacción sin Commit (abortada, abandonada o con el commit
///   truncado por el corte de luz) → se ignora por completo: nunca
///   ocurrió. Ojo: si su staging ya se había aplicado a medias… no puede
///   haberlo hecho — `WalTransaccion` aplica SÓLO tras escribir el
///   Commit (y si el apply a medias era de una transacción confirmada,
///   el replay la completa).
/// * Cola ilegible (CRC/truncado) → parada limpia del iterador: se
///   confía en el prefijo íntegro.
pub fn replay_wal(store: &mut dyn GraphStore, wal: &Wal) -> Result<InformeReplay, WalError> {
    // Pasada 1: quién llegó a Commit (y quién empezó sin llegar).
    let mut confirmadas: HashSet<TxId> = HashSet::new();
    let mut iniciadas: HashSet<TxId> = HashSet::new();
    for rec in wal.iter() {
        match rec.cuerpo {
            CuerpoWal::Begin => {
                iniciadas.insert(rec.tx_id);
            }
            CuerpoWal::Commit => {
                confirmadas.insert(rec.tx_id);
            }
            _ => {}
        }
    }

    // Pasada 2: redo de lo confirmado, en orden de LSN (= orden del log).
    let mut operaciones = 0usize;
    for rec in wal.iter() {
        if let CuerpoWal::Operacion(op) = rec.cuerpo
            && confirmadas.contains(&rec.tx_id)
        {
            aplicar_para_redo(store, &op).map_err(|causa| WalError::RedoFallido {
                lsn: rec.lsn,
                causa,
            })?;
            operaciones += 1;
        }
    }

    let descartadas = iniciadas.difference(&confirmadas).count();
    Ok(InformeReplay {
        transacciones_confirmadas: confirmadas.len(),
        transacciones_descartadas: descartadas,
        operaciones_reaplicadas: operaciones,
    })
}

// ─────────────────── La transacción con WAL ───────────────────

/// Resultado de un commit con WAL: lo que el cap. 27 ya contaba, más la
/// PRUEBA de la durabilidad (el LSN del registro Commit).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResumenCommitWal {
    /// Nodos escritos.
    pub nodos_escritos: usize,
    /// Aristas escritas.
    pub aristas_escritas: usize,
    /// Nodos borrados explícitamente (las cascadas no cuentan).
    pub nodos_borrados: usize,
    /// Aristas borradas explícitamente.
    pub aristas_borradas: usize,
    /// LSN del registro Commit: a partir de aquí la transacción es
    /// durable en el log.
    pub lsn_commit: Lsn,
}

impl ResumenCommitWal {
    /// Total de operaciones.
    pub fn total_operaciones(&self) -> usize {
        self.nodos_escritos + self.aristas_escritas + self.nodos_borrados + self.aristas_borradas
    }
}

impl fmt::Display for ResumenCommitWal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "commit wal: {} operaciones ({} nodos y {} aristas escritos, {} nodos y {} \
             aristas borrados) — DURABLE: commit record en lsn {} + sync",
            self.total_operaciones(),
            self.nodos_escritos,
            self.aristas_escritas,
            self.nodos_borrados,
            self.aristas_borradas,
            self.lsn_commit
        )
    }
}

/// Resultado de un rollback: el buffer se descarta, el store no se tocó
/// y el marker Rollback queda en el log para que el replay sepa que esta
/// transacción murió deliberadamente.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResumenRollbackWal {
    /// Operaciones descartadas del buffer.
    pub operaciones_descartadas: usize,
    /// LSN del registro Rollback.
    pub lsn_rollback: Lsn,
}

impl fmt::Display for ResumenRollbackWal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "rollback wal: {} operaciones descartadas (marker en lsn {}; el store no se \
             tocó)",
            self.operaciones_descartadas, self.lsn_rollback
        )
    }
}

/// Una transacción con protocolo write-ahead: begin → staging → commit
/// en dos fases (LOG primero, APPLY después) / rollback.
///
/// Hereda el staging del cap. 27 (mismo buffer, misma validación eager,
/// mismo «un único escritor» por préstamo exclusivo — ahora sobre DOS
/// objetos: store y WAL) y le añade la fase que lo cambia todo:
///
/// ```text
/// COMMIT:
///   1. re-validar el buffer (punto de no retorno)
///   2. por cada op:   log_write(op)  [sync según política]
///   3.                log_write(Commit) + sync   ← DURABILIDAD
///   4. por cada op:   apply(store)               ← puede fallar…
/// ```
///
/// Si el paso 4 falla a mitad (el `StoreQueFalla` del cap. 27, o el
/// «corte de luz»), el paso 3 YA terminó: el log contiene la
/// transacción COMPLETA y su commit. `replay_wal` la completa
/// (roll-forward). Ésa es la decisión de diseño del capítulo:
/// commit-marker-ANTES-del-apply; la alternativa (marker al final)
/// dejaría el apply a medias SIN commit, y rescatarlo exigiría UNDO —
/// ARIES, cap. 29.
///
/// ```
/// use vol2_liradb::{Edge, GraphStore, MemoryStore, Node, Wal, WalTransaccion, replay_wal};
///
/// let mut store = MemoryStore::new();
/// let mut wal = Wal::new();
/// {
///     let mut tx = WalTransaccion::begin(&mut store, &mut wal);
///     tx.put_node(Node::new(0, "Person")).unwrap();
///     tx.put_node(Node::new(1, "Person")).unwrap();
///     tx.put_edge(Edge::new(0, 0, 1, "KNOWS")).unwrap();
///     let resumen = tx.commit().unwrap();
///     assert_eq!(resumen.total_operaciones(), 3);
/// }
///
/// // El «reinicio»: store vacío + replay del WAL = lo confirmado vuelve.
/// let mut renacido = MemoryStore::new();
/// let informe = replay_wal(&mut renacido, &wal).unwrap();
/// assert_eq!(informe.operaciones_reaplicadas, 3);
/// assert_eq!(renacido.node_count(), 2);
/// assert_eq!(renacido.edge_count(), 1);
/// ```
pub struct WalTransaccion<'a> {
    store: &'a mut dyn GraphStore,
    wal: &'a mut Wal,
    tx_id: TxId,
    buffer: Vec<Operacion>,
}

impl<'a> WalTransaccion<'a> {
    /// BEGIN: abre la transacción y escribe el registro Begin al WAL.
    ///
    /// El Begin no se sincroniza: si el corte de luz llega antes del
    /// commit, la ausencia de Commit ya cuenta la historia.
    pub fn begin(store: &'a mut dyn GraphStore, wal: &'a mut Wal) -> Self {
        let (tx_id, _lsn) = wal.begin_tx();
        WalTransaccion {
            store,
            wal,
            tx_id,
            buffer: Vec::new(),
        }
    }

    /// El TxId de esta transacción (para inspección del log).
    pub fn tx_id(&self) -> TxId {
        self.tx_id
    }

    /// Añade una operación al buffer validándola eager (cap. 27 tal
    /// cual): si no es válida, se expulsa y la transacción sigue viva.
    pub fn stage(&mut self, operacion: Operacion) -> Result<(), TransaccionError> {
        self.buffer.push(operacion);
        match validar_buffer(self.store, &self.buffer) {
            Ok(()) => Ok(()),
            Err(e) => {
                self.buffer.pop();
                Err(e)
            }
        }
    }

    /// Insertar un nodo (staging).
    pub fn put_node(&mut self, node: Node) -> Result<(), TransaccionError> {
        self.stage(Operacion::PutNode(node))
    }

    /// Insertar una arista (staging).
    pub fn put_edge(&mut self, edge: Edge) -> Result<(), TransaccionError> {
        self.stage(Operacion::PutEdge(edge))
    }

    /// Eliminar un nodo (staging).
    pub fn delete_node(&mut self, id: NodeId) -> Result<(), TransaccionError> {
        self.stage(Operacion::DeleteNode(id))
    }

    /// Eliminar una arista (staging).
    pub fn delete_edge(&mut self, id: EdgeId) -> Result<(), TransaccionError> {
        self.stage(Operacion::DeleteEdge(id))
    }

    /// Vista del buffer (para explain/depuración).
    pub fn operaciones(&self) -> &[Operacion] {
        &self.buffer
    }

    /// Operaciones acumuladas.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// ¿Buffer vacío? (commit vacío = sólo el Begin y el Commit).
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// COMMIT en dos fases: LOG (write-ahead + commit record + sync) y
    /// después APPLY.
    ///
    /// El orden es la tesis del capítulo: cuando el apply empieza, TODO
    /// el intento ya es durable. Un fallo del apply devuelve
    /// [`WalError::ApplyFallido`] — y la RESOLUCIÓN ya no es «el store
    /// quedó a medias y no hay vuelta atrás» (cap. 27) sino «ejecuta
    /// `replay_wal` y la transacción se completa».
    pub fn commit(self) -> Result<ResumenCommitWal, WalError> {
        // Punto de no retorno (1/2): re-validación del buffer entero.
        validar_buffer(self.store, &self.buffer).map_err(WalError::Validacion)?;

        // FASE LOG — el write-AHEAD: cada operación al log ANTES que al
        // store, con flush según la política.
        for op in &self.buffer {
            self.wal
                .log_write(self.tx_id, CuerpoWal::Operacion(op.clone()));
            if self.wal.politica() == PoliticaFlush::CadaEscritura {
                self.wal.sync();
            }
        }
        // Commit record + sync: EL punto de durabilidad. A partir de
        // aquí la transacción existe aunque el proceso muera.
        let lsn_commit = self.wal.log_write(self.tx_id, CuerpoWal::Commit);
        self.wal.sync();

        // FASE APPLY — el store puede fallar aquí; el log ya lo tiene TODO.
        let mut resumen = ResumenCommitWal {
            lsn_commit,
            ..ResumenCommitWal::default()
        };
        for (indice, op) in self.buffer.iter().enumerate() {
            let ya = resumen.total_operaciones();
            match op {
                Operacion::PutNode(n) => match self.store.put_node(n.clone()) {
                    Ok(()) => resumen.nodos_escritos += 1,
                    Err(causa) => {
                        return Err(WalError::ApplyFallido {
                            indice,
                            aplicadas: ya,
                            causa,
                        });
                    }
                },
                Operacion::PutEdge(e) => match self.store.put_edge(e.clone()) {
                    Ok(()) => resumen.aristas_escritas += 1,
                    Err(causa) => {
                        return Err(WalError::ApplyFallido {
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
                        return Err(WalError::ApplyFallido {
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
                        return Err(WalError::ApplyFallido {
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

    /// ROLLBACK: descarta el buffer y escribe el marker Rollback.
    ///
    /// El store no se ha tocado NUNCA (nada se aplica fuera del commit),
    /// así que el descarte es limpio por construcción — la lección del
    /// cap. 27 sigue intacta. El marker no se sincroniza: si se pierde,
    /// la transacción sigue sin Commit y el replay la ignora igual.
    pub fn rollback(self) -> ResumenRollbackWal {
        let lsn_rollback = self.wal.log_write(self.tx_id, CuerpoWal::Rollback);
        ResumenRollbackWal {
            operaciones_descartadas: self.buffer.len(),
            lsn_rollback,
        }
    }
}

// ─────────────────── La re-valoración ACID tras el WAL ───────────────────

/// La re-valoración honesta de las garantías ACID DESPUÉS del cap. 28
/// (mismos tipos que [`crate::cap27_transacciones::informe_acid`], que
/// queda intacto como el informe del cap. 27).
///
/// ```text
/// A — Atomicidad:   PARCIAL (antes: parcial). El roll-forward vía
///                   replay COMPLETA una transacción cuyo apply quedó a
///                   medias… pero hay que EJECUTAR el replay: el
///                   arranque automático es la recuperación (cap. 29).
/// C — Consistencia: PARCIAL, sin cambios: el WAL reproduce exactamente
///                   lo confirmado; no añade restricciones (cap. 30).
/// I — Aislamiento:  PARCIAL, sin cambios: un único escritor por préstamo
///                   exclusivo; group commit real exige concurrencia
///                   (cap. 30).
/// D — Durabilidad:  PARCIAL (antes: NINGUNA). El commit ES durable EN
///                   EL LOG (commit record + sync, contado en tests);
///                   el store sigue en RAM — sobrevivir al reinicio
///                   completo es cap. 29.
/// ```
pub fn informe_acid_post_wal() -> Vec<EntradaAcid> {
    vec![
        EntradaAcid {
            garantia: GarantiaAcid::Atomicidad,
            nivel: NivelGarantia::Parcial,
            como_esta_hoy: "el roll-forward vía replay_wal COMPLETA una transacción cuyo \
                            apply quedó a medias (demostrado en tests con el StoreQueFalla \
                            del cap. 27), pero el replay hay que ejecutarlo: el arranque \
                            automático es la recuperación del cap. 29",
            capitulo_que_la_cierra: 29,
        },
        EntradaAcid {
            garantia: GarantiaAcid::Consistencia,
            nivel: NivelGarantia::Parcial,
            como_esta_hoy: "sin cambios respecto al cap. 27: sólo invariantes estructurales; \
                            el WAL reproduce exactamente lo confirmado, no añade \
                            restricciones declarativas",
            capitulo_que_la_cierra: 30,
        },
        EntradaAcid {
            garantia: GarantiaAcid::Aislamiento,
            nivel: NivelGarantia::Parcial,
            como_esta_hoy: "sin cambios: un único escritor por préstamo exclusivo &mut; el \
                            group commit REAL (varias transacciones compartiendo un fsync) \
                            exige concurrencia — cap. 30",
            capitulo_que_la_cierra: 30,
        },
        EntradaAcid {
            garantia: GarantiaAcid::Durabilidad,
            nivel: NivelGarantia::Parcial,
            como_esta_hoy: "el commit ES durable EN EL LOG (registro Commit + sync, \
                            contado en tests); el store sigue en RAM: sobrevivir al \
                            reinicio completo (reopen + replay automático) es cap. 29",
            capitulo_que_la_cierra: 29,
        },
    ]
}

// ─────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests_wal {
    use super::*;
    use crate::cap08_graph_store::MemoryStore;
    use crate::cap27_transacciones::informe_acid;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    /// Un nodo con props de TODOS los tipos de `Value`.
    fn nodo_rico(id: NodeId) -> Node {
        let mut n = Node::new(id, "Person");
        n.labels.push("Active".into());
        n.props.insert("entero".into(), Value::Int(-42));
        n.props
            .insert("real".into(), Value::Float(std::f64::consts::PI));
        n.props.insert("nombre".into(), Value::String("Ada".into()));
        n.props.insert("vivo".into(), Value::Bool(true));
        n.props.insert("nada".into(), Value::Null);
        n.props
            .insert("crudo".into(), Value::Bytes(vec![1, 2, 3, 0xFF]));
        n
    }

    // ── Serialización ─────────────────────────────────────────────

    #[test]
    fn wal_record_roundtrip_todos_los_tipos() {
        let registros = vec![
            WalRecord {
                lsn: 1,
                tx_id: 7,
                cuerpo: CuerpoWal::Begin,
            },
            WalRecord {
                lsn: 2,
                tx_id: 7,
                cuerpo: CuerpoWal::Operacion(Operacion::PutNode(nodo_rico(9))),
            },
            WalRecord {
                lsn: 3,
                tx_id: 7,
                cuerpo: CuerpoWal::Operacion(Operacion::PutEdge(Edge::new(3, 9, 9, "KNOWS"))),
            },
            WalRecord {
                lsn: 4,
                tx_id: 7,
                cuerpo: CuerpoWal::Operacion(Operacion::DeleteNode(9)),
            },
            WalRecord {
                lsn: 5,
                tx_id: 7,
                cuerpo: CuerpoWal::Operacion(Operacion::DeleteEdge(3)),
            },
            WalRecord {
                lsn: 6,
                tx_id: 7,
                cuerpo: CuerpoWal::Commit,
            },
            WalRecord {
                lsn: 7,
                tx_id: 8,
                cuerpo: CuerpoWal::Rollback,
            },
        ];
        for rec in &registros {
            let codificado = encode_wal_record(rec);
            let (dec, rest) = decode_wal_record(&codificado).unwrap();
            assert_eq!(dec, *rec, "roundtrip falla para {rec}");
            assert!(rest.is_empty());
        }
        // Y la cadena completa, de una tirada.
        let mut bytes = Vec::new();
        for rec in &registros {
            bytes.extend(encode_wal_record(rec));
        }
        assert_eq!(decodificar_wal(&bytes).unwrap(), registros);
    }

    #[test]
    fn encode_wal_record_es_determinista() {
        // Las props vienen de un HashMap (orden de iteración arbitrario):
        // el encoding DEBE ordenarlas para que el mismo registro produzca
        // siempre los mismos bytes (y el mismo CRC).
        let a = encode_wal_record(&WalRecord {
            lsn: 1,
            tx_id: 1,
            cuerpo: CuerpoWal::Operacion(Operacion::PutNode(nodo_rico(0))),
        });
        let b = encode_wal_record(&WalRecord {
            lsn: 1,
            tx_id: 1,
            cuerpo: CuerpoWal::Operacion(Operacion::PutNode(nodo_rico(0))),
        });
        assert_eq!(a, b);
        assert_eq!(
            crc32_simple(&a[..a.len() - 4]),
            crc32_simple(&b[..b.len() - 4])
        );
    }

    // ── LSNs y estructura del log ─────────────────────────────────

    #[test]
    fn lsn_monotonos_asignados_por_el_wal() {
        let mut wal = Wal::new();
        let mut store = MemoryStore::new();
        for i in 0..3u64 {
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_node(Node::new(i as usize, "Person")).unwrap();
            tx.commit().unwrap();
        }
        let lsns: Vec<Lsn> = wal.iter().map(|r| r.lsn).collect();
        assert_eq!(lsns.first(), Some(&1));
        // Consecutivos desde el 1: sin huecos ni repeticiones.
        for ventana in lsns.windows(2) {
            assert_eq!(ventana[1], ventana[0] + 1);
        }
        assert_eq!(wal.lsn_siguiente(), lsns.last().unwrap() + 1);
        assert_eq!(wal.record_count(), lsns.len());
    }

    #[test]
    fn begin_marker_es_el_primer_registro_de_la_tx() {
        let mut store = MemoryStore::new();
        let mut wal = Wal::new();
        let mut tx = WalTransaccion::begin(&mut store, &mut wal);
        let tx_id = tx.tx_id();
        tx.put_node(Node::new(0, "Person")).unwrap();
        tx.commit().unwrap();

        let primero = wal.iter().next().unwrap();
        assert_eq!(primero.tx_id, tx_id);
        assert_eq!(primero.cuerpo, CuerpoWal::Begin);
        // Y el registro Display es legible.
        assert!(primero.to_string().contains("BEGIN"));
    }

    // ── El protocolo: flush, sync y políticas ─────────────────────

    #[test]
    fn politica_por_defecto_y_syncs_por_escritura() {
        // La política por defecto es la regla de oro LITERAL: un sync por
        // cada log_write (3 operaciones) más el sync del commit = 4.
        let mut store = MemoryStore::new();
        let mut wal = Wal::new();
        assert_eq!(wal.politica(), PoliticaFlush::CadaEscritura);

        let mut tx = WalTransaccion::begin(&mut store, &mut wal);
        tx.put_node(Node::new(0, "A")).unwrap();
        tx.put_node(Node::new(1, "B")).unwrap();
        tx.put_node(Node::new(2, "C")).unwrap();
        tx.commit().unwrap();
        assert_eq!(wal.syncs(), 4);
    }

    #[test]
    fn solo_commit_dura_con_un_unico_sync_y_el_mismo_resultado() {
        // SoloCommit: 3 operaciones + commit = UN solo fsync. Es la
        // semilla del group commit (la variante concurrente es cap. 30).
        let mut store_a = MemoryStore::new();
        let mut wal_a = Wal::con_politica(PoliticaFlush::SoloCommit);
        let mut tx = WalTransaccion::begin(&mut store_a, &mut wal_a);
        tx.put_node(Node::new(0, "A")).unwrap();
        tx.put_node(Node::new(1, "B")).unwrap();
        tx.put_node(Node::new(2, "C")).unwrap();
        tx.commit().unwrap();
        assert_eq!(wal_a.syncs(), 1);

        // Mismo contenido de log y mismo resultado de replay que con la
        // política estricta: la OPTIMIZACIÓN no cambia la semántica.
        let mut store_b = MemoryStore::new();
        let mut wal_b = Wal::new();
        let mut tx = WalTransaccion::begin(&mut store_b, &mut wal_b);
        tx.put_node(Node::new(0, "A")).unwrap();
        tx.put_node(Node::new(1, "B")).unwrap();
        tx.put_node(Node::new(2, "C")).unwrap();
        tx.commit().unwrap();
        assert_eq!(wal_b.syncs(), 4);

        let mut ra = MemoryStore::new();
        let mut rb = MemoryStore::new();
        replay_wal(&mut ra, &wal_a).unwrap();
        replay_wal(&mut rb, &wal_b).unwrap();
        assert_eq!(ra.node_count(), rb.node_count());
        assert_eq!(ra.get_node(0), rb.get_node(0));
    }

    #[test]
    fn commit_aplica_todo_y_es_durable() {
        let mut store = MemoryStore::new();
        let mut wal = Wal::new();
        let mut tx = WalTransaccion::begin(&mut store, &mut wal);
        tx.put_node(Node::new(0, "Person")).unwrap();
        tx.put_node(Node::new(1, "City")).unwrap();
        tx.put_edge(Edge::new(0, 0, 1, "LIVES_IN")).unwrap();
        let resumen = tx.commit().unwrap();

        assert_eq!(resumen.nodos_escritos, 2);
        assert_eq!(resumen.aristas_escritas, 1);
        assert_eq!(resumen.total_operaciones(), 3);
        assert_eq!(store.node_count(), 2);
        assert_eq!(store.edge_count(), 1);
        // El commit ES durable: el último registro es el Commit, con el
        // LSN que anuncia el resumen, y hubo al menos un sync.
        let ultimo = wal.iter().last().unwrap();
        assert_eq!(ultimo.cuerpo, CuerpoWal::Commit);
        assert_eq!(resumen.lsn_commit, ultimo.lsn);
        assert!(wal.syncs() >= 1);
        assert!(resumen.to_string().contains("DURABLE"));
        assert!(resumen.to_string().contains("lsn"));
    }

    #[test]
    fn rollback_descarta_y_escribe_marker() {
        let mut store = MemoryStore::new();
        let mut wal = Wal::new();
        let mut tx = WalTransaccion::begin(&mut store, &mut wal);
        tx.put_node(Node::new(9, "Intruso")).unwrap();
        let resumen = tx.rollback();

        assert_eq!(resumen.operaciones_descartadas, 1);
        assert_eq!(store.node_count(), 0); // el store no se tocó
        let ultimo = wal.iter().last().unwrap();
        assert_eq!(ultimo.cuerpo, CuerpoWal::Rollback);
        assert_eq!(resumen.lsn_rollback, ultimo.lsn);
        // El marker no necesita fsync: la ausencia de Commit ya aborta.
        assert_eq!(wal.syncs(), 0);

        // Y el replay ignora la transacción COMPLETA.
        let mut store2 = MemoryStore::new();
        let informe = replay_wal(&mut store2, &wal).unwrap();
        assert_eq!(informe.transacciones_descartadas, 1);
        assert_eq!(informe.transacciones_confirmadas, 0);
        assert_eq!(informe.operaciones_reaplicadas, 0);
        assert_eq!(store2.node_count(), 0);
    }

    // ── Redo ──────────────────────────────────────────────────────

    #[test]
    fn replay_reconstruye_lo_confirmado_en_store_vacio() {
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

        // El «reinicio»: store nuevo, mismo log.
        let mut renacido = MemoryStore::new();
        let informe = replay_wal(&mut renacido, &wal).unwrap();
        assert_eq!(informe.transacciones_confirmadas, 1);
        assert_eq!(informe.transacciones_descartadas, 1);
        assert_eq!(informe.operaciones_reaplicadas, 3);
        assert_eq!(renacido.node_count(), 2);
        assert_eq!(renacido.edge_count(), 1);
        assert!(renacido.get_node(2).is_none(), "la abortada no vuelve");
        // El grafo renacido es IGUAL al original (el replay es fiel).
        assert_eq!(
            renacido.get_node(0).unwrap().labels,
            store.get_node(0).unwrap().labels
        );
    }

    #[test]
    fn replay_es_idempotente() {
        let mut store = MemoryStore::new();
        let mut wal = Wal::new();
        {
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_node(Node::new(0, "A")).unwrap();
            tx.put_node(Node::new(1, "B")).unwrap();
            tx.put_edge(Edge::new(0, 0, 1, "KNOWS")).unwrap();
            tx.commit().unwrap();
        }
        let primero = replay_wal(&mut store, &wal).unwrap();
        let segundo = replay_wal(&mut store, &wal).unwrap();
        let tercero = replay_wal(&mut store, &wal).unwrap();
        // El redo procesa las mismas operaciones otra vez…
        assert_eq!(segundo, primero);
        assert_eq!(tercero, primero);
        // …pero el store NO cambia: re-replay no duplica NADA.
        assert_eq!(store.node_count(), 2);
        assert_eq!(store.edge_count(), 1);
        assert_eq!(store.out_edges(0), vec![0]);
    }

    #[test]
    fn tx_abandonada_sin_commit_no_sobrevive_al_replay() {
        // Drop sin commit ni rollback: el proceso «murió» con la tx
        // abierta. La ausencia de Commit YA es la respuesta del log.
        let mut store = MemoryStore::new();
        let mut wal = Wal::new();
        {
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_node(Node::new(5, "Efímero")).unwrap();
            // drop implícito: ni commit ni rollback.
        }
        let mut store2 = MemoryStore::new();
        let informe = replay_wal(&mut store2, &wal).unwrap();
        assert_eq!(informe.transacciones_confirmadas, 0);
        assert_eq!(informe.transacciones_descartadas, 1);
        assert_eq!(store2.node_count(), 0);
    }

    #[test]
    fn transacciones_intercaladas_solo_la_confirmada_sobrevive() {
        // El log admite INTERCALACIÓN (dos tx abiertas a la vez, como en
        // un sistema con group commit): lo que decide es el Commit.
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

        let mut store = MemoryStore::new();
        let informe = replay_wal(&mut store, &wal).unwrap();
        assert_eq!(informe.transacciones_confirmadas, 1);
        assert_eq!(informe.transacciones_descartadas, 1);
        assert_eq!(informe.operaciones_reaplicadas, 1);
        assert_eq!(store.node_count(), 1);
        assert!(store.get_node(1).is_some());
        assert!(store.get_node(0).is_none());
        assert!(store.get_node(2).is_none());
        assert!(informe.to_string().contains("1 transacciones confirmadas"));
    }

    // ── EL TEST-TESIS: el apply a medias del cap. 27, rescatado ──

    /// Store de pruebas que delega en un `MemoryStore` y FALLA en la
    /// N-ésima escritura: el mismo `StoreQueFalla` del cap. 27, traído
    /// aquí para INVERTIR sus tests de regresión.
    struct StoreQueFalla {
        inner: MemoryStore,
        escrituras: u64,
        /// Nº de escritura que debe fallar (1-based).
        fallar_en: u64,
        /// Cómo fallar: devolviendo Err o con pánico (el «corte de luz»).
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
    fn apply_fallido_a_medias_rescatado_por_replay() {
        // El escenario EXACTO del cap. 27: el buffer valida, el store
        // dice «no» en la 3ª escritura de 4. Allí el test afirmaba
        // node_count()==2 y «sin log no hay vuelta atrás». Aquí la
        // vuelta atrás existe — hacia DELANTE.
        let mut store = StoreQueFalla {
            inner: MemoryStore::new(),
            escrituras: 0,
            fallar_en: 3,
            con_panic: false,
        };
        let mut wal = Wal::new();
        let mut tx = WalTransaccion::begin(&mut store, &mut wal);
        tx.put_node(Node::new(0, "A")).unwrap();
        tx.put_node(Node::new(1, "B")).unwrap();
        tx.put_node(Node::new(2, "C")).unwrap(); // el store fallará aquí
        tx.put_node(Node::new(3, "D")).unwrap();
        let err = tx.commit().unwrap_err();

        assert_eq!(
            err,
            WalError::ApplyFallido {
                indice: 2,
                aplicadas: 2,
                causa: StoreError::UnknownNode(usize::MAX)
            }
        );
        assert!(err.to_string().contains("replay_wal"));
        // Mismo estado a medias que en el cap. 27…
        assert_eq!(store.node_count(), 2);
        // …pero el log YA contiene TODO + Commit + sync (la diferencia):
        assert_eq!(wal.syncs(), 1 + 4); // commit + 4 escrituras (CadaEscritura)
        let ultimo = wal.iter().last().unwrap();
        assert_eq!(ultimo.cuerpo, CuerpoWal::Commit);

        // EL RESCATE: replay sobre el MISMO store a medias → COMPLETA.
        let informe = replay_wal(&mut store, &wal).unwrap();
        assert_eq!(informe.operaciones_reaplicadas, 4);
        assert_eq!(store.node_count(), 4); // LA TRANSACCIÓN COMPLETA
        assert!(store.get_node(3).is_some());
    }

    #[test]
    fn corte_de_luz_a_mitad_de_apply_rescatado_por_replay() {
        // El «corte de luz» del cap. 27 (pánico entre dos escrituras):
        // allí quedaba «a medias y NADIE recuerda qué faltaba». Ahora el
        // log SÍ recuerda — y el replay completa.
        let mut store = StoreQueFalla {
            inner: MemoryStore::new(),
            escrituras: 0,
            fallar_en: 2,
            con_panic: true,
        };
        let mut wal = Wal::new();
        let resultado = catch_unwind(AssertUnwindSafe(|| {
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_node(Node::new(0, "A")).unwrap();
            tx.put_node(Node::new(1, "B")).unwrap(); // pánico AQUÍ
            tx.put_node(Node::new(2, "C")).unwrap();
            tx.put_node(Node::new(3, "D")).unwrap();
            tx.commit()
        }));
        let panico = resultado.unwrap_err();
        let mensaje = panico
            .downcast_ref::<String>()
            .expect("el pánico lleva un mensaje String");
        assert!(mensaje.contains("corte de luz"));
        // Cap. 27: a medias, sin memoria. Cap. 28: a medias, CON log.
        assert_eq!(store.node_count(), 1);
        assert!(matches!(wal.iter().last(), Some(r) if r.cuerpo == CuerpoWal::Commit));

        let informe = replay_wal(&mut store, &wal).unwrap();
        assert_eq!(informe.operaciones_reaplicadas, 4);
        assert_eq!(store.node_count(), 4);
    }

    // ── Corrupción y colas truncadas ──────────────────────────────

    #[test]
    fn crc_invalido_detectado() {
        let rec1 = WalRecord {
            lsn: 1,
            tx_id: 1,
            cuerpo: CuerpoWal::Begin,
        };
        let rec2 = WalRecord {
            lsn: 2,
            tx_id: 1,
            cuerpo: CuerpoWal::Operacion(Operacion::PutNode(nodo_rico(0))),
        };
        let mut bytes = encode_wal_record(&rec1);
        bytes.extend(encode_wal_record(&rec2));
        // Tocar el ÚLTIMO byte = tocar el CRC del último registro.
        let ultimo = bytes.len() - 1;
        bytes[ultimo] ^= 0xFF;

        let err = decodificar_wal(&bytes).unwrap_err();
        match err {
            WalError::CrcInvalido {
                lsn,
                esperado,
                leido,
            } => {
                // El cuerpo del registro dañado sigue declarando lsn 2.
                assert_eq!(lsn, Some(2));
                assert_ne!(esperado, leido);
            }
            otra => panic!("esperaba CrcInvalido, llegó {otra:?}"),
        }
        assert!(err.to_string().contains("CRC"));
    }

    #[test]
    fn registro_truncado_detectado() {
        let rec = WalRecord {
            lsn: 1,
            tx_id: 1,
            cuerpo: CuerpoWal::Operacion(Operacion::PutNode(nodo_rico(0))),
        };
        let completo = encode_wal_record(&rec);
        // Cortar 3 bytes: el length-prefix reclama más de lo que hay.
        let cortado = &completo[..completo.len() - 3];
        let err = decodificar_wal(cortado).unwrap_err();
        assert!(matches!(err, WalError::RegistroTruncado { .. }));
        assert!(err.to_string().contains("corte de luz"));
        // Y un log de menos de 25 bytes ni siquiera abre.
        let err = decodificar_wal(&completo[..10]).unwrap_err();
        assert!(matches!(
            err,
            WalError::RegistroTruncado {
                necesitados: 25,
                ..
            }
        ));
    }

    #[test]
    fn cola_truncada_el_commit_no_llego_no_confirma() {
        // El corte de luz DURANTE la escritura del commit record: la tx
        // no es durable y el replay la descarta SIN error (parada limpia).
        let mut store = MemoryStore::new();
        let mut wal = Wal::new();
        {
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_node(Node::new(0, "A")).unwrap();
            tx.commit().unwrap(); // Begin(lsn1) Op(lsn2) Commit(lsn3)
        }
        let len = wal.as_bytes().len();
        wal.truncar_a_bytes(len - 2); // el Commit queda a medias

        let mut renacido = MemoryStore::new();
        let informe = replay_wal(&mut renacido, &wal).unwrap(); // parada LIMPIA
        assert_eq!(informe.transacciones_confirmadas, 0);
        assert_eq!(informe.transacciones_descartadas, 1);
        assert_eq!(renacido.node_count(), 0);
    }

    #[test]
    fn corrupcion_al_inicio_el_replay_para_en_el_prefijo_integro() {
        // Un byte tocado en el PRIMER registro: el iterador para ahí
        // mismo — se confía sólo en el prefijo íntegro (que aquí es
        // vacío). El replay NO falla: sencillamente no ve nada.
        let mut store = MemoryStore::new();
        let mut wal = Wal::new();
        {
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_node(Node::new(0, "A")).unwrap();
            tx.commit().unwrap();
        }
        {
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_node(Node::new(1, "B")).unwrap();
            tx.commit().unwrap();
        }
        // Corromper DENTRO del primer registro (byte 10: zona lsn/tx).
        // Los tests viven en el módulo: pueden tocar `bytes` a mano,
        // como el cap. 27 sembraba `tx.buffer`.
        wal.bytes[10] ^= 0xFF;

        let informe = replay_wal(&mut store, &wal).unwrap(); // parada limpia
        assert_eq!(informe.transacciones_confirmadas, 0);
        assert_eq!(informe.operaciones_reaplicadas, 0);
        // El store queda EXACTAMENTE como estaba: el prefijo íntegro no
        // aportó nada, pero tampoco estropeó nada.
        assert_eq!(store.node_count(), 2);
    }

    #[test]
    fn lsn_invalido_en_cadena_detectado() {
        // Dos registros con un HUECO de LSN (bytes quitados de en medio):
        // el CRC de ambos es válido, pero la cadena delata el hueco.
        let r1 = WalRecord {
            lsn: 1,
            tx_id: 1,
            cuerpo: CuerpoWal::Begin,
        };
        let r5 = WalRecord {
            lsn: 5,
            tx_id: 1,
            cuerpo: CuerpoWal::Commit,
        };
        let mut bytes = encode_wal_record(&r1);
        bytes.extend(encode_wal_record(&r5));
        let err = decodificar_wal(&bytes).unwrap_err();
        assert_eq!(
            err,
            WalError::LsnInvalido {
                leido: 5,
                esperado: 2
            }
        );
        assert!(err.to_string().contains("hueco"));
    }

    // ── Truncado (log truncation del guion) ───────────────────────

    #[test]
    fn truncar_hasta_lsn_deja_lo_posterior_y_no_reutiliza_lsns() {
        let mut store = MemoryStore::new();
        let mut wal = Wal::new();
        {
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_node(Node::new(0, "A")).unwrap();
            tx.put_node(Node::new(1, "B")).unwrap();
            tx.commit().unwrap(); // lsns 1..4
        }
        // CONTRATO: 0 y 1 ya son durable en el store → se puede truncar.
        assert_eq!(wal.truncar_hasta_lsn(4), 4);
        assert_eq!(wal.as_bytes().len(), 0);
        assert_eq!(wal.record_count(), 0);

        {
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_node(Node::new(2, "C")).unwrap();
            tx.commit().unwrap(); // lsns 5..7: JAMÁS se reutilizan
        }
        assert!(wal.lsn_siguiente() > 4);
        let lsns: Vec<Lsn> = wal.iter().map(|r| r.lsn).collect();
        assert_eq!(lsns.first(), Some(&5));

        // Replay sobre el MISMO store: sólo re-procesa lo NO truncado
        // (lo truncado ya está — idempotente, sin duplicar).
        let informe = replay_wal(&mut store, &wal).unwrap();
        assert_eq!(informe.transacciones_confirmadas, 1);
        assert_eq!(informe.operaciones_reaplicadas, 1);
        assert_eq!(store.node_count(), 3);
    }

    #[test]
    fn truncado_la_deuda_documentada_replay_no_recupera_lo_truncado() {
        // La otra cara del contrato, como test: truncar lo que NO es
        // durable y hacer replay sobre un store VACÍO lo PIERDE. El
        // checkpoint que evita este pie (cap. 29) decidirá «hasta dónde»
        // de forma automática; hoy lo firma el llamador.
        let mut store = MemoryStore::new();
        let mut wal = Wal::new();
        {
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_node(Node::new(0, "A")).unwrap();
            tx.commit().unwrap();
        }
        let _ = wal.truncar_hasta_lsn(3); // ignora el contrato a propósito

        let mut vacio = MemoryStore::new();
        let informe = replay_wal(&mut vacio, &wal).unwrap();
        assert_eq!(informe.transacciones_confirmadas, 0);
        assert_eq!(vacio.node_count(), 0); // el nodo 0 NO vuelve
    }

    // ── La re-valoración ACID tras el WAL ─────────────────────────

    #[test]
    fn informe_post_wal_actualiza_d_y_reasigna_caps() {
        let antes = informe_acid();
        let despues = informe_acid_post_wal();
        assert_eq!(despues.len(), 4);
        // Orden A, C, I, D — como el del cap. 27.
        assert_eq!(
            despues.iter().map(|e| e.garantia).collect::<Vec<_>>(),
            antes
                .entradas()
                .iter()
                .map(|e| e.garantia)
                .collect::<Vec<_>>()
        );
        // D: de NINGUNA a PARCIAL — el commit ya es durable EN EL LOG.
        let d_antes = antes.por_garantia(GarantiaAcid::Durabilidad).unwrap();
        let d_despues = &despues[3];
        assert_eq!(d_antes.nivel, NivelGarantia::Ninguna);
        assert_eq!(d_despues.nivel, NivelGarantia::Parcial);
        assert_eq!(d_antes.capitulo_que_la_cierra, 28);
        assert_eq!(d_despues.capitulo_que_la_cierra, 29);
        assert!(d_despues.como_esta_hoy.contains("LOG"));
        // A: sigue PARCIAL pero la CIERRA el 29 (falta el arranque
        // automático del replay).
        let a_antes = antes.por_garantia(GarantiaAcid::Atomicidad).unwrap();
        let a_despues = &despues[0];
        assert_eq!(a_antes.nivel, a_despues.nivel);
        assert_eq!(a_antes.capitulo_que_la_cierra, 28);
        assert_eq!(a_despues.capitulo_que_la_cierra, 29);
        assert!(a_despues.como_esta_hoy.contains("roll-forward"));
        // C e I: sin cambios (cap. 30).
        assert_eq!(despues[1].capitulo_que_la_cierra, 30);
        assert_eq!(despues[2].capitulo_que_la_cierra, 30);
    }

    // ── Errores ───────────────────────────────────────────────────

    #[test]
    fn errores_display_y_std_error() {
        let e1 = WalError::Serializacion("formato roto".into());
        assert!(e1.to_string().contains("formato roto"));

        let e2 = WalError::CrcInvalido {
            lsn: Some(9),
            esperado: 0xAB,
            leido: 0xCD,
        };
        assert!(e2.to_string().contains("lsn 9"));

        let e3 = WalError::ApplyFallido {
            indice: 2,
            aplicadas: 2,
            causa: StoreError::UnknownNode(7),
        };
        assert!(e3.to_string().contains("replay_wal COMPLETA"));

        let e4 = WalError::Validacion(TransaccionError::OperacionInvalida {
            indice: 0,
            causa: StoreError::DuplicateNode(1),
        });
        assert!(e4.to_string().contains("commit rechazado"));

        let e5 = WalError::RedoFallido {
            lsn: 12,
            causa: StoreError::InvalidEdgeEndpoints {
                source: 1,
                target: 2,
            },
        };
        assert!(e5.to_string().contains("contrato de durabilidad"));

        use std::error::Error;
        assert_eq!(
            e3.source().unwrap().to_string(),
            StoreError::UnknownNode(7).to_string()
        );
        assert_eq!(
            e4.source().unwrap().to_string(),
            TransaccionError::OperacionInvalida {
                indice: 0,
                causa: StoreError::DuplicateNode(1)
            }
            .to_string()
        );
        assert_eq!(
            e5.source().unwrap().to_string(),
            StoreError::InvalidEdgeEndpoints {
                source: 1,
                target: 2
            }
            .to_string()
        );
        assert!(e1.source().is_none());
        assert!(e2.source().is_none());
    }

    // ── Semántica del replay sobre operaciones compuestas ─────────

    #[test]
    fn delete_y_recrear_via_replay() {
        // Pre-existente: 0 -> 1 con arista.
        let base = {
            let mut s = MemoryStore::new();
            s.put_node(Node::new(0, "Original")).unwrap();
            s.put_node(Node::new(1, "Quieto")).unwrap();
            s.put_edge(Edge::new(0, 0, 1, "KNOWS")).unwrap();
            s
        };
        let mut store = base.clone();
        let mut wal = Wal::new();
        {
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.delete_node(0).unwrap(); // arrastra la arista
            tx.put_node(Node::new(0, "Renacido")).unwrap();
            tx.commit().unwrap();
        }
        assert_eq!(store.node_count(), 2);
        assert_eq!(store.edge_count(), 0);
        assert_eq!(
            store.get_node(0).unwrap().labels,
            vec!["Renacido".to_string()]
        );

        // El MISMO resultado desde el store base + replay.
        let mut store2 = base;
        let informe = replay_wal(&mut store2, &wal).unwrap();
        assert_eq!(informe.operaciones_reaplicadas, 2);
        assert_eq!(store2.node_count(), 2);
        assert_eq!(store2.edge_count(), 0);
        assert_eq!(
            store2.get_node(0).unwrap().labels,
            vec!["Renacido".to_string()]
        );
    }

    #[test]
    fn todos_los_value_sobreviven_wal_y_replay() {
        let mut store = MemoryStore::new();
        let mut wal = Wal::new();
        {
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_node(nodo_rico(0)).unwrap();
            tx.put_edge(Edge::new(0, 0, 0, "SELF")).unwrap();
            tx.commit().unwrap();
        }
        let mut renacido = MemoryStore::new();
        replay_wal(&mut renacido, &wal).unwrap();
        let original = store.get_node(0).unwrap().clone();
        let copia = renacido.get_node(0).unwrap().clone();
        assert_eq!(original, copia); // props de TODOS los Value tipos, intactos
        assert_eq!(
            original.props.get("crudo"),
            Some(&Value::Bytes(vec![1, 2, 3, 0xFF]))
        );
        assert_eq!(renacido.edge_count(), 1);
    }

    #[test]
    fn replay_falla_ruidosamente_si_el_truncado_rompio_dependencias() {
        // El contrato de truncado roto EN SERIO: la tx que creaba los
        // nodos se truncó y una arista posterior queda huérfana. El redo
        // no contesta «casi-bien»: falla con el LSN señalado.
        let mut wal = Wal::new();
        let (tx1, _) = wal.begin_tx();
        wal.log_write(
            tx1,
            CuerpoWal::Operacion(Operacion::PutNode(Node::new(0, "A"))),
        );
        wal.log_write(
            tx1,
            CuerpoWal::Operacion(Operacion::PutNode(Node::new(1, "B"))),
        );
        wal.log_write(tx1, CuerpoWal::Commit);
        let (tx2, _) = wal.begin_tx();
        wal.log_write(
            tx2,
            CuerpoWal::Operacion(Operacion::PutEdge(Edge::new(0, 0, 1, "KNOWS"))),
        );
        wal.log_write(tx2, CuerpoWal::Commit);

        let _ = wal.truncar_hasta_lsn(4); // tx1 fuera… con sus nodos

        let mut vacio = MemoryStore::new();
        let err = replay_wal(&mut vacio, &wal).unwrap_err();
        match err {
            WalError::RedoFallido { lsn, causa } => {
                assert_eq!(lsn, 6); // la arista huérfana
                assert!(matches!(
                    causa,
                    StoreError::InvalidEdgeEndpoints {
                        source: 0,
                        target: 1
                    }
                ));
            }
            otra => panic!("esperaba RedoFallido, llegó {otra:?}"),
        }
    }
}
