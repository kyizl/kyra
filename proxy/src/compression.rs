use crate::proxy::{Direction, ObservationConfig, Observer, ObserverError, ObserverEvent};
use bytes::{Buf, Bytes};
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompressionPacketIds {
    pub client_to_upstream: Option<i32>,
    pub upstream_to_client: Option<i32>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CompressionNegotiationError {
    #[error("set_compression packet has a truncated varint")]
    TruncatedVarInt,
    #[error("set_compression packet has a varint longer than five bytes")]
    VarIntTooLong,
    #[error("set_compression threshold is negative")]
    NegativeThreshold,
    #[error("set_compression packet has trailing bytes")]
    TrailingBytes,
    #[error("compression threshold state is unavailable")]
    StateUnavailable,
}

#[derive(Clone)]
pub struct ProtocolCompressionNegotiator {
    ids: CompressionPacketIds,
    state: Arc<Mutex<ObservationConfig>>,
}

impl ProtocolCompressionNegotiator {
    pub fn new(ids: CompressionPacketIds, initial: ObservationConfig) -> Self {
        Self {
            ids,
            state: Arc::new(Mutex::new(initial)),
        }
    }

    pub fn ids(&self) -> CompressionPacketIds {
        self.ids
    }

    pub fn observation(&self) -> Result<ObservationConfig, CompressionNegotiationError> {
        self.state
            .lock()
            .map(|state| *state)
            .map_err(|_| CompressionNegotiationError::StateUnavailable)
    }

    pub fn negotiate(&self, event: &ObserverEvent) -> Result<bool, CompressionNegotiationError> {
        let expected = match event.direction {
            Direction::ClientToUpstream => self.ids.client_to_upstream,
            Direction::UpstreamToClient => self.ids.upstream_to_client,
        };
        let Some(expected) = expected else {
            return Ok(false);
        };
        let mut state = self
            .state
            .lock()
            .map_err(|_| CompressionNegotiationError::StateUnavailable)?;
        let negotiated = match event.direction {
            Direction::ClientToUpstream => state.client_to_upstream_compression,
            Direction::UpstreamToClient => state.upstream_to_client_compression,
        };
        if negotiated.is_some() {
            return Ok(false);
        }
        let mut payload = event.packet.payload.clone();
        let packet_id = read_varint(&mut payload)?;
        if packet_id != expected {
            return Ok(false);
        }
        let threshold = read_varint(&mut payload)?;
        if threshold < 0 {
            return Err(CompressionNegotiationError::NegativeThreshold);
        }
        if !payload.is_empty() {
            return Err(CompressionNegotiationError::TrailingBytes);
        }
        match event.direction {
            Direction::ClientToUpstream => state.client_to_upstream_compression = Some(threshold),
            Direction::UpstreamToClient => state.upstream_to_client_compression = Some(threshold),
        }
        Ok(true)
    }
}

impl Observer for ProtocolCompressionNegotiator {
    fn observe(&self, event: ObserverEvent) -> Result<(), ObserverError> {
        self.negotiate(&event)
            .map(|_| ())
            .map_err(|_| ObserverError)
    }

    fn compression_threshold(&self, direction: Direction) -> Option<i32> {
        self.observation().ok().and_then(|state| {
            let (direction_threshold, other_threshold) = match direction {
                Direction::ClientToUpstream => (
                    state.client_to_upstream_compression,
                    state.upstream_to_client_compression,
                ),
                Direction::UpstreamToClient => (
                    state.upstream_to_client_compression,
                    state.client_to_upstream_compression,
                ),
            };
            direction_threshold.or(other_threshold)
        })
    }
}

fn read_varint(input: &mut Bytes) -> Result<i32, CompressionNegotiationError> {
    let mut value = 0i32;
    for index in 0..5 {
        let byte = input
            .get(index)
            .copied()
            .ok_or(CompressionNegotiationError::TruncatedVarInt)?;
        value |= ((byte & 0x7f) as i32) << (index * 7);
        if byte & 0x80 == 0 {
            input.advance(index + 1);
            return Ok(value);
        }
    }
    Err(CompressionNegotiationError::VarIntTooLong)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Direction, FramedPacket};

    fn event(direction: Direction, payload: &[u8]) -> ObserverEvent {
        ObserverEvent {
            direction,
            packet: FramedPacket {
                wire: Bytes::copy_from_slice(payload),
                payload: Bytes::copy_from_slice(payload),
            },
        }
    }

    #[test]
    fn parses_threshold_per_direction() {
        let negotiator = ProtocolCompressionNegotiator::new(
            CompressionPacketIds {
                client_to_upstream: Some(5),
                upstream_to_client: Some(6),
            },
            ObservationConfig::default(),
        );
        assert!(negotiator
            .negotiate(&event(Direction::ClientToUpstream, &[5, 0x80, 0x01]))
            .unwrap());
        assert!(negotiator
            .negotiate(&event(Direction::UpstreamToClient, &[6, 0x10]))
            .unwrap());
        assert_eq!(
            negotiator.observation().unwrap(),
            ObservationConfig {
                client_to_upstream_compression: Some(128),
                upstream_to_client_compression: Some(16),
            }
        );
    }

    #[test]
    fn rejects_malformed_threshold_packets() {
        let negotiator = ProtocolCompressionNegotiator::new(
            CompressionPacketIds {
                client_to_upstream: Some(5),
                upstream_to_client: None,
            },
            ObservationConfig::default(),
        );
        assert_eq!(
            negotiator.negotiate(&event(Direction::ClientToUpstream, &[5])),
            Err(CompressionNegotiationError::TruncatedVarInt)
        );
        assert_eq!(
            negotiator.negotiate(&event(
                Direction::ClientToUpstream,
                &[5, 0x80, 0x80, 0x80, 0x80, 0x80]
            )),
            Err(CompressionNegotiationError::VarIntTooLong)
        );
        assert_eq!(
            negotiator.negotiate(&event(
                Direction::ClientToUpstream,
                &[5, 0xff, 0xff, 0xff, 0xff, 0x0f]
            )),
            Err(CompressionNegotiationError::NegativeThreshold)
        );
        assert_eq!(
            negotiator.negotiate(&event(Direction::ClientToUpstream, &[5, 1, 2])),
            Err(CompressionNegotiationError::TrailingBytes)
        );
    }

    #[test]
    fn negotiation_does_not_change_wire_bytes() {
        let wire = Bytes::from_static(&[3, 5, 1]);
        let event = ObserverEvent {
            direction: Direction::ClientToUpstream,
            packet: FramedPacket {
                wire: wire.clone(),
                payload: wire.slice(1..),
            },
        };
        let negotiator = ProtocolCompressionNegotiator::new(
            CompressionPacketIds {
                client_to_upstream: Some(5),
                upstream_to_client: None,
            },
            ObservationConfig::default(),
        );
        negotiator.negotiate(&event).unwrap();
        assert_eq!(event.packet.wire, wire);
    }

    #[test]
    fn server_negotiated_threshold_applies_to_both_directions() {
        let negotiator = ProtocolCompressionNegotiator::new(
            CompressionPacketIds {
                client_to_upstream: Some(5),
                upstream_to_client: Some(5),
            },
            ObservationConfig::default(),
        );
        negotiator
            .negotiate(&event(Direction::UpstreamToClient, &[5, 128, 1]))
            .unwrap();

        assert_eq!(
            Observer::compression_threshold(&negotiator, Direction::UpstreamToClient),
            Some(128)
        );
        assert_eq!(
            Observer::compression_threshold(&negotiator, Direction::ClientToUpstream),
            Some(128)
        );
    }
}
