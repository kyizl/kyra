use crate::{
    CompositeObserver, CompressionPacketIds, DecoderEvent, DecoderObserver, DecoderObserverError,
    ObservationConfig, Observer, ProtocolCompressionNegotiator,
};
use kyra_proto_spike::protocol::packet_ids;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc;

#[derive(Debug, Error)]
pub enum ObservationPipelineError {
    #[error(transparent)]
    Decoder(#[from] DecoderObserverError),
    #[error("unsupported protocol version {0}")]
    UnsupportedProtocol(i32),
}

pub struct ObservationPipeline {
    observer: Arc<dyn Observer>,
    decoder_events: mpsc::Receiver<DecoderEvent>,
    compression: ProtocolCompressionNegotiator,
}

impl ObservationPipeline {
    pub fn for_protocol(
        protocol_version: i32,
        capacity: usize,
        initial: ObservationConfig,
    ) -> Result<Self, ObservationPipelineError> {
        let Some(ids) = packet_ids::lookup(protocol_version) else {
            let (decoder, decoder_events) =
                DecoderObserver::transparent(protocol_version, capacity)?;
            let compression = ProtocolCompressionNegotiator::new(
                CompressionPacketIds {
                    client_to_upstream: None,
                    upstream_to_client: None,
                },
                initial,
            );
            let observer = Arc::new(CompositeObserver::new(vec![
                Arc::new(decoder),
                Arc::new(compression.clone()),
            ]));
            return Ok(Self {
                observer,
                decoder_events,
                compression,
            });
        };
        Self::new(
            protocol_version,
            capacity,
            CompressionPacketIds {
                client_to_upstream: Some(ids.set_compression),
                upstream_to_client: Some(ids.set_compression),
            },
            initial,
        )
    }

    pub fn new(
        protocol_version: i32,
        capacity: usize,
        compression_ids: CompressionPacketIds,
        initial: ObservationConfig,
    ) -> Result<Self, ObservationPipelineError> {
        let (decoder, decoder_events) = DecoderObserver::new(protocol_version, capacity)?;
        let compression = ProtocolCompressionNegotiator::new(compression_ids, initial);
        let observer = Arc::new(CompositeObserver::new(vec![
            Arc::new(decoder),
            Arc::new(compression.clone()),
        ]));
        Ok(Self {
            observer,
            decoder_events,
            compression,
        })
    }

    pub fn observer(&self) -> Arc<dyn Observer> {
        Arc::clone(&self.observer)
    }

    pub fn compression(&self) -> ProtocolCompressionNegotiator {
        self.compression.clone()
    }

    pub fn into_parts(
        self,
    ) -> (
        Arc<dyn Observer>,
        mpsc::Receiver<DecoderEvent>,
        ProtocolCompressionNegotiator,
    ) {
        (self.observer, self.decoder_events, self.compression)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Direction, FramedPacket, ObserverEvent};
    use bytes::Bytes;

    #[test]
    fn rejects_unsupported_protocols() {
        assert!(ObservationPipeline::for_protocol(776, 4, ObservationConfig::default()).is_ok());
    }

    #[tokio::test]
    async fn composes_decoder_and_compression_state() {
        let pipeline =
            ObservationPipeline::for_protocol(765, 4, ObservationConfig::default()).unwrap();
        let observer = pipeline.observer();
        let compression = pipeline.compression();
        let event = ObserverEvent {
            direction: Direction::UpstreamToClient,
            packet: FramedPacket {
                wire: Bytes::from_static(&[2, 3, 7]),
                payload: Bytes::from_static(&[3, 7]),
            },
        };
        observer.observe(event).unwrap();
        assert_eq!(
            compression
                .observation()
                .unwrap()
                .upstream_to_client_compression,
            Some(7)
        );
    }
}
