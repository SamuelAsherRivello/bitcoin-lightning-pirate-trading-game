#[cfg(not(target_arch = "wasm32"))]
use std::fmt;

use serde_json::{json, Value};

use crate::client::models::PolarAutomationProfile;

const BRIDGE_REQUEST_TIMEOUT_SECONDS: u64 = 90;
pub const REQUIRED_POLAR_TOOLS: &[&str] = &[
    "list_networks",
    "create_network",
    "start_network",
    "add_node",
    "start_node",
    "deposit_funds",
    "get_wallet_balance",
    "get_blockchain_info",
    "mine_blocks",
    "open_channel",
    "list_channels",
    "close_channel",
    "create_invoice",
    "pay_invoice",
];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PolarConnectorLogLevel {
    Off,
    On,
    Verbose,
}

impl PolarConnectorLogLevel {
    fn allows(self, required: Self) -> bool {
        self >= required && self != Self::Off
    }
}

pub async fn get_json(
    profile: &PolarAutomationProfile,
    path: &str,
    log_level: PolarConnectorLogLevel,
) -> Result<Value, String> {
    validate_local_profile(profile)?;
    platform::get_json(profile, path, log_level).await
}

pub async fn post_json(
    profile: &PolarAutomationProfile,
    path: &str,
    body: Value,
    log_level: PolarConnectorLogLevel,
) -> Result<Value, String> {
    validate_local_profile(profile)?;
    platform::post_json(profile, path, body, log_level).await
}

pub async fn execute_tool(
    profile: &PolarAutomationProfile,
    tool: &str,
    arguments: Value,
    log_level: PolarConnectorLogLevel,
) -> Result<Value, String> {
    let response = post_json(
        profile,
        "/api/mcp/execute",
        json!({
            "tool": tool,
            "arguments": arguments,
        }),
        log_level,
    )
    .await?;

    match extract_mcp_error(&response) {
        Some(error) => Err(format_mcp_error(tool, &error)),
        None => Ok(response),
    }
}

pub fn bridge_url(profile: &PolarAutomationProfile, path: &str) -> String {
    format!(
        "{}{}",
        profile.bridge_url.trim().trim_end_matches('/'),
        path
    )
}

pub fn bridge_request_timeout_message(method: &str, url: &str) -> String {
    format!("Polar bridge {method} {url} timed out after {BRIDGE_REQUEST_TIMEOUT_SECONDS} seconds.")
}

pub fn is_transient_bridge_request_error(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    message.contains("polar bridge request failed")
        && (message.contains("failed to fetch")
            || message.contains("connection reset")
            || message.contains("connection refused")
            || message.contains("transport"))
}

pub fn format_mcp_error(tool: &str, error: &str) -> String {
    if is_missing_mcp_helper_error(error) {
        return format!(
            "Polar MCP tool {tool} could not run because Polar's automation helper file is missing. Restart Polar and the local Polar bridge, then retry the step."
        );
    }

    format!(
        "Polar MCP tool {tool} failed: {}",
        redact_sensitive_log_text(error)
    )
}

pub fn extract_mcp_error(value: &Value) -> Option<String> {
    let Value::Object(map) = value else {
        return None;
    };

    let success_is_false = map
        .get("success")
        .and_then(Value::as_bool)
        .map(|success| !success)
        .unwrap_or(false);

    match map.get("error") {
        Some(Value::String(error)) if !error.trim().is_empty() => Some(error.trim().to_string()),
        Some(Value::Object(error)) => error
            .get("message")
            .and_then(Value::as_str)
            .filter(|message| !message.trim().is_empty())
            .map(|message| message.trim().to_string())
            .or_else(|| {
                if success_is_false {
                    Some(Value::Object(error.clone()).to_string())
                } else {
                    None
                }
            }),
        Some(error) if success_is_false => Some(error.to_string()),
        _ if success_is_false => Some("Polar MCP tool returned success=false.".to_string()),
        _ => None,
    }
}

