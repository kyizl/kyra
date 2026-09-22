use crate::auth::{self, DeviceCode};
use crate::state::{AppState, ProxyStatus};
use arboard::{Clipboard, ImageData};
use base64::{engine::general_purpose::STANDARD, Engine};
use rfd::FileDialog;
use screenshots::Screen;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutWrapper};

const MIN_WIDTH_DELTA: u32 = 24;
const MAX_WIDTH_DELTA: u32 = 480;
const TELEMETRY_API: &str = "https://overlay.kyizl.is-a.dev";
const PRODUCTION_RELEASES_API: &str = "https://api.github.com/repos/kyizl/kyra/releases/latest";
const DEV_RELEASES_API: &str = "https://api.github.com/repos/kyizl/kyra/releases/tags/dev-latest";

fn releases_api() -> &'static str {
    match option_env!("KYRA_UPDATE_CHANNEL") {
        Some("dev") => DEV_RELEASES_API,
        _ => PRODUCTION_RELEASES_API,
    }
}

fn release_is_current(release: &serde_json::Value, version: &str) -> bool {
    if option_env!("KYRA_UPDATE_CHANNEL") != Some("dev") {
        return version.is_empty() || version == env!("CARGO_PKG_VERSION");
    }

    let Some(commit) = option_env!("KYRA_BUILD_COMMIT") else {
        return false;
    };
    !commit.is_empty()
        && release
            .get("body")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|body| body.contains(commit))
}

static HTTP_CLIENT: OnceLock<Result<reqwest::Client, String>> = OnceLock::new();

fn release_version(tag: Option<&str>) -> &str {
    tag.unwrap_or_default().trim_start_matches('v')
}

fn http_client() -> Result<reqwest::Client, String> {
    HTTP_CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(10))
                .timeout(std::time::Duration::from_secs(20))
                .build()
                .map_err(|error| error.to_string())
        })
        .clone()
}

#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum TelemetryEvent {
    Linking { authorize_url: String },
    Linked,
    Error { message: String },
}

#[derive(serde::Deserialize)]
struct OAuthStart {
    state: String,
    #[serde(rename = "authorizeUrl")]
    authorize_url: String,
}

#[derive(serde::Deserialize)]
#[serde(tag = "status", rename_all = "camelCase")]
enum OAuthPoll {
    Pending,
    Success {
        #[serde(rename = "apiKey")]
        api_key: String,
    },
    Error {
        message: String,
    },
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProxyStatusEvent<'a> {
    #[serde(rename = "type")]
    event_type: &'static str,
    network: &'a str,
    running: bool,
    port: u16,
    bind_host: &'a str,
    client_count: usize,
    error: &'a Option<String>,
}

#[derive(serde::Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
    digest: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenDialogResult {
    pub canceled: bool,
    pub file_paths: Vec<String>,
}

fn open_file_dialog(extensions: &'static [&'static str]) -> Result<OpenDialogResult, String> {
    let path = FileDialog::new()
        .add_filter("Files", extensions)
        .pick_file();
    Ok(match path {
        Some(path) => OpenDialogResult {
            canceled: false,
            file_paths: vec![path.to_string_lossy().into_owned()],
        },
        None => OpenDialogResult {
            canceled: true,
            file_paths: Vec::new(),
        },
    })
}

fn emit_proxy_status(app: &AppHandle, status: &ProxyStatus) -> Result<(), String> {
    for (network, value) in [
        ("pikanetwork", &status.pika),
        ("jartexnetwork", &status.jartex),
    ] {
        let event = ProxyStatusEvent {
            event_type: "status",
            network,
            running: value.running,
            port: value.port,
            bind_host: &value.bind_host,
            client_count: value.client_count,
            error: &value.error,
        };
        app.emit("proxy:event", event)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn main_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    app.get_webview_window("main")
        .ok_or_else(|| tauri::Error::WindowNotFound)
}

fn target_width(current: u32, desired: u32) -> Option<u32> {
    let delta = desired.saturating_sub(current);
    (MIN_WIDTH_DELTA..=MAX_WIDTH_DELTA)
        .contains(&delta)
        .then_some(desired)
}

fn image_mime(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        _ => "image/png",
    }
}

