use crate::local_secret;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub enum TrafficMode {
    #[default]
    Middleware,
    Direct,
}

#[derive(Default, Deserialize, Serialize)]
struct TrafficConfig {
    mode: TrafficMode,
    #[serde(default, rename = "hasApiKey")]
    has_api_key: bool,
    #[serde(
        default,
        rename = "encryptedApiKey",
        skip_serializing_if = "Option::is_none"
    )]
    encrypted_api_key: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficStatus {
    mode: TrafficMode,
    has_api_key: bool,
}

#[derive(Default)]
pub struct TrafficState(Mutex<()>);

pub struct StatsRoute {
    pub base: &'static str,
    pub headers: HeaderMap,
}

fn entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new("kyra-overlay", "pika-stats-api-key").map_err(|_| {
        "Secure storage is unavailable. Unlock your system keyring and try again.".to_owned()
    })
}

fn api_key(directory: &Path, config: &TrafficConfig) -> Result<Option<String>, String> {
    if let Some(encrypted) = &config.encrypted_api_key {
        return local_secret::decrypt(directory, encrypted).map(Some);
    }
    match entry()?.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => {
            Err("Could not read the API key. Unlock your system keyring and try again.".to_owned())
        }
    }
}

fn load(directory: &Path) -> Result<TrafficConfig, String> {
    match std::fs::read(directory.join("traffic.json")) {
        Ok(data) => serde_json::from_slice(&data).map_err(|error| error.to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(TrafficConfig::default()),
        Err(error) => Err(error.to_string()),
    }
}

fn persist(directory: &Path, config: &TrafficConfig) -> Result<(), String> {
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let data = serde_json::to_vec(config).map_err(|error| error.to_string())?;
    let temporary = directory.join("traffic.json.tmp");
    std::fs::write(&temporary, data).map_err(|error| error.to_string())?;
    std::fs::rename(temporary, directory.join("traffic.json")).map_err(|error| error.to_string())
}

fn route(network: &str, mode: TrafficMode, key: Option<&str>) -> Result<StatsRoute, String> {
    let mut headers = HeaderMap::new();
    let base = match (network, mode) {
        ("jartex", _) => "https://stats.jartexnetwork.com/api",
        ("pika", TrafficMode::Middleware) => "https://proxy.kyizl.is-a.dev/api",
        ("pika", TrafficMode::Direct) => {
            let key = key.ok_or("Save an API key in Settings → Traffic first.")?;
            let mut value =
                HeaderValue::from_str(key).map_err(|_| "The saved API key is invalid.")?;
            value.set_sensitive(true);
            headers.insert("X-Api-Key", value);
            "https://stats.pika-network.net/api"
        }
        _ => return Err("unsupported statistics network".to_owned()),
    };
    Ok(StatsRoute { base, headers })
}

impl TrafficState {
    pub fn stats_route(&self, app: &AppHandle, network: &str) -> Result<StatsRoute, String> {
        if network == "jartex" {
            return route(network, TrafficMode::Middleware, None);
        }
        let _guard = self
            .0
            .lock()
            .map_err(|_| "Traffic settings lock poisoned.")?;
        let directory = app
            .path()
            .app_data_dir()
            .map_err(|error| error.to_string())?;
        let config = load(&directory)?;
        let key = if config.mode == TrafficMode::Direct {
            api_key(&directory, &config)?
        } else {
            None
        };
        route(network, config.mode, key.as_deref())
    }
}

#[tauri::command]
pub fn traffic_get(
    app: AppHandle,
    state: State<'_, TrafficState>,
) -> Result<TrafficStatus, String> {
    let _guard = state
        .inner()
        .0
        .lock()
        .map_err(|_| "Traffic settings lock poisoned.")?;
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let config = load(&directory)?;
    Ok(TrafficStatus {
        mode: config.mode,
        has_api_key: config.has_api_key,
    })
}

#[tauri::command]
pub fn traffic_set_mode(
    app: AppHandle,
    state: State<'_, TrafficState>,
    mode: TrafficMode,
) -> Result<TrafficStatus, String> {
    let _guard = state
        .inner()
        .0
        .lock()
        .map_err(|_| "Traffic settings lock poisoned.")?;
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let mut config = load(&directory)?;
    if mode == TrafficMode::Direct && api_key(&directory, &config)?.is_none() {
        return Err("Save an API key in Settings → Traffic first.".to_owned());
    }
    config.mode = mode;
    persist(&directory, &config)?;
    Ok(TrafficStatus {
        mode,
        has_api_key: config.has_api_key,
    })
}

fn save_api_key(
    directory: &Path,
    value: &str,
    store_system_key: impl FnOnce(&str) -> Result<(), String>,
) -> Result<TrafficStatus, String> {
    let key = value.trim();
    if key.is_empty() || !key.bytes().all(|byte| (0x21..=0x7e).contains(&byte)) {
        return Err("Enter a valid API key without spaces or control characters.".to_owned());
    }
    let current = load(directory)?;
    let encrypted_api_key = if current.encrypted_api_key.is_some() {
        Some(local_secret::encrypt(directory, key)?)
    } else {
        match store_system_key(key) {
            Ok(()) => None,
            Err(error) if current.has_api_key => return Err(error),
            Err(_) => Some(local_secret::encrypt(directory, key)?),
        }
    };
    persist(
        directory,
        &TrafficConfig {
            mode: TrafficMode::Direct,
            has_api_key: true,
            encrypted_api_key,
        },
    )?;
    Ok(TrafficStatus {
        mode: TrafficMode::Direct,
        has_api_key: true,
    })
}

fn remove_api_key(
    directory: &Path,
    remove_system_key: impl FnOnce() -> Result<(), String>,
) -> Result<TrafficStatus, String> {
    let config = load(directory)?;
    if config.encrypted_api_key.is_none() && config.has_api_key {
        remove_system_key()?;
    }
    persist(directory, &TrafficConfig::default())?;
    Ok(TrafficStatus {
        mode: TrafficMode::Middleware,
        has_api_key: false,
    })
}

#[tauri::command]
pub fn traffic_save_api_key(
    app: AppHandle,
    state: State<'_, TrafficState>,
    api_key: String,
) -> Result<TrafficStatus, String> {
    let _guard = state
        .inner()
        .0
        .lock()
        .map_err(|_| "Traffic settings lock poisoned.")?;
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    save_api_key(&directory, &api_key, |key| {
        entry()?.set_password(key).map_err(|_| {
            "Could not save the API key. Unlock your system keyring and try again.".to_owned()
        })
    })
}

#[tauri::command]
pub fn traffic_remove_api_key(
    app: AppHandle,
    state: State<'_, TrafficState>,
) -> Result<TrafficStatus, String> {
    let _guard = state
        .inner()
        .0
        .lock()
        .map_err(|_| "Traffic settings lock poisoned.")?;
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    remove_api_key(&directory, || match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err(
            "Could not remove the API key. Unlock your system keyring and try again.".to_owned(),
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_middleware_and_persists_mode_without_secrets() {
        let directory = tempfile::tempdir().unwrap();
        assert_eq!(
            load(directory.path()).unwrap().mode,
            TrafficMode::Middleware
        );
        persist(
            directory.path(),
            &TrafficConfig {
                mode: TrafficMode::Direct,
                has_api_key: true,
                encrypted_api_key: None,
            },
        )
        .unwrap();
        assert_eq!(load(directory.path()).unwrap().mode, TrafficMode::Direct);
        let data = std::fs::read_to_string(directory.path().join("traffic.json")).unwrap();
        assert_eq!(data, r#"{"mode":"direct","hasApiKey":true}"#);
        persist(directory.path(), &TrafficConfig::default()).unwrap();
        assert_eq!(
            load(directory.path()).unwrap().mode,
            TrafficMode::Middleware
        );
    }

    #[test]
    fn saves_reloads_replaces_and_removes_keys_without_a_system_keyring() {
        let directory = tempfile::tempdir().unwrap();
        let status = save_api_key(directory.path(), "  local-test-secret  ", |_| {
            Err("No system keyring".to_owned())
        })
        .unwrap();
        assert_eq!(status.mode, TrafficMode::Direct);
        assert!(status.has_api_key);
        let config = load(directory.path()).unwrap();
        assert_eq!(
            api_key(directory.path(), &config).unwrap().as_deref(),
            Some("local-test-secret")
        );
        assert!(
            !std::fs::read_to_string(directory.path().join("traffic.json"))
                .unwrap()
                .contains("local-test-secret")
        );
        save_api_key(directory.path(), "replacement-secret", |_| {
            panic!("Local keys do not require a system keyring")
        })
        .unwrap();
        let config = load(directory.path()).unwrap();
        assert_eq!(
            api_key(directory.path(), &config).unwrap().as_deref(),
            Some("replacement-secret")
        );
        let status = remove_api_key(directory.path(), || {
            panic!("Local keys do not require a system keyring")
        })
        .unwrap();
        assert_eq!(status.mode, TrafficMode::Middleware);
        assert!(!status.has_api_key);
        assert!(load(directory.path()).unwrap().encrypted_api_key.is_none());
        assert_eq!(
            std::fs::read_to_string(directory.path().join("traffic.json")).unwrap(),
            r#"{"mode":"middleware","hasApiKey":false}"#
        );
    }

    #[test]
    fn preserves_system_stored_keys_when_a_locked_vault_rejects_a_change() {
        let directory = tempfile::tempdir().unwrap();
        save_api_key(directory.path(), "system-secret", |_| Ok(())).unwrap();
        let original = std::fs::read(directory.path().join("traffic.json")).unwrap();
        assert!(save_api_key(
            directory.path(),
            "replacement",
            |_| Err("locked".to_owned())
        )
        .is_err());
        assert!(remove_api_key(directory.path(), || Err("locked".to_owned())).is_err());
        assert_eq!(
            std::fs::read(directory.path().join("traffic.json")).unwrap(),
            original
        );
        assert!(load(directory.path()).unwrap().encrypted_api_key.is_none());
    }

    #[test]
    fn only_direct_pika_requests_receive_the_api_key() {
        let direct = route("pika", TrafficMode::Direct, Some("test-key")).unwrap();
        assert_eq!(direct.base, "https://stats.pika-network.net/api");
        assert_eq!(direct.headers["X-Api-Key"], "test-key");
        assert!(direct.headers["X-Api-Key"].is_sensitive());
        let middleware = route("pika", TrafficMode::Middleware, Some("test-key")).unwrap();
        assert_eq!(middleware.base, "https://proxy.kyizl.is-a.dev/api");
        assert!(middleware.headers.is_empty());
        let jartex = route("jartex", TrafficMode::Direct, Some("test-key")).unwrap();
        assert_eq!(jartex.base, "https://stats.jartexnetwork.com/api");
        assert!(jartex.headers.is_empty());
        assert!(route("pika", TrafficMode::Direct, None).is_err());
        assert!(route("pika", TrafficMode::Direct, Some("bad\r\nkey")).is_err());
        assert!(route("unknown", TrafficMode::Middleware, None).is_err());
    }
}
