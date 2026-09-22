use crate::crypto::{DecryptStream, EncryptStream};
use crate::framing::PacketReader;
use crate::protocol::brackets::{player_info_bracket, teams_bracket, teams_packet_name};
use crate::protocol::packet_ids;
use crate::protocol::{outbound, player_info, player_remove, teams};
use crate::varint::{read_varint, write_varint};
use bytes::{Buf, Bytes};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{info, warn};

#[derive(Debug, Error)]
pub enum ConnectionError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("protocol version {0} is not in the supported packet-id table")]
    UnsupportedProtocolVersion(i32),
    #[error("server closed the connection before login completed")]
    ClosedDuringLogin,
    #[error("unexpected packet id {packet_id} in {state} state")]
    UnexpectedPacket { state: &'static str, packet_id: i32 },
    #[error("malformed status response")]
    MalformedStatusResponse,
    #[error(transparent)]
    VarInt(#[from] crate::varint::VarIntError),
    #[error(transparent)]
    Framing(#[from] crate::framing::FramingError),
}

pub struct ConnectionConfig {
    pub host: String,
    pub server_address: Option<String>,
    pub port: u16,
    pub username: String,
    pub protocol_version: i32,
}

pub async fn status(config: &ConnectionConfig) -> Result<(), ConnectionError> {
    let stream = TcpStream::connect((config.host.as_str(), config.port)).await?;
    stream.set_nodelay(true)?;
    let mut conn = Connection {
        stream,
        reader: PacketReader::new(),
        scratch: [0u8; 4096],
        encrypt: None,
        decrypt: None,
        protocol_version: config.protocol_version,
    };

    let server_address = config.server_address.as_deref().unwrap_or(&config.host);
    conn.write_packet(&outbound::handshake_with_next_state(
        config.protocol_version,
        server_address,
        config.port,
        1,
    ))
    .await?;
    conn.write_packet(&[0x00]).await?;

    let mut response = conn.read_frame().await?;
    let response_id = read_varint(&mut response)?;
    if response_id != 0 {
        return Err(ConnectionError::UnexpectedPacket {
            state: "status",
            packet_id: response_id,
        });
    }
    let json_length = read_varint(&mut response)?;
    if json_length < 0 || response.len() != json_length as usize {
        return Err(ConnectionError::MalformedStatusResponse);
    }
    let json = String::from_utf8_lossy(&response).into_owned();
    info!(
        host = %config.host,
        protocol_version = config.protocol_version,
        response = %json,
        "live status probe succeeded"
    );

    conn.write_packet(&[0x01, 0, 0, 0, 0, 0, 0, 0, 0]).await?;
    let mut pong = conn.read_frame().await?;
    let pong_id = read_varint(&mut pong)?;
    if pong_id != 1 || pong.len() != 8 {
        return Err(ConnectionError::UnexpectedPacket {
            state: "status-pong",
            packet_id: pong_id,
        });
    }
    Ok(())
}

enum State {
    Login,
    Configuration,
    Play,
}

pub struct Connection {
    stream: TcpStream,
    reader: PacketReader,
    scratch: [u8; 4096],
    encrypt: Option<EncryptStream>,
    decrypt: Option<DecryptStream>,
    protocol_version: i32,
}

impl Connection {
    async fn read_frame(&mut self) -> Result<Bytes, ConnectionError> {
        loop {
            if let Some(packet) = self.reader.next_packet()? {
                return Ok(packet);
            }
            let n = self.stream.read(&mut self.scratch).await?;
            if n == 0 {
                return Err(ConnectionError::ClosedDuringLogin);
            }
            let mut chunk = self.scratch[..n].to_vec();
            if let Some(decrypt) = self.decrypt.as_mut() {
                decrypt.apply_keystream(&mut chunk);
            }
            self.reader.feed(&chunk);
        }
    }

    async fn write_packet(&mut self, payload: &[u8]) -> Result<(), ConnectionError> {
        let len = payload.len() as i32;
        let mut framed = Vec::with_capacity(crate::varint::varint_len(len) + payload.len());
        write_varint(&mut framed, len);
        framed.extend_from_slice(payload);
        if let Some(encrypt) = self.encrypt.as_mut() {
            encrypt.apply_keystream(&mut framed);
        }
        self.stream.write_all(&framed).await?;
        Ok(())
    }
}

pub async fn run(config: ConnectionConfig) -> Result<(), ConnectionError> {
    let ids = packet_ids::lookup(config.protocol_version).ok_or(
        ConnectionError::UnsupportedProtocolVersion(config.protocol_version),
    )?;

    let stream = TcpStream::connect((config.host.as_str(), config.port)).await?;
    stream.set_nodelay(true)?;

    let mut conn = Connection {
        stream,
        reader: PacketReader::new(),
        scratch: [0u8; 4096],
        encrypt: None,
        decrypt: None,
        protocol_version: config.protocol_version,
    };

    let server_address = config.server_address.as_deref().unwrap_or(&config.host);
    conn.write_packet(&outbound::handshake(
        config.protocol_version,
        server_address,
        config.port,
    ))
    .await?;
    conn.write_packet(&outbound::login_start(
        config.protocol_version,
        &config.username,
    ))
    .await?;

    let mut state = State::Login;

    loop {
        let mut payload = conn.read_frame().await?;
        let packet_id = read_varint(&mut payload)?;

        match state {
            State::Login => {
                if packet_id == ids.login_disconnect {
                    warn!(
                        message = %String::from_utf8_lossy(&payload),
                        "server sent login disconnect"
                    );
                    return Ok(());
                } else if packet_id == ids.set_compression {
                    let threshold = read_varint(&mut payload)?;
                    conn.reader.set_compression(Some(threshold));
                } else if packet_id == ids.encryption_request {
                    warn!(
                        "server requested premium/online-mode auth (encryption_request); \
                         this diagnostic build only supports offline-mode connections"
                    );
                    return Ok(());
                } else if packet_id == ids.login_success {
                    info!("login_success received");
                    if let Some(ack_id) = ids.login_acknowledged {
                        conn.write_packet(&outbound::login_acknowledged(ack_id))
                            .await?;
                        state = State::Configuration;
                    } else {
                        state = State::Play;
                    }
                }
            }
            State::Configuration => {
                if Some(packet_id) == ids.configuration_disconnect {
                    warn!(
                        message = %String::from_utf8_lossy(&payload),
                        "server sent configuration disconnect"
                    );
                    return Ok(());
                } else if Some(packet_id) == ids.configuration_finish_client {
                    if let Some(server_id) = ids.configuration_finish_server {
                        conn.write_packet(&outbound::configuration_finish_acknowledge(server_id))
                            .await?;
                    }
                    state = State::Play;
                    info!("entered play state");
                }
            }
            State::Play => {
                if packet_id == ids.play_disconnect {
                    warn!(
                        message = %String::from_utf8_lossy(&payload),
                        "server sent play disconnect"
                    );
                    return Ok(());
                } else if packet_id == ids.play_keep_alive_client {
                    let echo =
                        outbound::keep_alive_echo(ids.play_keep_alive_server, payload.chunk());
                    conn.write_packet(&echo).await?;
                } else if packet_id == ids.player_info {
                    match player_info_bracket(conn.protocol_version) {
                        Some(bracket) => match player_info::decode(&mut payload, bracket) {
                            Ok(entries) => {
                                for entry in &entries {
                                    let npc = crate::chat::is_npc(
                                        entry.gamemode,
                                        entry.display_name.as_ref(),
                                    );
                                    let plain_display_name = entry
                                        .display_name
                                        .as_ref()
                                        .map(crate::chat::parse_chat_to_plain)
                                        .unwrap_or_default();
                                    info!(
                                        name = entry.name.as_deref().unwrap_or(""),
                                        display_name = plain_display_name,
                                        added = entry.added,
                                        removed = entry.removed,
                                        npc,
                                        "player_info entry"
                                    );
                                }
                            }
                            Err(err) => warn!(%err, "player_info decode failed"),
                        },
                        None => warn!(
                            protocol_version = conn.protocol_version,
                            "no player_info bracket for this protocol version"
                        ),
                    }
                } else if Some(packet_id) == ids.player_remove {
                    match player_remove::decode(&mut payload) {
                        Ok(uuids) => info!(count = uuids.len(), "player_remove decoded"),
                        Err(err) => warn!(%err, "player_remove decode failed"),
                    }
                } else if packet_id == ids.teams {
                    match teams_bracket(conn.protocol_version) {
                        Some(bracket) => match teams::decode(&mut payload, bracket) {
                            Ok(event) => {
                                let is_game_team = crate::chat::is_game_team(&event.team_id);
                                let resolved_color = event.metadata.as_ref().map(|metadata| {
                                    crate::chat::extract_team_color(
                                        &metadata.prefix,
                                        &metadata.color_field,
                                        &event.team_id,
                                    )
                                });
                                let team_name = crate::chat::format_team_name(&event.team_id);
                                info!(
                                    team = event.team_id,
                                    mode = event.mode,
                                    is_game_team,
                                    team_name,
                                    color = resolved_color.as_deref().unwrap_or(""),
                                    packet_name = teams_packet_name(conn.protocol_version),
                                    "teams decoded"
                                );
                            }
                            Err(err) => warn!(%err, "teams decode failed"),
                        },
                        None => warn!(
                            protocol_version = conn.protocol_version,
                            "no teams bracket for this protocol version"
                        ),
                    }
                }
            }
        }
    }
}
