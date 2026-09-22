mod compression;
mod decoder;
mod encryption;
mod framing;
mod lifecycle;
mod observation;
mod proxy;

pub use compression::{
    CompressionNegotiationError, CompressionPacketIds, ProtocolCompressionNegotiator,
};
pub use decoder::{DecoderEvent, DecoderObserver, DecoderObserverError};
pub use encryption::{EncryptedStream, EncryptionError};
pub use framing::{FramedPacket, FramingError, PacketReader};
pub use lifecycle::{
    AuthenticationAttempt, AuthenticationLifecycle, AuthenticationMode,
    AuthenticationTransitionError, ConnectionPhase, ConnectionRuntime, ConnectionState,
    PendingPacketQueues, PendingPackets, QueueError, StateTransitionError,
};
pub use observation::{ObservationPipeline, ObservationPipelineError};
pub use proxy::{
    channel_observer, channel_observer_with_receiver, classify_connect_error, connect_upstream,
    run_connection, run_connection_with_observation, run_encrypted_connection, CompositeObserver,
    Direction, ObservationConfig, Observer, ObserverError, ObserverEvent, ObserverFactory, Proxy,
    ProxyConfig, ProxyError, RetryClass, RetryPolicy,
};