pub fn redact_sensitive_log_text(text: &str) -> String {
    let trimmed = text.trim();
    let normalized = trimmed.to_ascii_lowercase();
    if [
        "macaroon",
        "tls.cert",
        "tlscert",
        "credential",
        "password",
        "secret",
        "token",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
    {
        "[redacted sensitive Polar detail]".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn sanitized_log_value(value: &Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(key, value)| {
                    if is_sensitive_log_key(key) {
                        (key.clone(), Value::String("[redacted]".to_string()))
                    } else {
                        (key.clone(), sanitized_log_value(value))
                    }
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.iter().map(sanitized_log_value).collect()),
        _ => value.clone(),
    }
}

pub fn is_local_connector_url(url: &str) -> bool {
    PolarAutomationProfile::is_valid_local_bridge_url(url)
}

pub fn validate_local_profile(profile: &PolarAutomationProfile) -> Result<(), String> {
    if profile.is_local_bridge() {
        Ok(())
    } else {
        Err("The Polar MCP connector must use a local http://localhost:<port> or http://127.0.0.1:<port> bridge URL.".to_string())
    }
}

pub fn missing_required_tools<'a>(
    discovered_tools: impl IntoIterator<Item = &'a str>,
) -> Vec<&'static str> {
    let discovered = discovered_tools
        .into_iter()
        .map(str::trim)
        .collect::<std::collections::HashSet<_>>();

    REQUIRED_POLAR_TOOLS
        .iter()
        .copied()
        .filter(|tool| !discovered.contains(tool))
        .collect()
}

pub fn validate_required_tools<'a>(
    discovered_tools: impl IntoIterator<Item = &'a str>,
) -> Result<(), String> {
    let missing = missing_required_tools(discovered_tools);
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Polar MCP connector is missing required tool(s): {}.",
            missing.join(", ")
        ))
    }
}

fn is_sensitive_log_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key == "paths"
        || key.contains("macaroon")
        || key.contains("cert")
        || key.contains("credential")
        || key.contains("password")
        || key.contains("secret")
        || key.contains("token")
}

fn is_missing_mcp_helper_error(error: &str) -> bool {
    let normalized = error.to_ascii_lowercase();
    normalized.contains("enoent") && normalized.contains("no such file")
}

fn log_service_request(
    method: &str,
    url: &str,
    body: Option<&Value>,
    log_level: PolarConnectorLogLevel,
) {
    if !log_level.allows(PolarConnectorLogLevel::On) {
        return;
    }

    let message = match body {
        Some(body) => format!("[polar-service] request {method} {url} body={body}"),
        None => format!("[polar-service] request {method} {url}"),
    };
    log_to_terminal(&message);
}

fn log_service_response(
    method: &str,
    url: &str,
    result: &Result<Value, String>,
    log_level: PolarConnectorLogLevel,
) {
    if !log_level.allows(PolarConnectorLogLevel::On) {
        return;
    }

    let message = match result {
        Ok(value) if log_level >= PolarConnectorLogLevel::Verbose => {
            let value = sanitized_log_value(value);
            format!("[polar-service] response {method} {url} body={value}")
        }
        Ok(_) => format!("[polar-service] response {method} {url} ok"),
        Err(error) => format!(
            "[polar-service] error {method} {url} error={}",
            redact_sensitive_log_text(error)
        ),
    };
    log_to_terminal(&message);
}

#[cfg(target_arch = "wasm32")]
fn log_to_terminal(message: &str) {
    web_sys::console::log_1(&message.into());
}

#[cfg(not(target_arch = "wasm32"))]
fn log_to_terminal(message: &str) {
    println!("{message}");
}

#[cfg(not(target_arch = "wasm32"))]
fn is_transport_timeout(error: impl fmt::Display) -> bool {
    let normalized = error.to_string().to_ascii_lowercase();
    normalized.contains("timed out") || normalized.contains("timeout")
}

