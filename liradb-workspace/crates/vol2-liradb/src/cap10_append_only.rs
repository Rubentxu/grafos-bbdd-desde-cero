use crate::cap09_encoding::{decode_u32_le, encode_u32_le};

// ─────────────────── Cap 10: append-only log ───────────────────

/// Tipos de registros del log append-only.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RecordKind {
    /// Nodo nuevo (insert/update).
    PutNode = 1,
    /// Arista nueva (insert/update).
    PutEdge = 2,
    /// Eliminar nodo.
    DeleteNode = 3,
    /// Eliminar arista.
    DeleteEdge = 4,
    /// Commit point (checkpoint para recovery).
    Commit = 5,
}

/// Layout de un registro del log:
///   [record_len: u32 LE] [kind: u8] [id: u32 LE] [payload_len: u32 LE]
///   [payload bytes] [crc32: u32 LE]
///
/// El `record_len` cubre todo lo que sigue hasta (sin incluir) el siguiente
/// `record_len`. Permite al iterador saber dónde termina cada record.
///
/// El CRC32 cubre `kind || id || payload_len || payload`.
#[derive(Debug, Clone, PartialEq)]
pub struct LogRecord {
    pub kind: RecordKind,
    pub id: u32,
    pub payload: Vec<u8>,
}

/// Codifica un registro a bytes (incluyendo CRC32 y length prefix).
pub fn encode_log_record(rec: &LogRecord) -> Vec<u8> {
    let mut body = Vec::new();
    body.push(rec.kind as u8);
    body.extend_from_slice(&encode_u32_le(rec.id));
    body.extend_from_slice(&encode_u32_le(rec.payload.len() as u32));
    body.extend_from_slice(&rec.payload);

    let crc = crc32_simple(&body);
    let mut inner = body;
    inner.extend_from_slice(&encode_u32_le(crc));

    // Length prefix: longitud del "inner" (todo menos el propio length prefix).
    let len_prefix = encode_u32_le(inner.len() as u32);
    let mut out = Vec::with_capacity(4 + inner.len());
    out.extend_from_slice(&len_prefix);
    out.extend(inner);
    out
}

/// Decodifica un registro desde bytes (verifica CRC32). Usa length prefix.
pub fn decode_log_record(bytes: &[u8]) -> Result<(LogRecord, &[u8]), String> {
    // Mínimo: length(4) + kind(1) + id(4) + len(4) + crc(4) = 17.
    if bytes.len() < 17 {
        return Err(format!(
            "record: need at least 17 bytes, have {}",
            bytes.len()
        ));
    }
    let inner_len = decode_u32_le(bytes[..4].try_into().unwrap()) as usize;
    if bytes.len() < 4 + inner_len {
        return Err(format!(
            "record: truncated (need {} bytes, have {})",
            4 + inner_len,
            bytes.len()
        ));
    }
    let inner = &bytes[4..4 + inner_len];
    let body_len = inner.len() - 4;
    let body = &inner[..body_len];
    let crc_read = decode_u32_le(inner[body_len..].try_into().unwrap());
    let crc_calc = crc32_simple(body);
    if crc_read != crc_calc {
        return Err(format!(
            "crc mismatch: stored {crc_read:#x}, computed {crc_calc:#x}"
        ));
    }
    let kind = match body[0] {
        1 => RecordKind::PutNode,
        2 => RecordKind::PutEdge,
        3 => RecordKind::DeleteNode,
        4 => RecordKind::DeleteEdge,
        5 => RecordKind::Commit,
        other => return Err(format!("record: unknown kind {other}")),
    };
    let id = decode_u32_le(body[1..5].try_into().unwrap());
    let payload_len = decode_u32_le(body[5..9].try_into().unwrap()) as usize;
    if body.len() < 9 + payload_len {
        return Err("record: payload truncated".into());
    }
    let payload = body[9..9 + payload_len].to_vec();
    Ok((LogRecord { kind, id, payload }, &bytes[4 + inner_len..]))
}

/// CRC32 simplificado (polinomio IEEE 802.3, sin tabla).
///
/// Para producción usaríamos `crc32fast`. Esta implementación es
/// didáctica: O(n) por byte, sin dependencias.
pub fn crc32_simple(bytes: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = if crc & 1 != 0 { 0xEDB8_8320 } else { 0 };
            crc = (crc >> 1) ^ mask;
        }
    }
    crc ^ 0xFFFF_FFFF
}

