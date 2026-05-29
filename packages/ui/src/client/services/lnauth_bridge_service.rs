use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::client::models::{
    AuthAction, AuthSessionStatus, PlayerAuthSession, PlayerIdentity, UserAuthMode,
};

const LNAUTH_BRIDGE_PORT: u16 = 37374;
#[cfg(not(target_arch = "wasm32"))]
const LNAUTH_BRIDGE_TIMEOUT_SECONDS: u64 = 8;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
enum BridgeSessionStatus {
    Created,
    Approved,
    Expired,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct BridgeSessionResponse {
    session_id: String,
    challenge_id: String,
    lnurl: String,
    qr_payload: String,
    action: AuthAction,
    status: BridgeSessionStatus,
    expires_at: chrono::DateTime<Utc>,
    linking_key_fingerprint: Option<String>,
    failure_reason: Option<String>,
}

pub async fn begin_real_player_auth(
    bridge_url: String,
    action: AuthAction,
) -> Result<PlayerAuthSession, String> {
    let base_url = normalize_bridge_url(&bridge_url)?;
    let response = post_json(
        &format!("{base_url}/api/lnauth/session"),
        json!({
            "action": action,
            "callback_base_url": base_url,
        }),
    )
    .await?;
    let session: BridgeSessionResponse = serde_json::from_value(response)
        .map_err(|error| format!("LNAuth bridge returned invalid session JSON: {error}"))?;
    Ok(session.into_player_auth_session())
}

pub async fn get_real_player_auth_session(
    bridge_url: String,
    session_id: &str,
) -> Result<PlayerAuthSession, String> {
    let base_url = normalize_bridge_url(&bridge_url)?;
    let response = get_json(&format!("{base_url}/api/lnauth/session/{session_id}")).await?;
    let session: BridgeSessionResponse = serde_json::from_value(response)
        .map_err(|error| format!("LNAuth bridge returned invalid session JSON: {error}"))?;
    Ok(session.into_player_auth_session())
}

pub async fn test_lnauth_bridge_url(bridge_url: String) -> Result<(), String> {
    let base_url = normalize_bridge_url(&bridge_url)?;
    let response = get_json(&format!("{base_url}/api/lnauth/health")).await?;
    if response.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        Ok(())
    } else {
        Err("LNAuth bridge did not return a healthy response.".to_string())
    }
}

pub fn real_lnauth_bridge_hint() -> String {
    format!(
        "Start the LNAuth bridge with .\\Scripts\\Common\\RunWeb.ps1. The app may use localhost, and the bridge advertises a phone-reachable LAN callback on port {LNAUTH_BRIDGE_PORT} when a Wi-Fi IPv4 address is available."
    )
}

pub fn default_bridge_base_url() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let host = web_sys::window()
            .and_then(|window| window.location().hostname().ok())
            .filter(|host| !host.trim().is_empty())
            .unwrap_or_else(|| "127.0.0.1".to_string());
        format!("http://{host}:{LNAUTH_BRIDGE_PORT}")
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        format!("http://127.0.0.1:{LNAUTH_BRIDGE_PORT}")
    }
}

pub fn is_valid_lnauth_bridge_url(bridge_url: &str) -> bool {
    normalize_bridge_url(bridge_url).is_ok()
}

fn normalize_bridge_url(bridge_url: &str) -> Result<String, String> {
    let trimmed = bridge_url.trim().trim_end_matches('/');
    if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        return Err("Use an LNAuth bridge URL such as http://192.168.1.20:37374.".to_string());
    }

    let without_scheme = trimmed
        .strip_prefix("http://")
        .or_else(|| trimmed.strip_prefix("https://"))
        .unwrap_or(trimmed);
    if without_scheme.contains('/') || without_scheme.contains('?') || without_scheme.contains('#')
    {
        return Err("Use only the LNAuth bridge origin, without a path or query.".to_string());
    }

    let Some((host, port)) = without_scheme.rsplit_once(':') else {
        return Err("Include the LNAuth bridge port, usually 37374.".to_string());
    };
    if host.trim().is_empty() || !port.parse::<u16>().is_ok_and(|port| port > 0) {
        return Err("Use a valid LNAuth bridge host and port.".to_string());
    }

    let scheme = if trimmed.starts_with("https://") {
        "https"
    } else {
        "http"
    };
    let normalized_host =
        if host.eq_ignore_ascii_case("localhost") || matches!(host, "::1" | "[::1]") {
            "127.0.0.1"
        } else {
            host
        };

    Ok(format!("{scheme}://{normalized_host}:{port}"))
}

