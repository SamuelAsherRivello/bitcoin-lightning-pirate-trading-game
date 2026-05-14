use std::collections::HashSet;
use std::fmt;

use serde_json::{json, Value};

use crate::client::models::{
    DemoNodeId, PolarAutomationProfile, DEFAULT_BITCOIN_BACKEND_NAME, DEFAULT_NETWORK_NAME,
};

const DEMO_NODE_FUNDING_SATS: u64 = 1_000_000;
const DEMO_NODE_START_TIMEOUT_SECONDS: u16 = 180;
const DEMO_NODE_START_ATTEMPTS: u16 = DEMO_NODE_START_TIMEOUT_SECONDS / 2;
const DEMO_NODE_START_DELAY_MS: u32 = 2_000;
const DEMO_NODE_READY_TIMEOUT_SECONDS: u16 = 90;
const DEMO_NODE_READY_DELAY_MS: u32 = 1_500;
const DEMO_NODE_READY_ATTEMPTS: u16 =
    ((DEMO_NODE_READY_TIMEOUT_SECONDS as u32 * 1_000) / DEMO_NODE_READY_DELAY_MS) as u16;
const DEMO_SERVICE_LOG_LEVEL: DemoLogLevel = DemoLogLevel::On;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DemoLogLevel {
    Off,
    On,
    Verbose,
}

impl DemoLogLevel {
    fn allows(self, required: Self) -> bool {
        self >= required && self != Self::Off
    }
}

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolarLabHealthIssue {
    BridgeUnavailable(String),
    NetworkMissing {
        network_id: String,
    },
    NetworkStopped {
        network_id: String,
        status: String,
    },
    BitcoinBackendMissing {
        network_id: String,
        backend_name: String,
    },
    DemoNodeMissing {
        network_id: String,
        node_id: DemoNodeId,
    },
    DemoNodeStopped {
        network_id: String,
        node_id: DemoNodeId,
        status: Option<String>,
    },
}

