use std::collections::HashSet;

use serde_json::{json, Value};

use crate::client::models::{
    DemoNodeId, PolarAutomationProfile, DEFAULT_BITCOIN_BACKEND_NAME, DEFAULT_NETWORK_NAME,
};

const DEMO_NODE_FUNDING_SATS: u64 = 1_000_000;
const LOG_SERVICE_CALLS_TO_TERMINAL: bool = true;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolarServerEnsureStatus {
    Created,
    Existed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PolarServerEnsureResult {
    pub profile: PolarAutomationProfile,
    pub status: PolarServerEnsureStatus,
}

pub async fn test_bridge(profile: &PolarAutomationProfile) -> Result<(), String> {
    let response = get_json(profile, "/health").await?;
    let status = response
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or_default();

    if status == "ok" {
        Ok(())
    } else {
        Err("Polar bridge did not return a healthy status.".to_string())
    }
}

pub async fn ensure_server(
    profile: &PolarAutomationProfile,
) -> Result<PolarServerEnsureResult, String> {
    test_bridge(profile).await?;

    let requested_name = clean_network_id(profile);
    if requested_name.is_empty() {
        return Err("Enter a Polar server name before creating it.".to_string());
    }

    let networks = list_networks(profile).await?;
    if let Some(network_id) = find_network_id(&networks, &requested_name) {
        ensure_network_running(profile, &network_id).await?;
        let networks = list_networks(profile).await?;
        return Ok(PolarServerEnsureResult {
            profile: automation_profile_from_network(profile, &networks, network_id),
            status: PolarServerEnsureStatus::Existed,
        });
    }

    create_network(profile, &requested_name).await?;

    let networks = list_networks(profile).await?;
    let network_id = find_network_id(&networks, &requested_name)
        .unwrap_or_else(|| requested_name.trim().to_string());
    ensure_network_running(profile, &network_id).await?;
    let networks = list_networks(profile).await?;

    Ok(PolarServerEnsureResult {
        profile: automation_profile_from_network(profile, &networks, network_id),
        status: PolarServerEnsureStatus::Created,
    })
}

pub async fn resolve_automation_profile(
    profile: &PolarAutomationProfile,
) -> Result<PolarAutomationProfile, String> {
    test_bridge(profile).await?;
    let networks = list_networks(profile).await?;
    let network_id = resolve_network_id(profile, &networks)?;
    let bitcoin_backend_name = resolve_backend_name(profile, &networks, &network_id);

    Ok(PolarAutomationProfile {
        bridge_url: profile.bridge_url.trim().to_string(),
        network_id,
        bitcoin_backend_name,
    })
}

pub async fn create_demo_nodes(
    profile: &PolarAutomationProfile,
) -> Result<PolarAutomationProfile, String> {
    let resolved_profile = resolve_started_automation_profile(profile).await?;
    let network_id = clean_network_id(&resolved_profile);
    let backend_name = clean_backend_name(&resolved_profile);

    for node_id in DemoNodeId::ALL {
        create_or_prepare_demo_node(&resolved_profile, &network_id, &backend_name, node_id).await?;
    }

    execute_tool(
        &resolved_profile,
        "mine_blocks",
        json!({
            "networkId": network_id_argument(&network_id),
            "blocks": 6,
            "nodeName": backend_name,
        }),
    )
    .await?;

    Ok(resolved_profile)
}

pub async fn get_blockchain_height(profile: &PolarAutomationProfile) -> Result<u64, String> {
    let resolved_profile = resolve_started_automation_profile(profile).await?;
    get_blockchain_height_from_resolved(&resolved_profile).await
}

pub async fn mine_blocks(profile: &PolarAutomationProfile, blocks: u64) -> Result<u64, String> {
    let resolved_profile = resolve_started_automation_profile(profile).await?;
    let network_id = clean_network_id(&resolved_profile);
    let backend_name = clean_backend_name(&resolved_profile);

    execute_tool(
        &resolved_profile,
        "mine_blocks",
        json!({
            "networkId": network_id_argument(&network_id),
            "blocks": blocks,
            "nodeName": backend_name,
        }),
    )
    .await?;

    get_blockchain_height_from_resolved(&resolved_profile).await
}

pub async fn destroy_demo_nodes(
    profile: &PolarAutomationProfile,
) -> Result<PolarAutomationProfile, String> {
    let resolved_profile = resolve_started_automation_profile(profile).await?;
    let network_id = clean_network_id(&resolved_profile);

    for node_id in [DemoNodeId::Carol, DemoNodeId::Bob, DemoNodeId::Alice] {
        let result = execute_tool(
            &resolved_profile,
            "remove_node",
            json!({
                "networkId": network_id_argument(&network_id),
                "nodeName": polar_node_name(node_id),
            }),
        )
        .await;

        if let Err(message) = result {
            if !message.to_ascii_lowercase().contains("not found") {
                return Err(message);
            }
        }
    }

    Ok(resolved_profile)
}

async fn resolve_started_automation_profile(
    profile: &PolarAutomationProfile,
) -> Result<PolarAutomationProfile, String> {
    test_bridge(profile).await?;
    let networks = list_networks(profile).await?;
    let network_id = resolve_network_id(profile, &networks)?;
    ensure_network_started(&networks, &network_id)?;
    let bitcoin_backend_name = resolve_backend_name(profile, &networks, &network_id);

    Ok(PolarAutomationProfile {
        bridge_url: profile.bridge_url.trim().to_string(),
        network_id,
        bitcoin_backend_name,
    })
}

async fn create_or_prepare_demo_node(
    profile: &PolarAutomationProfile,
    network_id: &str,
    backend_name: &str,
    node_id: DemoNodeId,
) -> Result<(), String> {
    let desired_name = polar_node_name(node_id);
    let before_add = list_networks(profile).await?;
    let created_name = if lightning_node_exists(&before_add, network_id, desired_name) {
        desired_name.to_string()
    } else {
        let add_result = execute_tool(
            profile,
            "add_node",
            json!({
                "networkId": network_id_argument(network_id),
                "implementation": "LND",
            }),
        )
        .await?;

        if let Some(name) = extract_node_name(&add_result) {
            name
        } else {
            async_created_node_name(profile, network_id, &before_add)
                .await
                .ok_or_else(|| {
                    format!(
                        "Polar created an LND node, but the bridge did not expose its generated name. Rename the newest LND node to {desired_name} in Polar, then retry."
                    )
                })?
        }
    };

    if !created_name.eq_ignore_ascii_case(desired_name) {
        execute_tool(
            profile,
            "rename_node",
            json!({
                "networkId": network_id_argument(network_id),
                "oldName": created_name,
                "newName": desired_name,
            }),
        )
        .await?;
    }

    execute_tool(
        profile,
        "set_lightning_backend",
        json!({
            "networkId": network_id_argument(network_id),
            "lightningNodeName": desired_name,
            "bitcoinNodeName": backend_name,
        }),
    )
    .await?;

    start_node_if_needed(profile, network_id, desired_name).await?;

    execute_tool(
        profile,
        "deposit_funds",
        json!({
            "networkId": network_id_argument(network_id),
            "nodeName": desired_name,
            "sats": DEMO_NODE_FUNDING_SATS,
        }),
    )
    .await?;

    Ok(())
}

async fn async_created_node_name(
    profile: &PolarAutomationProfile,
    network_id: &str,
    before_add: &Value,
) -> Option<String> {
    let after_add = list_networks(profile).await.ok()?;
    find_new_lightning_node_name(before_add, &after_add, network_id)
}

async fn start_node_if_needed(
    profile: &PolarAutomationProfile,
    network_id: &str,
    node_name: &str,
) -> Result<(), String> {
    let networks = list_networks(profile).await?;
    if lightning_node_is_started(&networks, network_id, node_name) {
        return Ok(());
    }

    execute_tool(
        profile,
        "start_node",
        json!({
            "networkId": network_id_argument(network_id),
            "nodeName": node_name,
        }),
    )
    .await?;

    Ok(())
}

fn network_id_argument(network_id: &str) -> Value {
    network_id
        .parse::<u64>()
        .map(Value::from)
        .unwrap_or_else(|_| Value::String(network_id.to_string()))
}

fn lightning_node_exists(value: &Value, network_id: &str, node_name: &str) -> bool {
    lightning_node_summaries(value, network_id)
        .iter()
        .any(|node| {
            node.name
                .as_deref()
                .map(|name| name.eq_ignore_ascii_case(node_name))
                .unwrap_or(false)
        })
}

fn lightning_node_is_started(value: &Value, network_id: &str, node_name: &str) -> bool {
    lightning_node_summaries(value, network_id)
        .iter()
        .any(|node| {
            let name_matches = node
                .name
                .as_deref()
                .map(|name| name.eq_ignore_ascii_case(node_name))
                .unwrap_or(false);
            let status_started = node
                .status
                .as_deref()
                .map(|status| {
                    let status = status.trim().to_ascii_lowercase();
                    status == "started" || status == "running"
                })
                .unwrap_or(false);

            name_matches && status_started
        })
}

fn find_new_lightning_node_name(
    before_add: &Value,
    after_add: &Value,
    network_id: &str,
) -> Option<String> {
    let before_names = lightning_node_summaries(before_add, network_id)
        .into_iter()
        .filter_map(|node| node.name)
        .map(|name| name.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let mut created_names = lightning_node_summaries(after_add, network_id)
        .into_iter()
        .filter_map(|node| node.name)
        .filter(|name| !before_names.contains(&name.to_ascii_lowercase()))
        .collect::<Vec<_>>();

    if created_names.len() == 1 {
        created_names.pop()
    } else {
        None
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct LightningNodeSummary {
    name: Option<String>,
    status: Option<String>,
}

fn lightning_node_summaries(value: &Value, network_id: &str) -> Vec<LightningNodeSummary> {
    let mut nodes = Vec::new();
    if let Some(network) = find_network_value(value, network_id) {
        collect_lightning_node_summaries(network, &mut nodes);
    }

    nodes
}

fn find_network_value<'a>(value: &'a Value, requested: &str) -> Option<&'a Value> {
    match value {
        Value::Object(map) => {
            let id = map
                .get("id")
                .or_else(|| map.get("networkId"))
                .and_then(value_as_id_string);
            let name = map.get("name").and_then(Value::as_str);
            let matches_id = id
                .as_deref()
                .map(|value| value.eq_ignore_ascii_case(requested))
                .unwrap_or(false);
            let matches_name = name
                .map(|value| value.eq_ignore_ascii_case(requested))
                .unwrap_or(false);

            if map.contains_key("nodes") && (matches_id || matches_name) {
                return Some(value);
            }

            for nested in map.values() {
                if let Some(network) = find_network_value(nested, requested) {
                    return Some(network);
                }
            }

            None
        }
        Value::Array(items) => items
            .iter()
            .find_map(|item| find_network_value(item, requested)),
        _ => None,
    }
}

fn collect_lightning_node_summaries(value: &Value, nodes: &mut Vec<LightningNodeSummary>) {
    match value {
        Value::Object(map) => {
            if looks_like_lightning_node(value) {
                nodes.push(LightningNodeSummary {
                    name: map
                        .get("name")
                        .or_else(|| map.get("nodeName"))
                        .or_else(|| map.get("displayName"))
                        .or_else(|| map.get("alias"))
                        .and_then(Value::as_str)
                        .map(|name| name.trim().to_string())
                        .filter(|name| !name.is_empty()),
                    status: map
                        .get("status")
                        .and_then(Value::as_str)
                        .map(|status| status.trim().to_string())
                        .filter(|status| !status.is_empty()),
                });
                return;
            }

            for nested in map.values() {
                collect_lightning_node_summaries(nested, nodes);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_lightning_node_summaries(item, nodes);
            }
        }
        _ => {}
    }
}

fn looks_like_lightning_node(value: &Value) -> bool {
    let Value::Object(map) = value else {
        return false;
    };

    let type_text = map
        .get("type")
        .or_else(|| map.get("implementation"))
        .or_else(|| map.get("nodeType"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();

    type_text.contains("lightning")
        || type_text == "lnd"
        || type_text.contains("c-lightning")
        || type_text.contains("eclair")
        || type_text.contains("litd")
}

async fn list_networks(profile: &PolarAutomationProfile) -> Result<Value, String> {
    execute_tool(profile, "list_networks", json!({})).await
}

async fn create_network(
    profile: &PolarAutomationProfile,
    network_name: &str,
) -> Result<(), String> {
    let attempts = [
        ("create_network", json!({ "name": network_name })),
        ("create_network", json!({ "networkName": network_name })),
        ("add_network", json!({ "name": network_name })),
        ("add_network", json!({ "networkName": network_name })),
    ];
    let mut errors = Vec::new();

    for (tool, arguments) in attempts {
        match execute_tool(profile, tool, arguments).await {
            Ok(_) => return Ok(()),
            Err(error) => errors.push(format!("{tool} failed: {error}")),
        }
    }

    Err(format!(
        "Polar bridge could not create server {network_name}. {}",
        errors.join("; ")
    ))
}

async fn get_blockchain_height_from_resolved(
    profile: &PolarAutomationProfile,
) -> Result<u64, String> {
    let network_id = clean_network_id(profile);
    let backend_name = clean_backend_name(profile);
    let response = execute_tool(
        profile,
        "get_blockchain_info",
        json!({
            "networkId": network_id_argument(&network_id),
            "nodeName": backend_name,
        }),
    )
    .await?;

    extract_block_height(&response)
        .ok_or_else(|| "Polar bridge returned blockchain info without a block height.".to_string())
}

fn resolve_network_id(
    profile: &PolarAutomationProfile,
    networks: &Value,
) -> Result<String, String> {
    let requested = clean_network_id(profile);
    if !requested.is_empty() {
        return Ok(find_network_id(networks, &requested).unwrap_or(requested));
    }

    if let Some(default_id) = find_network_id(networks, DEFAULT_NETWORK_NAME) {
        return Ok(default_id);
    }

    if let Some(single_id) = find_single_network_id(networks) {
        return Ok(single_id);
    }

    Err(format!(
        "Polar bridge could not choose a network. Keep one Polar network open or name it {DEFAULT_NETWORK_NAME}."
    ))
}

fn resolve_backend_name(
    profile: &PolarAutomationProfile,
    networks: &Value,
    network_id: &str,
) -> String {
    let requested = clean_backend_name(profile);
    if !requested.is_empty() {
        return requested;
    }

    find_bitcoin_backend_name(networks, network_id)
        .unwrap_or_else(|| DEFAULT_BITCOIN_BACKEND_NAME.to_string())
}

fn automation_profile_from_network(
    profile: &PolarAutomationProfile,
    networks: &Value,
    network_id: String,
) -> PolarAutomationProfile {
    let bitcoin_backend_name = resolve_backend_name(profile, networks, &network_id);

    PolarAutomationProfile {
        bridge_url: profile.bridge_url.trim().to_string(),
        network_id,
        bitcoin_backend_name,
    }
}

fn ensure_network_started(networks: &Value, network_id: &str) -> Result<(), String> {
    let Some(status) = find_network_status(networks, network_id) else {
        return Ok(());
    };

    let status_lower = status.to_ascii_lowercase();
    if status_lower == "started" || status_lower == "running" {
        return Ok(());
    }

    Err(format!(
        "Polar network {network_id} is {status}. Start it in Polar before running Bitcoin or Lightning actions."
    ))
}

async fn ensure_network_running(
    profile: &PolarAutomationProfile,
    network_id: &str,
) -> Result<(), String> {
    let networks = list_networks(profile).await?;
    if network_is_started(&networks, network_id) {
        return Ok(());
    }

    start_network(profile, network_id).await?;

    let networks = list_networks(profile).await?;
    ensure_network_started(&networks, network_id)
}

async fn start_network(profile: &PolarAutomationProfile, network_id: &str) -> Result<(), String> {
    let attempts = [
        (
            "start_network",
            json!({ "networkId": network_id_argument(network_id) }),
        ),
        (
            "start_network",
            json!({ "id": network_id_argument(network_id) }),
        ),
        ("start_network", json!({ "name": network_id })),
        (
            "start_server",
            json!({ "networkId": network_id_argument(network_id) }),
        ),
        (
            "start_server",
            json!({ "id": network_id_argument(network_id) }),
        ),
        ("start_server", json!({ "name": network_id })),
    ];
    let mut errors = Vec::new();

    for (tool, arguments) in attempts {
        match execute_tool(profile, tool, arguments).await {
            Ok(_) => return Ok(()),
            Err(error) => errors.push(format!("{tool} failed: {error}")),
        }
    }

    Err(format!(
        "Polar bridge could not start network {network_id}. Start it in Polar, then retry. {}",
        errors.join("; ")
    ))
}

fn network_is_started(networks: &Value, network_id: &str) -> bool {
    find_network_status(networks, network_id)
        .map(|status| {
            let status = status.to_ascii_lowercase();
            status == "started" || status == "running"
        })
        .unwrap_or(false)
}

async fn execute_tool(
    profile: &PolarAutomationProfile,
    tool: &str,
    arguments: Value,
) -> Result<Value, String> {
    post_json(
        profile,
        "/api/mcp/execute",
        json!({
            "tool": tool,
            "arguments": arguments,
        }),
    )
    .await
}

fn clean_network_id(profile: &PolarAutomationProfile) -> String {
    profile.network_id.trim().to_string()
}

fn clean_backend_name(profile: &PolarAutomationProfile) -> String {
    profile.bitcoin_backend_name.trim().to_string()
}

fn polar_node_name(node_id: DemoNodeId) -> &'static str {
    match node_id {
        DemoNodeId::Alice => "alice",
        DemoNodeId::Bob => "bob",
        DemoNodeId::Carol => "carol",
    }
}

fn bridge_url(profile: &PolarAutomationProfile, path: &str) -> String {
    format!(
        "{}{}",
        profile.bridge_url.trim().trim_end_matches('/'),
        path
    )
}

fn log_service_request(method: &str, url: &str, body: Option<&Value>) {
    if !LOG_SERVICE_CALLS_TO_TERMINAL {
        return;
    }

    let message = match body {
        Some(body) => format!("[polar-service] request {method} {url} body={body}"),
        None => format!("[polar-service] request {method} {url}"),
    };
    log_to_terminal(&message);
}

fn log_service_response(method: &str, url: &str, result: &Result<Value, String>) {
    if !LOG_SERVICE_CALLS_TO_TERMINAL {
        return;
    }

    let message = match result {
        Ok(value) => format!("[polar-service] response {method} {url} body={value}"),
        Err(error) => format!("[polar-service] error {method} {url} error={error}"),
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

fn extract_node_name(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => {
            for key in ["nodeName", "name", "displayName", "alias"] {
                if let Some(name) = map.get(key).and_then(Value::as_str) {
                    if !name.trim().is_empty() {
                        return Some(name.trim().to_string());
                    }
                }
            }

            for nested in map.values() {
                if let Some(name) = extract_node_name(nested) {
                    return Some(name);
                }
            }

            None
        }
        Value::Array(items) => items.iter().find_map(extract_node_name),
        _ => None,
    }
}

fn find_network_id(value: &Value, requested: &str) -> Option<String> {
    match value {
        Value::Object(map) => {
            let id = map
                .get("id")
                .or_else(|| map.get("networkId"))
                .and_then(value_as_id_string);
            let name = map.get("name").and_then(Value::as_str);
            let matches_id = id
                .as_deref()
                .map(|value| value.eq_ignore_ascii_case(requested))
                .unwrap_or(false);
            let matches_name = name
                .map(|value| value.eq_ignore_ascii_case(requested))
                .unwrap_or(false);

            if matches_id || matches_name {
                return id;
            }

            for nested in map.values() {
                if let Some(id) = find_network_id(nested, requested) {
                    return Some(id);
                }
            }

            None
        }
        Value::Array(items) => items
            .iter()
            .find_map(|item| find_network_id(item, requested)),
        _ => None,
    }
}

fn find_network_status(value: &Value, requested: &str) -> Option<String> {
    match value {
        Value::Object(map) => {
            let id = map
                .get("id")
                .or_else(|| map.get("networkId"))
                .and_then(value_as_id_string);
            let name = map.get("name").and_then(Value::as_str);
            let matches_id = id
                .as_deref()
                .map(|value| value.eq_ignore_ascii_case(requested))
                .unwrap_or(false);
            let matches_name = name
                .map(|value| value.eq_ignore_ascii_case(requested))
                .unwrap_or(false);

            if matches_id || matches_name {
                return map
                    .get("status")
                    .and_then(Value::as_str)
                    .map(|status| status.trim().to_string())
                    .filter(|status| !status.is_empty());
            }

            for nested in map.values() {
                if let Some(status) = find_network_status(nested, requested) {
                    return Some(status);
                }
            }

            None
        }
        Value::Array(items) => items
            .iter()
            .find_map(|item| find_network_status(item, requested)),
        _ => None,
    }
}

fn find_single_network_id(value: &Value) -> Option<String> {
    let mut ids = Vec::new();
    collect_top_level_network_ids(value, &mut ids);
    ids.sort();
    ids.dedup();

    if ids.len() == 1 {
        ids.pop()
    } else {
        None
    }
}

fn collect_top_level_network_ids(value: &Value, ids: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for key in ["networks", "result", "data"] {
                if let Some(networks) = map.get(key) {
                    collect_top_level_network_ids(networks, ids);
                    return;
                }
            }

            if let Some(id) = map
                .get("id")
                .or_else(|| map.get("networkId"))
                .and_then(value_as_id_string)
            {
                ids.push(id);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_top_level_network_ids(item, ids);
            }
        }
        _ => {}
    }
}

fn find_bitcoin_backend_name(value: &Value, network_id: &str) -> Option<String> {
    match value {
        Value::Object(map) => {
            let id = map
                .get("id")
                .or_else(|| map.get("networkId"))
                .and_then(value_as_id_string);
            let name = map.get("name").and_then(Value::as_str);
            let matches_network = id
                .as_deref()
                .map(|value| value.eq_ignore_ascii_case(network_id))
                .unwrap_or(false)
                || name
                    .map(|value| value.eq_ignore_ascii_case(network_id))
                    .unwrap_or(false);

            if matches_network {
                if let Some(name) = find_backend_name_in_value(value) {
                    return Some(name);
                }
            }

            for nested in map.values() {
                if let Some(name) = find_bitcoin_backend_name(nested, network_id) {
                    return Some(name);
                }
            }

            None
        }
        Value::Array(items) => items
            .iter()
            .find_map(|item| find_bitcoin_backend_name(item, network_id)),
        _ => None,
    }
}

fn find_backend_name_in_value(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => {
            let type_text = map
                .get("type")
                .or_else(|| map.get("implementation"))
                .or_else(|| map.get("nodeType"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_ascii_lowercase();

            let looks_like_bitcoin = type_text.contains("bitcoin") || type_text.contains("core");
            if looks_like_bitcoin {
                for key in ["nodeName", "name", "displayName", "alias"] {
                    if let Some(name) = map.get(key).and_then(Value::as_str) {
                        if !name.trim().is_empty() {
                            return Some(name.trim().to_string());
                        }
                    }
                }
            }

            for nested in map.values() {
                if let Some(name) = find_backend_name_in_value(nested) {
                    return Some(name);
                }
            }

            None
        }
        Value::Array(items) => items.iter().find_map(find_backend_name_in_value),
        _ => None,
    }
}

fn value_as_id_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) if !text.trim().is_empty() => Some(text.trim().to_string()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

fn extract_block_height(value: &Value) -> Option<u64> {
    match value {
        Value::Object(map) => {
            for key in ["blocks", "blockHeight", "block_height", "height"] {
                if let Some(height) = map.get(key).and_then(value_as_u64) {
                    return Some(height);
                }
            }

            map.values().find_map(extract_block_height)
        }
        Value::Array(items) => items.iter().find_map(extract_block_height),
        _ => None,
    }
}

fn value_as_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64(),
        Value::String(text) => text.trim().parse::<u64>().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn polar_v4_networks_response() -> Value {
        json!({
            "networks": [
                {
                    "id": 1,
                    "name": DEFAULT_NETWORK_NAME,
                    "status": "Started",
                    "nodes": {
                        "bitcoin": [
                            {
                                "id": 0,
                                "networkId": 1,
                                "name": DEFAULT_BITCOIN_BACKEND_NAME,
                                "type": "bitcoin",
                                "implementation": "bitcoind"
                            }
                        ],
                        "lightning": []
                    }
                }
            ]
        })
    }

    #[test]
    fn finds_default_network_when_polar_returns_numeric_id() {
        let networks = polar_v4_networks_response();

        assert_eq!(
            find_network_id(&networks, DEFAULT_NETWORK_NAME),
            Some("1".to_string())
        );
    }

    #[test]
    fn finds_single_network_when_polar_returns_numeric_id() {
        let networks = polar_v4_networks_response();

        assert_eq!(find_single_network_id(&networks), Some("1".to_string()));
    }

    #[test]
    fn finds_bitcoin_backend_for_numeric_network_id() {
        let networks = polar_v4_networks_response();

        assert_eq!(
            find_bitcoin_backend_name(&networks, "1"),
            Some(DEFAULT_BITCOIN_BACKEND_NAME.to_string())
        );
    }

    #[test]
    fn sends_numeric_network_id_when_polar_uses_numeric_ids() {
        assert_eq!(network_id_argument("1"), json!(1));
    }

    #[test]
    fn detects_existing_started_lightning_nodes() {
        let networks = json!({
            "networks": [
                {
                    "id": 1,
                    "name": DEFAULT_NETWORK_NAME,
                    "status": "Started",
                    "nodes": {
                        "bitcoin": [],
                        "lightning": [
                            {
                                "name": "alice",
                                "type": "lightning",
                                "implementation": "LND",
                                "status": "Started"
                            }
                        ]
                    }
                }
            ]
        });

        assert!(lightning_node_exists(&networks, "1", "alice"));
        assert!(lightning_node_is_started(&networks, "1", "alice"));
    }

    #[test]
    fn discovers_node_name_added_without_return_value() {
        let before_add = polar_v4_networks_response();
        let after_add = json!({
            "networks": [
                {
                    "id": 1,
                    "name": DEFAULT_NETWORK_NAME,
                    "status": "Started",
                    "nodes": {
                        "bitcoin": [],
                        "lightning": [
                            {
                                "name": "node1",
                                "type": "lightning",
                                "implementation": "LND",
                                "status": "Started"
                            }
                        ]
                    }
                }
            ]
        });

        assert_eq!(
            find_new_lightning_node_name(&before_add, &after_add, "1"),
            Some("node1".to_string())
        );
    }

    #[test]
    fn extracts_block_height_from_blockchain_info() {
        let blockchain_info = json!({
            "success": true,
            "nodeName": DEFAULT_BITCOIN_BACKEND_NAME,
            "chain": "regtest",
            "blocks": 267,
            "headers": 267
        });

        assert_eq!(extract_block_height(&blockchain_info), Some(267));
    }

    #[test]
    fn detects_stopped_network_before_bitcoin_rpc() {
        let networks = json!({
            "networks": [
                {
                    "id": 2,
                    "name": DEFAULT_NETWORK_NAME,
                    "status": "Stopped",
                    "nodes": {
                        "bitcoin": [
                            {
                                "name": DEFAULT_BITCOIN_BACKEND_NAME,
                                "type": "bitcoin"
                            }
                        ],
                        "lightning": []
                    }
                }
            ]
        });

        assert_eq!(
            find_network_status(&networks, "2"),
            Some("Stopped".to_string())
        );
        assert!(!network_is_started(&networks, "2"));
        assert_eq!(
            ensure_network_started(&networks, "2"),
            Err("Polar network 2 is Stopped. Start it in Polar before running Bitcoin or Lightning actions.".to_string())
        );
    }

    #[test]
    fn allows_started_network_before_bitcoin_rpc() {
        let networks = polar_v4_networks_response();

        assert!(network_is_started(&networks, "1"));
        assert_eq!(ensure_network_started(&networks, "1"), Ok(()));
    }
}

#[cfg(target_arch = "wasm32")]
async fn get_json(profile: &PolarAutomationProfile, path: &str) -> Result<Value, String> {
    let url = bridge_url(profile, path);
    log_service_request("GET", &url, None);
    let result = match gloo_net::http::Request::get(&url).send().await {
        Ok(response) => response
            .json::<Value>()
            .await
            .map_err(|error| format!("Polar bridge returned invalid JSON: {error}")),
        Err(error) => Err(format!("Cannot reach Polar bridge: {error}")),
    };
    log_service_response("GET", &url, &result);
    result
}

#[cfg(target_arch = "wasm32")]
async fn post_json(
    profile: &PolarAutomationProfile,
    path: &str,
    body: Value,
) -> Result<Value, String> {
    let url = bridge_url(profile, path);
    log_service_request("POST", &url, Some(&body));
    let result = match gloo_net::http::Request::post(&url).json(&body) {
        Ok(request) => match request.send().await {
            Ok(response) => response
                .json::<Value>()
                .await
                .map_err(|error| format!("Polar bridge returned invalid JSON: {error}")),
            Err(error) => Err(format!("Polar bridge request failed: {error}")),
        },
        Err(error) => Err(format!("Cannot encode Polar bridge request: {error}")),
    };
    log_service_response("POST", &url, &result);
    result
}

#[cfg(not(target_arch = "wasm32"))]
async fn get_json(profile: &PolarAutomationProfile, path: &str) -> Result<Value, String> {
    let url = bridge_url(profile, path);
    log_service_request("GET", &url, None);
    let result = match ureq::get(&url).call() {
        Ok(response) => response
            .into_json::<Value>()
            .map_err(|error| format!("Polar bridge returned invalid JSON: {error}")),
        Err(error) => Err(format!("Cannot reach Polar bridge: {error}")),
    };
    log_service_response("GET", &url, &result);
    result
}

#[cfg(not(target_arch = "wasm32"))]
async fn post_json(
    profile: &PolarAutomationProfile,
    path: &str,
    body: Value,
) -> Result<Value, String> {
    let url = bridge_url(profile, path);
    log_service_request("POST", &url, Some(&body));
    let result = match ureq::post(&url)
        .set("Content-Type", "application/json")
        .send_json(body)
    {
        Ok(response) => response
            .into_json::<Value>()
            .map_err(|error| format!("Polar bridge returned invalid JSON: {error}")),
        Err(error) => Err(format!("Polar bridge request failed: {error}")),
    };
    log_service_response("POST", &url, &result);
    result
}
