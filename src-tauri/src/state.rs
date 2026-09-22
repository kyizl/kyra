use device_query::{DeviceQuery, DeviceState};
use discord_rpc_client::Client as DiscordClient;
use futures_util::{SinkExt, StreamExt};
use kyra_proxy::{DecoderEvent, ObservationConfig, ObservationPipeline, Proxy, ProxyConfig};
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::async_runtime::{self, JoinHandle};
use tauri::{AppHandle, Emitter, Manager};
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProxyNetworkStatus {
    pub running: bool,
    pub port: u16,
    pub bind_host: String,
    pub client_count: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProxyStatus {
    pub pika: ProxyNetworkStatus,
    pub jartex: ProxyNetworkStatus,
}

#[derive(Debug)]
pub struct AppState {
    proxy: Mutex<ProxyStatus>,
    runtime: Mutex<ProxyRuntime>,
    log_tail: Mutex<Option<JoinHandle<()>>>,
    support_socket: Mutex<Option<JoinHandle<()>>>,
    cursor_poll: Mutex<Option<JoinHandle<()>>>,
    rpc: Mutex<RpcState>,
    rpc_commands: Mutex<Option<Sender<RpcCommand>>>,
    telemetry_key: Mutex<Option<String>>,
}

impl Drop for AppState {
    fn drop(&mut self) {
        if let Ok(commands) = self.rpc_commands.get_mut() {
            if let Some(commands) = commands.take() {
                let _ = commands.send(RpcCommand::Destroy);
            }
        }
        if let Ok(runtime) = self.runtime.get_mut() {
            if let Some(task) = runtime.pika.take() {
                task.abort();
            }
            if let Some(task) = runtime.jartex.take() {
                task.abort();
            }
        }
        if let Ok(log_tail) = self.log_tail.get_mut() {
            if let Some(task) = log_tail.take() {
                task.abort();
            }
            if let Ok(support_socket) = self.support_socket.get_mut() {
                if let Some(task) = support_socket.take() {
                    task.abort();
                }
                if let Ok(cursor_poll) = self.cursor_poll.get_mut() {
                    if let Some(task) = cursor_poll.take() {
                        task.abort();
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
struct ProxyRuntime {
    pika: Option<JoinHandle<()>>,
    jartex: Option<JoinHandle<()>>,
    pika_clients: Option<Arc<AtomicUsize>>,
    jartex_clients: Option<Arc<AtomicUsize>>,
}

#[derive(Debug, Default)]
struct RpcState {
    enabled: bool,
    active: bool,
    network: Option<String>,
}

#[derive(Debug, Clone)]
enum RpcCommand {
    Apply {
        enabled: bool,
        active: bool,
        network: Option<String>,
    },
    Destroy,
}

fn rpc_worker(receiver: mpsc::Receiver<RpcCommand>) {
    let client_id = std::env::var("KYRA_DISCORD_CLIENT_ID")
        .ok()
        .and_then(|value| value.parse::<u64>().ok());
    let mut client: Option<DiscordClient> = None;
    while let Ok(command) = receiver.recv() {
        match command {
            RpcCommand::Apply {
                enabled,
                active,
                network,
            } => {
                if enabled && active {
                    let Some(client_id) = client_id else {
                        continue;
                    };
                    if client.is_none() {
                        let mut next = DiscordClient::new(client_id);
                        next.start();
                        client = Some(next);
                    }
                    if let Some(current) = client.as_mut() {
                        let label = network.as_deref().unwrap_or("Minecraft");
                        if current
                            .set_activity(|activity| {
                                activity
                                    .state(label.to_owned())
                                    .details("Kyra Overlay")
                                    .instance(false)
                            })
                            .is_err()
                        {
                            client = None;
                        }
                    }
                } else if let Some(current) = client.as_mut() {
                    if current.clear_activity().is_err() {
                        client = None;
                    }
                }
            }
            RpcCommand::Destroy => {
                if let Some(current) = client.as_mut() {
                    let _ = current.clear_activity();
                }
                break;
            }
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            proxy: Mutex::new(ProxyStatus {
                pika: ProxyNetworkStatus {
                    running: false,
                    port: 25566,
                    bind_host: "127.0.0.1".to_owned(),
                    client_count: 0,
                    error: None,
                },
                jartex: ProxyNetworkStatus {
                    running: false,
                    port: 25567,
                    bind_host: "127.0.0.1".to_owned(),
                    client_count: 0,
                    error: None,
                },
            }),
            runtime: Mutex::new(ProxyRuntime {
                pika: None,
                jartex: None,
                pika_clients: None,
                jartex_clients: None,
            }),
            log_tail: Mutex::new(None),
            support_socket: Mutex::new(None),
            cursor_poll: Mutex::new(None),
            rpc: Mutex::new(RpcState::default()),
            rpc_commands: Mutex::new(None),
            telemetry_key: Mutex::new(None),
        }
    }
}

impl AppState {
    pub fn set_rpc_enabled(&self, enabled: bool) -> Result<(), String> {
        let command = {
            let mut rpc = self
                .rpc
                .lock()
                .map_err(|_| "rpc state lock poisoned".to_owned())?;
            rpc.enabled = enabled;
            RpcCommand::Apply {
                enabled: rpc.enabled,
                active: rpc.active,
                network: rpc.network.clone(),
            }
        };
        self.dispatch_rpc(command)
    }

    pub fn set_rpc_active(&self, active: bool) -> Result<(), String> {
        let command = {
            let mut rpc = self
                .rpc
                .lock()
                .map_err(|_| "rpc state lock poisoned".to_owned())?;
            rpc.active = active;
            RpcCommand::Apply {
                enabled: rpc.enabled,
                active: rpc.active,
                network: rpc.network.clone(),
            }
        };
        self.dispatch_rpc(command)
    }

    pub fn set_rpc_network(&self, network: String) -> Result<(), String> {
        let command = {
            let mut rpc = self
                .rpc
                .lock()
                .map_err(|_| "rpc state lock poisoned".to_owned())?;
            rpc.network = Some(network);
            RpcCommand::Apply {
                enabled: rpc.enabled,
                active: rpc.active,
                network: rpc.network.clone(),
            }
        };
        self.dispatch_rpc(command)
    }

    pub fn destroy_rpc(&self) -> Result<(), String> {
        self.dispatch_rpc(RpcCommand::Destroy)?;
        *self
            .rpc
            .lock()
            .map_err(|_| "rpc state lock poisoned".to_owned())? = RpcState::default();
        Ok(())
    }

    fn dispatch_rpc(&self, command: RpcCommand) -> Result<(), String> {
        let mut commands = self
            .rpc_commands
            .lock()
            .map_err(|_| "rpc command lock poisoned".to_owned())?;
        if commands.is_none() {
            let (sender, receiver) = mpsc::channel();
            thread::Builder::new()
                .name("kyra-discord-rpc".to_owned())
                .spawn(move || rpc_worker(receiver))
                .map_err(|error| error.to_string())?;
            *commands = Some(sender);
        }
        commands
            .as_ref()
            .ok_or_else(|| "rpc worker unavailable".to_owned())?
            .send(command)
            .map_err(|error| error.to_string())
    }

    fn telemetry_entry() -> Result<keyring::Entry, String> {
        keyring::Entry::new("kyra-overlay", "telemetry-api-key").map_err(|error| error.to_string())
    }

    pub fn linked_api_key(&self, app: &AppHandle) -> Result<Option<String>, String> {
        let mut key = self
            .telemetry_key
            .lock()
            .map_err(|_| "telemetry state lock poisoned".to_owned())?;
        if key.is_none() {
            if let Ok(secret) = Self::telemetry_entry()?.get_password() {
                *key = Some(secret);
            } else {
                let path = app
                    .path()
                    .app_data_dir()
                    .map_err(|error| error.to_string())?
                    .join("telemetry.json");
                if let Ok(data) = std::fs::read_to_string(path) {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&data) {
                        *key = value
                            .get("apiKey")
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_owned);
                    }
                }
            }
        }
        Ok(key.clone())
    }

    pub fn save_linked_api_key(
        &self,
        app: &AppHandle,
        api_key: Option<&str>,
    ) -> Result<(), String> {
        let entry = Self::telemetry_entry()?;
        if let Some(api_key) = api_key {
            entry
                .set_password(api_key)
                .map_err(|error| error.to_string())?;
        } else if let Err(error) = entry.delete_credential() {
            if !matches!(error, keyring::Error::NoEntry) {
                return Err(error.to_string());
            }
        }
        let path = app
            .path()
            .app_data_dir()
            .map_err(|error| error.to_string())?
            .join("telemetry.json");
        if path.exists() {
            std::fs::remove_file(path).map_err(|error| error.to_string())?;
        }
        let mut key = self
            .telemetry_key
            .lock()
            .map_err(|_| "telemetry state lock poisoned".to_owned())?;
        *key = api_key.map(str::to_owned);
        Ok(())
    }

    pub fn start_support_socket(&self, app: &AppHandle, api_key: String) -> Result<(), String> {
        let mut socket = self
            .support_socket
            .lock()
            .map_err(|_| "support socket lock poisoned".to_owned())?;
        if socket.is_some() {
            return Ok(());
        }
        let app = app.clone();
        *socket = Some(async_runtime::spawn(async move {
            let url = format!(
                "wss://overlay.kyizl.is-a.dev/api/support/socket?key={}",
                urlencoding::encode(&api_key)
            );
            loop {
                let Ok((stream, _)) = connect_async(&url).await else {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    continue;
                };
                let _ = app.emit(
                    "support:socket-event",
                    serde_json::json!({
                        "type": "connected"
                    }),
                );
                let (mut writer, mut reader) = stream.split();
                let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(25));
                loop {
                    tokio::select! {
                        _ = heartbeat.tick() => {
                            if writer.send(Message::Text("ping".to_owned())).await.is_err() {
                                break;
                            }
                        }
                        message = reader.next() => {
                            match message {
                                Some(Ok(Message::Text(payload))) => {
                                    if payload != "pong" {
                                        if let Ok(event) = serde_json::from_str::<serde_json::Value>(&payload) {
                                            let _ = app.emit("support:socket-event", event);
                                        }
                                    }
                                }
                                Some(Ok(Message::Ping(payload))) => {
                                    if writer.send(Message::Pong(payload)).await.is_err() {
                                        break;
                                    }
                                }
                                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                                _ => {}
                            }
                        }
                    }
                }
                let _ = app.emit(
                    "support:socket-event",
                    serde_json::json!({
                        "type": "disconnected"
                    }),
                );
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }));
        Ok(())
    }

    pub fn stop_support_socket(&self) -> Result<(), String> {
        let mut socket = self
            .support_socket
            .lock()
            .map_err(|_| "support socket lock poisoned".to_owned())?;
        if let Some(task) = socket.take() {
            task.abort();
        }
        Ok(())
    }

    pub fn start_cursor_poll(&self, app: &AppHandle) -> Result<(), String> {
        let mut cursor_poll = self
            .cursor_poll
            .lock()
            .map_err(|_| "cursor poll lock poisoned".to_owned())?;
        if cursor_poll.is_some() {
            return Ok(());
        }
        let app = app.clone();
        *cursor_poll = Some(async_runtime::spawn(async move {
            let device = DeviceState::new();
            let mut previous = None;
            let mut previous_cursor = None;
            let mut window_geometry = None;
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(16));
            let mut geometry_tick = 0u8;
            loop {
                interval.tick().await;
                let cursor = device.get_mouse().coords;
                if previous_cursor == Some(cursor) {
                    continue;
                }
                previous_cursor = Some(cursor);
                if geometry_tick == 0 || window_geometry.is_none() {
                    let Some(window) = app.get_webview_window("main") else {
                        continue;
                    };
                    let Ok(position) = window.outer_position() else {
                        continue;
                    };
                    let Ok(size) = window.outer_size() else {
                        continue;
                    };
                    window_geometry = Some((position, size));
                }
                geometry_tick = (geometry_tick + 1) % 16;
                let Some((position, size)) = window_geometry else {
                    continue;
                };
                let Ok(screen) = screenshots::Screen::from_point(cursor.0, cursor.1) else {
                    continue;
                };
                let display = screen.display_info;
                let center_x = display.x + display.width as i32 / 2;
                let center_y = display.y + display.height as i32 / 2;
                if in_crosshair_dead_zone(cursor.0, cursor.1, center_x, center_y) {
                    if previous.take().is_some() {
                        let _ = app.emit("cursor:forwarded-move", None::<[i32; 2]>);
                    }
                    continue;
                }
                let relative = if cursor.0 >= position.x
                    && cursor.1 >= position.y
                    && cursor.0 < position.x + size.width as i32
                    && cursor.1 < position.y + size.height as i32
                {
                    Some([cursor.0 - position.x, cursor.1 - position.y])
                } else {
                    None
                };
                if previous == relative {
                    continue;
                }
                previous = relative;
                let _ = app.emit("cursor:forwarded-move", relative);
            }
        }));
        Ok(())
    }

    pub fn stop_cursor_poll(&self) -> Result<(), String> {
        let mut cursor_poll = self
            .cursor_poll
            .lock()
            .map_err(|_| "cursor poll lock poisoned".to_owned())?;
        if let Some(task) = cursor_poll.take() {
            task.abort();
        }
        Ok(())
    }

    pub fn proxy_status(&self) -> Result<ProxyStatus, String> {
        let mut status = self
            .proxy
            .lock()
            .map_err(|_| "proxy state lock poisoned".to_owned())?;
        let runtime = self
            .runtime
            .lock()
            .map_err(|_| "proxy runtime lock poisoned".to_owned())?;
        status.pika.client_count = runtime
            .pika_clients
            .as_ref()
            .map_or(0, |count| count.load(Ordering::Acquire));
        status.jartex.client_count = runtime
            .jartex_clients
            .as_ref()
            .map_or(0, |count| count.load(Ordering::Acquire));
        Ok(status.clone())
    }

    pub fn set_port(&self, network: &str, port: u16) -> Result<(), String> {
        if !(1024..=65535).contains(&port) {
            return Err("proxy port must be between 1024 and 65535".to_owned());
        }
        let runtime = self
            .runtime
            .lock()
            .map_err(|_| "proxy runtime lock poisoned".to_owned())?;
        if runtime.pika.is_some() || runtime.jartex.is_some() {
            return Err("stop the proxy before changing its port".to_owned());
        }
        drop(runtime);
        let mut status = self
            .proxy
            .lock()
            .map_err(|_| "proxy state lock poisoned".to_owned())?;
        match network {
            "pikanetwork" => status.pika.port = port,
            "jartexnetwork" => status.jartex.port = port,
            _ => return Err("unsupported proxy network".to_owned()),
        }
        Ok(())
    }

    pub fn set_bind_host(&self, bind_host: &str) -> Result<(), String> {
        if bind_host != "127.0.0.1" && bind_host != "0.0.0.0" {
            return Err("unsupported proxy bind host".to_owned());
        }
        let runtime = self
            .runtime
            .lock()
            .map_err(|_| "proxy runtime lock poisoned".to_owned())?;
        if runtime.pika.is_some() || runtime.jartex.is_some() {
            return Err("stop the proxy before changing its bind host".to_owned());
        }
        drop(runtime);
        let mut status = self
            .proxy
            .lock()
            .map_err(|_| "proxy state lock poisoned".to_owned())?;
        status.pika.bind_host = bind_host.to_owned();
        status.jartex.bind_host = bind_host.to_owned();
        Ok(())
    }

    pub async fn start_proxy(&self, app: AppHandle) -> Result<ProxyStatus, String> {
        {
            let runtime = self
                .runtime
                .lock()
                .map_err(|_| "proxy runtime lock poisoned".to_owned())?;
            if runtime.pika.is_some() || runtime.jartex.is_some() {
                return self.proxy_status();
            }
        }
        let status = self.proxy_status()?;
        let mut pika_config = ProxyConfig::new(
            format!("{}:{}", status.pika.bind_host, status.pika.port)
                .parse()
                .map_err(|error| format!("invalid Pika listen address: {error}"))?,
            "172.65.169.236:25565",
        );
        pika_config.upstream_host = Some("pika.host".to_owned());
        pika_config.observer_factory = Some(proxy_observer_factory(app.clone(), "pikanetwork"));
        let pika = Proxy::bind(pika_config)
            .await
            .map_err(|error| format!("failed to start Pika proxy: {error}"))?;
        eprintln!(
            "[kyra-proxy] Pika listener ready local={} upstream=pika.host:25565",
            pika.local_addr()
                .map_err(|error| format!("failed to read Pika listen address: {error}"))?
        );
        let mut jartex_config = ProxyConfig::new(
            format!("{}:{}", status.jartex.bind_host, status.jartex.port)
                .parse()
                .map_err(|error| format!("invalid Jartex listen address: {error}"))?,
            "play.jartex.fun:25565",
        );
        jartex_config.observer_factory = Some(proxy_observer_factory(app, "jartexnetwork"));
        let jartex = Proxy::bind(jartex_config)
            .await
            .map_err(|error| format!("failed to start Jartex proxy: {error}"))?;
        eprintln!(
            "[kyra-proxy] Jartex listener ready local={} upstream=play.jartex.fun:25565",
            jartex
                .local_addr()
                .map_err(|error| format!("failed to read Jartex listen address: {error}"))?
        );
        let pika_clients = pika.client_counter();
        let jartex_clients = jartex.client_counter();
        let pika_task = async_runtime::spawn(async move {
            if let Err(error) = pika.serve().await {
                eprintln!("Pika proxy stopped: {error}");
            }
        });
        let jartex_task = async_runtime::spawn(async move {
            if let Err(error) = jartex.serve().await {
                eprintln!("Jartex proxy stopped: {error}");
            }
        });
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| "proxy runtime lock poisoned".to_owned())?;
        runtime.pika = Some(pika_task);
        runtime.jartex = Some(jartex_task);
        runtime.pika_clients = Some(pika_clients);
        runtime.jartex_clients = Some(jartex_clients);
        drop(runtime);
        let mut status = self
            .proxy
            .lock()
            .map_err(|_| "proxy state lock poisoned".to_owned())?;
        status.pika.running = true;
        status.jartex.running = true;
        status.pika.error = None;
        status.jartex.error = None;
        Ok(status.clone())
    }

    pub fn proxy_configuration_matches(
        &self,
        pika_port: u16,
        jartex_port: u16,
        bind_host: &str,
    ) -> Result<bool, String> {
        let status = self.proxy_status()?;
        Ok(status.pika.running
            && status.pika.port == pika_port
            && status.jartex.port == jartex_port
            && status.pika.bind_host == bind_host
            && status.jartex.bind_host == bind_host)
    }

    pub fn stop_proxy(&self) -> Result<ProxyStatus, String> {
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| "proxy runtime lock poisoned".to_owned())?;
        if let Some(task) = runtime.pika.take() {
            task.abort();
        }
        if let Some(task) = runtime.jartex.take() {
            task.abort();
        }
        runtime.pika_clients = None;
        runtime.jartex_clients = None;
        drop(runtime);
        let mut status = self
            .proxy
            .lock()
            .map_err(|_| "proxy state lock poisoned".to_owned())?;
        status.pika.running = false;
        status.jartex.running = false;
        status.pika.client_count = 0;
        status.jartex.client_count = 0;
        Ok(status.clone())
    }

    pub fn set_log_path(&self, app: AppHandle, path: Option<String>) -> Result<(), String> {
        let mut log_tail = self
            .log_tail
            .lock()
            .map_err(|_| "log tail lock poisoned".to_owned())?;
        if let Some(task) = log_tail.take() {
            task.abort();
        }
        let Some(path) = path else {
            return Ok(());
        };
        let path = PathBuf::from(path);
        *log_tail = Some(async_runtime::spawn(async move {
            let mut offset = 0usize;
            let mut pending = String::new();
            loop {
                if let Ok(mut file) = std::fs::File::open(&path) {
                    let length = file.metadata().map(|metadata| metadata.len()).unwrap_or(0);
                    if length < offset as u64 {
                        offset = 0;
                        pending.clear();
                    }
                    if file.seek(SeekFrom::Start(offset as u64)).is_err() {
                        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                        continue;
                    }
                    let mut data = Vec::new();
                    if file.read_to_end(&mut data).is_ok() && !data.is_empty() {
                        offset += data.len();
                        let chunk = String::from_utf8_lossy(&data);
                        pending.push_str(&chunk);
                        let mut lines = Vec::new();
                        while let Some(index) = pending.find('\n') {
                            let line = pending.drain(..=index).collect::<String>();
                            lines.push(strip_minecraft_colors(line.trim_end_matches(['\r', '\n'])));
                        }
                        if !lines.is_empty() {
                            if let Err(error) = app.emit("log:line", lines) {
                                eprintln!("Log event emission failed: {error}");
                            }
                        }
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            }
        }));
        Ok(())
    }
}

fn proxy_observer_factory(app: AppHandle, network: &'static str) -> kyra_proxy::ObserverFactory {
    Arc::new(move |protocol| {
        let pipeline =
            ObservationPipeline::for_protocol(protocol, 4096, ObservationConfig::default()).ok()?;
        let (observer, mut events, _) = pipeline.into_parts();
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            while let Some(event) = events.recv().await {
                match event {
                    DecoderEvent::PlayerInfo {
                        added_names,
                        removed_names,
                        ..
                    } => {
                        for username in added_names {
                            let payload = serde_json::json!({
                                "type": "player-join",
                                "network": network,
                                "username": username,
                            });
                            if let Err(error) = app.emit("proxy:event", payload) {
                                eprintln!("failed to emit proxy player event: {error}");
                            }
                        }
                        for username in removed_names {
                            let payload = serde_json::json!({
                                "type": "player-quit",
                                "network": network,
                                "username": username,
                            });
                            if let Err(error) = app.emit("proxy:event", payload) {
                                eprintln!("failed to emit proxy player removal event: {error}");
                            }
                        }
                    }
                    DecoderEvent::DecodeFailure { error, .. }
                    | DecoderEvent::Malformed { error, .. } => {
                        eprintln!("[kyra-proxy] {network} packet decode failed: {error}");
                    }
                    DecoderEvent::Unknown { .. } => {}
                    DecoderEvent::PlayerRemove { names, .. } => {
                        for username in names {
                            let payload = serde_json::json!({
                                "type": "player-quit",
                                "network": network,
                                "username": username,
                            });
                            if let Err(error) = app.emit("proxy:event", payload) {
                                eprintln!("failed to emit proxy player removal event: {error}");
                            }
                        }
                    }
                    DecoderEvent::Teams { teams, .. } => {
                        let payload = serde_json::json!({
                            "type": "teams-update",
                            "network": network,
                            "teams": teams
                                .into_iter()
                                .filter(|team| {
                                    team.name != "all"
                                        && team.is_game_team()
                                        && !team.players.is_empty()
                                })
                                .map(|team| {
                                    serde_json::json!({
                                        "name": team.display_name,
                                        "displayName": team.display_name,
                                        "color": team.color,
                                        "players": team.players,
                                    })
                                })
                                .collect::<Vec<_>>(),
                        });
                        if let Err(error) = app.emit("proxy:event", payload) {
                            eprintln!("failed to emit proxy team event: {error}");
                        }
                    }
                }
            }
        });
        Some(observer)
    })
}

fn in_crosshair_dead_zone(x: i32, y: i32, center_x: i32, center_y: i32) -> bool {
    (x - center_x).abs() <= 3 && (y - center_y).abs() <= 3
}

fn strip_minecraft_colors(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut skip = false;
    for character in input.chars() {
        if skip {
            skip = false;
        } else if character == '§' || character == '\u{fffd}' {
            skip = true;
        } else {
            output.push(character);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{in_crosshair_dead_zone, strip_minecraft_colors, AppState};

    #[test]
    fn strips_minecraft_and_replacement_color_markers() {
        assert_eq!(
            strip_minecraft_colors("§aGreen \u{fffd}bBlue"),
            "Green Blue"
        );
    }

    #[test]
    fn validates_proxy_configuration() {
        let state = AppState::default();
        assert!(state.set_port("pikanetwork", 25570).is_ok());
        assert!(state.set_port("pikanetwork", 80).is_err());
        assert!(state.set_bind_host("0.0.0.0").is_ok());
        assert!(state.set_bind_host("localhost").is_err());
        assert_eq!(state.proxy_status().unwrap().pika.port, 25570);
    }

    #[test]
    fn crosshair_dead_zone_includes_boundaries_only() {
        assert!(in_crosshair_dead_zone(97, 100, 100, 100));
        assert!(in_crosshair_dead_zone(103, 103, 100, 100));
        assert!(!in_crosshair_dead_zone(104, 100, 100, 100));
        assert!(!in_crosshair_dead_zone(100, 104, 100, 100));
    }
}
