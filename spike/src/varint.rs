use bytes::{Buf, BufMut};
use thiserror::Error;

const SEGMENT_BITS: u8 = 0x7F;
const CONTINUE_BIT: u8 = 0x80;
const MAX_VARINT_BYTES: usize = 5;

#[derive(Debug, Error)]
pub enum VarIntError {
    #[error("VarInt is longer than {MAX_VARINT_BYTES} bytes")]
    TooLong,
    #[error("buffer ended before VarInt was complete")]
    Truncated,
}

pub fn try_read_varint(buf: &[u8]) -> Result<Option<(i32, usize)>, VarIntError> {
    let mut value: i32 = 0;
    let mut position = 0u32;

    for (index, &byte) in buf.iter().enumerate() {
        if index >= MAX_VARINT_BYTES {
            return Err(VarIntError::TooLong);
        }

        value |= i32::from(byte & SEGMENT_BITS) << position;

        if byte & CONTINUE_BIT == 0 {
            return Ok(Some((value, index + 1)));
        }

        position += 7;
    }

    Ok(None)
}

pub fn read_varint(buf: &mut impl Buf) -> Result<i32, VarIntError> {
    let mut value: i32 = 0;
    let mut position = 0u32;

    for index in 0..MAX_VARINT_BYTES {
        if !buf.has_remaining() {
            return Err(VarIntError::Truncated);
        }
        let byte = buf.get_u8();
        value |= i32::from(byte & SEGMENT_BITS) << position;

        if byte & CONTINUE_BIT == 0 {
            return Ok(value);
        }
        position += 7;
        let _ = index;
    }

    Err(VarIntError::TooLong)
}

pub fn write_varint(buf: &mut impl BufMut, mut value: i32) {
    loop {
        let mut byte = (value as u32 & u32::from(SEGMENT_BITS)) as u8;
        value = ((value as u32) >> 7) as i32;
        if value != 0 {
            byte |= CONTINUE_BIT;
        }
        buf.put_u8(byte);
        if value == 0 {
            break;
        }
    }
}

pub fn varint_len(mut value: i32) -> usize {
    let mut len = 0;
    loop {
        len += 1;
        value = ((value as u32) >> 7) as i32;
        if value == 0 {
            break;
        }
    }
    len
}

pub fn read_string(buf: &mut impl Buf) -> Result<String, VarIntError> {
    let len = read_varint(buf)? as usize;
    if buf.remaining() < len {
        return Err(VarIntError::Truncated);
    }
    let mut bytes = vec![0u8; len];
    buf.copy_to_slice(&mut bytes);
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

pub fn skip_string(buf: &mut impl Buf) -> Result<(), VarIntError> {
    let len = read_varint(buf)? as usize;
    skip_bytes(buf, len)
}

pub fn skip_bytes(buf: &mut impl Buf, len: usize) -> Result<(), VarIntError> {
    if buf.remaining() < len {
        return Err(VarIntError::Truncated);
    }
    buf.advance(len);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_known_values() {
        for &value in &[0i32, 1, 127, 128, 255, 25565, -1, i32::MIN, i32::MAX] {
            let mut buf = Vec::new();
            write_varint(&mut buf, value);
            assert_eq!(buf.len(), varint_len(value));
            let mut cursor = &buf[..];
            let decoded = read_varint(&mut cursor).unwrap();
            assert_eq!(decoded, value);
            assert!(cursor.is_empty());
        }
    }

    #[test]
    fn matches_wiki_vg_worked_examples() {
        let cases: &[(i32, &[u8])] = &[
            (0, &[0x00]),
            (1, &[0x01]),
            (2, &[0x02]),
            (127, &[0x7f]),
            (128, &[0x80, 0x01]),
            (255, &[0xff, 0x01]),
            (25565, &[0xdd, 0xc7, 0x01]),
            (2097151, &[0xff, 0xff, 0x7f]),
            (2147483647, &[0xff, 0xff, 0xff, 0xff, 0x07]),
            (-1, &[0xff, 0xff, 0xff, 0xff, 0x0f]),
            (i32::MIN, &[0x80, 0x80, 0x80, 0x80, 0x08]),
        ];
        for &(value, expected) in cases {
            let mut buf = Vec::new();
            write_varint(&mut buf, value);
            assert_eq!(buf, expected, "encoding mismatch for {value}");
            let mut cursor = expected;
            assert_eq!(read_varint(&mut cursor).unwrap(), value);
        }
    }

    #[test]
    fn try_read_reports_incomplete_buffer() {
        assert_eq!(try_read_varint(&[0x80]).unwrap(), None);
        assert_eq!(try_read_varint(&[0x80, 0x80]).unwrap(), None);
        assert_eq!(try_read_varint(&[0x80, 0x01]).unwrap(), Some((128, 2)));
    }

    #[test]
    fn rejects_overlong_varint() {
        let overlong = [0x80, 0x80, 0x80, 0x80, 0x80, 0x01];
        assert!(matches!(
            try_read_varint(&overlong),
            Err(VarIntError::TooLong)
        ));
    }
}