#[tauri::command]
pub fn window_minimize(app: AppHandle) -> tauri::Result<()> {
    main_window(&app)?.minimize()
}

#[tauri::command]
pub fn window_close(app: AppHandle) -> tauri::Result<()> {
    main_window(&app)?.close()
}

#[tauri::command]
pub fn window_open_external(app: AppHandle, url: String) -> Result<(), String> {
    let window = main_window(&app).map_err(|error| error.to_string())?;
    window
        .set_ignore_cursor_events(false)
        .map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())?;
    opener::open(url).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn clipboard_set_text(text: String) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|error| error.to_string())?;
    clipboard.set_text(text).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn updater_check(app: AppHandle) -> Result<(), String> {
    app.emit(
        "updater:status",
        serde_json::json!({ "status": "checking" }),
    )
    .map_err(|error| error.to_string())?;
    let response = http_client()?
        .get(releases_api())
        .header("User-Agent", "Kyra-Overlay")
        .send()
        .await;
    let response = match response {
        Ok(response) => response,
        Err(error) => {
            app.emit(
                "updater:status",
                serde_json::json!({ "status": "error", "error": error.to_string() }),
            )
            .map_err(|emit_error| emit_error.to_string())?;
            return Ok(());
        }
    };
    let release = match response.error_for_status() {
        Ok(response) => response
            .json::<serde_json::Value>()
            .await
            .map_err(|error| error.to_string())?,
        Err(error) => {
            app.emit(
                "updater:status",
                serde_json::json!({ "status": "error", "error": error.to_string() }),
            )
            .map_err(|emit_error| emit_error.to_string())?;
            return Ok(());
        }
    };
    let version = release_version(release.get("tag_name").and_then(serde_json::Value::as_str));
    let current = env!("CARGO_PKG_VERSION");
    let status = if release_is_current(&release, version) {
        serde_json::json!({ "status": "up-to-date", "version": current })
    } else {
        serde_json::json!({ "status": "available", "version": version })
    };
    app.emit("updater:status", status)
        .map_err(|error| error.to_string())
}

fn installer_suffix() -> &'static str {
    if cfg!(target_os = "windows") {
        ".exe"
    } else if cfg!(target_os = "macos") {
        ".dmg"
    } else {
        ".appimage"
    }
}

