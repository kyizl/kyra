use rand::Rng;
use reqwest::{Client, StatusCode};
use serde::Serialize;
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
use tokio::task::JoinHandle;
use uuid::Uuid;

const ENDPOINT: &str = "https://overlay.kyizl.is-a.dev/api/telemetry/ping";
const BUILD: &str = "rust";
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(120);
const HEARTBEAT_JITTER: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
pub struct UsageConfig {
    pub endpoint: String,
    pub interval: Duration,
    pub jitter: Duration,
    pub request_timeout: Duration,
    pub shutdown_timeout: Duration,
}

impl Default for UsageConfig {
    fn default() -> Self {
        Self {
            endpoint: ENDPOINT.to_owned(),
            interval: HEARTBEAT_INTERVAL,
            jitter: HEARTBEAT_JITTER,
            request_timeout: REQUEST_TIMEOUT,
            shutdown_timeout: SHUTDOWN_TIMEOUT,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum PingEvent {
    Start,
    Heartbeat,
    End,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Ping<'a> {
    key: &'a str,
    session_id: &'a str,
    build: &'a str,
    version: &'a str,
    event: PingEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Accepted,
    Rejected,
    Failed,
}

struct Reporter {
    client: Client,
    config: Arc<UsageConfig>,
    key: String,
    session_id: String,
    version: String,
}

impl Reporter {
    async fn send(&self, event: PingEvent, timeout: Duration) -> Outcome {
        let ping = Ping {
            key: &self.key,
            session_id: &self.session_id,
            build: BUILD,
            version: &self.version,
            event,
        };
        let response = self
            .client
            .post(&self.config.endpoint)
            .timeout(timeout)
            .json(&ping)
            .send()
            .await;
        match response {
            Ok(response) if response.status() == StatusCode::UNAUTHORIZED => Outcome::Rejected,
            Ok(response) if response.status().is_success() => Outcome::Accepted,
            _ => Outcome::Failed,
        }
    }
}

struct Session {
    key: String,
    reporter: Arc<Reporter>,
    task: JoinHandle<()>,
}

impl fmt::Debug for Session {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Session")
            .field("session_id", &self.reporter.session_id)
            .field("finished", &self.task.is_finished())
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct UsageReporter {
    config: Arc<UsageConfig>,
    session: Mutex<Option<Session>>,
}

impl Default for UsageReporter {
    fn default() -> Self {
        Self::new(UsageConfig::default())
    }
}

fn delay_with(config: &UsageConfig, unit: f64) -> Duration {
    let seconds = config.interval.as_secs_f64() + config.jitter.as_secs_f64() * unit;
    Duration::from_secs_f64(seconds.max(0.0))
}

fn next_delay(config: &UsageConfig) -> Duration {
    delay_with(config, rand::thread_rng().gen_range(-1.0..=1.0))
}

async fn run(reporter: Arc<Reporter>) {
    let config = Arc::clone(&reporter.config);
    if reporter
        .send(PingEvent::Start, config.request_timeout)
        .await
        == Outcome::Rejected
    {
        return;
    }
    loop {
        tokio::time::sleep(next_delay(&config)).await;
        if reporter
            .send(PingEvent::Heartbeat, config.request_timeout)
            .await
            == Outcome::Rejected
        {
            return;
        }
    }
}

impl UsageReporter {
    pub fn new(config: UsageConfig) -> Self {
        Self {
            config: Arc::new(config),
            session: Mutex::new(None),
        }
    }

    pub fn start(&self, client: Client, api_key: &str, version: &str) {
        let mut slot = self.session.lock().unwrap_or_else(PoisonError::into_inner);
        if slot.as_ref().is_some_and(|session| session.key == api_key) {
            return;
        }
        if let Some(previous) = slot.take() {
            previous.task.abort();
        }
        let reporter = Arc::new(Reporter {
            client,
            config: Arc::clone(&self.config),
            key: api_key.to_owned(),
            session_id: Uuid::new_v4().to_string(),
            version: version.to_owned(),
        });
        let task = tokio::spawn(run(Arc::clone(&reporter)));
        *slot = Some(Session {
            key: api_key.to_owned(),
            reporter,
            task,
        });
    }

    pub fn suspend(&self) {
        let previous = self
            .session
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(previous) = previous {
            previous.task.abort();
        }
    }

    pub async fn stop(&self) {
        let previous = self
            .session
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        let Some(previous) = previous else {
            return;
        };
        let live = !previous.task.is_finished();
        previous.task.abort();
        if live {
            previous
                .reporter
                .send(PingEvent::End, self.config.shutdown_timeout)
                .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{delay_with, Ping, PingEvent, UsageConfig, UsageReporter};
    use reqwest::Client;
    use serde_json::Value;
    use std::sync::atomic::{AtomicU16, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    struct MockServer {
        endpoint: String,
        requests: Arc<Mutex<Vec<Value>>>,
        status: Arc<AtomicU16>,
    }

    impl MockServer {
        async fn start(status: u16) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let endpoint = format!(
                "http://{}/api/telemetry/ping",
                listener.local_addr().unwrap()
            );
            let requests = Arc::new(Mutex::new(Vec::new()));
            let status = Arc::new(AtomicU16::new(status));
            let captured = Arc::clone(&requests);
            let code = Arc::clone(&status);
            tokio::spawn(async move {
                loop {
                    let Ok((mut socket, _)) = listener.accept().await else {
                        return;
                    };
                    let captured = Arc::clone(&captured);
                    let code = Arc::clone(&code);
                    tokio::spawn(async move {
                        let mut buffer = Vec::new();
                        let mut chunk = [0u8; 1024];
                        let body = loop {
                            let read = socket.read(&mut chunk).await.unwrap_or(0);
                            if read == 0 {
                                return;
                            }
                            buffer.extend_from_slice(&chunk[..read]);
                            let text = String::from_utf8_lossy(&buffer).into_owned();
                            if let Some(split) = text.find("\r\n\r\n") {
                                let length = text[..split]
                                    .lines()
                                    .find_map(|line| {
                                        let lower = line.to_ascii_lowercase();
                                        lower
                                            .strip_prefix("content-length:")
                                            .and_then(|value| value.trim().parse::<usize>().ok())
                                    })
                                    .unwrap_or(0);
                                if buffer.len() >= split + 4 + length {
                                    break text[split + 4..split + 4 + length].to_owned();
                                }
                            }
                        };
                        captured
                            .lock()
                            .unwrap()
                            .push(serde_json::from_str(&body).unwrap());
                        let reply = format!(
                            "HTTP/1.1 {} X\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                            code.load(Ordering::SeqCst)
                        );
                        let _ = socket.write_all(reply.as_bytes()).await;
                    });
                }
            });
            Self {
                endpoint,
                requests,
                status,
            }
        }

        fn events(&self) -> Vec<String> {
            self.requests
                .lock()
                .unwrap()
                .iter()
                .map(|request| request["event"].as_str().unwrap().to_owned())
                .collect()
        }

        fn count(&self) -> usize {
            self.requests.lock().unwrap().len()
        }
    }

    fn fast_config(endpoint: &str) -> UsageConfig {
        UsageConfig {
            endpoint: endpoint.to_owned(),
            interval: Duration::from_millis(40),
            jitter: Duration::ZERO,
            request_timeout: Duration::from_secs(2),
            shutdown_timeout: Duration::from_secs(2),
        }
    }

    #[test]
    fn payload_contains_only_the_permitted_fields() {
        let ping = Ping {
            key: "key",
            session_id: "3f2b8c1e-5a4d-4e7b-9c1a-2d3e4f5a6b7c",
            build: "rust",
            version: "1.2.3",
            event: PingEvent::Heartbeat,
        };
        let value = serde_json::to_value(&ping).unwrap();
        let mut fields: Vec<&str> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        fields.sort_unstable();
        assert_eq!(fields, ["build", "event", "key", "sessionId", "version"]);
        assert_eq!(value["event"], "heartbeat");
    }

    #[test]
    fn heartbeat_delay_stays_within_the_jitter_window() {
        let config = UsageConfig::default();
        assert_eq!(delay_with(&config, -1.0), Duration::from_secs(110));
        assert_eq!(delay_with(&config, 0.0), Duration::from_secs(120));
        assert_eq!(delay_with(&config, 1.0), Duration::from_secs(130));
    }

    #[test]
    fn heartbeat_delay_never_goes_negative() {
        let config = UsageConfig {
            interval: Duration::from_secs(1),
            jitter: Duration::from_secs(5),
            ..UsageConfig::default()
        };
        assert_eq!(delay_with(&config, -1.0), Duration::ZERO);
    }

    #[tokio::test]
    async fn reports_start_heartbeats_and_end_for_one_session() {
        let server = MockServer::start(204).await;
        let reporter = UsageReporter::new(fast_config(&server.endpoint));
        reporter.start(Client::new(), "secret-key", "1.2.3");
        tokio::time::sleep(Duration::from_millis(250)).await;
        reporter.stop().await;

        let events = server.events();
        assert_eq!(events.first().map(String::as_str), Some("start"));
        assert_eq!(events.last().map(String::as_str), Some("end"));
        assert!(events.iter().filter(|event| *event == "heartbeat").count() >= 2);
        assert_eq!(events.iter().filter(|event| *event == "start").count(), 1);
        assert_eq!(events.iter().filter(|event| *event == "end").count(), 1);

        let requests = server.requests.lock().unwrap();
        let session_id = requests[0]["sessionId"].as_str().unwrap().to_owned();
        assert_eq!(session_id.len(), 36);
        for request in requests.iter() {
            assert_eq!(request["sessionId"], session_id.as_str());
            assert_eq!(request["key"], "secret-key");
            assert_eq!(request["build"], "rust");
            assert_eq!(request["version"], "1.2.3");
        }
    }

    #[tokio::test]
    async fn starting_twice_with_the_same_key_keeps_a_single_session() {
        let server = MockServer::start(204).await;
        let reporter = UsageReporter::new(fast_config(&server.endpoint));
        reporter.start(Client::new(), "key", "1.0.0");
        reporter.start(Client::new(), "key", "1.0.0");
        tokio::time::sleep(Duration::from_millis(100)).await;
        reporter.stop().await;
        assert_eq!(
            server
                .events()
                .iter()
                .filter(|event| *event == "start")
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn a_rejected_key_stops_reporting_and_is_not_retried() {
        let server = MockServer::start(401).await;
        let reporter = UsageReporter::new(fast_config(&server.endpoint));
        reporter.start(Client::new(), "bad-key", "1.0.0");
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(server.events(), ["start"]);

        reporter.start(Client::new(), "bad-key", "1.0.0");
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(server.count(), 1);

        server.status.store(204, Ordering::SeqCst);
        reporter.start(Client::new(), "new-key", "1.0.0");
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(server.count() > 1);
        reporter.suspend();
    }

    #[tokio::test]
    async fn a_rejected_session_does_not_send_an_end_event() {
        let server = MockServer::start(401).await;
        let reporter = UsageReporter::new(fast_config(&server.endpoint));
        reporter.start(Client::new(), "bad-key", "1.0.0");
        tokio::time::sleep(Duration::from_millis(150)).await;
        reporter.stop().await;
        assert_eq!(server.events(), ["start"]);
    }

    #[tokio::test]
    async fn suspending_stops_all_further_requests() {
        let server = MockServer::start(204).await;
        let reporter = UsageReporter::new(fast_config(&server.endpoint));
        reporter.start(Client::new(), "key", "1.0.0");
        tokio::time::sleep(Duration::from_millis(120)).await;
        reporter.suspend();
        let after_suspend = server.count();
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(server.count(), after_suspend);
        assert!(!server.events().contains(&"end".to_owned()));
    }

    #[tokio::test]
    async fn an_unreachable_server_never_panics_and_reporting_continues() {
        let reporter = UsageReporter::new(fast_config("http://127.0.0.1:9/api/telemetry/ping"));
        reporter.start(Client::new(), "key", "1.0.0");
        tokio::time::sleep(Duration::from_millis(200)).await;
        reporter.stop().await;
    }

    #[tokio::test]
    async fn stopping_without_a_session_is_a_no_op() {
        UsageReporter::default().stop().await;
    }
}