impl BridgeSessionResponse {
    fn into_player_auth_session(self) -> PlayerAuthSession {
        let status = match self.status {
            BridgeSessionStatus::Created => AuthSessionStatus::Created,
            BridgeSessionStatus::Approved => AuthSessionStatus::Approved,
            BridgeSessionStatus::Expired => AuthSessionStatus::Expired,
            BridgeSessionStatus::Failed => AuthSessionStatus::Failed,
        };
        let player_identity = self
            .linking_key_fingerprint
            .clone()
            .map(|linking_key_fingerprint| PlayerIdentity {
                display_label: "LNAuth player".to_string(),
                linking_key_fingerprint,
                authenticated_at: Utc::now(),
                last_seen_at: Some(Utc::now()),
            });

        PlayerAuthSession {
            session_id: self.session_id,
            challenge_id: self.challenge_id,
            lnurl: self.lnurl,
            qr_payload: self.qr_payload,
            action: self.action,
            status,
            expires_at: Some(self.expires_at),
            player_identity,
            failure_reason: self.failure_reason,
        }
    }
}

pub fn should_use_real_bridge(mode: UserAuthMode) -> bool {
    mode == UserAuthMode::LnAuth
}

#[cfg(target_arch = "wasm32")]
async fn get_json(url: &str) -> Result<Value, String> {
    match gloo_net::http::Request::get(url).send().await {
        Ok(response) => response
            .json::<Value>()
            .await
            .map_err(|error| format!("LNAuth bridge returned invalid JSON: {error}")),
        Err(error) => Err(format!(
            "Cannot reach LNAuth bridge at {url}: {error}. {}",
            real_lnauth_bridge_hint()
        )),
    }
}

#[cfg(target_arch = "wasm32")]
async fn post_json(url: &str, body: Value) -> Result<Value, String> {
    match gloo_net::http::Request::post(url).json(&body) {
        Ok(request) => match request.send().await {
            Ok(response) => response
                .json::<Value>()
                .await
                .map_err(|error| format!("LNAuth bridge returned invalid JSON: {error}")),
            Err(error) => Err(format!(
                "Cannot reach LNAuth bridge at {url}: {error}. {}",
                real_lnauth_bridge_hint()
            )),
        },
        Err(error) => Err(format!("Cannot encode LNAuth bridge request: {error}")),
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn get_json(url: &str) -> Result<Value, String> {
    let url = url.to_string();
    run_bridge_request(move || {
        let response = ureq::get(&url)
            .timeout(std::time::Duration::from_secs(
                LNAUTH_BRIDGE_TIMEOUT_SECONDS,
            ))
            .call()
            .map_err(|error| {
                format!(
                    "Cannot reach LNAuth bridge at {url}: {error}. {}",
                    real_lnauth_bridge_hint()
                )
            })?;
        response.into_json::<Value>().map_err(|error| {
            format!(
                "LNAuth bridge returned invalid JSON from {url}: {error}. {}",
                real_lnauth_bridge_hint()
            )
        })
    })
    .await
}

#[cfg(not(target_arch = "wasm32"))]
async fn post_json(url: &str, body: Value) -> Result<Value, String> {
    let url = url.to_string();
    run_bridge_request(move || {
        let response = ureq::post(&url)
            .timeout(std::time::Duration::from_secs(
                LNAUTH_BRIDGE_TIMEOUT_SECONDS,
            ))
            .set("Content-Type", "application/json")
            .send_json(body)
            .map_err(|error| {
                format!(
                    "Cannot reach LNAuth bridge at {url}: {error}. {}",
                    real_lnauth_bridge_hint()
                )
            })?;
        response.into_json::<Value>().map_err(|error| {
            format!(
                "LNAuth bridge returned invalid JSON from {url}: {error}. {}",
                real_lnauth_bridge_hint()
            )
        })
    })
    .await
}

#[cfg(not(target_arch = "wasm32"))]
async fn run_bridge_request<F>(request: F) -> Result<Value, String>
where
    F: FnOnce() -> Result<Value, String> + Send + 'static,
{
    let (sender, receiver) = futures::channel::oneshot::channel();
    std::thread::spawn(move || {
        let _ = sender.send(request());
    });

    receiver
        .await
        .map_err(|_| "LNAuth bridge worker stopped before returning a response.".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_lan_lnauth_bridge_url_with_port() {
        assert!(is_valid_lnauth_bridge_url("http://192.168.15.51:37374"));
    }

    #[test]
    fn rejects_lnauth_bridge_url_without_port() {
        assert!(!is_valid_lnauth_bridge_url("http://192.168.15.51"));
    }

    #[test]
    fn rejects_lnauth_bridge_url_with_path() {
        assert!(!is_valid_lnauth_bridge_url(
            "http://192.168.15.51:37374/api/lnauth/health"
        ));
    }

    #[test]
    fn normalizes_localhost_bridge_url_to_ipv4_loopback() {
        assert_eq!(
            normalize_bridge_url("http://localhost:37374").as_deref(),
            Ok("http://127.0.0.1:37374")
        );
    }
}
