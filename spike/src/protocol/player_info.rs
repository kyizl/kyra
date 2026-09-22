use crate::chat::parse_wire_string_component;
use crate::nbt::{read_anonymous_root_simplified, NbtError};
use crate::protocol::brackets::PlayerInfoBracket;
use crate::protocol::game_profile::{
    read_bool, read_game_profile, read_uuid, skip_chat_session_if_present, skip_crypto_if_present,
};
use crate::varint::{read_string, read_varint, VarIntError};
use bytes::Buf;
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PlayerInfoDecodeError {
    #[error(transparent)]
    VarInt(#[from] VarIntError),
    #[error(transparent)]
    Nbt(#[from] NbtError),
    #[error("unknown legacy player_info action id {0}")]
    UnknownLegacyAction(i32),
    #[error("buffer ended before the player_info bitflags byte")]
    TruncatedFlags,
}

#[derive(Debug, Clone, Default)]
pub struct PlayerInfoEntry {
    pub uuid: [u8; 16],
    pub added: bool,
    pub removed: bool,
    pub name: Option<String>,
    pub gamemode: Option<i32>,
    pub display_name: Option<Value>,
}

pub fn decode(
    buf: &mut impl Buf,
    bracket: PlayerInfoBracket,
) -> Result<Vec<PlayerInfoEntry>, PlayerInfoDecodeError> {
    match bracket {
        PlayerInfoBracket::LegacyAction => decode_legacy(buf, false),
        PlayerInfoBracket::LegacyActionWithCrypto => decode_legacy(buf, true),
        PlayerInfoBracket::BitflagsJsonName => decode_modern(buf, ModernShape::json_name()),
        PlayerInfoBracket::BitflagsNbtName => decode_modern(buf, ModernShape::nbt_name()),
        PlayerInfoBracket::BitflagsNbtNameListOrder => {
            decode_modern(buf, ModernShape::nbt_name_list_order())
        }
        PlayerInfoBracket::BitflagsNbtNameListOrderHat | PlayerInfoBracket::Latest => {
            decode_modern(buf, ModernShape::nbt_name_list_order_hat())
        }
    }
}

fn decode_legacy(
    buf: &mut impl Buf,
    has_crypto: bool,
) -> Result<Vec<PlayerInfoEntry>, PlayerInfoDecodeError> {
    let action = read_varint(buf)?;
    let count = read_varint(buf)?.max(0) as usize;
    let mut entries = Vec::new();

    for _ in 0..count {
        let uuid = read_uuid(buf)?;
        let mut entry = PlayerInfoEntry {
            uuid,
            ..Default::default()
        };

        match action {
            0 => {
                let profile = read_game_profile(buf)?;
                entry.name = Some(profile.name);
                entry.gamemode = Some(read_varint(buf)?);
                let _ping = read_varint(buf)?;
                if read_bool(buf)? {
                    let raw = read_string(buf)?;
                    entry.display_name = Some(parse_wire_string_component(&raw));
                }
                if has_crypto {
                    skip_crypto_if_present(buf)?;
                }
                entry.added = true;
            }
            1 => {
                entry.gamemode = Some(read_varint(buf)?);
            }
            2 => {
                let _ping = read_varint(buf)?;
            }
            3 => {
                if read_bool(buf)? {
                    let raw = read_string(buf)?;
                    entry.display_name = Some(parse_wire_string_component(&raw));
                }
            }
            4 => {
                entry.removed = true;
            }
            other => return Err(PlayerInfoDecodeError::UnknownLegacyAction(other)),
        }

        entries.push(entry);
    }

    Ok(entries)
}

struct ModernShape {
    display_name_is_nbt: bool,
    has_list_order: bool,
    has_hat: bool,
}

impl ModernShape {
    fn json_name() -> Self {
        Self {
            display_name_is_nbt: false,
            has_list_order: false,
            has_hat: false,
        }
    }

    fn nbt_name() -> Self {
        Self {
            display_name_is_nbt: true,
            has_list_order: false,
            has_hat: false,
        }
    }

    fn nbt_name_list_order() -> Self {
        Self {
            display_name_is_nbt: true,
            has_list_order: true,
            has_hat: false,
        }
    }

    fn nbt_name_list_order_hat() -> Self {
        Self {
            display_name_is_nbt: true,
            has_list_order: true,
            has_hat: true,
        }
    }
}

fn read_flags_byte(buf: &mut impl Buf) -> Result<u8, PlayerInfoDecodeError> {
    if !buf.has_remaining() {
        return Err(PlayerInfoDecodeError::TruncatedFlags);
    }
    Ok(buf.get_u8())
}

fn decode_modern(
    buf: &mut impl Buf,
    shape: ModernShape,
) -> Result<Vec<PlayerInfoEntry>, PlayerInfoDecodeError> {
    let count = read_varint(buf)?.max(0) as usize;
    let mut entries = Vec::new();

    for _ in 0..count {
        let uuid = read_uuid(buf)?;
        let flags = read_flags_byte(buf)?;

        let add_player = flags & 0x01 != 0;
        let initialize_chat = flags & 0x02 != 0;
        let update_game_mode = flags & 0x04 != 0;
        let update_listed = flags & 0x08 != 0;
        let update_latency = flags & 0x10 != 0;
        let update_display_name = flags & 0x20 != 0;
        let (update_hat, update_list_order) = match (shape.has_hat, shape.has_list_order) {
            (true, true) => (flags & 0x40 != 0, flags & 0x80 != 0),
            (false, true) => (false, flags & 0x40 != 0),
            _ => (false, false),
        };

        let mut entry = PlayerInfoEntry {
            uuid,
            ..Default::default()
        };

        if add_player {
            let profile = read_game_profile(buf)?;
            entry.name = Some(profile.name);
            entry.added = true;
        }

        skip_chat_session_if_present(buf, initialize_chat)?;

        if update_game_mode {
            entry.gamemode = Some(read_varint(buf)?);
        }

        if update_listed {
            let _listed = read_varint(buf)?;
        }

        if update_latency {
            let _latency = read_varint(buf)?;
        }

        if update_display_name && read_bool(buf)? {
            let value = if shape.display_name_is_nbt {
                read_anonymous_root_simplified(buf)?
            } else {
                let raw = read_string(buf)?;
                parse_wire_string_component(&raw)
            };
            entry.display_name = Some(value);
        }

        if update_list_order {
            let _list_priority = read_varint(buf)?;
        }

        if update_hat {
            let _show_hat = read_bool(buf)?;
        }

        entries.push(entry);
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::varint::write_varint;

    fn write_string(out: &mut Vec<u8>, s: &str) {
        write_varint(out, s.len() as i32);
        out.extend_from_slice(s.as_bytes());
    }

    fn sample_uuid(byte: u8) -> [u8; 16] {
        [byte; 16]
    }

    #[test]
    fn decodes_legacy_add_player() {
        let mut out = Vec::new();
        write_varint(&mut out, 0);
        write_varint(&mut out, 1);
        out.extend(sample_uuid(0xAA));
        write_string(&mut out, "Notch");
        write_varint(&mut out, 0);
        write_varint(&mut out, 5);
        write_varint(&mut out, 0);
        out.push(0);

        let mut cursor = &out[..];
        let entries = decode(&mut cursor, PlayerInfoBracket::LegacyAction).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].added);
        assert_eq!(entries[0].name.as_deref(), Some("Notch"));
        assert_eq!(entries[0].gamemode, Some(5));
        assert!(cursor.is_empty());
    }

    #[test]
    fn decodes_legacy_remove_player() {
        let mut out = Vec::new();
        write_varint(&mut out, 4);
        write_varint(&mut out, 1);
        out.extend(sample_uuid(0xBB));

        let mut cursor = &out[..];
        let entries = decode(&mut cursor, PlayerInfoBracket::LegacyAction).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].removed);
        assert!(cursor.is_empty());
    }

    #[test]
    fn decodes_legacy_with_crypto_add_player_and_consumes_crypto_field() {
        let mut out = Vec::new();
        write_varint(&mut out, 0);
        write_varint(&mut out, 1);
        out.extend(sample_uuid(0xCC));
        write_string(&mut out, "Steve");
        write_varint(&mut out, 0);
        write_varint(&mut out, 0);
        write_varint(&mut out, 3);
        out.push(0);
        out.push(1);
        out.extend([0u8; 8]);
        write_varint(&mut out, 2);
        out.extend([1, 2]);
        write_varint(&mut out, 1);
        out.extend([3]);

        let mut cursor = &out[..];
        let entries = decode(&mut cursor, PlayerInfoBracket::LegacyActionWithCrypto).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name.as_deref(), Some("Steve"));
        assert!(cursor.is_empty());
    }

    fn build_modern_add_entry(uuid: [u8; 16], name: &str, gamemode: i32) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(uuid);
        out.push(0b0000_0101);
        write_string(&mut out, name);
        write_varint(&mut out, 0);
        write_varint(&mut out, gamemode);
        out
    }

    #[test]
    fn decodes_bitflags_json_name_add_player() {
        let mut out = Vec::new();
        write_varint(&mut out, 1);
        out.extend(build_modern_add_entry(sample_uuid(0x11), "Alex", 2));

        let mut cursor = &out[..];
        let entries = decode(&mut cursor, PlayerInfoBracket::BitflagsJsonName).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].added);
        assert_eq!(entries[0].name.as_deref(), Some("Alex"));
        assert_eq!(entries[0].gamemode, Some(2));
        assert!(cursor.is_empty());
    }

    #[test]
    fn decodes_bitflags_nbt_display_name() {
        let mut out = Vec::new();
        write_varint(&mut out, 1);
        out.extend(sample_uuid(0x22));
        out.push(0b0010_0001);
        write_string(&mut out, "Herobrine");
        write_varint(&mut out, 0);
        out.push(1);
        out.push(8);
        out.extend(4u16.to_be_bytes());
        out.extend(b"Ghos");

        let mut cursor = &out[..];
        let entries = decode(&mut cursor, PlayerInfoBracket::BitflagsNbtName).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].added);
        assert_eq!(entries[0].display_name, Some(serde_json::json!("Ghos")));
        assert!(cursor.is_empty());
    }

    #[test]
    fn decodes_list_order_bracket_and_consumes_trailing_field() {
        let mut out = Vec::new();
        write_varint(&mut out, 1);
        out.extend(sample_uuid(0x33));
        out.push(0b0100_0001);
        write_string(&mut out, "Player1");
        write_varint(&mut out, 0);
        write_varint(&mut out, 42);

        let mut cursor = &out[..];
        let entries = decode(&mut cursor, PlayerInfoBracket::BitflagsNbtNameListOrder).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].added);
        assert!(cursor.is_empty());
    }

    #[test]
    fn decodes_list_order_and_hat_bracket_with_correct_field_order() {
        let mut out = Vec::new();
        write_varint(&mut out, 1);
        out.extend(sample_uuid(0x44));
        out.push(0b1100_0001);
        write_string(&mut out, "Player2");
        write_varint(&mut out, 0);
        write_varint(&mut out, 7);
        out.push(1);

        let mut cursor = &out[..];
        let entries = decode(&mut cursor, PlayerInfoBracket::BitflagsNbtNameListOrderHat).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].added);
        assert!(cursor.is_empty());
    }

    #[test]
    fn decodes_multiple_entries_in_one_packet() {
        let mut out = Vec::new();
        write_varint(&mut out, 2);
        out.extend(build_modern_add_entry(sample_uuid(0x01), "One", 0));
        out.extend(build_modern_add_entry(sample_uuid(0x02), "Two", 0));

        let mut cursor = &out[..];
        let entries = decode(&mut cursor, PlayerInfoBracket::BitflagsJsonName).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name.as_deref(), Some("One"));
        assert_eq!(entries[1].name.as_deref(), Some("Two"));
        assert!(cursor.is_empty());
    }
}
