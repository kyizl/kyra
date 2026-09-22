use crate::varint::write_varint;
use uuid::Uuid;

pub fn handshake(protocol_version: i32, server_address: &str, server_port: u16) -> Vec<u8> {
    handshake_with_next_state(protocol_version, server_address, server_port, 2)
}

pub fn handshake_with_next_state(
    protocol_version: i32,
    server_address: &str,
    server_port: u16,
    next_state: i32,
) -> Vec<u8> {
    let mut out = Vec::new();
    write_varint(&mut out, 0x00);
    write_varint(&mut out, protocol_version);
    write_varint(&mut out, server_address.len() as i32);
    out.extend_from_slice(server_address.as_bytes());
    out.extend(server_port.to_be_bytes());
    write_varint(&mut out, next_state);
    out
}

pub fn offline_player_uuid(username: &str) -> Uuid {
    Uuid::new_v3(
        &Uuid::NAMESPACE_DNS,
        format!("OfflinePlayer:{username}").as_bytes(),
    )
}

fn write_string(out: &mut Vec<u8>, s: &str) {
    write_varint(out, s.len() as i32);
    out.extend_from_slice(s.as_bytes());
}

pub fn login_start(protocol_version: i32, username: &str) -> Vec<u8> {
    let mut out = Vec::new();
    write_varint(&mut out, 0x00);
    write_string(&mut out, username);

    let uuid = offline_player_uuid(username);
    let uuid_bytes = *uuid.as_bytes();

    match protocol_version {
        i32::MIN..=758 => {}
        759 => {
            out.push(0);
        }
        760 => {
            out.push(0);
            out.push(0);
        }
        761..=763 => {
            out.push(1);
            out.extend(uuid_bytes);
        }
        _ => {
            out.extend(uuid_bytes);
        }
    }

    out
}

pub fn login_acknowledged(packet_id: i32) -> Vec<u8> {
    let mut out = Vec::new();
    write_varint(&mut out, packet_id);
    out
}

pub fn configuration_finish_acknowledge(packet_id: i32) -> Vec<u8> {
    let mut out = Vec::new();
    write_varint(&mut out, packet_id);
    out
}

pub fn keep_alive_echo(packet_id: i32, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    write_varint(&mut out, packet_id);
    out.extend_from_slice(payload);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::varint::read_varint;

    #[test]
    fn handshake_encodes_expected_fields() {
        let packet = handshake(765, "play.example.com", 25565);
        let mut cursor = &packet[..];
        assert_eq!(read_varint(&mut cursor).unwrap(), 0x00);
        assert_eq!(read_varint(&mut cursor).unwrap(), 765);
        let addr_len = read_varint(&mut cursor).unwrap() as usize;
        let (addr_bytes, rest) = cursor.split_at(addr_len);
        assert_eq!(addr_bytes, b"play.example.com");
        let (port_bytes, rest) = rest.split_at(2);
        assert_eq!(u16::from_be_bytes([port_bytes[0], port_bytes[1]]), 25565);
        let mut rest_cursor = rest;
        assert_eq!(read_varint(&mut rest_cursor).unwrap(), 2);
        assert!(rest_cursor.is_empty());
    }

    #[test]
    fn offline_uuid_is_deterministic_for_same_username() {
        let a = offline_player_uuid("Notch");
        let b = offline_player_uuid("Notch");
        assert_eq!(a, b);
        let c = offline_player_uuid("Herobrine");
        assert_ne!(a, c);
    }

    #[test]
    fn login_start_omits_uuid_field_pre_1_19() {
        let packet = login_start(758, "Steve");
        let mut cursor = &packet[..];
        assert_eq!(read_varint(&mut cursor).unwrap(), 0x00);
        let name_len = read_varint(&mut cursor).unwrap() as usize;
        let (name_bytes, rest) = cursor.split_at(name_len);
        assert_eq!(name_bytes, b"Steve");
        assert!(rest.is_empty());
    }

    #[test]
    fn login_start_includes_mandatory_uuid_at_1_20_2_plus() {
        let packet = login_start(765, "Steve");
        let mut cursor = &packet[..];
        assert_eq!(read_varint(&mut cursor).unwrap(), 0x00);
        let name_len = read_varint(&mut cursor).unwrap() as usize;
        let (_name, rest) = cursor.split_at(name_len);
        assert_eq!(rest.len(), 16);
    }

    #[test]
    fn keep_alive_echo_reflects_payload_verbatim() {
        let payload = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let packet = keep_alive_echo(0x12, &payload);
        let mut cursor = &packet[..];
        assert_eq!(read_varint(&mut cursor).unwrap(), 0x12);
        assert_eq!(cursor, &payload);
    }
}
