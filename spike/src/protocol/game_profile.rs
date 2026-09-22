use crate::varint::{read_string, read_varint, skip_bytes, skip_string, VarIntError};
use bytes::Buf;

pub fn read_uuid(buf: &mut impl Buf) -> Result<[u8; 16], VarIntError> {
    if buf.remaining() < 16 {
        return Err(VarIntError::Truncated);
    }
    let mut uuid = [0u8; 16];
    buf.copy_to_slice(&mut uuid);
    Ok(uuid)
}

pub fn read_bool(buf: &mut impl Buf) -> Result<bool, VarIntError> {
    if !buf.has_remaining() {
        return Err(VarIntError::Truncated);
    }
    Ok(buf.get_u8() != 0)
}

pub struct GameProfile {
    pub name: String,
}

pub fn read_game_profile(buf: &mut impl Buf) -> Result<GameProfile, VarIntError> {
    let name = read_string(buf)?;
    let property_count = read_varint(buf)?.max(0) as usize;
    for _ in 0..property_count {
        skip_string(buf)?;
        skip_string(buf)?;
        if read_bool(buf)? {
            skip_string(buf)?;
        }
    }
    Ok(GameProfile { name })
}

pub fn skip_chat_session_if_present(buf: &mut impl Buf, flag_set: bool) -> Result<(), VarIntError> {
    if !flag_set {
        return Ok(());
    }
    if read_bool(buf)? {
        skip_bytes(buf, 16)?;
        skip_bytes(buf, 8)?;
        let key_len = read_varint(buf)?.max(0) as usize;
        skip_bytes(buf, key_len)?;
        let sig_len = read_varint(buf)?.max(0) as usize;
        skip_bytes(buf, sig_len)?;
    }
    Ok(())
}

pub fn skip_crypto_if_present(buf: &mut impl Buf) -> Result<(), VarIntError> {
    if read_bool(buf)? {
        skip_bytes(buf, 8)?;
        let key_len = read_varint(buf)?.max(0) as usize;
        skip_bytes(buf, key_len)?;
        let sig_len = read_varint(buf)?.max(0) as usize;
        skip_bytes(buf, sig_len)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::varint::write_varint;

    fn write_string(out: &mut Vec<u8>, s: &str) {
        write_varint(out, s.len() as i32);
        out.extend_from_slice(s.as_bytes());
    }

    #[test]
    fn reads_uuid_as_raw_sixteen_bytes() {
        let bytes: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let mut cursor = &bytes[..];
        assert_eq!(read_uuid(&mut cursor).unwrap(), bytes);
        assert!(cursor.is_empty());
    }

    #[test]
    fn reads_game_profile_with_properties() {
        let mut out = Vec::new();
        write_string(&mut out, "Notch");
        write_varint(&mut out, 2);
        write_string(&mut out, "textures");
        write_string(&mut out, "base64value");
        out.push(1);
        write_string(&mut out, "sig");
        write_string(&mut out, "another_prop");
        write_string(&mut out, "value");
        out.push(0);

        let mut cursor = &out[..];
        let profile = read_game_profile(&mut cursor).unwrap();
        assert_eq!(profile.name, "Notch");
        assert!(cursor.is_empty());
    }

    #[test]
    fn skips_absent_chat_session_when_flag_not_set() {
        let mut cursor = &[][..];
        skip_chat_session_if_present(&mut cursor, false).unwrap();
    }

    #[test]
    fn skips_present_chat_session_correctly() {
        let mut out = Vec::new();
        out.push(1);
        out.extend([0u8; 16]);
        out.extend([0u8; 8]);
        write_varint(&mut out, 3);
        out.extend([9, 9, 9]);
        write_varint(&mut out, 2);
        out.extend([8, 8]);
        out.push(0xFF);

        let mut cursor = &out[..];
        skip_chat_session_if_present(&mut cursor, true).unwrap();
        assert_eq!(cursor.remaining(), 1);
    }
}
