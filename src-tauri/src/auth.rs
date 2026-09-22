use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;

const CLIENT_ID: &str = "00000000402b5328";
const DEVICE_CODE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const XBOX_USER_AUTH_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_AUTH_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MINECRAFT_AUTH_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MINECRAFT_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";
const SCOPE: &str = "XboxLive.signin offline_access";
const MIN_POLL_INTERVAL_SECONDS: u64 = 1;
const MAX_POLL_INTERVAL_SECONDS: u64 = 60;
const MAX_DEVICE_CODE_LIFETIME_SECONDS: u64 = 1800;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("authentication response was invalid: {0}")]
    InvalidResponse(String),
    #[error("authentication was denied or expired")]
    Denied,
    #[error("secure credential storage failed: {0}")]
    Storage(String),
    #[error("credential serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCode {
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in_seconds: u64,
    pub interval_seconds: u64,
    pub device_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MinecraftProfile {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftToken {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64,
    pub profile: MinecraftProfile,
}

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct XboxResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: DisplayClaims,
}

#[derive(Debug, Deserialize)]
struct DisplayClaims {
    xui: Vec<XuiClaim>,
}

#[derive(Debug, Deserialize)]
struct XuiClaim {
    uhs: String,
}

#[derive(Debug, Deserialize)]
struct MinecraftAuthResponse {
    access_token: String,
    expires_in: u64,
}

pub async fn begin(client: &Client) -> Result<DeviceCode, AuthError> {
    let response = client
        .post(DEVICE_CODE_URL)
        .form(&[("client_id", CLIENT_ID), ("scope", SCOPE)])
        .send()
        .await?
        .error_for_status()?
        .json::<DeviceCodeResponse>()
        .await?;
    Ok(DeviceCode {
        user_code: response.user_code,
        verification_uri: response.verification_uri,
        expires_in_seconds: response.expires_in,
        interval_seconds: response.interval.unwrap_or(5),
        device_code: response.device_code,
    })
}

pub async fn finish(
    client: &Client,
    device_code: &str,
    interval_seconds: u64,
    expires_in_seconds: u64,
) -> Result<MinecraftToken, AuthError> {
    let interval_seconds =
        interval_seconds.clamp(MIN_POLL_INTERVAL_SECONDS, MAX_POLL_INTERVAL_SECONDS);
    let expires_in_seconds = expires_in_seconds.min(MAX_DEVICE_CODE_LIFETIME_SECONDS);
    if expires_in_seconds == 0 {
        return Err(AuthError::Denied);
    }
    let deadline = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| AuthError::InvalidResponse(error.to_string()))?
        .as_secs()
        .saturating_add(expires_in_seconds);
    loop {
        let response = client
            .post(TOKEN_URL)
            .form(&[
                ("client_id", CLIENT_ID),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("device_code", device_code),
            ])
            .send()
            .await?;
        let payload = response.json::<TokenResponse>().await?;
        if let Some(access_token) = payload.access_token {
            let refresh_token = payload
                .refresh_token
                .ok_or_else(|| AuthError::InvalidResponse("missing refresh token".to_owned()))?;
            return exchange_minecraft(
                client,
                &access_token,
                &refresh_token,
                payload.expires_in.unwrap_or(3600),
            )
            .await;
        }
        match payload.error.as_deref() {
            Some("authorization_pending") => {}
            Some("slow_down") => {
                tokio::time::sleep(Duration::from_secs(interval_seconds + 5)).await
            }
            Some("expired_token") | Some("access_denied") => return Err(AuthError::Denied),
            Some(error) => return Err(AuthError::InvalidResponse(error.to_owned())),
            None => {
                return Err(AuthError::InvalidResponse(
                    "missing token response".to_owned(),
                ))
            }
        }
        if SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| AuthError::InvalidResponse(error.to_string()))?
            .as_secs()
            >= deadline
        {
            return Err(AuthError::Denied);
        }
        tokio::time::sleep(Duration::from_secs(interval_seconds)).await;
    }
}

