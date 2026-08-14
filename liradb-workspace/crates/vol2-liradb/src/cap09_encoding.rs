use crate::cap07_modelo::Value;

// ─────────────────── Cap 9: encoding binario ───────────────────

pub const FORMAT_VERSION: u32 = 1;

pub fn encode_u32_le(value: u32) -> [u8; 4] {
    value.to_le_bytes()
}
pub fn decode_u32_le(bytes: &[u8; 4]) -> u32 {
    u32::from_le_bytes(*bytes)
}
pub fn encode_i64_le(value: i64) -> [u8; 8] {
    value.to_le_bytes()
}
pub fn decode_i64_le(bytes: &[u8; 8]) -> i64 {
    i64::from_le_bytes(*bytes)
}
pub fn encode_f64_le(value: f64) -> [u8; 8] {
    value.to_le_bytes()
}
pub fn decode_f64_le(bytes: &[u8; 8]) -> f64 {
    f64::from_le_bytes(*bytes)
}

pub fn encode_string(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + s.len());
    out.extend_from_slice(&encode_u32_le(s.len() as u32));
    out.extend_from_slice(s.as_bytes());
    out
}

pub fn decode_string(bytes: &[u8]) -> Result<(String, &[u8]), String> {
    if bytes.len() < 4 {
        return Err("string: too short".into());
    }
    let mut lb = [0u8; 4];
    lb.copy_from_slice(&bytes[..4]);
    let len = decode_u32_le(&lb) as usize;
    if bytes.len() < 4 + len {
        return Err("string: payload truncated".into());
    }
    let s = std::str::from_utf8(&bytes[4..4 + len])
        .map_err(|e| e.to_string())?
        .to_string();
    Ok((s, &bytes[4 + len..]))
}

pub fn encode_value(v: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    match v {
        Value::Null => out.push(0),
        Value::Bool(b) => {
            out.push(1);
            out.push(u8::from(*b));
        }
        Value::Int(i) => {
            out.push(2);
            out.extend_from_slice(&encode_i64_le(*i));
        }
        Value::Float(f) => {
            out.push(3);
            out.extend_from_slice(&encode_f64_le(*f));
        }
        Value::String(s) => {
            out.push(4);
            out.extend_from_slice(&encode_string(s));
        }
        Value::Bytes(b) => {
            out.push(5);
            out.extend_from_slice(&encode_u32_le(b.len() as u32));
            out.extend_from_slice(b);
        }
    }
    out
}

pub fn decode_value(bytes: &[u8]) -> Result<(Value, &[u8]), String> {
    if bytes.is_empty() {
        return Err("value: empty".into());
    }
    let tag = bytes[0];
    let rest = &bytes[1..];
    match tag {
        0 => Ok((Value::Null, rest)),
        1 => {
            if rest.is_empty() {
                return Err("bool: missing".into());
            }
            Ok((Value::Bool(rest[0] != 0), &rest[1..]))
        }
        2 => {
            if rest.len() < 8 {
                return Err("int: need 8 bytes".into());
            }
            let mut b = [0u8; 8];
            b.copy_from_slice(&rest[..8]);
            Ok((Value::Int(decode_i64_le(&b)), &rest[8..]))
        }
        3 => {
            if rest.len() < 8 {
                return Err("float: need 8 bytes".into());
            }
            let mut b = [0u8; 8];
            b.copy_from_slice(&rest[..8]);
            Ok((Value::Float(decode_f64_le(&b)), &rest[8..]))
        }
        4 => {
            let (s, rest) = decode_string(rest)?;
            Ok((Value::String(s), rest))
        }
        5 => {
            if rest.len() < 4 {
                return Err("bytes: need length".into());
            }
            let mut lb = [0u8; 4];
            lb.copy_from_slice(&rest[..4]);
            let len = decode_u32_le(&lb) as usize;
            if rest.len() < 4 + len {
                return Err("bytes: truncated".into());
            }
            let b = rest[4..4 + len].to_vec();
            Ok((Value::Bytes(b), &rest[4 + len..]))
        }
        other => Err(format!("value: tag {other}")),
    }
}

pub fn encode_header() -> [u8; 8] {
    let mut out = [0u8; 8];
    out[..4].copy_from_slice(&encode_u32_le(0x4C_44_42_31));
    out[4..].copy_from_slice(&encode_u32_le(FORMAT_VERSION));
    out
}

pub fn decode_header(bytes: &[u8; 8]) -> Result<u32, String> {
    let magic = decode_u32_le(bytes[..4].try_into().unwrap());
    if magic != 0x4C_44_42_31 {
        return Err(format!("magic mismatch: got {magic:#x}"));
    }
    Ok(decode_u32_le(bytes[4..].try_into().unwrap()))
}

#[cfg(test)]
mod tests_encoding {
    use super::*;

    #[test]
    fn value_roundtrip() {
        for v in [
            Value::Null,
            Value::Bool(true),
            Value::Bool(false),
            Value::Int(42),
            Value::Int(-1234567890),
            Value::Float(std::f64::consts::PI),
            Value::String("hola, mundo!".into()),
            Value::Bytes(vec![1, 2, 3, 4, 5]),
        ] {
            let enc = encode_value(&v);
            let (dec, rest) = decode_value(&enc).unwrap();
            assert_eq!(dec, v);
            assert!(rest.is_empty());
        }
    }

    #[test]
    fn string_roundtrip() {
        let s = "abcdefghij";
        let enc = encode_string(s);
        let (dec, rest) = decode_string(&enc).unwrap();
        assert_eq!(dec, s);
        assert!(rest.is_empty());
    }

    #[test]
    fn header_roundtrip() {
        let h = encode_header();
        let v = decode_header(&h).unwrap();
        assert_eq!(v, FORMAT_VERSION);
    }
}
