use crate::{Observer, ObserverError, ObserverEvent};
use kyra_proto_spike::chat;
use kyra_proto_spike::protocol::brackets::{player_info_bracket, teams_bracket};
use kyra_proto_spike::protocol::{packet_ids, player_info, player_remove, teams};
use kyra_proto_spike::varint::read_varint;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio::sync::mpsc;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DecoderObserverError {
    #[error("unsupported protocol version {0}")]
    UnsupportedProtocol(i32),
    #[error("decoder observer capacity must be greater than zero")]
    InvalidCapacity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecoderEvent {
    Unknown {
        packet_id: i32,
        packet: ObserverEvent,
    },
    PlayerInfo {
        packet: ObserverEvent,
        entries: usize,
        added_names: Vec<String>,
        removed_names: Vec<String>,
    },
    PlayerRemove {
        packet: ObserverEvent,
        uuids: usize,
        names: Vec<String>,
    },
    Teams {
        packet: ObserverEvent,
        teams: Vec<DecoderTeam>,
    },
    DecodeFailure {
        packet: ObserverEvent,
        error: String,
    },
    Malformed {
        packet: ObserverEvent,
        error: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecoderTeam {
    pub name: String,
    pub display_name: String,
    pub color: String,
    pub players: Vec<String>,
}

impl DecoderTeam {
    pub fn is_game_team(&self) -> bool {
        chat::is_game_team(&self.name)
    }
}

pub struct DecoderObserver {
    protocol_version: i32,
    sender: mpsc::Sender<DecoderEvent>,
    players: Arc<Mutex<HashMap<[u8; 16], String>>>,
    teams: Arc<Mutex<HashMap<String, DecoderTeam>>>,
    lobby_mode: Arc<Mutex<bool>>,
    last_emitted_teams: Arc<Mutex<Vec<DecoderTeam>>>,
}

impl DecoderObserver {
    pub fn new(
        protocol_version: i32,
        capacity: usize,
    ) -> Result<(Self, mpsc::Receiver<DecoderEvent>), DecoderObserverError> {
        if packet_ids::lookup(protocol_version).is_none() {
            return Err(DecoderObserverError::UnsupportedProtocol(protocol_version));
        }
        if capacity == 0 {
            return Err(DecoderObserverError::InvalidCapacity);
        }
        let (sender, receiver) = mpsc::channel(capacity);
        Ok((
            Self {
                protocol_version,
                sender,
                players: Arc::new(Mutex::new(HashMap::new())),
                teams: Arc::new(Mutex::new(HashMap::new())),
                lobby_mode: Arc::new(Mutex::new(false)),
                last_emitted_teams: Arc::new(Mutex::new(Vec::new())),
            },
            receiver,
        ))
    }

    pub fn transparent(
        protocol_version: i32,
        capacity: usize,
    ) -> Result<(Self, mpsc::Receiver<DecoderEvent>), DecoderObserverError> {
        if capacity == 0 {
            return Err(DecoderObserverError::InvalidCapacity);
        }
        let (sender, receiver) = mpsc::channel(capacity);
        Ok((
            Self {
                protocol_version,
                sender,
                players: Arc::new(Mutex::new(HashMap::new())),
                teams: Arc::new(Mutex::new(HashMap::new())),
                lobby_mode: Arc::new(Mutex::new(false)),
                last_emitted_teams: Arc::new(Mutex::new(Vec::new())),
            },
            receiver,
        ))
    }

    fn emit(&self, event: DecoderEvent) -> Result<(), ObserverError> {
        self.sender.try_send(event).map_err(|_| ObserverError)
    }

    fn emit_unknown(&self, event: DecoderEvent) {
        let _ = self.sender.try_send(event);
    }
}

impl Observer for DecoderObserver {
    fn observe(&self, event: ObserverEvent) -> Result<(), ObserverError> {
        let mut payload = event.packet.payload.clone();
        let packet = event.clone();
        let packet_id = match read_varint(&mut payload) {
            Ok(value) => value,
            Err(error) => {
                return self.emit(DecoderEvent::Malformed {
                    packet,
                    error: error.to_string(),
                })
            }
        };
        let Some(ids) = packet_ids::lookup(self.protocol_version) else {
            self.emit_unknown(DecoderEvent::Unknown { packet_id, packet });
            return Ok(());
        };
        if packet_id == ids.player_info {
            let bracket = player_info_bracket(self.protocol_version).ok_or(ObserverError)?;
            return match player_info::decode(&mut payload, bracket) {
                Ok(entries) => {
                    let mut players = self.players.lock().map_err(|_| ObserverError)?;
                    let mut added_names = Vec::new();
                    let mut removed_names = Vec::new();
                    for entry in entries {
                        if entry.removed {
                            if let Some(name) = players.remove(&entry.uuid) {
                                removed_names.push(name);
                            }
                        } else if entry.added {
                            if let Some(name) = entry.name {
                                players.insert(entry.uuid, name.clone());
                                added_names.push(name);
                            }
                        }
                    }
                    self.emit(DecoderEvent::PlayerInfo {
                        packet,
                        entries: added_names.len() + removed_names.len(),
                        added_names,
                        removed_names,
                    })
                }
                Err(error) => self.emit(DecoderEvent::DecodeFailure {
                    packet,
                    error: error.to_string(),
                }),
            };
        }
        if Some(packet_id) == ids.player_remove {
            return match player_remove::decode(&mut payload) {
                Ok(uuids) => {
                    let mut players = self.players.lock().map_err(|_| ObserverError)?;
                    let names = uuids
                        .iter()
                        .filter_map(|uuid| players.remove(uuid))
                        .collect();
                    self.emit(DecoderEvent::PlayerRemove {
                        packet,
                        uuids: uuids.len(),
                        names,
                    })
                }
                Err(error) => self.emit(DecoderEvent::DecodeFailure {
                    packet,
                    error: error.to_string(),
                }),
            };
        }
        if packet_id == ids.teams {
            let bracket = teams_bracket(self.protocol_version).ok_or(ObserverError)?;
            return match teams::decode(&mut payload, bracket) {
                Ok(event) => {
                    let mut tracked = self.teams.lock().map_err(|_| ObserverError)?;
                    let mut lobby_mode = self.lobby_mode.lock().map_err(|_| ObserverError)?;
                    let is_lobby_team = event.team_id.starts_with("TAB-Sidebar-")
                        || (!chat::is_game_team(&event.team_id)
                            && event.team_id.ends_with('A')
                            && event
                                .players
                                .as_ref()
                                .is_some_and(|players| players.len() == 1));
                    if is_lobby_team {
                        *lobby_mode = true;
                        tracked.clear();
                        let mut last_emitted =
                            self.last_emitted_teams.lock().map_err(|_| ObserverError)?;
                        if !last_emitted.is_empty() {
                            last_emitted.clear();
                            self.emit_unknown(DecoderEvent::Teams {
                                packet: packet.clone(),
                                teams: Vec::new(),
                            });
                        }
                    } else if chat::is_game_team(&event.team_id) && *lobby_mode && event.mode == 0 {
                        *lobby_mode = false;
                    }
                    if *lobby_mode && chat::is_game_team(&event.team_id) {
                        return Ok(());
                    }
                    match event.mode {
                        1 => {
                            tracked.remove(&event.team_id);
                        }
                        0 | 2 => {
                            let existing_players = tracked
                                .get(&event.team_id)
                                .map(|team| team.players.clone())
                                .unwrap_or_default();
                            let players = event.players.clone().unwrap_or(existing_players);
                            let (display_name, color) = event
                                .metadata
                                .as_ref()
                                .map(|metadata| {
                                    (
                                        chat::format_team_name(&event.team_id),
                                        chat::extract_team_color(
                                            &metadata.prefix,
                                            &metadata.color_field,
                                            &event.team_id,
                                        ),
                                    )
                                })
                                .unwrap_or_else(|| {
                                    (chat::format_team_name(&event.team_id), "#AAAAAA".to_owned())
                                });
                            tracked.insert(
                                event.team_id.clone(),
                                DecoderTeam {
                                    name: event.team_id.clone(),
                                    display_name,
                                    color,
                                    players,
                                },
                            );
                        }
                        3 => {
                            if let Some(players) = event.players {
                                if let Some(team) = tracked.get_mut(&event.team_id) {
                                    for player in players {
                                        if !team.players.iter().any(|name| name == &player) {
                                            team.players.push(player);
                                        }
                                    }
                                }
                            }
                        }
                        4 => {
                            if let Some(players) = event.players {
                                if let Some(team) = tracked.get_mut(&event.team_id) {
                                    team.players.retain(|name| {
                                        !players.iter().any(|removed| removed == name)
                                    });
                                }
                            }
                        }
                        _ => {}
                    }
                    let mut teams = tracked
                        .values()
                        .filter(|team| team.is_game_team())
                        .cloned()
                        .collect::<Vec<_>>();
                    teams.sort_by(|left, right| left.name.cmp(&right.name));
                    let mut last_emitted =
                        self.last_emitted_teams.lock().map_err(|_| ObserverError)?;
                    if *last_emitted != teams {
                        *last_emitted = teams.clone();
                        self.emit_unknown(DecoderEvent::Teams { packet, teams });
                    }
                    Ok(())
                }
                Err(error) => self.emit(DecoderEvent::DecodeFailure {
                    packet,
                    error: error.to_string(),
                }),
            };
        }
        self.emit_unknown(DecoderEvent::Unknown { packet_id, packet });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Direction, FramedPacket};
    use bytes::Bytes;

    fn packet(payload: &[u8]) -> ObserverEvent {
        ObserverEvent {
            direction: Direction::UpstreamToClient,
            packet: FramedPacket {
                wire: Bytes::from(payload.to_vec()),
                payload: Bytes::from(payload.to_vec()),
            },
        }
    }

    fn write_varint(mut value: i32, output: &mut Vec<u8>) {
        loop {
            let mut byte = (value as u8) & 0x7f;
            value = ((value as u32) >> 7) as i32;
            if value != 0 {
                byte |= 0x80;
            }
            output.push(byte);
            if value == 0 {
                return;
            }
        }
    }

    #[test]
    fn rejects_unsupported_protocols() {
        assert!(matches!(
            DecoderObserver::new(776, 4),
            Err(DecoderObserverError::UnsupportedProtocol(776))
        ));
    }

    #[test]
    fn rejects_zero_capacity() {
        assert!(matches!(
            DecoderObserver::new(765, 0),
            Err(DecoderObserverError::InvalidCapacity)
        ));
    }

    #[tokio::test]
    async fn emits_unknown_packets_without_decoding() {
        let (observer, mut events) = DecoderObserver::new(765, 1).unwrap();
        observer.observe(packet(&[127])).unwrap();
        assert!(matches!(
            events.recv().await,
            Some(DecoderEvent::Unknown { packet_id: 127, .. })
        ));
    }

    #[tokio::test]
    async fn decodes_legacy_player_names() {
        let (observer, mut events) = DecoderObserver::new(47, 4).unwrap();
        let mut payload = Vec::new();
        write_varint(56, &mut payload);
        write_varint(0, &mut payload);
        write_varint(1, &mut payload);
        payload.extend_from_slice(&[0; 16]);
        write_varint(7, &mut payload);
        payload.extend_from_slice(b"whateva");
        write_varint(0, &mut payload);
        write_varint(0, &mut payload);
        write_varint(0, &mut payload);
        payload.push(0);

        observer.observe(packet(&payload)).unwrap();

        assert!(matches!(
            events.recv().await,
            Some(DecoderEvent::PlayerInfo { added_names, .. })
                if added_names == vec!["whateva".to_owned()]
        ));

        let mut removal = Vec::new();
        write_varint(56, &mut removal);
        write_varint(4, &mut removal);
        write_varint(1, &mut removal);
        removal.extend_from_slice(&[0; 16]);
        observer.observe(packet(&removal)).unwrap();
        assert!(matches!(
            events.recv().await,
            Some(DecoderEvent::PlayerInfo { removed_names, .. })
                if removed_names == vec!["whateva".to_owned()]
        ));
    }

    #[tokio::test]
    async fn transparently_observes_unknown_protocols() {
        let (observer, mut events) = DecoderObserver::transparent(999, 1).unwrap();
        observer.observe(packet(&[127])).unwrap();
        assert!(matches!(
            events.recv().await,
            Some(DecoderEvent::Unknown { packet_id: 127, .. })
        ));
    }

    #[tokio::test]
    async fn preserves_team_members_when_metadata_is_updated() {
        let (observer, mut events) = DecoderObserver::new(47, 4).unwrap();

        let mut create = Vec::new();
        write_varint(62, &mut create);
        write_varint(5, &mut create);
        create.extend_from_slice(b"1-red");
        create.push(0);
        for value in ["Red", "", "always"] {
            write_varint(value.len() as i32, &mut create);
            create.extend_from_slice(value.as_bytes());
        }
        create.push(0);
        create.push(0);
        create.push(1);
        write_varint(1, &mut create);
        write_varint(7, &mut create);
        create.extend_from_slice(b"whateva");
        observer.observe(packet(&create)).unwrap();
        let _ = events.recv().await;

        let mut update = Vec::new();
        write_varint(62, &mut update);
        write_varint(5, &mut update);
        update.extend_from_slice(b"1-red");
        update.push(2);
        for value in ["Red", "", "always"] {
            write_varint(value.len() as i32, &mut update);
            update.extend_from_slice(value.as_bytes());
        }
        update.push(0);
        update.push(0);
        update.push(1);
        observer.observe(packet(&update)).unwrap();

        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(50), events.recv())
                .await
                .is_err()
        );
        let teams = observer.teams.lock().unwrap();
        assert_eq!(
            teams.get("1-red").map(|team| team.players.clone()),
            Some(vec!["whateva".to_owned()])
        );
    }

    #[tokio::test]
    async fn emits_malformed_packets_without_panicking() {
        let (observer, mut events) = DecoderObserver::new(765, 1).unwrap();
        observer.observe(packet(&[0x80])).unwrap();
        assert!(matches!(
            events.recv().await,
            Some(DecoderEvent::Malformed { .. })
        ));
    }
}