pub async fn refresh(client: &Client, token: &MinecraftToken) -> Result<MinecraftToken, AuthError> {
    let response = client
        .post(TOKEN_URL)
        .form(&[
            ("client_id", CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("refresh_token", token.refresh_token.as_str()),
            ("scope", SCOPE),
        ])
        .send()
        .await?
        .error_for_status()?
        .json::<TokenResponse>()
        .await?;
    let microsoft_token = response
        .access_token
        .ok_or_else(|| AuthError::InvalidResponse("missing refreshed access token".to_owned()))?;
    let refresh_token = response
        .refresh_token
        .unwrap_or_else(|| token.refresh_token.clone());
    exchange_minecraft(
        client,
        &microsoft_token,
        &refresh_token,
        response.expires_in.unwrap_or(3600),
    )
    .await
}

async fn exchange_minecraft(
    client: &Client,
    microsoft_token: &str,
    refresh_token: &str,
    expires_in: u64,
) -> Result<MinecraftToken, AuthError> {
    let xbox_user = client
        .post(XBOX_USER_AUTH_URL)
        .json(&serde_json::json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={microsoft_token}")
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }))
        .send()
        .await?
        .error_for_status()?
        .json::<XboxResponse>()
        .await?;
    let user_hash = xbox_user
        .display_claims
        .xui
        .first()
        .map(|claim| claim.uhs.clone())
        .ok_or_else(|| AuthError::InvalidResponse("missing Xbox user hash".to_owned()))?;
    let xsts = client
        .post(XSTS_AUTH_URL)
        .json(&serde_json::json!({
            "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbox_user.token] },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }))
        .send()
        .await?
        .error_for_status()?
        .json::<XboxResponse>()
        .await?;
    let minecraft = client
        .post(MINECRAFT_AUTH_URL)
        .json(&serde_json::json!({
            "identityToken": format!("XBL3.0 x={user_hash};{}", xsts.token)
        }))
        .send()
        .await?
        .error_for_status()?
        .json::<MinecraftAuthResponse>()
        .await?;
    let profile = client
        .get(MINECRAFT_PROFILE_URL)
        .bearer_auth(&minecraft.access_token)
        .send()
        .await?
        .error_for_status()?
        .json::<MinecraftProfile>()
        .await?;
    let expires_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| AuthError::InvalidResponse(error.to_string()))?
        .as_secs()
        .saturating_add(expires_in.min(minecraft.expires_in));
    Ok(MinecraftToken {
        access_token: minecraft.access_token,
        refresh_token: refresh_token.to_owned(),
        expires_at,
        profile,
    })
}

pub fn load() -> Result<Option<MinecraftToken>, AuthError> {
    let entry = keyring::Entry::new("kyra-overlay", "microsoft-minecraft-token")
        .map_err(|error| AuthError::Storage(error.to_string()))?;
    match entry.get_password() {
        Ok(value) => Ok(Some(serde_json::from_str(&value)?)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(AuthError::Storage(error.to_string())),
    }
}

pub fn save(token: &MinecraftToken) -> Result<(), AuthError> {
    let entry = keyring::Entry::new("kyra-overlay", "microsoft-minecraft-token")
        .map_err(|error| AuthError::Storage(error.to_string()))?;
    entry
        .set_password(&serde_json::to_string(token)?)
        .map_err(|error| AuthError::Storage(error.to_string()))
}

pub fn clear() -> Result<(), AuthError> {
    let entry = keyring::Entry::new("kyra-overlay", "microsoft-minecraft-token")
        .map_err(|error| AuthError::Storage(error.to_string()))?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(AuthError::Storage(error.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DeviceCodeResponse, MinecraftProfile, TokenResponse, MAX_DEVICE_CODE_LIFETIME_SECONDS,
        MAX_POLL_INTERVAL_SECONDS, MIN_POLL_INTERVAL_SECONDS,
    };

    #[test]
    fn parses_device_code_response_defaults() {
        let response: DeviceCodeResponse = serde_json::from_str(
            r#"{
                "device_code": "device",
                "user_code": "ABCD-EFGH",
                "verification_uri": "https://microsoft.com/devicelogin",
                "expires_in": 900
            }"#,
        )
        .unwrap();
        assert_eq!(response.device_code, "device");
        assert_eq!(response.user_code, "ABCD-EFGH");
        assert_eq!(response.interval, None);
    }

    #[test]
    fn parses_pending_and_denied_token_responses() {
        let pending: TokenResponse =
            serde_json::from_str(r#"{"error":"authorization_pending"}"#).unwrap();
        let denied: TokenResponse = serde_json::from_str(r#"{"error":"access_denied"}"#).unwrap();
        assert_eq!(pending.error.as_deref(), Some("authorization_pending"));
        assert_eq!(denied.error.as_deref(), Some("access_denied"));
        assert!(pending.access_token.is_none());
        assert!(denied.refresh_token.is_none());
    }

    #[test]
    fn profile_serializes_with_stable_renderer_fields() {
        let profile = MinecraftProfile {
            id: "profile-id".to_owned(),
            name: "Player".to_owned(),
        };
        assert_eq!(
            serde_json::to_value(profile).unwrap(),
            serde_json::json!({
                "id": "profile-id",
                "name": "Player"
            })
        );
    }

    #[test]
    fn polling_limits_are_positive_and_bounded() {
        assert_eq!(MIN_POLL_INTERVAL_SECONDS, 1);
        assert_eq!(MAX_POLL_INTERVAL_SECONDS, 60);
        assert_eq!(MAX_DEVICE_CODE_LIFETIME_SECONDS, 1800);
    }
}