/// Log append-only en memoria (para tests y referencia).
/// En producción, el "disco" sería un `File` con `O_APPEND`.
#[derive(Debug, Default)]
pub struct AppendOnlyLog {
    /// Bytes del log (suma de todos los registros encodificados).
    bytes: Vec<u8>,
    /// Contador de registros.
    count: usize,
}

impl AppendOnlyLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Añade un registro al log (append-only).
    pub fn append(&mut self, rec: &LogRecord) -> usize {
        let encoded = encode_log_record(rec);
        let offset = self.bytes.len();
        self.bytes.extend_from_slice(&encoded);
        self.count += 1;
        offset
    }

    /// Tamaño total del log en bytes.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// ¿Está vacío?
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Número de registros.
    pub fn record_count(&self) -> usize {
        self.count
    }

    /// Bytes crudos (para inspección).
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Itera sobre todos los registros decodificados.
    pub fn iter(&self) -> LogIterator<'_> {
        LogIterator {
            bytes: &self.bytes,
            pos: 0,
        }
    }

    /// Trunca el log a partir de un offset (para tests de recovery).
    pub fn truncate_to(&mut self, len: usize) {
        self.bytes.truncate(len);
    }
}

/// Iterador sobre registros de un `AppendOnlyLog`.
pub struct LogIterator<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Iterator for LogIterator<'a> {
    type Item = LogRecord;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.bytes.len() {
            return None;
        }
        match decode_log_record(&self.bytes[self.pos..]) {
            Ok((_rec, rest)) => {
                self.pos = self.bytes.len() - rest.len();
                Some(_rec)
            }
            Err(_) => None,
        }
    }
}

#[cfg(test)]
mod tests_log {
    use super::*;

    #[test]
    fn crc32_known_value_empty() {
        // CRC32 del input vacío es 0.
        assert_eq!(crc32_simple(b""), 0);
    }

    #[test]
    fn crc32_known_value_a() {
        // CRC32("a") = 0xE8B7BE43
        assert_eq!(crc32_simple(b"a"), 0xE8B7_BE43);
    }

    #[test]
    fn log_record_roundtrip() {
        let rec = LogRecord {
            kind: RecordKind::PutNode,
            id: 42,
            payload: vec![1, 2, 3, 4],
        };
        let encoded = encode_log_record(&rec);
        let (decoded, rest) = decode_log_record(&encoded).unwrap();
        assert_eq!(decoded, rec);
        assert!(rest.is_empty());
    }

    #[test]
    fn log_record_corrupto_falla() {
        let rec = LogRecord {
            kind: RecordKind::PutNode,
            id: 42,
            payload: vec![1, 2, 3, 4],
        };
        let mut encoded = encode_log_record(&rec);
        // Corromper el último byte (CRC).
        let last = encoded.len() - 1;
        encoded[last] ^= 0xFF;
        assert!(decode_log_record(&encoded).is_err());
    }

    #[test]
    fn append_only_log_basico() {
        let mut log = AppendOnlyLog::new();
        log.append(&LogRecord {
            kind: RecordKind::PutNode,
            id: 0,
            payload: vec![10],
        });
        log.append(&LogRecord {
            kind: RecordKind::PutEdge,
            id: 0,
            payload: vec![20, 30, 40],
        });
        log.append(&LogRecord {
            kind: RecordKind::Commit,
            id: 0,
            payload: vec![],
        });

        assert_eq!(log.record_count(), 3);
        // Smoke test del iterador: al menos no debe estar vacío.
        let records: Vec<LogRecord> = log.iter().collect();
        assert!(
            !records.is_empty(),
            "iter returned empty; bytes={:?}",
            log.as_bytes()
        );
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].kind, RecordKind::PutNode);
        assert_eq!(records[1].kind, RecordKind::PutEdge);
        assert_eq!(records[2].kind, RecordKind::Commit);
    }

    #[test]
    fn log_recovery_desde_offset() {
        // Simula un crash: truncamos a la mitad y comprobamos que los
        // registros válidos hasta ese punto se pueden leer.
        let mut log = AppendOnlyLog::new();
        for i in 0..10 {
            log.append(&LogRecord {
                kind: RecordKind::PutNode,
                id: i,
                payload: vec![i as u8; 10],
            });
        }
        let mid = log.len() / 2;
        log.truncate_to(mid);
        let records: Vec<LogRecord> = log.iter().collect();
        // Debe leer al menos un registro completo antes del corte.
        assert!(!records.is_empty());
        // Todos los IDs leídos deben ser válidos.
        for r in &records {
            assert_eq!(r.kind, RecordKind::PutNode);
        }
    }
}
