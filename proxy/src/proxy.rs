use crate::framing::{FramingError, PacketReader};
use crate::EncryptedStream;
use crate::EncryptionError;
use std::io;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout, Duration};

fn proxy_debug_enabled() -> bool {
    std::env::var("KYRA_PROXY_DEBUG")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn proxy_packet_debug_enabled() -> bool {
    std::env::var("KYRA_PROXY_PACKET_DEBUG")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn upstream_port(address: &str) -> u16 {
    address
        .rsplit_once(':')
        .and_then(|(_, port)| port.parse().ok())
        .unwrap_or(25565)
}

fn proxy_debug(message: impl std::fmt::Display) {
    if proxy_debug_enabled() {
        eprintln!("[kyra-proxy] {message}");
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryClass {
    Dns,
    Timeout,
    ConnectionRefused,
    NotRetryable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub jitter: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 4,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(2),
            jitter: Duration::from_millis(50),
        }
    }
}

impl RetryPolicy {
    pub fn backoff(&self, attempt: u32, jitter_seed: u64) -> Duration {
        let exponent = attempt.saturating_sub(1).min(31);
        let base = self
            .initial_backoff
            .checked_mul(1u32 << exponent)
            .unwrap_or(self.max_backoff)
            .min(self.max_backoff);
        if self.jitter.is_zero() {
            return base;
        }
        let jitter = Duration::from_nanos(
            self.jitter
                .as_nanos()
                .saturating_mul((jitter_seed % 1_000_001) as u128)
                .checked_div(1_000_000)
                .unwrap_or(u128::MAX)
                .min(u64::MAX as u128) as u64,
        );
        base.saturating_add(jitter)
            .min(self.max_backoff.saturating_add(self.jitter))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    ClientToUpstream,
    UpstreamToClient,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObserverEvent {
    pub direction: Direction,
    pub packet: crate::framing::FramedPacket,
}

#[derive(Debug, Error)]
#[error("observer rejected event")]
pub struct ObserverError;

pub trait Observer: Send + Sync + 'static {
    fn observe(&self, event: ObserverEvent) -> Result<(), ObserverError>;

    fn compression_threshold(&self, _direction: Direction) -> Option<i32> {
        None
    }
}

pub struct CompositeObserver {
    observers: Vec<Arc<dyn Observer>>,
}

impl CompositeObserver {
    pub fn new(observers: Vec<Arc<dyn Observer>>) -> Self {
        Self { observers }
    }
}

impl Observer for CompositeObserver {
    fn observe(&self, event: ObserverEvent) -> Result<(), ObserverError> {
        let mut rejected = false;
        for observer in &self.observers {
            if observer.observe(event.clone()).is_err() {
                rejected = true;
            }
        }
        if rejected {
            Err(ObserverError)
        } else {
            Ok(())
        }
    }

    fn compression_threshold(&self, direction: Direction) -> Option<i32> {
        self.observers
            .iter()
            .find_map(|observer| observer.compression_threshold(direction))
    }
}

impl<F> Observer for F
where
    F: Fn(ObserverEvent) -> Result<(), ObserverError> + Send + Sync + 'static,
{
    fn observe(&self, event: ObserverEvent) -> Result<(), ObserverError> {
        self(event)
    }
}

pub type ObserverFactory = Arc<dyn Fn(i32) -> Option<Arc<dyn Observer>> + Send + Sync + 'static>;

#[derive(Clone)]
pub struct ProxyConfig {
    pub listen_addr: SocketAddr,
    pub upstream_addr: String,
    pub upstream_host: Option<String>,
    pub connect_timeout: Duration,
    pub retry_policy: RetryPolicy,
    pub observer: Option<Arc<dyn Observer>>,
    pub observer_factory: Option<ObserverFactory>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ObservationConfig {
    pub client_to_upstream_compression: Option<i32>,
    pub upstream_to_client_compression: Option<i32>,
}

impl ProxyConfig {
    pub fn new(listen_addr: SocketAddr, upstream_addr: impl Into<String>) -> Self {
        Self {
            listen_addr,
            upstream_addr: upstream_addr.into(),
            upstream_host: None,
            connect_timeout: Duration::from_secs(15),
            retry_policy: RetryPolicy::default(),
            observer: None,
            observer_factory: None,
        }
    }
}

#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("failed to bind listener: {0}")]
    Bind(#[source] io::Error),
    #[error("failed to accept client: {0}")]
    Accept(#[source] io::Error),
    #[error("upstream connection failed: {0}")]
    Connect(#[source] io::Error),
    #[error("upstream connection timed out")]
    ConnectTimeout,
    #[error("forwarding failed: {0}")]
    Forward(#[source] io::Error),
    #[error("packet framing failed: {0}")]
    Framing(#[from] FramingError),
    #[error("encryption setup failed: {0}")]
    Encryption(#[from] EncryptionError),
}

pub struct Proxy {
    listener: TcpListener,
    config: ProxyConfig,
    observation: ObservationConfig,
    clients: Arc<AtomicUsize>,
}

impl Proxy {
    pub async fn bind(config: ProxyConfig) -> Result<Self, ProxyError> {
        let listener = TcpListener::bind(config.listen_addr)
            .await
            .map_err(ProxyError::Bind)?;
        Ok(Self {
            listener,
            config,
            observation: ObservationConfig::default(),
            clients: Arc::new(AtomicUsize::new(0)),
        })
    }

    pub async fn bind_with_observation(
        config: ProxyConfig,
        observation: ObservationConfig,
    ) -> Result<Self, ProxyError> {
        let listener = TcpListener::bind(config.listen_addr)
            .await
            .map_err(ProxyError::Bind)?;
        Ok(Self {
            listener,
            config,
            observation,
            clients: Arc::new(AtomicUsize::new(0)),
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr, ProxyError> {
        self.listener.local_addr().map_err(ProxyError::Bind)
    }

    pub fn client_count(&self) -> usize {
        self.clients.load(Ordering::Acquire)
    }

    pub fn client_counter(&self) -> Arc<AtomicUsize> {
        Arc::clone(&self.clients)
    }

    pub async fn accept(&self) -> Result<(), ProxyError> {
        let (client, _) = self.listener.accept().await.map_err(ProxyError::Accept)?;
        self.clients.fetch_add(1, Ordering::AcqRel);
        let result = run_connection_with_observation(client, &self.config, self.observation).await;
        self.clients.fetch_sub(1, Ordering::AcqRel);
        result
    }

    pub async fn serve(&self) -> Result<(), ProxyError> {
        self.serve_loop(None).await
    }

    pub async fn serve_with_errors(
        &self,
        errors: mpsc::Sender<ProxyError>,
    ) -> Result<(), ProxyError> {
        self.serve_loop(Some(errors)).await
    }

    async fn serve_loop(&self, errors: Option<mpsc::Sender<ProxyError>>) -> Result<(), ProxyError> {
        loop {
            let (client, peer) = self.listener.accept().await.map_err(ProxyError::Accept)?;
            proxy_debug(format_args!(
                "accepted client peer={peer} listen={} upstream={}",
                self.listener
                    .local_addr()
                    .map_or_else(|_| "?".to_owned(), |value| value.to_string()),
                self.config.upstream_addr
            ));
            let config = self.config.clone();
            let observation = self.observation;
            let errors = errors.clone();
            let clients = Arc::clone(&self.clients);
            clients.fetch_add(1, Ordering::AcqRel);
            tokio::spawn(async move {
                if let Err(error) =
                    run_connection_with_observation(client, &config, observation).await
                {
                    proxy_debug(format_args!("connection peer={peer} failed: {error}"));
                    if let Some(errors) = errors {
                        let _ = errors.send(error).await;
                    }
                } else {
                    proxy_debug(format_args!("connection peer={peer} closed cleanly"));
                }
                clients.fetch_sub(1, Ordering::AcqRel);
            });
        }
    }
}

pub async fn run_connection(client: TcpStream, config: &ProxyConfig) -> Result<(), ProxyError> {
    run_connection_with_observation(client, config, ObservationConfig::default()).await
}

pub async fn run_connection_with_observation(
    client: TcpStream,
    config: &ProxyConfig,
    observation: ObservationConfig,
) -> Result<(), ProxyError> {
    let client_peer = client.peer_addr().ok();
    proxy_debug(format_args!(
        "opening upstream peer={} address={} attempts={}",
        client_peer.map_or_else(|| "?".to_owned(), |value| value.to_string()),
        config.upstream_addr,
        config.retry_policy.max_attempts.max(1)
    ));
    client.set_nodelay(true).map_err(ProxyError::Forward)?;
    let upstream = connect_upstream(
        &config.upstream_addr,
        config.connect_timeout,
        config.retry_policy,
    )
    .await?;
    proxy_debug(format_args!(
        "upstream connected peer={} address={}",
        client_peer.map_or_else(|| "?".to_owned(), |value| value.to_string()),
        config.upstream_addr
    ));
    upstream.set_nodelay(true).map_err(ProxyError::Forward)?;
    let (client_read, client_write) = client.into_split();
    let (upstream_read, upstream_write) = upstream.into_split();
    let observer = config.observer.clone().or_else(|| {
        config
            .observer_factory
            .clone()
            .map(|factory| Arc::new(ProtocolObserver::new(factory)) as Arc<dyn Observer>)
    });
    let client_to_upstream = forward(
        client_read,
        upstream_write,
        Direction::ClientToUpstream,
        observer.clone(),
        observation.client_to_upstream_compression,
        Some(
            config
                .upstream_host
                .clone()
                .unwrap_or_else(|| upstream_host(&config.upstream_addr)),
        ),
        Some(upstream_port(&config.upstream_addr)),
    );
    let upstream_to_client = forward(
        upstream_read,
        client_write,
        Direction::UpstreamToClient,
        observer,
        observation.upstream_to_client_compression,
        None,
        None,
    );
    tokio::try_join!(client_to_upstream, upstream_to_client)?;
    Ok(())
}

struct ProtocolObserver {
    factory: ObserverFactory,
    selected: Mutex<Option<Arc<dyn Observer>>>,
}

impl ProtocolObserver {
    fn new(factory: ObserverFactory) -> Self {
        Self {
            factory,
            selected: Mutex::new(None),
        }
    }
}

impl Observer for ProtocolObserver {
    fn observe(&self, event: ObserverEvent) -> Result<(), ObserverError> {
        let selected = self.selected.lock().map_err(|_| ObserverError)?;
        if let Some(observer) = selected.as_ref() {
            return observer.observe(event);
        }
        drop(selected);
        if event.direction != Direction::ClientToUpstream {
            return Ok(());
        }
        let payload = event.packet.payload.as_ref();
        let mut offset = 0;
        let packet_id = read_varint(payload, &mut offset).ok_or(ObserverError)?;
        if packet_id != 0 {
            return Ok(());
        }
        let protocol = read_varint(payload, &mut offset).ok_or(ObserverError)?;
        let observer = (self.factory)(protocol).ok_or(ObserverError)?;
        observer.observe(event)?;
        self.selected
            .lock()
            .map_err(|_| ObserverError)?
            .replace(observer);
        Ok(())
    }

    fn compression_threshold(&self, direction: Direction) -> Option<i32> {
        self.selected.lock().ok().and_then(|observer| {
            observer
                .as_ref()
                .and_then(|value| value.compression_threshold(direction))
        })
    }
}

pub async fn run_encrypted_connection(
    client: TcpStream,
    config: &ProxyConfig,
    client_key: &[u8],
    client_iv: &[u8],
    upstream_key: &[u8],
    upstream_iv: &[u8],
    observation: ObservationConfig,
) -> Result<(), ProxyError> {
    client.set_nodelay(true).map_err(ProxyError::Forward)?;
    let upstream = connect_upstream(
        &config.upstream_addr,
        config.connect_timeout,
        config.retry_policy,
    )
    .await?;
    upstream.set_nodelay(true).map_err(ProxyError::Forward)?;
    let client = EncryptedStream::new(client, client_key, client_iv)?;
    let upstream = EncryptedStream::new(upstream, upstream_key, upstream_iv)?;
    let (client_read, client_write) = tokio::io::split(client);
    let (upstream_read, upstream_write) = tokio::io::split(upstream);
    let observer = config.observer.clone();
    let client_to_upstream = forward(
        client_read,
        upstream_write,
        Direction::ClientToUpstream,
        observer.clone(),
        observation.client_to_upstream_compression,
        Some(
            config
                .upstream_host
                .clone()
                .unwrap_or_else(|| upstream_host(&config.upstream_addr)),
        ),
        Some(upstream_port(&config.upstream_addr)),
    );
    let upstream_to_client = forward(
        upstream_read,
        client_write,
        Direction::UpstreamToClient,
        observer,
        observation.upstream_to_client_compression,
        None,
        None,
    );
    tokio::try_join!(client_to_upstream, upstream_to_client)?;
    Ok(())
}

pub fn classify_connect_error(error: &io::Error) -> RetryClass {
    match error.kind() {
        io::ErrorKind::ConnectionRefused => RetryClass::ConnectionRefused,
        io::ErrorKind::TimedOut => RetryClass::Timeout,
        io::ErrorKind::NotFound | io::ErrorKind::AddrNotAvailable => RetryClass::Dns,
        _ if matches!(error.raw_os_error(), Some(11001..=11006)) => RetryClass::Dns,
        _ => RetryClass::NotRetryable,
    }
}

pub async fn connect_upstream(
    address: &str,
    connect_timeout: Duration,
    policy: RetryPolicy,
) -> Result<TcpStream, ProxyError> {
    let attempts = policy.max_attempts.max(1);
    for attempt in 1..=attempts {
        proxy_debug(format_args!(
            "upstream connect attempt={attempt}/{attempts} address={address}"
        ));
        let result = timeout(connect_timeout, TcpStream::connect(address)).await;
        match result {
            Ok(Ok(stream)) => {
                proxy_debug(format_args!("upstream connect succeeded address={address}"));
                return Ok(stream);
            }
            Ok(Err(error)) => {
                proxy_debug(format_args!(
                    "upstream connect failed attempt={attempt}/{attempts} kind={:?} error={error}",
                    classify_connect_error(&error)
                ));
                if attempt == attempts || classify_connect_error(&error) == RetryClass::NotRetryable
                {
                    return Err(ProxyError::Connect(error));
                }
            }
            Err(_) => {
                proxy_debug(format_args!(
                    "upstream connect timed out attempt={attempt}/{attempts}"
                ));
                if attempt == attempts {
                    return Err(ProxyError::ConnectTimeout);
                }
            }
        }
        sleep(policy.backoff(attempt, attempt as u64)).await;
    }
    Err(ProxyError::ConnectTimeout)
}

fn upstream_host(address: &str) -> String {
    address
        .rsplit_once(':')
        .map_or_else(|| address.to_owned(), |(host, _)| host.to_owned())
}

fn read_varint(bytes: &[u8], offset: &mut usize) -> Option<i32> {
    let mut value = 0i32;
    let mut shift = 0;
    while *offset < bytes.len() && shift <= 28 {
        let byte = bytes[*offset];
        *offset += 1;
        value |= ((byte & 0x7f) as i32) << shift;
        if byte & 0x80 == 0 {
            return Some(value);
        }
        shift += 7;
    }
    None
}

fn write_varint(mut value: i32, output: &mut Vec<u8>) {
    loop {
        let mut byte = (value & 0x7f) as u8;
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

type RewrittenHandshake = (Vec<u8>, Vec<u8>);

fn rewrite_handshake(
    buffer: &[u8],
    host: &str,
    port: u16,
) -> Option<Result<RewrittenHandshake, ()>> {
    let mut offset = 0;
    let packet_length = read_varint(buffer, &mut offset)?;
    if !(0..=1_048_576).contains(&packet_length) {
        return Some(Err(()));
    }
    let packet_length = packet_length as usize;
    if buffer.len() < offset + packet_length {
        return None;
    }
    let packet_end = offset + packet_length;
    let packet = &buffer[offset..packet_end];
    let mut packet_offset = 0;
    let packet_id = match read_varint(packet, &mut packet_offset) {
        Some(value) => value,
        None => return Some(Err(())),
    };
    if packet_id != 0 {
        return Some(Err(()));
    }
    if read_varint(packet, &mut packet_offset).is_none() {
        return Some(Err(()));
    }
    let address_length_start = packet_offset;
    let address_length = match read_varint(packet, &mut packet_offset) {
        Some(value) => value,
        None => return Some(Err(())),
    };
    if address_length < 0 {
        return Some(Err(()));
    }
    let address_length = address_length as usize;
    if packet_offset + address_length + 2 > packet.len() {
        return Some(Err(()));
    }
    let address_start = packet_offset;
    let address_end = address_start + address_length;
    let mut rewritten_packet = Vec::with_capacity(packet.len() + host.len());
    rewritten_packet.extend_from_slice(&packet[..address_length_start]);
    write_varint(host.len() as i32, &mut rewritten_packet);
    rewritten_packet.extend_from_slice(host.as_bytes());
    rewritten_packet.extend_from_slice(&port.to_be_bytes());
    rewritten_packet.extend_from_slice(&packet[address_end + 2..]);
    let mut rewritten = Vec::with_capacity(rewritten_packet.len() + 5);
    write_varint(rewritten_packet.len() as i32, &mut rewritten);
    rewritten.extend_from_slice(&rewritten_packet);
    Some(Ok((rewritten, buffer[packet_end..].to_vec())))
}

async fn forward<R, W>(
    mut reader: R,
    mut writer: W,
    direction: Direction,
    observer: Option<Arc<dyn Observer>>,
    compression_threshold: Option<i32>,
    rewrite_host: Option<String>,
    rewrite_port: Option<u16>,
) -> Result<(), ProxyError>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut bytes = [0u8; 16 * 1024];
    let mut packet_reader = PacketReader::new();
    packet_reader.set_compression(compression_threshold);
    let mut handshake = rewrite_host.as_ref().map(|_| Vec::with_capacity(256));
    let mut handshake_pending = rewrite_host.is_some();
    let mut read_count = 0u64;
    let direction_name = match direction {
        Direction::ClientToUpstream => "client->upstream",
        Direction::UpstreamToClient => "upstream->client",
    };
    loop {
        let count = reader.read(&mut bytes).await.map_err(ProxyError::Forward)?;
        if count == 0 {
            proxy_debug(format_args!("{direction_name} eof reads={read_count}"));
            if handshake_pending {
                if let Some(buffer) = handshake.take() {
                    writer
                        .write_all(&buffer)
                        .await
                        .map_err(ProxyError::Forward)?;
                }
            }
            writer.shutdown().await.map_err(ProxyError::Forward)?;
            return Ok(());
        }
        read_count += 1;
        if read_count <= 3 {
            proxy_debug(format_args!(
                "{direction_name} read={read_count} bytes={} first_bytes={} text={}",
                count,
                hex_preview(&bytes[..count]),
                text_preview(&bytes[..count])
            ));
        }
        let mut forwarded = &bytes[..count];
        let mut rewritten = None;
        let mut trailing = Vec::new();
        if handshake_pending {
            if let Some(buffer) = handshake.as_mut() {
                buffer.extend_from_slice(forwarded);
                if buffer.len() > 1_048_576 {
                    handshake_pending = false;
                    rewritten = Some(std::mem::take(buffer));
                } else if let Some(host) = rewrite_host.as_deref() {
                    match rewrite_handshake(buffer, host, rewrite_port.unwrap_or(25565)) {
                        Some(Ok((value, remainder))) => {
                            proxy_debug(format_args!(
                                "{direction_name} handshake rewritten original_bytes={} rewritten_bytes={} target_host={} bytes={}",
                                buffer.len(),
                                value.len(),
                                host,
                                hex_preview(&value)
                            ));
                            handshake_pending = false;
                            rewritten = Some(value);
                            trailing = remainder;
                            buffer.clear();
                        }
                        Some(Err(())) => {
                            proxy_debug(format_args!(
                                "{direction_name} handshake parse failed; forwarding unchanged bytes={}",
                                buffer.len()
                            ));
                            handshake_pending = false;
                            rewritten = Some(std::mem::take(buffer));
                        }
                        None => {}
                    }
                }
            }
            if let Some(value) = rewritten.as_ref() {
                writer.write_all(value).await.map_err(ProxyError::Forward)?;
                forwarded = &[];
            }
            if !trailing.is_empty() {
                writer
                    .write_all(&trailing)
                    .await
                    .map_err(ProxyError::Forward)?;
            }
        }
        if !handshake_pending && rewritten.is_none() {
            writer
                .write_all(forwarded)
                .await
                .map_err(ProxyError::Forward)?;
        }
        if let Some(observer) = observer.as_ref() {
            let observed_bytes = rewritten.as_deref().unwrap_or(&bytes[..count]);
            packet_reader.set_compression(
                observer
                    .compression_threshold(direction)
                    .or(compression_threshold),
            );
            packet_reader.feed(observed_bytes);
            loop {
                match packet_reader.next_packet() {
                    Ok(Some(packet)) => {
                        if proxy_packet_debug_enabled() {
                            proxy_debug(format_args!(
                                "{direction_name} observed packet wire_bytes={} payload_bytes={}",
                                packet.wire.len(),
                                packet.payload.len()
                            ));
                        }
                        if observer
                            .observe(ObserverEvent { direction, packet })
                            .is_err()
                        {
                            proxy_debug(format_args!("{direction_name} observer rejected packet"));
                        }
                        packet_reader.set_compression(
                            observer
                                .compression_threshold(direction)
                                .or(compression_threshold),
                        );
                    }
                    Ok(None) => break,
                    Err(_) => {
                        packet_reader = PacketReader::new();
                        break;
                    }
                }
            }
        }
    }
}

fn hex_preview(bytes: &[u8]) -> String {
    bytes
        .iter()
        .take(32)
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn text_preview(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let start = text.find('{').unwrap_or(0);
    let text = &text[start..];
    text.chars()
        .filter(|character| !character.is_control() || matches!(character, '\n' | '\r' | '\t'))
        .take(512)
        .collect::<String>()
}

pub fn channel_observer(capacity: usize) -> (mpsc::Sender<ObserverEvent>, impl Observer) {
    let (sender, _, observer) = channel_observer_with_receiver(capacity);
    (sender, observer)
}

pub fn channel_observer_with_receiver(
    capacity: usize,
) -> (
    mpsc::Sender<ObserverEvent>,
    mpsc::Receiver<ObserverEvent>,
    impl Observer,
) {
    let (sender, receiver) = mpsc::channel(capacity);
    let observer_sender = sender.clone();
    let observer = move |event: ObserverEvent| -> Result<(), ObserverError> {
        observer_sender.try_send(event).map_err(|_| ObserverError)
    };
    (sender, receiver, observer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FramedPacket;
    use bytes::Bytes;
    use flate2::{write::ZlibEncoder, Compression};
    use std::io::ErrorKind;
    use std::io::Write;
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn handshake_packet(host: &str) -> Vec<u8> {
        handshake_packet_with_protocol(host, 765)
    }

    fn handshake_packet_with_protocol(host: &str, protocol: i32) -> Vec<u8> {
        let mut body = Vec::new();
        write_varint(0, &mut body);
        write_varint(protocol, &mut body);
        write_varint(host.len() as i32, &mut body);
        body.extend_from_slice(host.as_bytes());
        body.extend_from_slice(&25565u16.to_be_bytes());
        write_varint(2, &mut body);
        let mut packet = Vec::new();
        write_varint(body.len() as i32, &mut packet);
        packet.extend_from_slice(&body);
        packet
    }

    #[test]
    fn rewrites_handshake_hostname_and_preserves_following_bytes() {
        let mut input = handshake_packet("localhost:25566");
        input.extend_from_slice(&[9, 8, 7]);
        let (output, remainder) = rewrite_handshake(&input, "play.pika-network.net", 25565)
            .unwrap()
            .unwrap();
        assert_eq!(remainder, vec![9, 8, 7]);
        assert!(!output
            .windows("localhost:25566".len())
            .any(|window| { window == b"localhost:25566" }));
        assert!(output
            .windows("play.pika-network.net".len())
            .any(|window| window == b"play.pika-network.net"));
        let mut offset = 0;
        let packet_length = read_varint(&output, &mut offset).unwrap() as usize;
        let packet_end = offset + packet_length;
        let mut packet_offset = 0;
        let packet = &output[offset..packet_end];
        assert_eq!(read_varint(packet, &mut packet_offset), Some(0));
        assert_eq!(read_varint(packet, &mut packet_offset), Some(765));
        let host_length = read_varint(packet, &mut packet_offset).unwrap() as usize;
        assert_eq!(
            &packet[packet_offset..packet_offset + host_length],
            b"play.pika-network.net"
        );
    }

    #[test]
    fn waits_for_fragmented_handshake() {
        let packet = handshake_packet("localhost:25567");
        let midpoint = packet.len() / 2;
        assert!(rewrite_handshake(&packet[..midpoint], "play.jartex.fun", 25565).is_none());
        assert!(rewrite_handshake(&packet, "play.jartex.fun", 25565).is_some());
    }

    #[test]
    fn rewrites_handshakes_without_protocol_version_assumptions() {
        for protocol in [47, 340, 393, 764, 765, 775, 776, 9999] {
            let mut packet = Vec::new();
            let mut body = Vec::new();
            write_varint(0, &mut body);
            write_varint(protocol, &mut body);
            write_varint(15, &mut body);
            body.extend_from_slice(b"localhost:25566");
            body.extend_from_slice(&25565u16.to_be_bytes());
            write_varint(2, &mut body);
            write_varint(body.len() as i32, &mut packet);
            packet.extend_from_slice(&body);
            let (rewritten, remainder) = rewrite_handshake(&packet, "play.pika-network.net", 25565)
                .unwrap()
                .unwrap();
            assert!(remainder.is_empty());
            assert!(rewritten
                .windows("play.pika-network.net".len())
                .any(|window| { window == b"play.pika-network.net" }));
        }
    }

    #[tokio::test]
    async fn forwards_raw_bytes_when_observer_fails() {
        let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream_addr = upstream.local_addr().unwrap();
        let received = Arc::new(Mutex::new(Vec::new()));
        let received_clone = received.clone();
        tokio::spawn(async move {
            let (mut stream, _) = upstream.accept().await.unwrap();
            let mut bytes = vec![0; 4];
            stream.read_exact(&mut bytes).await.unwrap();
            received_clone.lock().unwrap().extend(bytes);
        });
        let proxy = Proxy::bind(ProxyConfig {
            listen_addr: "127.0.0.1:0".parse().unwrap(),
            upstream_addr: upstream_addr.to_string(),
            upstream_host: None,
            connect_timeout: Duration::from_secs(1),
            retry_policy: RetryPolicy::default(),
            observer: Some(Arc::new(|_: ObserverEvent| Err(ObserverError))),
            observer_factory: None,
        })
        .await
        .unwrap();
        let listener_addr = proxy.local_addr().unwrap();
        let task = tokio::spawn(async move { proxy.accept().await });
        let mut client = TcpStream::connect(listener_addr).await.unwrap();
        client.write_all(&[0, 0, 0, 0]).await.unwrap();
        client.shutdown().await.unwrap();
        task.await.unwrap().unwrap();
        assert_eq!(*received.lock().unwrap(), vec![0, 0, 0, 0]);
    }

    #[tokio::test]
    async fn channel_observer_delivers_to_explicit_receiver() {
        let (_, mut receiver, observer) = channel_observer_with_receiver(1);
        let packet = crate::framing::FramedPacket {
            wire: vec![1, 2].into(),
            payload: vec![2].into(),
        };
        observer
            .observe(ObserverEvent {
                direction: Direction::ClientToUpstream,
                packet: packet.clone(),
            })
            .unwrap();
        let event = receiver.recv().await.unwrap();
        assert_eq!(event.packet, packet);
    }

    #[tokio::test]
    async fn serve_accepts_multiple_clients() {
        let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream_addr = upstream.local_addr().unwrap();
        let received = Arc::new(Mutex::new(Vec::new()));
        let received_clone = received.clone();
        tokio::spawn(async move {
            for _ in 0..2 {
                let (mut stream, _) = upstream.accept().await.unwrap();
                let mut byte = [0; 1];
                stream.read_exact(&mut byte).await.unwrap();
                received_clone.lock().unwrap().push(byte[0]);
            }
        });
        let proxy = Proxy::bind(ProxyConfig::new(
            "127.0.0.1:0".parse().unwrap(),
            upstream_addr.to_string(),
        ))
        .await
        .unwrap();
        let address = proxy.local_addr().unwrap();
        let server = tokio::spawn(async move { proxy.serve().await });
        for byte in [3, 7] {
            let mut client = TcpStream::connect(address).await.unwrap();
            client.write_all(&[byte]).await.unwrap();
            client.shutdown().await.unwrap();
        }
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if received.lock().unwrap().len() == 2 {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        server.abort();
        let mut values = received.lock().unwrap().clone();
        values.sort_unstable();
        assert_eq!(values, vec![3, 7]);
    }

    #[tokio::test]
    async fn forwards_and_observes_unknown_packet_without_decoding() {
        let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream_addr = upstream.local_addr().unwrap();
        let expected = vec![3, 0xff, 0x80, 0x01];
        let expected_clone = expected.clone();
        let upstream_received = tokio::spawn(async move {
            let (mut stream, _) = upstream.accept().await.unwrap();
            let mut received = vec![0; expected_clone.len()];
            stream.read_exact(&mut received).await.unwrap();
            received
        });
        let (observer_sender, mut observer_receiver, observer) = channel_observer_with_receiver(1);
        drop(observer_sender);
        let proxy = Proxy::bind(ProxyConfig {
            listen_addr: "127.0.0.1:0".parse().unwrap(),
            upstream_addr: upstream_addr.to_string(),
            upstream_host: None,
            connect_timeout: Duration::from_secs(1),
            retry_policy: RetryPolicy::default(),
            observer: Some(Arc::new(observer)),
            observer_factory: None,
        })
        .await
        .unwrap();
        let address = proxy.local_addr().unwrap();
        let task = tokio::spawn(async move { proxy.accept().await });
        let mut client = TcpStream::connect(address).await.unwrap();
        client.write_all(&expected).await.unwrap();
        client.shutdown().await.unwrap();
        assert_eq!(upstream_received.await.unwrap(), expected);
        let event = observer_receiver.recv().await.unwrap();
        assert_eq!(event.packet.wire.as_ref(), &[3, 0xff, 0x80, 0x01]);
        assert_eq!(event.packet.payload.as_ref(), &[0xff, 0x80, 0x01]);
        task.await.unwrap().unwrap();
    }

    #[test]
    fn classifies_only_connect_failures_as_retryable() {
        assert_eq!(
            classify_connect_error(&io::Error::from(ErrorKind::ConnectionRefused)),
            RetryClass::ConnectionRefused
        );
        assert_eq!(
            classify_connect_error(&io::Error::from(ErrorKind::TimedOut)),
            RetryClass::Timeout
        );
        assert_eq!(
            classify_connect_error(&io::Error::from(ErrorKind::NotFound)),
            RetryClass::Dns
        );
        assert_eq!(
            classify_connect_error(&io::Error::from(ErrorKind::InvalidData)),
            RetryClass::NotRetryable
        );
    }

    #[test]
    fn retry_backoff_is_exponential_and_bounded() {
        let policy = RetryPolicy {
            max_attempts: 5,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_millis(500),
            jitter: Duration::from_millis(50),
        };
        assert!(policy.backoff(1, 0) >= Duration::from_millis(100));
        assert!(policy.backoff(2, 1_000_000) >= policy.backoff(1, 0));
        assert!(policy.backoff(5, 1_000_000) <= Duration::from_millis(550));
        assert_eq!(policy.backoff(2, 0), Duration::from_millis(200));
    }

    #[tokio::test]
    async fn observes_compressed_frame_while_forwarding_exact_wire_bytes() {
        let payload = vec![0x7f; 32];
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&payload).unwrap();
        let compressed = encoder.finish().unwrap();
        let mut body = vec![payload.len() as u8];
        body.extend(compressed);
        let mut wire = vec![body.len() as u8];
        wire.extend(body);

        let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream_addr = upstream.local_addr().unwrap();
        let expected = wire.clone();
        let received = tokio::spawn(async move {
            let (mut stream, _) = upstream.accept().await.unwrap();
            let mut bytes = vec![0; expected.len()];
            stream.read_exact(&mut bytes).await.unwrap();
            bytes
        });
        let (_sender, mut events, observer) = channel_observer_with_receiver(1);
        let proxy = Proxy::bind_with_observation(
            ProxyConfig {
                listen_addr: "127.0.0.1:0".parse().unwrap(),
                upstream_addr: upstream_addr.to_string(),
                upstream_host: None,
                connect_timeout: Duration::from_secs(1),
                retry_policy: RetryPolicy::default(),
                observer: Some(Arc::new(observer)),
                observer_factory: None,
            },
            ObservationConfig {
                client_to_upstream_compression: Some(1),
                upstream_to_client_compression: None,
            },
        )
        .await
        .unwrap();
        let address = proxy.local_addr().unwrap();
        let task = tokio::spawn(async move { proxy.accept().await });
        let mut client = TcpStream::connect(address).await.unwrap();
        client.write_all(&wire).await.unwrap();
        client.shutdown().await.unwrap();
        assert_eq!(received.await.unwrap(), wire);
        let event = events.recv().await.unwrap();
        assert_eq!(event.packet.wire.as_ref(), wire.as_slice());
        assert_eq!(event.packet.payload.as_ref(), payload.as_slice());
        task.await.unwrap().unwrap();
    }

    #[test]
    fn composite_observer_dispatches_all_observers() {
        let first = Arc::new(std::sync::Mutex::new(0));
        let second = Arc::new(std::sync::Mutex::new(0));
        let first_target = Arc::clone(&first);
        let second_target = Arc::clone(&second);
        let observer = CompositeObserver::new(vec![
            Arc::new(move |_event: ObserverEvent| {
                *first_target.lock().unwrap() += 1;
                Ok(())
            }),
            Arc::new(move |_event: ObserverEvent| {
                *second_target.lock().unwrap() += 1;
                Err(ObserverError)
            }),
        ]);
        let event = ObserverEvent {
            direction: Direction::ClientToUpstream,
            packet: FramedPacket {
                wire: Bytes::from_static(&[1]),
                payload: Bytes::from_static(&[0]),
            },
        };
        assert!(observer.observe(event).is_err());
        assert_eq!(*first.lock().unwrap(), 1);
        assert_eq!(*second.lock().unwrap(), 1);
    }
}
