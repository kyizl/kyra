use crate::protocol::game_profile::read_uuid;
use crate::varint::{read_varint, VarIntError};
use bytes::Buf;

pub fn decode(buf: &mut impl Buf) -> Result<Vec<[u8; 16]>, VarIntError> {
    let count = read_varint(buf)?.max(0) as usize;
    let mut uuids = Vec::new();
    for _ in 0..count {
        uuids.push(read_uuid(buf)?);
    }
    Ok(uuids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::varint::write_varint;

    #[test]
    fn decodes_multiple_uuids() {
        let mut out = Vec::new();
        write_varint(&mut out, 2);
        out.extend([0xAAu8; 16]);
        out.extend([0xBBu8; 16]);

        let mut cursor = &out[..];
        let uuids = decode(&mut cursor).unwrap();
        assert_eq!(uuids, vec![[0xAAu8; 16], [0xBBu8; 16]]);
        assert!(cursor.is_empty());
    }

    #[test]
    fn decodes_empty_array() {
        let mut out = Vec::new();
        write_varint(&mut out, 0);
        let mut cursor = &out[..];
        assert_eq!(decode(&mut cursor).unwrap(), Vec::<[u8; 16]>::new());
    }
}
