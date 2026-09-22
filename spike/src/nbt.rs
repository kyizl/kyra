use bytes::Buf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NbtError {
    #[error("buffer ended before NBT value was complete")]
    Truncated,
    #[error("unknown NBT tag id {0}")]
    UnknownTag(u8),
    #[error("NBT nesting depth exceeded {0}")]
    TooDeep(u32),
}

const MAX_DEPTH: u32 = 64;

#[derive(Debug, Clone, PartialEq)]
pub enum NbtValue {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List(Vec<NbtValue>),
    Compound(Vec<(String, NbtValue)>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

fn require(buf: &impl Buf, n: usize) -> Result<(), NbtError> {
    if buf.remaining() < n {
        Err(NbtError::Truncated)
    } else {
        Ok(())
    }
}

fn read_nbt_string(buf: &mut impl Buf) -> Result<String, NbtError> {
    require(buf, 2)?;
    let len = buf.get_u16() as usize;
    require(buf, len)?;
    let mut raw = vec![0u8; len];
    buf.copy_to_slice(&mut raw);
    Ok(String::from_utf8_lossy(&raw).into_owned())
}

fn read_payload(buf: &mut impl Buf, tag_id: u8, depth: u32) -> Result<NbtValue, NbtError> {
    if depth > MAX_DEPTH {
        return Err(NbtError::TooDeep(MAX_DEPTH));
    }

    match tag_id {
        1 => {
            require(buf, 1)?;
            Ok(NbtValue::Byte(buf.get_i8()))
        }
        2 => {
            require(buf, 2)?;
            Ok(NbtValue::Short(buf.get_i16()))
        }
        3 => {
            require(buf, 4)?;
            Ok(NbtValue::Int(buf.get_i32()))
        }
        4 => {
            require(buf, 8)?;
            Ok(NbtValue::Long(buf.get_i64()))
        }
        5 => {
            require(buf, 4)?;
            Ok(NbtValue::Float(buf.get_f32()))
        }
        6 => {
            require(buf, 8)?;
            Ok(NbtValue::Double(buf.get_f64()))
        }
        7 => {
            require(buf, 4)?;
            let len = buf.get_i32().max(0) as usize;
            require(buf, len)?;
            let mut out = Vec::with_capacity(len);
            for _ in 0..len {
                out.push(buf.get_i8());
            }
            Ok(NbtValue::ByteArray(out))
        }
        8 => Ok(NbtValue::String(read_nbt_string(buf)?)),
        9 => {
            require(buf, 5)?;
            let elem_tag = buf.get_u8();
            let claimed_len = buf.get_i32().max(0) as usize;
            let mut out = Vec::new();
            for _ in 0..claimed_len {
                if elem_tag == 0 {
                    break;
                }
                out.push(read_payload(buf, elem_tag, depth + 1)?);
            }
            Ok(NbtValue::List(out))
        }
        10 => {
            let mut fields = Vec::new();
            loop {
                require(buf, 1)?;
                let child_tag = buf.get_u8();
                if child_tag == 0 {
                    break;
                }
                let name = read_nbt_string(buf)?;
                let value = read_payload(buf, child_tag, depth + 1)?;
                fields.push((name, value));
            }
            Ok(NbtValue::Compound(fields))
        }
        11 => {
            require(buf, 4)?;
            let len = buf.get_i32().max(0) as usize;
            require(buf, len.saturating_mul(4))?;
            let mut out = Vec::with_capacity(len);
            for _ in 0..len {
                out.push(buf.get_i32());
            }
            Ok(NbtValue::IntArray(out))
        }
        12 => {
            require(buf, 4)?;
            let len = buf.get_i32().max(0) as usize;
            require(buf, len.saturating_mul(8))?;
            let mut out = Vec::with_capacity(len);
            for _ in 0..len {
                out.push(buf.get_i64());
            }
            Ok(NbtValue::LongArray(out))
        }
        other => Err(NbtError::UnknownTag(other)),
    }
}

pub fn read_anonymous_root(buf: &mut impl Buf) -> Result<NbtValue, NbtError> {
    require(buf, 1)?;
    let tag_id = buf.get_u8();
    if tag_id == 0 {
        return Ok(NbtValue::Compound(Vec::new()));
    }
    read_payload(buf, tag_id, 0)
}

pub fn simplify(value: &NbtValue) -> serde_json::Value {
    match value {
        NbtValue::Byte(b) => serde_json::Value::from(*b),
        NbtValue::Short(s) => serde_json::Value::from(*s),
        NbtValue::Int(i) => serde_json::Value::from(*i),
        NbtValue::Long(l) => serde_json::Value::from(*l),
        NbtValue::Float(f) => serde_json::Number::from_f64(f64::from(*f))
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        NbtValue::Double(d) => serde_json::Number::from_f64(*d)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        NbtValue::ByteArray(arr) => {
            serde_json::Value::Array(arr.iter().map(|b| serde_json::Value::from(*b)).collect())
        }
        NbtValue::String(s) => serde_json::Value::String(s.clone()),
        NbtValue::List(items) => serde_json::Value::Array(items.iter().map(simplify).collect()),
        NbtValue::Compound(fields) => {
            let map = fields
                .iter()
                .map(|(k, v)| (k.clone(), simplify(v)))
                .collect();
            serde_json::Value::Object(map)
        }
        NbtValue::IntArray(arr) => {
            serde_json::Value::Array(arr.iter().map(|i| serde_json::Value::from(*i)).collect())
        }
        NbtValue::LongArray(arr) => {
            serde_json::Value::Array(arr.iter().map(|l| serde_json::Value::from(*l)).collect())
        }
    }
}

pub fn read_anonymous_root_simplified(buf: &mut impl Buf) -> Result<serde_json::Value, NbtError> {
    let raw = read_anonymous_root(buf)?;
    Ok(simplify(&raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TAG_END: u8 = 0;
    const TAG_STRING: u8 = 8;
    const TAG_LIST: u8 = 9;
    const TAG_COMPOUND: u8 = 10;

    fn tag_string(name: &str, value: &str) -> Vec<u8> {
        let mut out = vec![TAG_STRING];
        out.extend((name.len() as u16).to_be_bytes());
        out.extend(name.as_bytes());
        out.extend((value.len() as u16).to_be_bytes());
        out.extend(value.as_bytes());
        out
    }

    #[test]
    fn reads_anonymous_string_component() {
        let mut bytes = vec![TAG_STRING];
        bytes.extend(5u16.to_be_bytes());
        bytes.extend(b"hello");

        let mut cursor = &bytes[..];
        let value = read_anonymous_root(&mut cursor).unwrap();
        assert_eq!(value, NbtValue::String("hello".to_string()));
        assert!(cursor.is_empty());
    }

    #[test]
    fn reads_anonymous_compound_with_text_and_extra() {
        let mut extra_entry_fields = Vec::new();
        extra_entry_fields.extend(tag_string("text", "Foo"));
        extra_entry_fields.extend(tag_string("color", "red"));
        extra_entry_fields.push(TAG_END);

        let mut extra_list_payload = vec![TAG_COMPOUND];
        extra_list_payload.extend(1i32.to_be_bytes());
        extra_list_payload.extend(extra_entry_fields);

        let mut root_compound_fields = vec![TAG_COMPOUND];
        root_compound_fields.extend(tag_string("text", ""));
        root_compound_fields.push(TAG_LIST);
        root_compound_fields.extend(5u16.to_be_bytes());
        root_compound_fields.extend(b"extra");
        root_compound_fields.extend(extra_list_payload);
        root_compound_fields.push(TAG_END);

        let mut cursor = &root_compound_fields[..];
        let value = read_anonymous_root(&mut cursor).unwrap();
        let simplified = simplify(&value);

        assert_eq!(simplified["text"], serde_json::json!(""));
        assert_eq!(simplified["extra"][0]["text"], serde_json::json!("Foo"));
        assert_eq!(simplified["extra"][0]["color"], serde_json::json!("red"));
        assert!(cursor.is_empty());
    }

    #[test]
    fn rejects_unknown_tag_id() {
        let bytes = [200u8];
        let mut cursor = &bytes[..];
        assert!(matches!(
            read_anonymous_root(&mut cursor),
            Err(NbtError::UnknownTag(200))
        ));
    }

    #[test]
    fn truncated_buffer_errors_instead_of_panicking() {
        let bytes = [8u8, 0, 5, b'h', b'i'];
        let mut cursor = &bytes[..];
        assert!(matches!(
            read_anonymous_root(&mut cursor),
            Err(NbtError::Truncated)
        ));
    }
}
