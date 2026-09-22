use crate::chat::{parse_wire_string_component, TeamColorField};
use crate::nbt::{read_anonymous_root_simplified, NbtError};
use crate::protocol::brackets::TeamsBracket;
use crate::varint::{read_string, read_varint, skip_bytes, skip_string, VarIntError};
use bytes::Buf;
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TeamsDecodeError {
    #[error(transparent)]
    VarInt(#[from] VarIntError),
    #[error(transparent)]
    Nbt(#[from] NbtError),
    #[error("buffer ended before the team mode byte")]
    TruncatedMode,
}

#[derive(Debug, Clone)]
pub struct TeamMetadata {
    pub prefix: Value,
    pub color_field: TeamColorField,
}

#[derive(Debug, Clone)]
pub struct TeamsEvent {
    pub team_id: String,
    pub mode: i32,
    pub metadata: Option<TeamMetadata>,
    pub players: Option<Vec<String>>,
}

enum TextFieldType {
    WireString,
    AnonymousNbt,
}

enum EnumFieldType {
    WireString,
    Varint,
}

enum FlatField {
    Name(TextFieldType),
    Prefix(TextFieldType),
    Suffix(TextFieldType),
    FriendlyFire,
    NameTagVisibility(EnumFieldType),
    CollisionRule(EnumFieldType),
    Color,
    Formatting,
}

fn read_i8(buf: &mut impl Buf) -> Result<i8, TeamsDecodeError> {
    if !buf.has_remaining() {
        return Err(TeamsDecodeError::TruncatedMode);
    }
    Ok(buf.get_i8())
}

fn skip_text_field(buf: &mut impl Buf, kind: &TextFieldType) -> Result<(), TeamsDecodeError> {
    match kind {
        TextFieldType::WireString => skip_string(buf)?,
        TextFieldType::AnonymousNbt => {
            read_anonymous_root_simplified(buf)?;
        }
    }
    Ok(())
}

fn read_text_field(buf: &mut impl Buf, kind: &TextFieldType) -> Result<Value, TeamsDecodeError> {
    Ok(match kind {
        TextFieldType::WireString => parse_wire_string_component(&read_string(buf)?),
        TextFieldType::AnonymousNbt => read_anonymous_root_simplified(buf)?,
    })
}

fn skip_enum_field(buf: &mut impl Buf, kind: &EnumFieldType) -> Result<(), TeamsDecodeError> {
    match kind {
        EnumFieldType::WireString => skip_string(buf)?,
        EnumFieldType::Varint => {
            read_varint(buf)?;
        }
    }
    Ok(())
}

fn read_players(buf: &mut impl Buf, mode: i32) -> Result<Option<Vec<String>>, TeamsDecodeError> {
    if mode == 0 || mode == 3 || mode == 4 {
        let count = read_varint(buf)?.max(0) as usize;
        let mut players = Vec::new();
        for _ in 0..count {
            players.push(read_string(buf)?);
        }
        Ok(Some(players))
    } else {
        Ok(None)
    }
}

fn decode_flat(
    buf: &mut impl Buf,
    field_order: &[FlatField],
) -> Result<TeamsEvent, TeamsDecodeError> {
    let team_id = read_string(buf)?;
    let mode = i32::from(read_i8(buf)?);
    let has_metadata = mode == 0 || mode == 2;

    let mut prefix = Value::Null;
    let mut color_field = TeamColorField::None;

    if has_metadata {
        for field in field_order {
            match field {
                FlatField::Name(kind) => skip_text_field(buf, kind)?,
                FlatField::Prefix(kind) => prefix = read_text_field(buf, kind)?,
                FlatField::Suffix(kind) => skip_text_field(buf, kind)?,
                FlatField::FriendlyFire => skip_bytes(buf, 1)?,
                FlatField::NameTagVisibility(kind) => skip_enum_field(buf, kind)?,
                FlatField::CollisionRule(kind) => skip_enum_field(buf, kind)?,
                FlatField::Color => color_field = TeamColorField::Numeric(i32::from(read_i8(buf)?)),
                FlatField::Formatting => color_field = TeamColorField::Numeric(read_varint(buf)?),
            }
        }
    }

    let players = read_players(buf, mode)?;

    Ok(TeamsEvent {
        team_id,
        mode,
        metadata: has_metadata.then_some(TeamMetadata {
            prefix,
            color_field,
        }),
        players,
    })
}

fn decode_nested_container(buf: &mut impl Buf) -> Result<TeamsEvent, TeamsDecodeError> {
    let team_id = read_string(buf)?;
    let mode = i32::from(read_i8(buf)?);
    let has_metadata = mode == 0 || mode == 2;

    let mut prefix = Value::Null;
    let mut color_field = TeamColorField::None;

    if has_metadata {
        read_anonymous_root_simplified(buf)?;
        skip_bytes(buf, 1)?;
        read_varint(buf)?;
        read_varint(buf)?;
        let formatting = read_varint(buf)?;
        color_field = TeamColorField::Numeric(formatting);
        prefix = read_anonymous_root_simplified(buf)?;
        read_anonymous_root_simplified(buf)?;
    }

    let players = read_players(buf, mode)?;

    Ok(TeamsEvent {
        team_id,
        mode,
        metadata: has_metadata.then_some(TeamMetadata {
            prefix,
            color_field,
        }),
        players,
    })
}

pub fn decode(buf: &mut impl Buf, bracket: TeamsBracket) -> Result<TeamsEvent, TeamsDecodeError> {
    use EnumFieldType::{Varint, WireString};
    use FlatField::*;
    use TeamsBracket::*;
    use TextFieldType::{AnonymousNbt, WireString as Str};

    match bracket {
        Legacy8 => decode_flat(
            buf,
            &[
                Name(Str),
                Prefix(Str),
                Suffix(Str),
                FriendlyFire,
                NameTagVisibility(WireString),
                Color,
            ],
        ),
        Legacy9To12 => decode_flat(
            buf,
            &[
                Name(Str),
                Prefix(Str),
                Suffix(Str),
                FriendlyFire,
                NameTagVisibility(WireString),
                CollisionRule(WireString),
                Color,
            ],
        ),
        ModernString => decode_flat(
            buf,
            &[
                Name(Str),
                FriendlyFire,
                NameTagVisibility(WireString),
                CollisionRule(WireString),
                Formatting,
                Prefix(Str),
                Suffix(Str),
            ],
        ),
        ModernNbt => decode_flat(
            buf,
            &[
                Name(AnonymousNbt),
                FriendlyFire,
                NameTagVisibility(WireString),
                CollisionRule(WireString),
                Formatting,
                Prefix(AnonymousNbt),
                Suffix(AnonymousNbt),
            ],
        ),
        ModernNbtMappedEnums => decode_flat(
            buf,
            &[
                Name(AnonymousNbt),
                FriendlyFire,
                NameTagVisibility(Varint),
                CollisionRule(Varint),
                Formatting,
                Prefix(AnonymousNbt),
                Suffix(AnonymousNbt),
            ],
        ),
        NestedContainer => decode_nested_container(buf),
    }
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
    fn decodes_legacy8_add_team_with_color() {
        let mut out = Vec::new();
        write_string(&mut out, "team_red");
        out.push(0);
        write_string(&mut out, "Red Team");
        write_string(&mut out, "");
        write_string(&mut out, "");
        out.push(1);
        write_string(&mut out, "always");
        out.push(12);
        write_varint(&mut out, 1);
        write_string(&mut out, "Steve");

        let mut cursor = &out[..];
        let event = decode(&mut cursor, TeamsBracket::Legacy8).unwrap();
        assert_eq!(event.team_id, "team_red");
        assert_eq!(event.mode, 0);
        let metadata = event.metadata.unwrap();
        assert!(matches!(metadata.color_field, TeamColorField::Numeric(12)));
        assert_eq!(event.players, Some(vec!["Steve".to_string()]));
        assert!(cursor.is_empty());
    }

    #[test]
    fn decodes_legacy9to12_with_collision_rule_field() {
        let mut out = Vec::new();
        write_string(&mut out, "team_blue");
        out.push(2);
        write_string(&mut out, "Blue Team");
        write_string(&mut out, "");
        write_string(&mut out, "");
        out.push(0);
        write_string(&mut out, "always");
        write_string(&mut out, "always");
        out.push(9);

        let mut cursor = &out[..];
        let event = decode(&mut cursor, TeamsBracket::Legacy9To12).unwrap();
        assert_eq!(event.mode, 2);
        assert!(matches!(
            event.metadata.unwrap().color_field,
            TeamColorField::Numeric(9)
        ));
        assert_eq!(event.players, None);
        assert!(cursor.is_empty());
    }

    #[test]
    fn decodes_modern_string_with_reordered_fields_and_prefix_color() {
        let mut out = Vec::new();
        write_string(&mut out, "team_aqua");
        out.push(0);
        write_string(&mut out, "Aqua Team");
        out.push(0);
        write_string(&mut out, "always");
        write_string(&mut out, "always");
        write_varint(&mut out, 11);
        write_string(&mut out, "{\"text\":\"\",\"color\":\"red\"}");
        write_string(&mut out, "");
        write_varint(&mut out, 0);

        let mut cursor = &out[..];
        let event = decode(&mut cursor, TeamsBracket::ModernString).unwrap();
        let metadata = event.metadata.unwrap();
        assert_eq!(
            metadata.prefix,
            serde_json::json!({"text": "", "color": "red"})
        );
        assert!(matches!(metadata.color_field, TeamColorField::Numeric(11)));
        assert_eq!(event.players, Some(vec![]));
        assert!(cursor.is_empty());
    }

    #[test]
    fn decodes_modern_nbt_prefix() {
        let mut out = Vec::new();
        write_string(&mut out, "team_green");
        out.push(0);
        out.push(8);
        out.extend(0u16.to_be_bytes());
        out.push(0);
        write_string(&mut out, "always");
        write_string(&mut out, "always");
        write_varint(&mut out, 10);
        out.push(8);
        out.extend(3u16.to_be_bytes());
        out.extend(b"[G]");
        out.push(8);
        out.extend(0u16.to_be_bytes());
        write_varint(&mut out, 0);

        let mut cursor = &out[..];
        let event = decode(&mut cursor, TeamsBracket::ModernNbt).unwrap();
        let metadata = event.metadata.unwrap();
        assert_eq!(metadata.prefix, serde_json::json!("[G]"));
        assert!(cursor.is_empty());
    }

    #[test]
    fn decodes_modern_nbt_mapped_enums_with_varint_skips() {
        let mut out = Vec::new();
        write_string(&mut out, "team_pink");
        out.push(2);
        out.push(8);
        out.extend(0u16.to_be_bytes());
        out.push(0);
        write_varint(&mut out, 0);
        write_varint(&mut out, 1);
        write_varint(&mut out, 13);
        out.push(8);
        out.extend(0u16.to_be_bytes());
        out.push(8);
        out.extend(0u16.to_be_bytes());

        let mut cursor = &out[..];
        let event = decode(&mut cursor, TeamsBracket::ModernNbtMappedEnums).unwrap();
        assert!(matches!(
            event.metadata.unwrap().color_field,
            TeamColorField::Numeric(13)
        ));
        assert!(cursor.is_empty());
    }

    #[test]
    fn decodes_nested_container_add_mode() {
        let mut out = Vec::new();
        write_string(&mut out, "team_cyan");
        out.push(0);
        out.push(8);
        out.extend(0u16.to_be_bytes());
        out.push(0b0000_0011);
        write_varint(&mut out, 1);
        write_varint(&mut out, 0);
        write_varint(&mut out, 3);
        out.push(8);
        out.extend(3u16.to_be_bytes());
        out.extend(b"[C]");
        out.push(8);
        out.extend(0u16.to_be_bytes());
        write_varint(&mut out, 1);
        write_string(&mut out, "Alex");

        let mut cursor = &out[..];
        let event = decode(&mut cursor, TeamsBracket::NestedContainer).unwrap();
        assert_eq!(event.team_id, "team_cyan");
        assert_eq!(event.mode, 0);
        let metadata = event.metadata.unwrap();
        assert!(matches!(metadata.color_field, TeamColorField::Numeric(3)));
        assert_eq!(metadata.prefix, serde_json::json!("[C]"));
        assert_eq!(event.players, Some(vec!["Alex".to_string()]));
        assert!(cursor.is_empty());
    }

    #[test]
    fn decodes_remove_mode_with_no_metadata_and_no_players() {
        let mut out = Vec::new();
        write_string(&mut out, "team_old");
        out.push(1);

        let mut cursor = &out[..];
        let event = decode(&mut cursor, TeamsBracket::ModernString).unwrap();
        assert_eq!(event.mode, 1);
        assert!(event.metadata.is_none());
        assert!(event.players.is_none());
        assert!(cursor.is_empty());
    }

    #[test]
    fn decodes_leave_mode_with_players_but_no_metadata() {
        let mut out = Vec::new();
        write_string(&mut out, "team_x");
        out.push(4);
        write_varint(&mut out, 2);
        write_string(&mut out, "A");
        write_string(&mut out, "B");

        let mut cursor = &out[..];
        let event = decode(&mut cursor, TeamsBracket::ModernString).unwrap();
        assert_eq!(event.mode, 4);
        assert!(event.metadata.is_none());
        assert_eq!(event.players, Some(vec!["A".to_string(), "B".to_string()]));
        assert!(cursor.is_empty());
    }
}