#[tauri::command]
pub async fn updater_install(app: AppHandle) -> Result<(), String> {
    let release = http_client()?
        .get(releases_api())
        .header("User-Agent", "Kyra-Overlay")
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .json::<serde_json::Value>()
        .await
        .map_err(|error| error.to_string())?;
    let asset = release
        .get("assets")
        .and_then(serde_json::Value::as_array)
        .and_then(|assets| {
            assets
                .iter()
                .filter_map(|asset| serde_json::from_value::<ReleaseAsset>(asset.clone()).ok())
                .find(|asset| {
                    asset
                        .name
                        .to_ascii_lowercase()
                        .ends_with(installer_suffix())
                })
        })
        .ok_or_else(|| "no installer asset is available for this platform".to_owned())?;
    app.emit(
        "updater:status",
        serde_json::json!({ "status": "downloading", "version": release_version(release.get("tag_name").and_then(serde_json::Value::as_str)) }),
    )
    .map_err(|error| error.to_string())?;
    let bytes = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|error| error.to_string())?
        .get(asset.browser_download_url)
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .bytes()
        .await
        .map_err(|error| error.to_string())?;
    if let Some(expected) = asset
        .digest
        .as_deref()
        .and_then(|digest| digest.strip_prefix("sha256:"))
    {
        let actual = format!("{:x}", Sha256::digest(&bytes));
        if actual != expected {
            return Err(
                "downloaded installer checksum does not match the release digest".to_owned(),
            );
        }
    }
    let path = std::env::temp_dir().join(&asset.name);
    tokio::fs::write(&path, bytes)
        .await
        .map_err(|error| error.to_string())?;
    app.emit(
        "updater:status",
        serde_json::json!({ "status": "installing" }),
    )
    .map_err(|error| error.to_string())?;
    if cfg!(target_os = "windows") {
        std::process::Command::new(&path)
            .spawn()
            .map_err(|error| error.to_string())?;
        app.exit(0);
    } else {
        opener::open(&path).map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn auth_start() -> Result<DeviceCode, String> {
    auth::begin(&http_client()?)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn auth_poll(
    device_code: String,
    interval_seconds: u64,
    expires_in_seconds: u64,
) -> Result<auth::MinecraftProfile, String> {
    let token = auth::finish(
        &http_client()?,
        &device_code,
        interval_seconds,
        expires_in_seconds,
    )
    .await
    .map_err(|error| error.to_string())?;
    auth::save(&token).map_err(|error| error.to_string())?;
    Ok(token.profile)
}

#[tauri::command]
pub fn auth_status() -> Result<Option<auth::MinecraftProfile>, String> {
    auth::load()
        .map(|token| token.map(|token| token.profile))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn auth_refresh() -> Result<auth::MinecraftProfile, String> {
    let token = auth::load()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "no stored Microsoft session".to_owned())?;
    let refreshed = auth::refresh(&http_client()?, &token)
        .await
        .map_err(|error| error.to_string())?;
    auth::save(&refreshed).map_err(|error| error.to_string())?;
    Ok(refreshed.profile)
}

#[tauri::command]
pub fn auth_logout() -> Result<(), String> {
    auth::clear().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn rpc_set_enabled(state: tauri::State<'_, AppState>, enabled: bool) -> Result<(), String> {
    state.set_rpc_enabled(enabled)
}

#[tauri::command]
pub fn rpc_set_active(state: tauri::State<'_, AppState>, active: bool) -> Result<(), String> {
    state.set_rpc_active(active)
}

#[tauri::command]
pub fn rpc_set_network(state: tauri::State<'_, AppState>, network: String) -> Result<(), String> {
    state.set_rpc_network(network)
}

#[tauri::command]
pub fn rpc_destroy(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.destroy_rpc()
}

fn stats_base(network: &str) -> Result<&'static str, String> {
    match network {
        "pika" => Ok("https://stats.pika-network.net/api"),
        "jartex" => Ok("https://stats.jartexnetwork.com/api"),
        _ => Err("unsupported statistics network".to_owned()),
    }
}

#[tauri::command]
pub async fn telemetry_is_linked(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<bool, String> {
    let Some(api_key) = state.linked_api_key(&app)? else {
        return Ok(false);
    };
    let response = http_client()?
        .get(format!(
            "{TELEMETRY_API}/api/telemetry/verify?key={}",
            urlencoding::encode(&api_key)
        ))
        .send()
        .await;
    match response {
        Ok(response) if response.status().is_success() => {
            let body = response
                .json::<serde_json::Value>()
                .await
                .map_err(|error| error.to_string())?;
            if body.get("valid").and_then(serde_json::Value::as_bool) == Some(false) {
                state.save_linked_api_key(&app, None)?;
                Ok(false)
            } else {
                Ok(true)
            }
        }
        Ok(_) => Ok(true),
        Err(_) => Ok(true),
    }
}

#[tauri::command]
pub async fn telemetry_start_link(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let client = match http_client() {
        Ok(client) => client,
        Err(error) => return Err(error),
    };
    let response = match client
        .get(format!("{TELEMETRY_API}/oauth/discord/start"))
        .send()
        .await
    {
        Ok(response) => response,
        Err(_) => {
            app.emit(
                "telemetry:event",
                TelemetryEvent::Error {
                    message: "Could not reach the server. Please try again.".to_owned(),
                },
            )
            .map_err(|error| error.to_string())?;
            return Ok(());
        }
    };
    let start = match response.error_for_status() {
        Ok(response) => match response.json::<OAuthStart>().await {
            Ok(start) => start,
            Err(_) => {
                app.emit(
                    "telemetry:event",
                    TelemetryEvent::Error {
                        message: "Could not reach the server. Please try again.".to_owned(),
                    },
                )
                .map_err(|error| error.to_string())?;
                return Ok(());
            }
        },
        Err(_) => {
            app.emit(
                "telemetry:event",
                TelemetryEvent::Error {
                    message: "Could not reach the server. Please try again.".to_owned(),
                },
            )
            .map_err(|error| error.to_string())?;
            return Ok(());
        }
    };
    app.emit(
        "telemetry:event",
        TelemetryEvent::Linking {
            authorize_url: start.authorize_url.clone(),
        },
    )
    .map_err(|error| error.to_string())?;
    if opener::open(&start.authorize_url).is_err() {
        app.emit(
            "telemetry:event",
            TelemetryEvent::Error {
                message: "Could not open the browser. Please try again.".to_owned(),
            },
        )
        .map_err(|error| error.to_string())?;
        return Ok(());
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(600);
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        if std::time::Instant::now() >= deadline {
            app.emit(
                "telemetry:event",
                TelemetryEvent::Error {
                    message: "Sign-in timed out. Please try again.".to_owned(),
                },
            )
            .map_err(|error| error.to_string())?;
            return Ok(());
        }
        let poll = client
            .get(format!(
                "{TELEMETRY_API}/oauth/discord/poll?state={}",
                urlencoding::encode(&start.state)
            ))
            .send()
            .await;
        let Ok(response) = poll else {
            continue;
        };
        let Ok(result) = response.json::<OAuthPoll>().await else {
            continue;
        };
        match result {
            OAuthPoll::Pending => {}
            OAuthPoll::Success { api_key } => {
                state.save_linked_api_key(&app, Some(&api_key))?;
                app.emit("telemetry:event", TelemetryEvent::Linked)
                    .map_err(|error| error.to_string())?;
                return Ok(());
            }
            OAuthPoll::Error { message } => {
                app.emit("telemetry:event", TelemetryEvent::Error { message })
                    .map_err(|error| error.to_string())?;
                return Ok(());
            }
        }
    }
}

#[tauri::command]
pub async fn stats_fetch(
    network: String,
    username: String,
    interval: String,
    mode: String,
    concurrent: Option<bool>,
) -> Result<serde_json::Value, String> {
    let base = stats_base(&network)?;
    let client = match http_client() {
        Ok(client) => client,
        Err(error) => return Err(error),
    };
    let profile_url = format!("{base}/profile/{}", urlencoding::encode(&username));
    let profile_response = client
        .get(&profile_url)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let profile_status = profile_response.status();
    if profile_status.as_u16() == 404 || profile_status.as_u16() == 400 {
        return Ok(serde_json::json!({
            "profile": null,
            "stats": null,
            "notFound": true,
            "rateLimit": false,
            "statsDisabled": false
        }));
    }
    if profile_status.as_u16() == 429 {
        return Ok(serde_json::json!({
            "profile": null,
            "stats": null,
            "notFound": false,
            "rateLimit": true,
            "statsDisabled": false
        }));
    }
    let profile = if profile_status.is_success() {
        profile_response
            .json::<serde_json::Value>()
            .await
            .map_err(|error| error.to_string())?
    } else {
        serde_json::Value::Null
    };
    let canonical_username = profile
        .get("username")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(&username);
    let stats_url = format!(
        "{base}/profile/{}/leaderboard?type=bedwars&interval={interval}&mode={mode}",
        urlencoding::encode(canonical_username)
    );
    let stats_response = client.get(stats_url).send().await;
    let (stats, stats_disabled) = match stats_response {
        Ok(response) if response.status().as_u16() == 204 => (serde_json::Value::Null, true),
        Ok(response) if response.status().is_success() => (
            response
                .json::<serde_json::Value>()
                .await
                .map_err(|error| error.to_string())?,
            false,
        ),
        Ok(_) | Err(_) => (serde_json::Value::Null, false),
    };
    let _ = concurrent;
    Ok(serde_json::json!({
        "profile": profile,
        "stats": stats,
        "notFound": false,
        "rateLimit": false,
        "statsDisabled": stats_disabled
    }))
}

#[tauri::command]
pub async fn stats_get(
    network: String,
    username: String,
    interval: String,
    mode: String,
) -> Result<serde_json::Value, String> {
    let result = stats_fetch(network, username, interval, mode, Some(false)).await?;
    Ok(result
        .get("stats")
        .cloned()
        .unwrap_or(serde_json::Value::Null))
}

#[tauri::command]
pub async fn stats_clan(network: String, name: String) -> Result<serde_json::Value, String> {
    let base = stats_base(&network)?;
    http_client()?
        .get(format!("{base}/clans/{}", urlencoding::encode(&name)))
        .send()
        .await
        .map_err(|error| error.to_string())?
        .json::<serde_json::Value>()
        .await
        .map_err(|error| error.to_string())
}

async fn support_request(
    app: &AppHandle,
    state: &AppState,
    path: &str,
    method: reqwest::Method,
    body: Option<serde_json::Value>,
) -> serde_json::Value {
    let key = match state.linked_api_key(app) {
        Ok(Some(key)) => key,
        Ok(None) => {
            return serde_json::json!({
                "ok": false,
                "error": "Link your Discord account to use Support."
            });
        }
        Err(error) => return serde_json::json!({ "ok": false, "error": error }),
    };
    let client = match http_client() {
        Ok(client) => client,
        Err(error) => return serde_json::json!({ "ok": false, "error": error }),
    };
    let mut request = client
        .request(method, format!("https://overlay.kyizl.is-a.dev{path}"))
        .bearer_auth(key)
        .header("Content-Type", "application/json");
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = match request.send().await {
        Ok(response) => response,
        Err(_) => {
            return serde_json::json!({
                "ok": false,
                "error": "Could not reach the support server. Check your connection."
            });
        }
    };
    if !response.status().is_success() {
        let error = response
            .json::<serde_json::Value>()
            .await
            .ok()
            .and_then(|value| {
                value
                    .get("error")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| "Something went wrong. Please try again.".to_owned());
        return serde_json::json!({ "ok": false, "error": error });
    }
    match response.json::<serde_json::Value>().await {
        Ok(data) => serde_json::json!({ "ok": true, "data": data }),
        Err(error) => serde_json::json!({ "ok": false, "error": error.to_string() }),
    }
}

#[tauri::command]
pub async fn support_list(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    Ok(support_request(
        &app,
        &state,
        "/api/support/conversations",
        reqwest::Method::GET,
        None,
    )
    .await)
}

#[tauri::command]
pub async fn support_create(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    subject: String,
    message: String,
) -> Result<serde_json::Value, String> {
    Ok(support_request(
        &app,
        &state,
        "/api/support/conversations",
        reqwest::Method::POST,
        Some(serde_json::json!({ "subject": subject, "message": message })),
    )
    .await)
}

#[tauri::command]
pub async fn support_get(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<serde_json::Value, String> {
    Ok(support_request(
        &app,
        &state,
        &format!("/api/support/conversations/{}", urlencoding::encode(&id)),
        reqwest::Method::GET,
        None,
    )
    .await)
}

#[tauri::command]
pub async fn support_reply(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
    message: String,
) -> Result<serde_json::Value, String> {
    Ok(support_request(
        &app,
        &state,
        &format!(
            "/api/support/conversations/{}/messages",
            urlencoding::encode(&id)
        ),
        reqwest::Method::POST,
        Some(serde_json::json!({ "message": message })),
    )
    .await)
}

#[tauri::command]
pub fn support_socket_connect(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let api_key = state
        .linked_api_key(&app)?
        .ok_or_else(|| "Link your Discord account to use Support.".to_owned())?;
    state.start_support_socket(&app, api_key)
}

#[tauri::command]
pub fn support_socket_disconnect(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.stop_support_socket()
}

#[tauri::command]
pub fn perf_dump(app: AppHandle, renderer_snapshot: serde_json::Value) -> Result<String, String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("perf-logs");
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis();
    let target = directory.join(format!("session-{timestamp}.json"));
    let payload = serde_json::json!({
        "generatedAt": timestamp,
        "renderer": renderer_snapshot
    });
    std::fs::write(
        &target,
        serde_json::to_vec_pretty(&payload).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    Ok(target.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn perf_start_trace() -> bool {
    false
}

#[tauri::command]
pub fn perf_stop_trace() -> Option<String> {
    None
}

#[tauri::command]
pub fn perf_open_log_dir(app: AppHandle) -> Result<String, String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("perf-logs");
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    opener::open(&directory).map_err(|error| error.to_string())?;
    Ok(directory.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn shortcuts_register(app: AppHandle, shortcuts: Vec<String>) -> Result<(), String> {
    let manager = app.global_shortcut();
    manager
        .unregister_all()
        .map_err(|error| error.to_string())?;
    for shortcut in shortcuts
        .into_iter()
        .filter(|shortcut| !shortcut.is_empty())
    {
        let parsed =
            ShortcutWrapper::try_from(shortcut.as_str()).map_err(|error| error.to_string())?;
        let event_app = app.clone();
        manager
            .register(parsed)
            .map_err(|error| error.to_string())?;
        manager
            .on_shortcut(
                ShortcutWrapper::try_from(shortcut.as_str()).map_err(|error| error.to_string())?,
                move |_app, _shortcut, _event| {
                    if let Err(error) = event_app.emit("shortcut:fired", shortcut.clone()) {
                        eprintln!("Shortcut event emission failed: {error}");
                    }
                },
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn window_show(app: AppHandle) -> tauri::Result<()> {
    main_window(&app)?.show()
}

#[tauri::command]
pub fn window_focus(app: AppHandle) -> tauri::Result<()> {
    let window = main_window(&app)?;
    window.show()?;
    window.set_focus()
}

#[tauri::command]
pub fn window_toggle_minimize(app: AppHandle) -> tauri::Result<()> {
    let window = main_window(&app)?;
    if window.is_minimized()? {
        window.unminimize()
    } else {
        window.minimize()
    }
}

#[tauri::command]
pub fn window_set_ignore_mouse(app: AppHandle, ignore: bool) -> tauri::Result<()> {
    main_window(&app)?.set_ignore_cursor_events(ignore)
}

#[tauri::command]
pub fn cursor_poll_start(app: AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.start_cursor_poll(&app)
}

#[tauri::command]
pub fn cursor_poll_stop(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.stop_cursor_poll()
}

#[tauri::command]
pub fn window_set_always_on_top(app: AppHandle, enabled: bool) -> tauri::Result<()> {
    main_window(&app)?.set_always_on_top(enabled)
}

#[tauri::command]
pub fn window_screenshot(app: AppHandle) -> Result<(), String> {
    let window = main_window(&app).map_err(|error| error.to_string())?;
    let position = window.outer_position().map_err(|error| error.to_string())?;
    let size = window.inner_size().map_err(|error| error.to_string())?;
    let screen = Screen::from_point(position.x, position.y).map_err(|error| error.to_string())?;
    let info = screen.display_info;
    let x = position.x - info.x;
    let y = position.y - info.y;
    let image = screen
        .capture_area(x, y, size.width, size.height)
        .map_err(|error| error.to_string())?;
    let mut clipboard = Clipboard::new().map_err(|error| error.to_string())?;
    clipboard
        .set_image(ImageData {
            width: image.width() as usize,
            height: image.height() as usize,
            bytes: Cow::Borrowed(image.as_raw()),
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn window_fit_content_width(app: AppHandle, desired_width: u32) -> tauri::Result<()> {
    let window = main_window(&app)?;
    let size = window.inner_size()?;
    if let Some(width) = target_width(size.width, desired_width) {
        window.set_size(tauri::Size::Physical(tauri::PhysicalSize {
            width,
            height: size.height,
        }))?;
    }
    Ok(())
}

#[tauri::command]
pub fn proxy_get_status(state: tauri::State<'_, AppState>) -> Result<ProxyStatus, String> {
    state.proxy_status()
}

#[tauri::command]
pub fn proxy_set_port(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    network: String,
    port: u16,
) -> Result<(), String> {
    state.set_port(&network, port)?;
    emit_proxy_status(&app, &state.proxy_status()?)
}

#[tauri::command]
pub fn proxy_set_bind_host(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    bind_host: String,
) -> Result<(), String> {
    state.set_bind_host(&bind_host)?;
    emit_proxy_status(&app, &state.proxy_status()?)
}

#[tauri::command]
pub async fn proxy_start(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<ProxyStatus, String> {
    let status = state.start_proxy(app.clone()).await?;
    emit_proxy_status(&app, &status)?;
    Ok(status)
}

#[tauri::command]
pub async fn proxy_configure_and_start(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    pika_port: u16,
    jartex_port: u16,
    bind_host: String,
) -> Result<ProxyStatus, String> {
    if state.proxy_configuration_matches(pika_port, jartex_port, &bind_host)? {
        return state.proxy_status();
    }
    if state.proxy_status()?.pika.running {
        state.stop_proxy()?;
    }
    state.set_port("pikanetwork", pika_port)?;
    state.set_port("jartexnetwork", jartex_port)?;
    state.set_bind_host(&bind_host)?;
    let status = state.start_proxy(app.clone()).await?;
    emit_proxy_status(&app, &status)?;
    Ok(status)
}

#[tauri::command]
pub fn proxy_stop(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<ProxyStatus, String> {
    let status = state.stop_proxy()?;
    emit_proxy_status(&app, &status)?;
    Ok(status)
}

#[tauri::command]
pub fn app_get_path(app: AppHandle, name: String) -> Result<String, String> {
    let path = match name.as_str() {
        "home" => app.path().home_dir(),
        "appData" => app.path().app_data_dir(),
        "userData" => app.path().app_config_dir(),
        "temp" => app.path().temp_dir(),
        "exe" => std::env::current_exe().map_err(tauri::Error::Io),
        "resources" => app.path().resource_dir(),
        _ => return Err("unsupported application path name".to_owned()),
    }
    .map_err(|error| error.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn app_read_file_base64(file_path: String) -> Result<String, String> {
    let data = std::fs::read(&file_path).map_err(|error| error.to_string())?;
    let mime = image_mime(Path::new(&file_path));
    Ok(format!("data:{mime};base64,{}", STANDARD.encode(data)))
}

#[tauri::command]
pub fn app_open_image_dialog(app: AppHandle) -> Result<OpenDialogResult, String> {
    let _ = app;
    open_file_dialog(&["png", "jpg", "jpeg", "gif", "webp", "bmp"])
}

#[tauri::command]
pub fn app_find_lunar_log(app: AppHandle) -> Result<String, String> {
    let home = app.path().home_dir().map_err(|error| error.to_string())?;
    let mut candidates = vec![
        home.join(".lunarclient").join("offline"),
        home.join(".lunarclient").join("profiles").join("lunar"),
        home.join(".lunarclient").join("profiles"),
    ];
    if cfg!(target_os = "windows") {
        if let Ok(app_data) = app.path().app_data_dir() {
            if let Some(parent) = app_data.parent() {
                candidates.push(parent.join("Local").join("lunarclient").join("offline"));
                candidates.push(
                    parent
                        .join("Local")
                        .join("lunarclient")
                        .join("profiles")
                        .join("lunar"),
                );
                candidates.push(parent.join("Local").join("lunarclient").join("profiles"));
            }
        }
    } else if cfg!(target_os = "macos") {
        if let Ok(app_data) = app.path().app_data_dir() {
            candidates.push(app_data.join("lunarclient").join("offline"));
            candidates.push(app_data.join("lunarclient").join("profiles").join("lunar"));
            candidates.push(app_data.join("lunarclient").join("profiles"));
        }
    }
    let mut latest: Option<(PathBuf, std::time::SystemTime)> = None;
    for base in candidates {
        let Ok(versions) = std::fs::read_dir(base) else {
            continue;
        };
        for version in versions.flatten() {
            let path = version.path().join("logs").join("latest.log");
            let Ok(metadata) = std::fs::metadata(&path) else {
                continue;
            };
            if !metadata.is_file() {
                continue;
            }
            let Ok(modified) = metadata.modified() else {
                continue;
            };
            if latest
                .as_ref()
                .map(|(_, current)| modified > *current)
                .unwrap_or(true)
            {
                latest = Some((path, modified));
            }
        }
    }
    Ok(latest
        .map(|(path, _)| path)
        .unwrap_or_else(|| {
            home.join(".lunarclient")
                .join("offline")
                .join("multiver")
                .join("logs")
                .join("latest.log")
        })
        .to_string_lossy()
        .into_owned())
}

#[tauri::command]
pub fn log_check_path(path: String) -> bool {
    std::fs::metadata(path)
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
}

#[tauri::command]
pub fn log_open_dialog(app: AppHandle) -> Result<OpenDialogResult, String> {
    let _ = app;
    open_file_dialog(&["log"])
}

#[tauri::command]
pub fn log_set_path(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    path: Option<String>,
) -> Result<(), String> {
    state.set_log_path(app, path)
}

#[cfg(test)]
mod tests {
    use super::{target_width, OpenDialogResult};
    use crate::state::{ProxyNetworkStatus, ProxyStatus};
    use std::path::Path;

    #[test]
    fn only_grows_within_configured_width_delta() {
        assert_eq!(target_width(800, 823), None);
        assert_eq!(target_width(800, 824), Some(824));
        assert_eq!(target_width(800, 1280), Some(1280));
        assert_eq!(target_width(800, 1281), None);
        assert_eq!(target_width(800, 700), None);
    }

    #[test]
    fn uses_expected_image_mime_types() {
        assert_eq!(
            super::image_mime(Path::new("background.JPEG")),
            "image/jpeg"
        );
        assert_eq!(
            super::image_mime(Path::new("background.unknown")),
            "image/png"
        );
    }

    #[test]
    fn preserves_ipc_payload_field_names() {
        let status = ProxyStatus {
            pika: ProxyNetworkStatus {
                running: true,
                port: 25566,
                bind_host: "127.0.0.1".to_owned(),
                client_count: 2,
                error: None,
            },
            jartex: ProxyNetworkStatus {
                running: false,
                port: 25567,
                bind_host: "127.0.0.1".to_owned(),
                client_count: 0,
                error: Some("offline".to_owned()),
            },
        };
        let value = serde_json::to_value(status).unwrap();
        assert_eq!(value["pika"]["bindHost"], "127.0.0.1");
        assert_eq!(value["pika"]["clientCount"], 2);
        assert_eq!(value["jartex"]["error"], "offline");
        assert!(value["pika"].get("bind_host").is_none());
    }

    #[test]
    fn preserves_dialog_result_shape() {
        let result = OpenDialogResult {
            canceled: false,
            file_paths: vec!["C:\\image.png".to_owned()],
        };
        let value = serde_json::to_value(result).unwrap();
        assert_eq!(value["canceled"], false);
        assert_eq!(value["filePaths"][0], "C:\\image.png");
    }

    #[test]
    fn normalizes_release_versions() {
        assert_eq!(super::release_version(Some("v1.11.1")), "1.11.1");
        assert_eq!(super::release_version(Some("1.12.0")), "1.12.0");
        assert_eq!(super::release_version(None), "");
    }
}