#[cfg(target_arch = "wasm32")]
async fn with_bridge_request_timeout<F, T>(future: F, method: &str, url: &str) -> Result<T, String>
where
    F: std::future::Future<Output = T>,
{
    let timeout =
        gloo_timers::future::TimeoutFuture::new((BRIDGE_REQUEST_TIMEOUT_SECONDS * 1_000) as u32);
    futures::pin_mut!(future);
    futures::pin_mut!(timeout);

    match futures::future::select(future, timeout).await {
        futures::future::Either::Left((value, _)) => Ok(value),
        futures::future::Either::Right((_, _)) => Err(bridge_request_timeout_message(method, url)),
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn run_bridge_request_on_worker<F>(request: F) -> Result<Value, String>
where
    F: FnOnce() -> Result<Value, String> + Send + 'static,
{
    let (sender, receiver) = futures::channel::oneshot::channel();
    std::thread::spawn(move || {
        let _ = sender.send(request());
    });

    receiver
        .await
        .map_err(|_| "Polar bridge worker stopped before returning a response.".to_string())?
}

#[cfg(target_arch = "wasm32")]
mod platform {
    use serde_json::Value;

    use super::{
        bridge_url, log_service_request, log_service_response, with_bridge_request_timeout,
        PolarConnectorLogLevel,
    };
    use crate::client::models::PolarAutomationProfile;

    pub async fn get_json(
        profile: &PolarAutomationProfile,
        path: &str,
        log_level: PolarConnectorLogLevel,
    ) -> Result<Value, String> {
        let url = bridge_url(profile, path);
        log_service_request("GET", &url, None, log_level);
        let result = match with_bridge_request_timeout(
            gloo_net::http::Request::get(&url).send(),
            "GET",
            &url,
        )
        .await
        {
            Err(error) => Err(error),
            Ok(Ok(response)) => response
                .json::<Value>()
                .await
                .map_err(|error| format!("Polar bridge returned invalid JSON: {error}")),
            Ok(Err(error)) => Err(format!("Cannot reach Polar bridge: {error}")),
        };
        log_service_response("GET", &url, &result, log_level);
        result
    }

    pub async fn post_json(
        profile: &PolarAutomationProfile,
        path: &str,
        body: Value,
        log_level: PolarConnectorLogLevel,
    ) -> Result<Value, String> {
        let url = bridge_url(profile, path);
        log_service_request("POST", &url, Some(&body), log_level);
        let result = match gloo_net::http::Request::post(&url).json(&body) {
            Ok(request) => match with_bridge_request_timeout(request.send(), "POST", &url).await {
                Err(error) => Err(error),
                Ok(Ok(response)) => response
                    .json::<Value>()
                    .await
                    .map_err(|error| format!("Polar bridge returned invalid JSON: {error}")),
                Ok(Err(error)) => Err(format!("Polar bridge request failed: {error}")),
            },
            Err(error) => Err(format!("Cannot encode Polar bridge request: {error}")),
        };
        log_service_response("POST", &url, &result, log_level);
        result
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod platform {
    use std::time::Duration;

    use serde_json::Value;

    use super::{
        bridge_request_timeout_message, bridge_url, is_transport_timeout, log_service_request,
        log_service_response, run_bridge_request_on_worker, PolarConnectorLogLevel,
        BRIDGE_REQUEST_TIMEOUT_SECONDS,
    };
    use crate::client::models::PolarAutomationProfile;

    pub async fn get_json(
        profile: &PolarAutomationProfile,
        path: &str,
        log_level: PolarConnectorLogLevel,
    ) -> Result<Value, String> {
        let url = bridge_url(profile, path);
        log_service_request("GET", &url, None, log_level);
        let request_url = url.clone();
        let result = run_bridge_request_on_worker(move || {
            match ureq::get(&request_url)
                .timeout(Duration::from_secs(BRIDGE_REQUEST_TIMEOUT_SECONDS))
                .call()
            {
                Ok(response) => response
                    .into_json::<Value>()
                    .map_err(|error| format!("Polar bridge returned invalid JSON: {error}")),
                Err(error) if is_transport_timeout(&error) => {
                    Err(bridge_request_timeout_message("GET", &request_url))
                }
                Err(error) => Err(format!("Cannot reach Polar bridge: {error}")),
            }
        })
        .await;
        log_service_response("GET", &url, &result, log_level);
        result
    }

    pub async fn post_json(
        profile: &PolarAutomationProfile,
        path: &str,
        body: Value,
        log_level: PolarConnectorLogLevel,
    ) -> Result<Value, String> {
        let url = bridge_url(profile, path);
        log_service_request("POST", &url, Some(&body), log_level);
        let request_url = url.clone();
        let result = run_bridge_request_on_worker(move || {
            match ureq::post(&request_url)
                .timeout(Duration::from_secs(BRIDGE_REQUEST_TIMEOUT_SECONDS))
                .set("Content-Type", "application/json")
                .send_json(body)
            {
                Ok(response) => response
                    .into_json::<Value>()
                    .map_err(|error| format!("Polar bridge returned invalid JSON: {error}")),
                Err(error) if is_transport_timeout(&error) => {
                    Err(bridge_request_timeout_message("POST", &request_url))
                }
                Err(error) => Err(format!("Polar bridge request failed: {error}")),
            }
        })
        .await;
        log_service_response("POST", &url, &result, log_level);
        result
    }
}