impl fmt::Display for PolarLabHealthIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BridgeUnavailable(message) => write!(formatter, "{message}"),
            Self::NetworkMissing { network_id } => {
                write!(formatter, "Polar network {network_id} is missing.")
            }
            Self::NetworkStopped { network_id, status } => {
                write!(formatter, "Polar network {network_id} is {status}.")
            }
            Self::BitcoinBackendMissing {
                network_id,
                backend_name,
            } => write!(
                formatter,
                "Polar network {network_id} is missing Bitcoin backend {backend_name}."
            ),
            Self::DemoNodeMissing { node_id, .. } => {
                write!(formatter, "Polar demo node {} is missing.", node_id.label())
            }
            Self::DemoNodeStopped {
                node_id, status, ..
            } => write!(
                formatter,
                "Polar demo node {} is {}.",
                node_id.label(),
                status.as_deref().unwrap_or("not started")
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PolarLabHealthReport {
    pub profile: PolarAutomationProfile,
    pub block_height: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DemoNodeFundingPlan {
    AlreadyFunded,
    NeedsFunding(u64),
}

pub async fn test_bridge(profile: &PolarAutomationProfile) -> Result<(), String> {
    test_bridge_with_log_level(profile, DemoLogLevel::On).await
}

async fn test_bridge_with_log_level(
    profile: &PolarAutomationProfile,
    log_level: DemoLogLevel,
) -> Result<(), String> {
    let response = get_json(profile, "/health", log_level).await?;
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
    if let Some(network_id) = find_network_id_by_name(&networks, &requested_name) {
        ensure_network_running(profile, &network_id).await?;
        let networks = list_networks(profile).await?;
        ensure_named_network_running(&networks, &requested_name)?;
        return Ok(PolarServerEnsureResult {
            profile: automation_profile_from_network(profile, &networks, network_id),
            status: PolarServerEnsureStatus::Existed,
        });
    }

    create_network(profile, &requested_name).await?;

    let networks = list_networks(profile).await?;
    let network_id = find_network_id_by_name(&networks, &requested_name).ok_or_else(|| {
        format!(
            "Polar bridge created server {requested_name}, but it is not listed by that name yet."
        )
    })?;
    ensure_network_running(profile, &network_id).await?;
    let networks = list_networks(profile).await?;
    ensure_named_network_running(&networks, &requested_name)?;

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
    required_balance_sats: u64,
) -> Result<PolarAutomationProfile, String> {
    create_demo_nodes_with_progress(profile, required_balance_sats, |_| {}).await
}

pub async fn create_demo_nodes_with_progress<F>(
    profile: &PolarAutomationProfile,
    required_balance_sats: u64,
    mut report_progress: F,
) -> Result<PolarAutomationProfile, String>
where
    F: FnMut(String),
{
    let resolved_profile = resolve_started_automation_profile(profile).await?;
    let network_id = clean_network_id(&resolved_profile);
    let backend_name = clean_backend_name(&resolved_profile);

    close_demo_channels_with_progress(&resolved_profile, &mut report_progress).await?;

    for node_id in DemoNodeId::ALL {
        report_progress(format!("Preparing {} in Polar...", node_id.label()));
        let funding_plan = create_or_prepare_demo_node(
            &resolved_profile,
            &network_id,
            &backend_name,
            node_id,
            &mut report_progress,
        )
        .await?;

        match funding_plan {
            DemoNodeFundingPlan::AlreadyFunded => {
                report_progress(format!(
                    "{} already matches the step 03 goal. Checking readiness...",
                    node_id.label()
                ));
            }
            DemoNodeFundingPlan::NeedsFunding(sats) => {
                report_progress(format!(
                    "{} needs {sats} sats. Depositing only that amount...",
                    node_id.label()
                ));
                deposit_demo_node_funds(&resolved_profile, &network_id, node_id, sats).await?;

                report_progress(format!(
                    "Mining blocks for {} and checking wallet balance...",
                    node_id.label()
                ));
                mine_demo_blocks(&resolved_profile, &network_id, &backend_name).await?;
            }
        }

        wait_for_demo_node_ready(&resolved_profile, required_balance_sats, node_id).await?;
        report_progress(format!(
            "{} is running and funded. Continuing...",
            node_id.label()
        ));
    }

    report_progress("Verifying Alice, Bob, and Carol together...".to_string());
    wait_for_demo_lab_ready(&resolved_profile, required_balance_sats).await?;

    Ok(resolved_profile)
}

pub async fn close_demo_channels(
    profile: &PolarAutomationProfile,
) -> Result<PolarAutomationProfile, String> {
    close_demo_channels_with_progress(profile, |_| {}).await
}

pub async fn close_demo_channels_with_progress<F>(
    profile: &PolarAutomationProfile,
    mut report_progress: F,
) -> Result<PolarAutomationProfile, String>
where
    F: FnMut(String),
{
    let resolved_profile = resolve_started_automation_profile(profile).await?;
    let network_id = clean_network_id(&resolved_profile);
    let networks = list_networks(&resolved_profile).await?;
    let mut closed_channel_points = HashSet::new();

    report_progress("Checking demo player channels...".to_string());

    for node_id in DemoNodeId::ALL {
        let node_name = polar_node_name(node_id);
        if !lightning_node_exists(&networks, &network_id, node_name) {
            report_progress(format!(
                "{} is not in Polar yet. Skipping channel cleanup...",
                node_id.label()
            ));
            continue;
        }

        let channels = list_node_channels(&resolved_profile, &network_id, node_name).await?;

        for channel_point in extract_channel_points(&channels) {
            if !closed_channel_points.insert(channel_point.clone()) {
                continue;
            }

            report_progress(format!(
                "Closing channel {channel_point} from {}...",
                node_id.label()
            ));
            close_node_channel(&resolved_profile, &network_id, node_name, &channel_point).await?;
        }
    }

    if closed_channel_points.is_empty() {
        report_progress("No demo player channels needed closing.".to_string());
    } else {
        report_progress(format!(
            "Closed {} demo player channel(s).",
            closed_channel_points.len()
        ));
    }

    Ok(resolved_profile)
}

pub async fn get_blockchain_height(profile: &PolarAutomationProfile) -> Result<u64, String> {
    let resolved_profile =
        resolve_started_automation_profile_with_log_level(profile, DemoLogLevel::On).await?;
    get_blockchain_height_from_resolved(&resolved_profile).await
}

pub async fn get_blockchain_height_verification_poll(
    profile: &PolarAutomationProfile,
) -> Result<u64, String> {
    let resolved_profile =
        resolve_started_automation_profile_with_log_level(profile, DemoLogLevel::Verbose).await?;
    get_blockchain_height_from_resolved_with_log_level(&resolved_profile, DemoLogLevel::Verbose)
        .await
}

pub async fn mine_blocks(profile: &PolarAutomationProfile, blocks: u64) -> Result<u64, String> {
    let resolved_profile =
        resolve_started_automation_profile_with_log_level(profile, DemoLogLevel::On).await?;
    mine_blocks_from_resolved(&resolved_profile, blocks).await?;

    get_blockchain_height_from_resolved(&resolved_profile).await
}

async fn mine_blocks_from_resolved(
    resolved_profile: &PolarAutomationProfile,
    blocks: u64,
) -> Result<(), String> {
    let network_id = clean_network_id(&resolved_profile);
    let backend_name = clean_backend_name(&resolved_profile);

    execute_tool(
        resolved_profile,
        "mine_blocks",
        json!({
            "networkId": network_id_argument(&network_id),
            "blocks": blocks,
            "nodeName": backend_name,
        }),
    )
    .await
    .map(|_| ())
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

pub async fn delete_polar_network(profile: &PolarAutomationProfile) -> Result<(), String> {
    test_bridge(profile).await?;
    let networks = list_networks(profile).await?;
    let network_id = resolve_network_id(profile, &networks)?;

    let attempts = [
        (
            "delete_network",
            json!({ "networkId": network_id_argument(&network_id) }),
        ),
        (
            "delete_network",
            json!({ "id": network_id_argument(&network_id) }),
        ),
        ("delete_network", json!({ "name": network_id })),
        (
            "remove_network",
            json!({ "networkId": network_id_argument(&network_id) }),
        ),
        (
            "remove_network",
            json!({ "id": network_id_argument(&network_id) }),
        ),
        ("remove_network", json!({ "name": network_id })),
    ];
    let mut errors = Vec::new();

    for (tool, arguments) in attempts {
        match execute_tool(profile, tool, arguments).await {
            Ok(_) => return Ok(()),
            Err(error) => errors.push(format!("{tool} failed: {error}")),
        }
    }

    Err(format!(
        "Polar bridge could not delete server {}. {}",
        clean_network_id(profile),
        errors.join("; ")
    ))
}

pub async fn validate_lab_health(
    profile: &PolarAutomationProfile,
) -> Result<PolarLabHealthReport, PolarLabHealthIssue> {
    validate_lab_health_with_log_level(profile, DemoLogLevel::On).await
}

pub async fn validate_lab_health_verification_poll(
    profile: &PolarAutomationProfile,
) -> Result<PolarLabHealthReport, PolarLabHealthIssue> {
    validate_lab_health_with_log_level(profile, DemoLogLevel::Verbose).await
}

async fn validate_lab_health_with_log_level(
    profile: &PolarAutomationProfile,
    log_level: DemoLogLevel,
) -> Result<PolarLabHealthReport, PolarLabHealthIssue> {
    test_bridge_with_log_level(profile, log_level)
        .await
        .map_err(PolarLabHealthIssue::BridgeUnavailable)?;

    let networks = list_networks_with_log_level(profile, log_level)
        .await
        .map_err(PolarLabHealthIssue::BridgeUnavailable)?;
    let resolved_profile = inspect_lab_health(profile, &networks)?;
    let block_height =
        get_blockchain_height_from_resolved_with_log_level(&resolved_profile, log_level)
            .await
            .ok();

    Ok(PolarLabHealthReport {
        profile: resolved_profile,
        block_height,
    })
}

async fn resolve_started_automation_profile(
    profile: &PolarAutomationProfile,
) -> Result<PolarAutomationProfile, String> {
    resolve_started_automation_profile_with_log_level(profile, DemoLogLevel::On).await
}

async fn resolve_started_automation_profile_with_log_level(
    profile: &PolarAutomationProfile,
    log_level: DemoLogLevel,
) -> Result<PolarAutomationProfile, String> {
    test_bridge_with_log_level(profile, log_level).await?;
    let networks = list_networks_with_log_level(profile, log_level).await?;
    let network_id = resolve_network_id(profile, &networks)?;
    ensure_network_started(&networks, &network_id)?;
    let bitcoin_backend_name = resolve_backend_name(profile, &networks, &network_id);

    Ok(PolarAutomationProfile {
        bridge_url: profile.bridge_url.trim().to_string(),
        network_id,
        bitcoin_backend_name,
    })
}

fn inspect_lab_health(
    profile: &PolarAutomationProfile,
    networks: &Value,
) -> Result<PolarAutomationProfile, PolarLabHealthIssue> {
    let requested = clean_network_id(profile);
    let network_id = if requested.is_empty() {
        find_network_id(networks, DEFAULT_NETWORK_NAME)
            .or_else(|| find_single_network_id(networks))
            .ok_or_else(|| PolarLabHealthIssue::NetworkMissing {
                network_id: DEFAULT_NETWORK_NAME.to_string(),
            })?
    } else {
        find_network_id(networks, &requested).ok_or_else(|| {
            PolarLabHealthIssue::NetworkMissing {
                network_id: requested.clone(),
            }
        })?
    };

    if let Some(status) = find_network_status(networks, &network_id) {
        let status_lower = status.to_ascii_lowercase();
        if status_lower != "started" && status_lower != "running" {
            return Err(PolarLabHealthIssue::NetworkStopped { network_id, status });
        }
    }

    let bitcoin_backend_name = resolve_backend_name(profile, networks, &network_id);
    if !bitcoin_backend_exists(networks, &network_id, &bitcoin_backend_name) {
        return Err(PolarLabHealthIssue::BitcoinBackendMissing {
            network_id,
            backend_name: bitcoin_backend_name,
        });
    }

    for node_id in DemoNodeId::ALL {
        let node_name = polar_node_name(node_id);
        if !lightning_node_exists(networks, &network_id, node_name) {
            return Err(PolarLabHealthIssue::DemoNodeMissing {
                network_id,
                node_id,
            });
        }

        if !lightning_node_is_started(networks, &network_id, node_name) {
            return Err(PolarLabHealthIssue::DemoNodeStopped {
                network_id: network_id.clone(),
                node_id,
                status: lightning_node_status(networks, &network_id, node_name),
            });
        }
    }

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
    report_progress: &mut impl FnMut(String),
) -> Result<DemoNodeFundingPlan, String> {
    let desired_name = polar_node_name(node_id);
    let before_add = list_networks(profile).await?;

    if let Some(existing_name) = find_lightning_node_name(&before_add, network_id, desired_name) {
        if existing_name != desired_name {
            report_progress(format!(
                "Renaming {} to {}...",
                existing_name,
                node_id.label()
            ));
            rename_demo_node(profile, network_id, &existing_name, desired_name).await?;
        }

        return prepare_existing_demo_node(
            profile,
            network_id,
            desired_name,
            node_id,
            report_progress,
        )
        .await;
    }

    let before_add = list_networks(profile).await?;
    let created_name =
        create_lightning_node(profile, network_id, desired_name, &before_add).await?;
    if created_name != desired_name {
        rename_demo_node(profile, network_id, &created_name, desired_name).await?;
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

    report_progress(format!("Starting {} in Polar...", node_id.label()));
    start_node_if_needed(profile, network_id, desired_name).await?;

    Ok(DemoNodeFundingPlan::NeedsFunding(DEMO_NODE_FUNDING_SATS))
}

async fn prepare_existing_demo_node(
    profile: &PolarAutomationProfile,
    network_id: &str,
    node_name: &str,
    node_id: DemoNodeId,
    report_progress: &mut impl FnMut(String),
) -> Result<DemoNodeFundingPlan, String> {
    report_progress(format!("Starting {} in Polar...", node_id.label()));
    start_node_if_needed(profile, network_id, node_name).await?;

    let balance =
        wait_for_lightning_wallet_balance(profile, network_id, node_name, node_id).await?;
    if balance == DEMO_NODE_FUNDING_SATS {
        return Ok(DemoNodeFundingPlan::AlreadyFunded);
    }

    if balance < DEMO_NODE_FUNDING_SATS {
        return Ok(DemoNodeFundingPlan::NeedsFunding(
            DEMO_NODE_FUNDING_SATS - balance,
        ));
    }

    report_progress(format!(
        "{} has {balance} sats, above the step 03 goal. Rebuilding the node...",
        node_id.label()
    ));
    remove_demo_node_by_name(profile, network_id, node_name).await?;

    let before_add = list_networks(profile).await?;
    let created_name = create_lightning_node(profile, network_id, node_name, &before_add).await?;
    if created_name != node_name {
        rename_demo_node(profile, network_id, &created_name, node_name).await?;
    }

    report_progress(format!("Starting {} in Polar...", node_id.label()));
    start_node_if_needed(profile, network_id, node_name).await?;

    Ok(DemoNodeFundingPlan::NeedsFunding(DEMO_NODE_FUNDING_SATS))
}

async fn create_lightning_node(
    profile: &PolarAutomationProfile,
    network_id: &str,
    desired_name: &str,
    before_add: &Value,
) -> Result<String, String> {
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
        return Ok(name);
    }

    async_created_node_name(profile, network_id, before_add)
        .await
        .ok_or_else(|| {
            format!(
                "Polar created an LND node, but the bridge did not expose its generated name. Rename the newest LND node to {desired_name} in Polar, then retry."
            )
        })
}

async fn remove_demo_node_by_name(
    profile: &PolarAutomationProfile,
    network_id: &str,
    node_name: &str,
) -> Result<(), String> {
    let result = execute_tool(
        profile,
        "remove_node",
        json!({
            "networkId": network_id_argument(network_id),
            "nodeName": node_name,
        }),
    )
    .await;

    if let Err(message) = result {
        if !message.to_ascii_lowercase().contains("not found") {
            return Err(message);
        }
    }

    Ok(())
}

async fn rename_demo_node(
    profile: &PolarAutomationProfile,
    network_id: &str,
    old_name: &str,
    new_name: &str,
) -> Result<(), String> {
    execute_tool(
        profile,
        "rename_node",
        json!({
            "networkId": network_id_argument(network_id),
            "oldName": old_name,
            "newName": new_name,
        }),
    )
    .await?;

    Ok(())
}

async fn deposit_demo_node_funds(
    profile: &PolarAutomationProfile,
    network_id: &str,
    node_id: DemoNodeId,
    sats: u64,
) -> Result<(), String> {
    execute_tool(
        profile,
        "deposit_funds",
        json!({
            "networkId": network_id_argument(network_id),
            "nodeName": polar_node_name(node_id),
            "sats": sats,
        }),
    )
    .await?;

    Ok(())
}

async fn mine_demo_blocks(
    profile: &PolarAutomationProfile,
    network_id: &str,
    backend_name: &str,
) -> Result<(), String> {
    execute_tool(
        profile,
        "mine_blocks",
        json!({
            "networkId": network_id_argument(network_id),
            "blocks": 6,
            "nodeName": backend_name,
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

    let status = lightning_node_status(&networks, network_id, node_name)
        .unwrap_or_else(|| "not started".to_string());
    if !status.eq_ignore_ascii_case("starting") {
        execute_tool(
            profile,
            "start_node",
            json!({
                "networkId": network_id_argument(network_id),
                "nodeName": node_name,
            }),
        )
        .await?;
    }

    wait_for_lightning_node_started(profile, network_id, node_name).await
}

async fn wait_for_lightning_node_started(
    profile: &PolarAutomationProfile,
    network_id: &str,
    node_name: &str,
) -> Result<(), String> {
    let mut last_status = None;

    for attempt in 1..=DEMO_NODE_START_ATTEMPTS {
        let networks = list_networks(profile).await?;
        if lightning_node_is_started(&networks, network_id, node_name) {
            log_to_terminal(&format!(
                "[polar-service] node-start-ready node={node_name} attempt={attempt}"
            ));
            return Ok(());
        }

        last_status = lightning_node_status(&networks, network_id, node_name);
        log_to_terminal(&format!(
            "[polar-service] node-start-wait node={node_name} attempt={attempt}/{DEMO_NODE_START_ATTEMPTS} status={}",
            last_status.as_deref().unwrap_or("unknown")
        ));
        if attempt < DEMO_NODE_START_ATTEMPTS {
            wait_for_demo_node_start_delay().await;
        }
    }

    Err(format!(
        "Polar demo node {node_name} did not finish starting within {DEMO_NODE_START_TIMEOUT_SECONDS} seconds. Last status: {}.",
        last_status.unwrap_or_else(|| "unknown".to_string())
    ))
}

async fn wait_for_lightning_wallet_balance(
    profile: &PolarAutomationProfile,
    network_id: &str,
    node_name: &str,
    node_id: DemoNodeId,
) -> Result<u64, String> {
    let mut last_error = None;

    for attempt in 1..=DEMO_NODE_READY_ATTEMPTS {
        match get_lightning_wallet_balance(profile, network_id, node_name).await {
            Ok(balance) => return Ok(balance),
            Err(message) => {
                last_error = Some(message);
                if attempt < DEMO_NODE_READY_ATTEMPTS {
                    wait_for_demo_node_ready_delay().await;
                }
            }
        }
    }

    Err(format!(
        "{} is not ready for wallet checks yet: {}",
        node_id.label(),
        last_error.unwrap_or_else(|| "wallet balance did not become available".to_string())
    ))
}

async fn wait_for_demo_lab_ready(
    profile: &PolarAutomationProfile,
    required_balance_sats: u64,
) -> Result<(), String> {
    let mut last_error = None;

    for attempt in 1..=DEMO_NODE_READY_ATTEMPTS {
        match verify_demo_lab_ready(profile, required_balance_sats).await {
            Ok(()) => return Ok(()),
            Err(message) => {
                last_error = Some(message);
                if attempt < DEMO_NODE_READY_ATTEMPTS {
                    wait_for_demo_node_ready_delay().await;
                }
            }
        }
    }

    Err(format!(
        "Polar demo nodes were created, but the lab did not become ready. {}",
        last_error.unwrap_or_else(|| "Retry after Polar finishes starting LND.".to_string())
    ))
}

async fn wait_for_demo_node_ready(
    profile: &PolarAutomationProfile,
    required_balance_sats: u64,
    node_id: DemoNodeId,
) -> Result<(), String> {
    let mut last_error = None;

    for attempt in 1..=DEMO_NODE_READY_ATTEMPTS {
        match verify_demo_node_ready(profile, required_balance_sats, node_id).await {
            Ok(()) => return Ok(()),
            Err(message) => {
                last_error = Some(message);
                if attempt < DEMO_NODE_READY_ATTEMPTS {
                    wait_for_demo_node_ready_delay().await;
                }
            }
        }
    }

    Err(format!(
        "{} was created, but Polar did not report a ready funded wallet. {}",
        node_id.label(),
        last_error.unwrap_or_else(|| "Retry after Polar finishes starting LND.".to_string())
    ))
}

async fn verify_demo_node_ready(
    profile: &PolarAutomationProfile,
    required_balance_sats: u64,
    node_id: DemoNodeId,
) -> Result<(), String> {
    let networks = list_networks(profile).await?;
    let network_id = clean_network_id(profile);
    let backend_name = clean_backend_name(profile);
    ensure_network_started(&networks, &network_id)?;

    if !bitcoin_backend_exists(&networks, &network_id, &backend_name) {
        return Err(format!(
            "Polar Bitcoin backend {backend_name} is not listed for network {network_id}."
        ));
    }

    let node_name = polar_node_name(node_id);
    if !lightning_node_exists(&networks, &network_id, node_name) {
        return Err(format!("Polar demo node {} is missing.", node_id.label()));
    }

    if !lightning_node_is_started(&networks, &network_id, node_name) {
        let status = lightning_node_status(&networks, &network_id, node_name)
            .unwrap_or_else(|| "not started".to_string());
        return Err(format!("Polar demo node {} is {status}.", node_id.label()));
    }

    let balance = get_lightning_wallet_balance(profile, &network_id, node_name)
        .await
        .map_err(|message| {
            format!(
                "{} is not ready for wallet checks yet: {message}",
                node_id.label()
            )
        })?;

    if !wallet_balance_matches_app_rules(balance, required_balance_sats) {
        return Err(format!(
            "{} has {balance} sats available, but the app needs exactly {DEMO_NODE_FUNDING_SATS} sats for a fresh demo node.",
            node_id.label()
        ));
    }

    Ok(())
}

async fn verify_demo_lab_ready(
    profile: &PolarAutomationProfile,
    required_balance_sats: u64,
) -> Result<(), String> {
    let networks = list_networks(profile).await?;
    let resolved_profile =
        inspect_lab_health(profile, &networks).map_err(|issue| issue.to_string())?;
    let network_id = clean_network_id(&resolved_profile);

    for node_id in DemoNodeId::ALL {
        let node_name = polar_node_name(node_id);
        let balance = get_lightning_wallet_balance(&resolved_profile, &network_id, node_name)
            .await
            .map_err(|message| {
                format!(
                    "{} is not ready for wallet checks yet: {message}",
                    node_id.label()
                )
            })?;

        if !wallet_balance_matches_app_rules(balance, required_balance_sats) {
            return Err(format!(
                "{} has {balance} sats available, but the app needs exactly {DEMO_NODE_FUNDING_SATS} sats for a fresh demo node.",
                node_id.label()
            ));
        }
    }

    Ok(())
}

async fn get_lightning_wallet_balance(
    profile: &PolarAutomationProfile,
    network_id: &str,
    node_name: &str,
) -> Result<u64, String> {
    let response = execute_tool(
        profile,
        "get_wallet_balance",
        json!({
            "networkId": network_id_argument(network_id),
            "nodeName": node_name,
        }),
    )
    .await?;

    extract_wallet_balance_sats(&response)
        .ok_or_else(|| format!("Polar returned wallet balance for {node_name} without sats."))
}

async fn list_node_channels(
    profile: &PolarAutomationProfile,
    network_id: &str,
    node_name: &str,
) -> Result<Value, String> {
    execute_tool(
        profile,
        "list_channels",
        json!({
            "networkId": network_id_argument(network_id),
            "nodeName": node_name,
        }),
    )
    .await
}

async fn close_node_channel(
    profile: &PolarAutomationProfile,
    network_id: &str,
    node_name: &str,
    channel_point: &str,
) -> Result<(), String> {
    execute_tool(
        profile,
        "close_channel",
        json!({
            "networkId": network_id_argument(network_id),
            "nodeName": node_name,
            "channelPoint": channel_point,
        }),
    )
    .await
    .map(|_| ())
}

fn network_id_argument(network_id: &str) -> Value {
    network_id
        .parse::<u64>()
        .map(Value::from)
        .unwrap_or_else(|_| Value::String(network_id.to_string()))
}

fn wallet_balance_matches_app_rules(balance: u64, required_balance_sats: u64) -> bool {
    balance == DEMO_NODE_FUNDING_SATS && balance >= required_balance_sats
}

fn extract_channel_points(value: &Value) -> Vec<String> {
    let mut channel_points = Vec::new();
    collect_channel_points(value, &mut channel_points);
    channel_points.sort();
    channel_points.dedup();
    channel_points
}

fn collect_channel_points(value: &Value, channel_points: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for key in [
                "channelPoint",
                "channel_point",
                "fundingTxid",
                "funding_txid",
            ] {
                if let Some(channel_point) = map
                    .get(key)
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    channel_points.push(channel_point.to_string());
                    break;
                }
            }

            if let (Some(txid), Some(output_index)) = (
                map.get("txid").and_then(Value::as_str),
                map.get("output_index")
                    .or_else(|| map.get("outputIndex"))
                    .and_then(value_as_id_string),
            ) {
                let txid = txid.trim();
                if !txid.is_empty() {
                    channel_points.push(format!("{txid}:{output_index}"));
                }
            }

            for nested in map.values() {
                collect_channel_points(nested, channel_points);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_channel_points(item, channel_points);
            }
        }
        _ => {}
    }
}

fn lightning_node_exists(value: &Value, network_id: &str, node_name: &str) -> bool {
    find_lightning_node_name(value, network_id, node_name).is_some()
}

fn find_lightning_node_name(value: &Value, network_id: &str, node_name: &str) -> Option<String> {
    lightning_node_summaries(value, network_id)
        .into_iter()
        .find_map(|node| {
            node.name
                .filter(|name| name.eq_ignore_ascii_case(node_name))
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

fn lightning_node_status(value: &Value, network_id: &str, node_name: &str) -> Option<String> {
    lightning_node_summaries(value, network_id)
        .into_iter()
        .find(|node| {
            node.name
                .as_deref()
                .map(|name| name.eq_ignore_ascii_case(node_name))
                .unwrap_or(false)
        })
        .and_then(|node| node.status)
}

fn bitcoin_backend_exists(value: &Value, network_id: &str, backend_name: &str) -> bool {
    find_network_value(value, network_id)
        .map(|network| value_contains_bitcoin_backend(network, backend_name))
        .unwrap_or(false)
}

fn value_contains_bitcoin_backend(value: &Value, backend_name: &str) -> bool {
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
            let name_matches = ["nodeName", "name", "displayName", "alias"]
                .iter()
                .any(|key| {
                    map.get(*key)
                        .and_then(Value::as_str)
                        .map(|name| name.eq_ignore_ascii_case(backend_name))
                        .unwrap_or(false)
                });

            (looks_like_bitcoin && name_matches)
                || map
                    .values()
                    .any(|nested| value_contains_bitcoin_backend(nested, backend_name))
        }
        Value::Array(items) => items
            .iter()
            .any(|item| value_contains_bitcoin_backend(item, backend_name)),
        _ => false,
    }
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
    list_networks_with_log_level(profile, DemoLogLevel::On).await
}

async fn list_networks_with_log_level(
    profile: &PolarAutomationProfile,
    log_level: DemoLogLevel,
) -> Result<Value, String> {
    execute_tool_with_log_level(profile, "list_networks", json!({}), log_level).await
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
    get_blockchain_height_from_resolved_with_log_level(profile, DemoLogLevel::On).await
}

async fn get_blockchain_height_from_resolved_with_log_level(
    profile: &PolarAutomationProfile,
    log_level: DemoLogLevel,
) -> Result<u64, String> {
    let network_id = clean_network_id(profile);
    let backend_name = clean_backend_name(profile);
    let response = execute_tool_with_log_level(
        profile,
        "get_blockchain_info",
        json!({
            "networkId": network_id_argument(&network_id),
            "nodeName": backend_name,
        }),
        log_level,
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

fn ensure_named_network_running(networks: &Value, network_name: &str) -> Result<(), String> {
    let network_id = find_network_id_by_name(networks, network_name)
        .ok_or_else(|| format!("Polar server {network_name} is not listed by that name."))?;
    ensure_network_started(networks, &network_id)
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
    execute_tool_with_log_level(profile, tool, arguments, DemoLogLevel::On).await
}

async fn execute_tool_with_log_level(
    profile: &PolarAutomationProfile,
    tool: &str,
    arguments: Value,
    log_level: DemoLogLevel,
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
        Some(error) => Err(format!("Polar MCP tool {tool} failed: {error}")),
        None => Ok(response),
    }
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

fn log_service_request(method: &str, url: &str, body: Option<&Value>, log_level: DemoLogLevel) {
    if !DEMO_SERVICE_LOG_LEVEL.allows(log_level) {
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
    log_level: DemoLogLevel,
) {
    if !DEMO_SERVICE_LOG_LEVEL.allows(log_level) {
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

fn extract_mcp_error(value: &Value) -> Option<String> {
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

fn find_network_id_by_name(value: &Value, requested_name: &str) -> Option<String> {
    match value {
        Value::Object(map) => {
            let name_matches = map
                .get("name")
                .and_then(Value::as_str)
                .map(|name| name.eq_ignore_ascii_case(requested_name.trim()))
                .unwrap_or(false);

            if name_matches {
                return map
                    .get("id")
                    .or_else(|| map.get("networkId"))
                    .and_then(value_as_id_string)
                    .or_else(|| map.get("name").and_then(Value::as_str).map(str::to_string));
            }

            for nested in map.values() {
                if let Some(id) = find_network_id_by_name(nested, requested_name) {
                    return Some(id);
                }
            }

            None
        }
        Value::Array(items) => items
            .iter()
            .find_map(|item| find_network_id_by_name(item, requested_name)),
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

fn extract_wallet_balance_sats(value: &Value) -> Option<u64> {
    match value {
        Value::Object(map) => {
            for key in [
                "confirmed_balance",
                "confirmedBalance",
                "total_balance",
                "totalBalance",
                "balance",
                "sats",
                "sat",
            ] {
                if let Some(balance) = map.get(key).and_then(value_as_u64) {
                    return Some(balance);
                }
            }

            map.values().find_map(extract_wallet_balance_sats)
        }
        Value::Array(items) => items.iter().find_map(extract_wallet_balance_sats),
        Value::String(text) => extract_sats_from_text(text),
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

fn extract_sats_from_text(text: &str) -> Option<u64> {
    let normalized = text.to_ascii_lowercase();
    let bytes = normalized.as_bytes();

    for (index, _) in normalized.match_indices("sat") {
        let prefix = &bytes[..index];
        let mut digits = Vec::new();
        let mut started = false;

        for byte in prefix.iter().rev() {
            if byte.is_ascii_digit() {
                digits.push(*byte);
                started = true;
            } else if !started && byte.is_ascii_whitespace() {
                continue;
            } else if started && (*byte == b',' || *byte == b'_' || *byte == b' ') {
                continue;
            } else if started {
                break;
            }
        }

        if digits.is_empty() {
            continue;
        }

        digits.reverse();
        let value = String::from_utf8(digits).ok()?;
        if let Ok(sats) = value.parse::<u64>() {
            return Some(sats);
        }
    }

    None
}

#[cfg(target_arch = "wasm32")]
async fn wait_for_demo_node_start_delay() {
    gloo_timers::future::TimeoutFuture::new(DEMO_NODE_START_DELAY_MS).await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn wait_for_demo_node_start_delay() {
    futures_timer::Delay::new(std::time::Duration::from_millis(
        DEMO_NODE_START_DELAY_MS.into(),
    ))
    .await;
}

#[cfg(target_arch = "wasm32")]
async fn wait_for_demo_node_ready_delay() {
    gloo_timers::future::TimeoutFuture::new(DEMO_NODE_READY_DELAY_MS).await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn wait_for_demo_node_ready_delay() {
    futures_timer::Delay::new(std::time::Duration::from_millis(
        DEMO_NODE_READY_DELAY_MS.into(),
    ))
    .await;
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

    fn healthy_polar_lab_response() -> Value {
        json!({
            "networks": [
                {
                    "id": 1,
                    "name": DEFAULT_NETWORK_NAME,
                    "status": "Started",
                    "nodes": {
                        "bitcoin": [
                            {
                                "name": DEFAULT_BITCOIN_BACKEND_NAME,
                                "type": "bitcoin",
                                "implementation": "bitcoind"
                            }
                        ],
                        "lightning": [
                            {
                                "name": "alice",
                                "type": "lightning",
                                "implementation": "LND",
                                "status": "Started"
                            },
                            {
                                "name": "bob",
                                "type": "lightning",
                                "implementation": "LND",
                                "status": "Started"
                            },
                            {
                                "name": "carol",
                                "type": "lightning",
                                "implementation": "LND",
                                "status": "Started"
                            }
                        ]
                    }
                }
            ]
        })
    }

    fn health_profile() -> PolarAutomationProfile {
        PolarAutomationProfile {
            bridge_url: "http://localhost:37373".to_string(),
            network_id: DEFAULT_NETWORK_NAME.to_string(),
            bitcoin_backend_name: DEFAULT_BITCOIN_BACKEND_NAME.to_string(),
        }
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
    fn step_two_server_lookup_matches_name_not_unrelated_running_id() {
        let networks = json!({
            "networks": [
                {
                    "id": 7,
                    "name": "other-running-server",
                    "status": "Started",
                    "nodes": {}
                },
                {
                    "id": 8,
                    "name": DEFAULT_NETWORK_NAME,
                    "status": "Stopped",
                    "nodes": {}
                }
            ]
        });

        assert_eq!(
            find_network_id_by_name(&networks, DEFAULT_NETWORK_NAME),
            Some("8".to_string())
        );
        assert_ne!(
            find_network_id_by_name(&networks, DEFAULT_NETWORK_NAME),
            Some("7".to_string())
        );
    }

    #[test]
    fn step_two_server_lookup_does_not_treat_typed_name_as_id() {
        let networks = json!({
            "networks": [
                {
                    "id": 1,
                    "name": "other-server",
                    "status": "Started",
                    "nodes": {}
                }
            ]
        });

        assert_eq!(find_network_id(&networks, "1"), Some("1".to_string()));
        assert_eq!(find_network_id_by_name(&networks, "1"), None);
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
    fn finds_existing_lightning_node_with_exact_stored_name() {
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
                                "name": "Alice",
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
            find_lightning_node_name(&networks, "1", "alice"),
            Some("Alice".to_string())
        );
        assert_ne!(
            find_lightning_node_name(&networks, "1", "alice"),
            Some("alice".to_string())
        );
    }

    #[test]
    fn demo_node_balance_must_match_app_funding_exactly() {
        assert!(wallet_balance_matches_app_rules(
            DEMO_NODE_FUNDING_SATS,
            250_000
        ));
        assert!(!wallet_balance_matches_app_rules(
            DEMO_NODE_FUNDING_SATS + 1,
            250_000
        ));
        assert!(!wallet_balance_matches_app_rules(
            DEMO_NODE_FUNDING_SATS - 1,
            250_000
        ));
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
    fn extracts_wallet_balance_from_lnd_wallet_info() {
        let wallet_info = json!({
            "success": true,
            "nodeName": "alice",
            "confirmed_balance": "250000",
            "unconfirmed_balance": "750000"
        });

        assert_eq!(extract_wallet_balance_sats(&wallet_info), Some(250000));
    }

    #[test]
    fn extracts_wallet_balance_from_text_sats() {
        let wallet_info = json!({
            "success": true,
            "result": "Wallet balance for alice: 1,000,000 sats"
        });

        assert_eq!(extract_wallet_balance_sats(&wallet_info), Some(1_000_000));
    }

    #[test]
    fn extracts_top_level_mcp_error_response() {
        let response = json!({
            "error": "14 UNAVAILABLE: No connection established. Last error: connect ECONNREFUSED 127.0.0.1:10001"
        });

        assert_eq!(
            extract_mcp_error(&response),
            Some(
                "14 UNAVAILABLE: No connection established. Last error: connect ECONNREFUSED 127.0.0.1:10001"
                    .to_string()
            )
        );
    }

    #[test]
    fn extracts_structured_mcp_error_message_when_success_is_false() {
        let response = json!({
            "success": false,
            "error": {
                "message": "LND is still starting"
            }
        });

        assert_eq!(
            extract_mcp_error(&response),
            Some("LND is still starting".to_string())
        );
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

    #[test]
    fn health_classifier_accepts_healthy_lab() {
        let networks = healthy_polar_lab_response();

        let report = inspect_lab_health(&health_profile(), &networks).unwrap();

        assert_eq!(report.network_id, "1");
        assert_eq!(report.bitcoin_backend_name, DEFAULT_BITCOIN_BACKEND_NAME);
    }

    #[test]
    fn health_classifier_detects_missing_network() {
        let networks = json!({ "networks": [] });

        assert_eq!(
            inspect_lab_health(&health_profile(), &networks),
            Err(PolarLabHealthIssue::NetworkMissing {
                network_id: DEFAULT_NETWORK_NAME.to_string()
            })
        );
    }

    #[test]
    fn health_classifier_detects_stopped_network() {
        let mut networks = healthy_polar_lab_response();
        networks["networks"][0]["status"] = json!("Stopped");

        assert_eq!(
            inspect_lab_health(&health_profile(), &networks),
            Err(PolarLabHealthIssue::NetworkStopped {
                network_id: "1".to_string(),
                status: "Stopped".to_string()
            })
        );
    }

    #[test]
    fn health_classifier_detects_missing_bitcoin_backend() {
        let mut networks = healthy_polar_lab_response();
        networks["networks"][0]["nodes"]["bitcoin"] = json!([]);

        assert_eq!(
            inspect_lab_health(&health_profile(), &networks),
            Err(PolarLabHealthIssue::BitcoinBackendMissing {
                network_id: "1".to_string(),
                backend_name: DEFAULT_BITCOIN_BACKEND_NAME.to_string()
            })
        );
    }

    #[test]
    fn health_classifier_detects_missing_demo_node() {
        let mut networks = healthy_polar_lab_response();
        networks["networks"][0]["nodes"]["lightning"] = json!([
            {
                "name": "alice",
                "type": "lightning",
                "implementation": "LND",
                "status": "Started"
            },
            {
                "name": "bob",
                "type": "lightning",
                "implementation": "LND",
                "status": "Started"
            }
        ]);

        assert_eq!(
            inspect_lab_health(&health_profile(), &networks),
            Err(PolarLabHealthIssue::DemoNodeMissing {
                network_id: "1".to_string(),
                node_id: DemoNodeId::Carol
            })
        );
    }

    #[test]
    fn health_classifier_detects_stopped_demo_node() {
        let mut networks = healthy_polar_lab_response();
        networks["networks"][0]["nodes"]["lightning"][1]["status"] = json!("Stopped");

        assert_eq!(
            inspect_lab_health(&health_profile(), &networks),
            Err(PolarLabHealthIssue::DemoNodeStopped {
                network_id: "1".to_string(),
                node_id: DemoNodeId::Bob,
                status: Some("Stopped".to_string())
            })
        );
    }
}

#[cfg(target_arch = "wasm32")]
async fn get_json(
    profile: &PolarAutomationProfile,
    path: &str,
    log_level: DemoLogLevel,
) -> Result<Value, String> {
    let url = bridge_url(profile, path);
    log_service_request("GET", &url, None, log_level);
    let result = match gloo_net::http::Request::get(&url).send().await {
        Ok(response) => response
            .json::<Value>()
            .await
            .map_err(|error| format!("Polar bridge returned invalid JSON: {error}")),
        Err(error) => Err(format!("Cannot reach Polar bridge: {error}")),
    };
    log_service_response("GET", &url, &result, log_level);
    result
}

#[cfg(target_arch = "wasm32")]
async fn post_json(
    profile: &PolarAutomationProfile,
    path: &str,
    body: Value,
    log_level: DemoLogLevel,
) -> Result<Value, String> {
    let url = bridge_url(profile, path);
    log_service_request("POST", &url, Some(&body), log_level);
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
    log_service_response("POST", &url, &result, log_level);
    result
}

#[cfg(not(target_arch = "wasm32"))]
async fn get_json(
    profile: &PolarAutomationProfile,
    path: &str,
    log_level: DemoLogLevel,
) -> Result<Value, String> {
    let url = bridge_url(profile, path);
    log_service_request("GET", &url, None, log_level);
    let result = match ureq::get(&url).call() {
        Ok(response) => response
            .into_json::<Value>()
            .map_err(|error| format!("Polar bridge returned invalid JSON: {error}")),
        Err(error) => Err(format!("Cannot reach Polar bridge: {error}")),
    };
    log_service_response("GET", &url, &result, log_level);
    result
}

#[cfg(not(target_arch = "wasm32"))]
async fn post_json(
    profile: &PolarAutomationProfile,
    path: &str,
    body: Value,
    log_level: DemoLogLevel,
) -> Result<Value, String> {
    let url = bridge_url(profile, path);
    log_service_request("POST", &url, Some(&body), log_level);
    let result = match ureq::post(&url)
        .set("Content-Type", "application/json")
        .send_json(body)
    {
        Ok(response) => response
            .into_json::<Value>()
            .map_err(|error| format!("Polar bridge returned invalid JSON: {error}")),
        Err(error) => Err(format!("Polar bridge request failed: {error}")),
    };
    log_service_response("POST", &url, &result, log_level);
    result
}
