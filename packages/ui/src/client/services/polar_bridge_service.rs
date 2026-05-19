use std::collections::HashSet;
use std::fmt;

use serde_json::{json, Value};

use crate::client::models::{
    DemoNodeId, PolarAutomationProfile, DEFAULT_BITCOIN_BACKEND_NAME, DEFAULT_NETWORK_NAME,
};
use crate::client::services::polar_mcp_connector::{self, PolarConnectorLogLevel};

const DEMO_NODE_FUNDING_SATS: u64 = 1_000_000;
const DEMO_NODE_START_TIMEOUT_SECONDS: u16 = 30;
const DEMO_NODE_START_ATTEMPTS: u16 = DEMO_NODE_START_TIMEOUT_SECONDS / 3;
const DEMO_NODE_START_DELAY_MS: u32 = 3_000;
const DEMO_NODE_NETWORK_RESTART_SETTLE_MS: u32 = 6_000;
const DEMO_NODE_READY_TIMEOUT_SECONDS: u16 = 90;
const DEMO_NODE_READY_DELAY_MS: u32 = 1_500;
const DEMO_NODE_READY_ATTEMPTS: u16 =
    ((DEMO_NODE_READY_TIMEOUT_SECONDS as u32 * 1_000) / DEMO_NODE_READY_DELAY_MS) as u16;
const TAPROOT_NODE_START_ATTEMPTS: u16 = 20;
const GAME_TREASURY_READY_TIMEOUT_SECONDS: u16 = 240;
const GAME_TREASURY_READY_ATTEMPTS: u16 =
    ((GAME_TREASURY_READY_TIMEOUT_SECONDS as u32 * 1_000) / DEMO_NODE_READY_DELAY_MS) as u16;
const DELETE_NETWORK_TIMEOUT_SECONDS: u32 = 12;
const DELETE_NETWORK_SETTLE_MS: u32 = 5_000;
const DELETE_NETWORK_ATTEMPTS: u8 = 4;
const DELETE_NETWORK_STATUS_ATTEMPTS: u8 = 10;
const DELETE_ALL_NETWORK_PASSES: u8 = 3;
const TAPROOT_ASSETS_NODE_NAME: &str = "GAME_TAPROOT";
const LEGACY_TAPROOT_ASSETS_NODE_NAME: &str = "tapd";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TreasuryShellPolicy {
    AllowCreate,
    ReclaimOnly,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DemoLogLevel {
    Off,
    On,
    Verbose,
}

impl From<DemoLogLevel> for PolarConnectorLogLevel {
    fn from(value: DemoLogLevel) -> Self {
        match value {
            DemoLogLevel::Off => Self::Off,
            DemoLogLevel::On => Self::On,
            DemoLogLevel::Verbose => Self::Verbose,
        }
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

#[derive(Clone, Debug, PartialEq)]
pub struct PolarNetworkNodeNames {
    pub bitcoin_backend_name: String,
    pub game_treasury_name: Option<String>,
    pub user_node_names: Vec<(DemoNodeId, String)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DemoNodeFundingPlan {
    AlreadyFunded,
    NeedsFunding(u64),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DemoNodePreparation {
    created_node: bool,
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
        return Ok(PolarServerEnsureResult {
            profile: automation_profile_from_network(profile, &networks, network_id),
            status: PolarServerEnsureStatus::Existed,
        });
    }

    create_network(profile, &requested_name).await?;

    let network_id = wait_for_network_id_by_name(profile, &requested_name).await?;
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

pub async fn read_network_node_names(
    profile: &PolarAutomationProfile,
) -> Result<PolarNetworkNodeNames, String> {
    test_bridge(profile).await?;
    let networks = list_networks(profile).await?;
    let network_id = resolve_network_id(profile, &networks)?;
    let bitcoin_backend_name = find_bitcoin_backend_name(&networks, &network_id)
        .unwrap_or_else(|| resolve_backend_name(profile, &networks, &network_id));
    let game_treasury_name = find_lightning_node_name(
        &networks,
        &network_id,
        polar_node_name(DemoNodeId::GameTreasury),
    );
    let user_node_names = DemoNodeId::ALL
        .into_iter()
        .filter_map(|node_id| {
            find_lightning_node_name(&networks, &network_id, polar_node_name(node_id))
                .map(|name| (node_id, name))
        })
        .collect();

    Ok(PolarNetworkNodeNames {
        bitcoin_backend_name,
        game_treasury_name,
        user_node_names,
    })
}

pub async fn create_demo_nodes(
    profile: &PolarAutomationProfile,
    required_balance_sats: u64,
) -> Result<PolarAutomationProfile, String> {
    create_required_nodes_with_progress(profile, required_balance_sats, |_| {}).await
}

pub async fn create_demo_nodes_with_progress<F>(
    profile: &PolarAutomationProfile,
    required_balance_sats: u64,
    report_progress: F,
) -> Result<PolarAutomationProfile, String>
where
    F: FnMut(String),
{
    create_required_nodes_with_progress(profile, required_balance_sats, report_progress).await
}

pub async fn create_required_nodes_with_progress<F>(
    profile: &PolarAutomationProfile,
    _required_balance_sats: u64,
    mut report_progress: F,
) -> Result<PolarAutomationProfile, String>
where
    F: FnMut(String),
{
    let resolved_profile = resolve_automation_profile(profile).await?;
    let network_id = clean_network_id(&resolved_profile);

    report_progress("Reading current Polar node list...".to_string());
    delete_unwanted_required_topology_nodes(&resolved_profile, &network_id, &mut report_progress)
        .await?;

    report_progress(format!("Ensuring {DEFAULT_BITCOIN_BACKEND_NAME} exists..."));
    ensure_bitcoin_backend_node(&resolved_profile, &network_id).await?;

    ensure_game_treasury_node_shell(
        &resolved_profile,
        &network_id,
        TreasuryShellPolicy::AllowCreate,
    )
    .await?;

    for node_id in DemoNodeId::ALL {
        report_progress(format!(
            "Ensuring {} exists in Polar...",
            polar_node_name(node_id)
        ));
        create_or_prepare_demo_node(
            &resolved_profile,
            &network_id,
            DEFAULT_BITCOIN_BACKEND_NAME,
            node_id,
            &mut report_progress,
        )
        .await?;
    }

    for node_name in required_polar_base_node_names() {
        report_progress(format!("Requesting start for {node_name} in Polar..."));
        let networks = list_networks(&resolved_profile).await?;
        let status = any_node_status(&networks, &network_id, node_name)
            .unwrap_or_else(|| "not started".to_string());
        request_lightning_node_start(&resolved_profile, &network_id, node_name, &status).await?;
    }

    wait_for_named_polar_nodes_started(
        &resolved_profile,
        &network_id,
        &required_polar_base_node_names(),
    )
    .await?;

    report_progress(format!("Ensuring {TAPROOT_ASSETS_NODE_NAME} exists..."));
    ensure_taproot_assets_node(&PolarAutomationProfile {
        bitcoin_backend_name: DEFAULT_BITCOIN_BACKEND_NAME.to_string(),
        ..resolved_profile.clone()
    })
    .await?;

    for node_name in required_polar_node_names() {
        report_progress(format!("Requesting start for {node_name} in Polar..."));
        let networks = list_networks(&resolved_profile).await?;
        let status = any_node_status(&networks, &network_id, node_name)
            .unwrap_or_else(|| "not started".to_string());
        request_lightning_node_start(&resolved_profile, &network_id, node_name, &status).await?;
    }

    wait_for_required_polar_nodes_started(&resolved_profile, &network_id).await?;
    report_progress("Required Polar nodes are created and started.".to_string());

    Ok(PolarAutomationProfile {
        bitcoin_backend_name: DEFAULT_BITCOIN_BACKEND_NAME.to_string(),
        ..resolved_profile
    })
}

fn required_polar_node_names() -> [&'static str; 6] {
    [
        DEFAULT_BITCOIN_BACKEND_NAME,
        polar_node_name(DemoNodeId::GameTreasury),
        TAPROOT_ASSETS_NODE_NAME,
        polar_node_name(DemoNodeId::Alice),
        polar_node_name(DemoNodeId::Bob),
        polar_node_name(DemoNodeId::Carol),
    ]
}

fn required_polar_base_node_names() -> [&'static str; 5] {
    [
        DEFAULT_BITCOIN_BACKEND_NAME,
        polar_node_name(DemoNodeId::GameTreasury),
        polar_node_name(DemoNodeId::Alice),
        polar_node_name(DemoNodeId::Bob),
        polar_node_name(DemoNodeId::Carol),
    ]
}

async fn delete_unwanted_required_topology_nodes(
    profile: &PolarAutomationProfile,
    network_id: &str,
    report_progress: &mut impl FnMut(String),
) -> Result<(), String> {
    loop {
        let networks = list_networks(profile).await?;
        let Some(node) = first_unexpected_required_topology_node(&networks, network_id) else {
            return Ok(());
        };
        let Some(node_name) = node.name else {
            return Ok(());
        };

        if node.is_bitcoin
            && !bitcoin_node_exists(&networks, network_id, DEFAULT_BITCOIN_BACKEND_NAME)
        {
            report_progress(format!(
                "Renaming Bitcoin backend {node_name} to {DEFAULT_BITCOIN_BACKEND_NAME}..."
            ));
            rename_demo_node(
                profile,
                network_id,
                &node_name,
                DEFAULT_BITCOIN_BACKEND_NAME,
            )
            .await?;
            continue;
        }

        report_progress(format!("Deleting unexpected Polar node {node_name}..."));
        remove_demo_node_by_name(profile, network_id, &node_name).await?;
    }
}

fn first_unexpected_required_topology_node(
    value: &Value,
    network_id: &str,
) -> Option<LightningNodeSummary> {
    let required = required_polar_node_names();
    network_node_summaries(value, network_id)
        .into_iter()
        .find(|node| {
            node.name
                .as_deref()
                .map(|name| !required.contains(&name))
                .unwrap_or(false)
        })
}

#[cfg(test)]
fn first_unexpected_required_topology_node_name(value: &Value, network_id: &str) -> Option<String> {
    first_unexpected_required_topology_node(value, network_id).and_then(|node| node.name)
}

async fn ensure_bitcoin_backend_node(
    profile: &PolarAutomationProfile,
    network_id: &str,
) -> Result<(), String> {
    let networks = list_networks(profile).await?;
    if bitcoin_node_exists(&networks, network_id, DEFAULT_BITCOIN_BACKEND_NAME) {
        return Ok(());
    }

    for (tool, arguments) in add_bitcoin_node_attempts(network_id) {
        match execute_tool(profile, tool, arguments).await {
            Ok(_) => return Ok(()),
            Err(_) => {}
        }
    }

    let networks = list_networks(profile).await?;
    if bitcoin_node_exists(&networks, network_id, DEFAULT_BITCOIN_BACKEND_NAME) {
        Ok(())
    } else {
        Err(format!(
            "Polar bridge could not create Bitcoin backend {DEFAULT_BITCOIN_BACKEND_NAME}."
        ))
    }
}

fn add_bitcoin_node_attempts(network_id: &str) -> Vec<(&'static str, Value)> {
    vec![
        (
            "add_node",
            json!({
                "networkId": network_id_argument(network_id),
                "implementation": "bitcoind",
                "type": "bitcoin",
                "nodeType": "bitcoin",
                "name": DEFAULT_BITCOIN_BACKEND_NAME,
                "nodeName": DEFAULT_BITCOIN_BACKEND_NAME,
                "displayName": DEFAULT_BITCOIN_BACKEND_NAME,
                "alias": DEFAULT_BITCOIN_BACKEND_NAME,
            }),
        ),
        (
            "add_bitcoin_node",
            json!({
                "networkId": network_id_argument(network_id),
                "implementation": "bitcoind",
                "name": DEFAULT_BITCOIN_BACKEND_NAME,
                "nodeName": DEFAULT_BITCOIN_BACKEND_NAME,
            }),
        ),
    ]
}

pub async fn create_game_treasury_node(
    profile: &PolarAutomationProfile,
    required_balance_sats: u64,
) -> Result<PolarAutomationProfile, String> {
    let resolved_profile =
        resolve_started_automation_profile_with_log_level(profile, DemoLogLevel::On).await?;
    let network_id = clean_network_id(&resolved_profile);
    let backend_name = clean_backend_name(&resolved_profile);
    let mut report_progress = |_| {};

    ensure_game_treasury_node_shell(
        &resolved_profile,
        &network_id,
        TreasuryShellPolicy::AllowCreate,
    )
    .await?;
    let networks = list_networks(&resolved_profile).await?;
    let treasury_node_name = require_game_treasury_node_name(&networks, &network_id)?;
    let funding_plan = prepare_existing_game_treasury_node(
        &resolved_profile,
        &network_id,
        &backend_name,
        &treasury_node_name,
        required_balance_sats,
        &mut report_progress,
    )
    .await?;

    match funding_plan {
        DemoNodeFundingPlan::AlreadyFunded => {}
        DemoNodeFundingPlan::NeedsFunding(sats) => {
            deposit_demo_node_funds(
                &resolved_profile,
                &network_id,
                DemoNodeId::GameTreasury,
                sats,
            )
            .await?;
            mine_demo_blocks(&resolved_profile, &network_id, &backend_name).await?;
        }
    }

    wait_for_demo_node_ready(
        &resolved_profile,
        required_balance_sats,
        DemoNodeId::GameTreasury,
    )
    .await?;

    Ok(resolved_profile)
}

pub async fn fund_demo_user_nodes(
    profile: &PolarAutomationProfile,
    required_balance_sats: u64,
) -> Result<PolarAutomationProfile, String> {
    let resolved_profile =
        resolve_started_automation_profile_with_log_level(profile, DemoLogLevel::On).await?;
    let network_id = clean_network_id(&resolved_profile);
    let backend_name = clean_backend_name(&resolved_profile);

    for node_id in DemoNodeId::ALL {
        let node_name = polar_node_name(node_id);
        let balance =
            wait_for_lightning_wallet_balance(&resolved_profile, &network_id, node_name, node_id)
                .await?;
        let target_sats = demo_node_funding_target(node_id, required_balance_sats);
        if balance < target_sats {
            deposit_demo_node_funds(
                &resolved_profile,
                &network_id,
                node_id,
                target_sats - balance,
            )
            .await?;
            mine_demo_blocks(&resolved_profile, &network_id, &backend_name).await?;
        }
        wait_for_demo_node_ready(&resolved_profile, target_sats, node_id).await?;
    }

    Ok(resolved_profile)
}

pub async fn ensure_taproot_assets_node(
    profile: &PolarAutomationProfile,
) -> Result<PolarAutomationProfile, String> {
    let resolved_profile =
        resolve_started_automation_profile_with_log_level(profile, DemoLogLevel::On).await?;
    let network_id = clean_network_id(&resolved_profile);
    let backend_name = clean_backend_name(&resolved_profile);
    let treasury_node_name = polar_node_name(DemoNodeId::GameTreasury);
    let networks = list_networks(&resolved_profile).await?;

    if !lightning_node_exists(&networks, &network_id, treasury_node_name) {
        return Err(game_treasury_missing_message());
    }

    if find_taproot_assets_node_name(&networks, &network_id).is_some() {
        return Ok(resolved_profile);
    }

    let attempts = add_taproot_assets_node_attempts(&network_id, treasury_node_name, &backend_name);
    let mut errors = Vec::new();
    for (tool, arguments) in attempts {
        match execute_tool(&resolved_profile, tool, arguments).await {
            Ok(_) => {
                wait_for_taproot_assets_node(&resolved_profile, &network_id).await?;
                return Ok(resolved_profile);
            }
            Err(error) => errors.push(format!("{tool} failed: {error}")),
        }
    }

    Err(format!(
        "Polar bridge could not create Taproot Assets node {TAPROOT_ASSETS_NODE_NAME}. {}",
        errors.join("; ")
    ))
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

        let channels = match list_node_channels(&resolved_profile, &network_id, node_name).await {
            Ok(channels) => channels,
            Err(message) if can_skip_channel_cleanup_error(&message) => {
                report_progress(format!(
                    "Channel cleanup could not inspect {} channels. Continuing setup...",
                    node_id.label()
                ));
                continue;
            }
            Err(message) => return Err(message),
        };

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
        remove_demo_node_by_name(&resolved_profile, &network_id, polar_node_name(node_id)).await?;
    }

    Ok(resolved_profile)
}

pub async fn delete_polar_network(profile: &PolarAutomationProfile) -> Result<(), String> {
    test_bridge(profile).await?;
    let networks = list_networks(profile).await?;
    let network_id = resolve_network_id(profile, &networks)?;
    delete_polar_network_by_id(profile, &network_id)
        .await
        .map_err(|error| {
            format!(
                "Polar bridge could not delete server {}. {error}",
                clean_network_id(profile)
            )
        })
}

pub async fn delete_all_polar_networks(profile: &PolarAutomationProfile) -> Result<usize, String> {
    delete_all_polar_networks_with_progress(profile, |_| {})
        .await
        .map(|result| result.deleted_count)
}

pub async fn count_polar_networks(profile: &PolarAutomationProfile) -> Result<usize, String> {
    test_bridge(profile).await?;
    let networks = list_networks(profile).await?;
    Ok(top_level_network_ids(&networks).len())
}

pub async fn delete_all_polar_networks_with_progress<F>(
    profile: &PolarAutomationProfile,
    mut report_progress: F,
) -> Result<PolarDeleteAllResult, String>
where
    F: FnMut(PolarDeleteAllProgress),
{
    test_bridge(profile).await?;
    let networks = list_networks(profile).await?;
    let networks = top_level_network_records(&networks);
    let total = networks.len();
    let mut deleted = 0;
    let mut failed_networks = Vec::new();
    let mut remaining = networks;

    report_progress(PolarDeleteAllProgress::new(deleted, total));
    if remaining.is_empty() {
        return Ok(PolarDeleteAllResult {
            deleted_count: 0,
            failed_networks,
            remaining_networks: Vec::new(),
        });
    }

    for pass in 1..=DELETE_ALL_NETWORK_PASSES {
        failed_networks.clear();
        let pass_networks = remaining;
        for network in pass_networks {
            let mut progress = PolarDeleteAllProgress::new(deleted, total);
            progress.current_network_id = Some(network.id.clone());
            progress.current_network_name = Some(network.name.clone());
            progress.failed = failed_networks.len();
            report_progress(progress);

            match delete_all_network_record(profile, &network).await {
                Ok(()) => {
                    deleted += 1;
                    let mut progress = PolarDeleteAllProgress::new(deleted, total);
                    progress.current_network_id = Some(network.id.clone());
                    progress.current_network_name = Some(network.name.clone());
                    progress.failed = failed_networks.len();
                    report_progress(progress);
                }
                Err(error) => {
                    failed_networks.push(PolarDeleteAllNetworkFailure {
                        network: network.clone(),
                        error,
                    });
                    let mut progress = PolarDeleteAllProgress::new(deleted, total);
                    progress.current_network_id = Some(network.id.clone());
                    progress.current_network_name = Some(network.name.clone());
                    progress.failed = failed_networks.len();
                    report_progress(progress);
                }
            }
        }

        remaining = list_networks(profile)
            .await
            .map(|networks| top_level_network_records(&networks))
            .map_err(|error| {
                format!(
                    "Deleted {deleted} Polar network(s), but could not verify the final network list: {error}"
                )
            })?;

        if remaining.is_empty() {
            failed_networks.clear();
            break;
        }

        if pass < DELETE_ALL_NETWORK_PASSES {
            log_to_terminal(&format!(
                "[polar-service] delete-all-retry pass={}/{DELETE_ALL_NETWORK_PASSES} remaining={}",
                pass + 1,
                remaining.len()
            ));
            wait_for_delete_network_settle_delay().await;
        }
    }

    let result = PolarDeleteAllResult {
        deleted_count: deleted,
        failed_networks,
        remaining_networks: remaining,
    };

    if result.failed_networks.is_empty() && result.remaining_networks.is_empty() {
        Ok(result)
    } else {
        Err(delete_all_networks_failure_message(&result))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolarDeleteAllProgress {
    pub deleted: usize,
    pub total: usize,
    pub current_network_id: Option<String>,
    pub current_network_name: Option<String>,
    pub failed: usize,
}

impl PolarDeleteAllProgress {
    fn new(deleted: usize, total: usize) -> Self {
        Self {
            deleted,
            total,
            current_network_id: None,
            current_network_name: None,
            failed: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolarDeleteAllResult {
    pub deleted_count: usize,
    pub failed_networks: Vec<PolarDeleteAllNetworkFailure>,
    pub remaining_networks: Vec<PolarNetworkRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolarDeleteAllNetworkFailure {
    pub network: PolarNetworkRecord,
    pub error: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolarNetworkRecord {
    pub id: String,
    pub name: String,
    pub status: String,
}

async fn delete_all_network_record(
    profile: &PolarAutomationProfile,
    network: &PolarNetworkRecord,
) -> Result<(), String> {
    log_to_terminal(&format!(
        "[polar-service] delete-all-network-start network={} name={} status={}",
        network.id, network.name, network.status
    ));

    match delete_network_preparation_for_status(&network.status) {
        DeleteNetworkPreparation::StopThenDelete => {
            if let Err(error) = stop_network_for_delete_all_sweep(profile, &network.id).await {
                log_to_terminal(&format!(
                    "[polar-service] delete-all-stop-ignored network={} error={}",
                    network.id,
                    redact_sensitive_log_text(&error)
                ));
            }
        }
        DeleteNetworkPreparation::WaitThenDelete | DeleteNetworkPreparation::DeleteNow => {}
    }

    match delete_all_network_once_with_timeout(profile.clone(), network.id.clone()).await {
        Ok(()) => Ok(()),
        Err(error) if is_network_already_gone_error(&error) => Ok(()),
        Err(error) => Err(delete_network_error_message(&network.id, error)),
    }
}

async fn stop_network_for_delete_all_sweep(
    profile: &PolarAutomationProfile,
    network_id: &str,
) -> Result<(), String> {
    let stop = stop_network(profile, network_id);
    let timeout = delete_network_timeout_delay();
    futures::pin_mut!(stop);
    futures::pin_mut!(timeout);

    match futures::future::select(stop, timeout).await {
        futures::future::Either::Left((result, _)) => result,
        futures::future::Either::Right((_, _)) => Err(format!(
            "Timed out after {DELETE_NETWORK_TIMEOUT_SECONDS}s while requesting stop for Polar network {network_id}."
        )),
    }
}

async fn delete_all_network_once_with_timeout(
    profile: PolarAutomationProfile,
    network_id: String,
) -> Result<(), String> {
    let (_, result) = delete_polar_network_by_id_once_with_timeout(profile, network_id).await;
    result
}

fn delete_all_networks_failure_message(result: &PolarDeleteAllResult) -> String {
    if result.failed_networks.is_empty() {
        let mut message = format!(
            "Deleted {deleted} Polar network(s), but {} network(s) are still visible to the local Polar bridge.",
            result.remaining_networks.len(),
            deleted = result.deleted_count,
        );

        if !result.remaining_networks.is_empty() {
            message.push_str(" Remaining networks: ");
            message.push_str(&network_records_summary(&result.remaining_networks));
            message.push('.');
        }

        return message;
    }

    let mut message = format!(
        "Deleted {deleted} Polar network(s), but {} network(s) failed: {}",
        result.failed_networks.len(),
        failed_networks_summary(&result.failed_networks),
        deleted = result.deleted_count,
    );

    if delete_all_failures_include_corrupted_polar_records(&result.failed_networks) {
        message.push_str(
            ". Polar is still listing these networks, but its lifecycle helper cannot load their configuration. Restart Polar and the local Polar bridge, then run Delete all networks again.",
        );
    }

    if !result.remaining_networks.is_empty() {
        if !message.ends_with('.') {
            message.push('.');
        }
        message.push_str(" Remaining networks: ");
        message.push_str(&network_records_summary(&result.remaining_networks));
        message.push('.');
    }

    message
}

fn delete_all_failures_include_corrupted_polar_records(
    failures: &[PolarDeleteAllNetworkFailure],
) -> bool {
    failures.iter().any(|failure| {
        let error = failure.error.to_ascii_lowercase();
        error.contains("no configuration file provided")
            || error.contains("cannot read property 'nodes' of undefined")
    })
}

fn failed_networks_summary(failures: &[PolarDeleteAllNetworkFailure]) -> String {
    failures
        .iter()
        .map(|failure| {
            format!(
                "{} {} ({}): {}",
                failure.network.id, failure.network.name, failure.network.status, failure.error
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn network_records_summary(networks: &[PolarNetworkRecord]) -> String {
    networks
        .iter()
        .map(|network| format!("{} {} ({})", network.id, network.name, network.status))
        .collect::<Vec<_>>()
        .join("; ")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeleteNetworkOutcome {
    Deleted,
    AlreadyGone,
}

async fn delete_polar_network_by_id(
    profile: &PolarAutomationProfile,
    network_id: &str,
) -> Result<(), String> {
    delete_polar_network_by_id_with_retries(profile, network_id)
        .await
        .map(|_| ())
}

async fn delete_polar_network_by_id_with_retries(
    profile: &PolarAutomationProfile,
    network_id: &str,
) -> Result<DeleteNetworkOutcome, String> {
    log_to_terminal(&format!(
        "[polar-service] delete-network-start network={network_id}"
    ));

    let mut last_error = None;
    for attempt in 1..=DELETE_NETWORK_ATTEMPTS {
        if !network_exists(profile, network_id).await? {
            return Ok(DeleteNetworkOutcome::AlreadyGone);
        }

        match delete_polar_network_by_id_once(profile, network_id).await {
            Ok(()) => {
                wait_for_network_removed(profile, network_id).await?;
                return Ok(DeleteNetworkOutcome::Deleted);
            }
            Err(error) if is_network_already_gone_error(&error) => {
                if !network_exists(profile, network_id).await? {
                    return Ok(DeleteNetworkOutcome::AlreadyGone);
                }
                last_error = Some(error);
            }
            Err(error)
                if attempt < DELETE_NETWORK_ATTEMPTS
                    && is_retryable_delete_network_error(&error) =>
            {
                log_to_terminal(&format!(
                    "[polar-service] delete-network-retry network={network_id} attempt={attempt}/{DELETE_NETWORK_ATTEMPTS} error={}",
                    redact_sensitive_log_text(&error)
                ));
                last_error = Some(error);
                wait_for_delete_network_settle_delay().await;
            }
            Err(error) => return Err(error),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        format!(
            "Polar bridge could not delete network {network_id} after {DELETE_NETWORK_ATTEMPTS} attempts."
        )
    }))
}

async fn delete_polar_network_by_id_once(
    profile: &PolarAutomationProfile,
    network_id: &str,
) -> Result<(), String> {
    maybe_stop_network_before_delete(profile, network_id).await?;

    let (_, result) =
        delete_polar_network_by_id_once_with_timeout(profile.clone(), network_id.to_string()).await;
    result
}

async fn maybe_stop_network_before_delete(
    profile: &PolarAutomationProfile,
    network_id: &str,
) -> Result<(), String> {
    let networks = list_networks(profile).await?;
    let status = find_network_status(&networks, network_id)
        .unwrap_or_else(|| "unknown".to_string())
        .to_ascii_lowercase();

    match delete_network_preparation_for_status(&status) {
        DeleteNetworkPreparation::StopThenDelete => {
            stop_network(profile, network_id).await?;
            wait_for_network_stopped_before_delete(profile, network_id).await?;
        }
        DeleteNetworkPreparation::WaitThenDelete => {
            wait_for_network_stopped_before_delete(profile, network_id).await?;
        }
        DeleteNetworkPreparation::DeleteNow => {}
    }

    wait_for_delete_network_settle_delay().await;
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeleteNetworkPreparation {
    StopThenDelete,
    WaitThenDelete,
    DeleteNow,
}

fn delete_network_preparation_for_status(status: &str) -> DeleteNetworkPreparation {
    match status.trim().to_ascii_lowercase().as_str() {
        "started" | "running" | "starting" => DeleteNetworkPreparation::StopThenDelete,
        "stopping" => DeleteNetworkPreparation::WaitThenDelete,
        _ => DeleteNetworkPreparation::DeleteNow,
    }
}

async fn delete_polar_network_by_id_once_without_timeout(
    profile: &PolarAutomationProfile,
    network_id: &str,
) -> Result<(), String> {
    execute_tool(
        profile,
        "delete_network",
        json!({ "networkId": network_id_argument(network_id) }),
    )
    .await
    .map(|_| ())
    .map_err(|error| delete_network_error_message(network_id, error))
}

fn delete_network_error_message(network_id: &str, error: String) -> String {
    if is_locked_polar_network_delete_error(&error) {
        return format!(
            "Polar could not delete network {network_id} because Windows still has Polar/LND files locked. Stop the network in Polar or close Polar, then retry."
        );
    }

    error
}

fn is_locked_polar_network_delete_error(error: &str) -> bool {
    let error = error.to_ascii_lowercase();
    error.contains("eperm")
        || error.contains("enotempty")
        || error.contains("operation not permitted")
        || error.contains("directory not empty")
}

fn is_retryable_delete_network_error(error: &str) -> bool {
    let error = error.to_ascii_lowercase();
    is_locked_polar_network_delete_error(&error)
        || error.contains("cannot read property 'nodes' of undefined")
        || error.contains("no configuration file provided")
        || error.contains("must be stopped")
        || error.contains("needs to be stopped")
        || error.contains("stop the network")
        || error.contains("stop network")
        || error.contains("network is currently stopping")
        || error.contains("network is currently starting")
        || error.contains("network is currently started")
        || error.contains("network is currently running")
        || error.contains("timed out after")
        || error.contains("timeout")
}

fn is_network_already_gone_error(error: &str) -> bool {
    let error = error.to_ascii_lowercase();
    error.contains("network not found")
        || error.contains("could not find network")
        || error.contains("not found")
            && !error.contains("no configuration file provided")
            && !error.contains("polar/lnd files locked")
}

async fn delete_polar_network_by_id_once_with_timeout(
    profile: PolarAutomationProfile,
    network_id: String,
) -> (String, Result<(), String>) {
    let network_id_for_delete = network_id.clone();
    let deletion =
        delete_polar_network_by_id_once_without_timeout(&profile, &network_id_for_delete);
    let timeout = delete_network_timeout_delay();
    futures::pin_mut!(deletion);
    futures::pin_mut!(timeout);

    let result = match futures::future::select(deletion, timeout).await {
        futures::future::Either::Left((result, _)) => result,
        futures::future::Either::Right((_, _)) => {
            Err(delete_network_timeout_message(&network_id_for_delete))
        }
    };

    (network_id, result)
}

fn delete_network_timeout_message(network_id: &str) -> String {
    format!(
        "Timed out after {DELETE_NETWORK_TIMEOUT_SECONDS}s while deleting Polar network {network_id}."
    )
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
) -> Result<DemoNodePreparation, String> {
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

        set_lightning_backend(profile, network_id, desired_name, backend_name).await?;

        return Ok(DemoNodePreparation {
            created_node: false,
        });
    }

    let before_add = list_networks(profile).await?;
    let created_name =
        create_lightning_node(profile, network_id, desired_name, &before_add).await?;
    debug_assert_eq!(created_name, desired_name);

    set_lightning_backend(profile, network_id, desired_name, backend_name).await?;

    Ok(DemoNodePreparation { created_node: true })
}

#[allow(dead_code)]
async fn determine_demo_node_funding_plan(
    profile: &PolarAutomationProfile,
    network_id: &str,
    backend_name: &str,
    node_id: DemoNodeId,
    report_progress: &mut impl FnMut(String),
) -> Result<DemoNodeFundingPlan, String> {
    let node_name = polar_node_name(node_id);
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
    debug_assert_eq!(created_name, node_name);
    set_lightning_backend(profile, network_id, node_name, backend_name).await?;
    request_lightning_node_start(profile, network_id, node_name, "not started").await?;
    wait_for_lightning_node_started(profile, network_id, node_name, backend_name).await?;

    Ok(DemoNodeFundingPlan::NeedsFunding(DEMO_NODE_FUNDING_SATS))
}

async fn prepare_existing_game_treasury_node(
    profile: &PolarAutomationProfile,
    network_id: &str,
    backend_name: &str,
    node_name: &str,
    required_balance_sats: u64,
    report_progress: &mut impl FnMut(String),
) -> Result<DemoNodeFundingPlan, String> {
    set_lightning_backend(profile, network_id, node_name, backend_name).await?;

    report_progress(format!("Starting {node_name} in Polar..."));
    start_node_if_needed(profile, network_id, node_name, backend_name).await?;

    let balance =
        wait_for_lightning_wallet_balance(profile, network_id, node_name, DemoNodeId::GameTreasury)
            .await?;
    let target_sats = demo_node_funding_target(DemoNodeId::GameTreasury, required_balance_sats);
    if balance >= target_sats {
        return Ok(DemoNodeFundingPlan::AlreadyFunded);
    }

    Ok(DemoNodeFundingPlan::NeedsFunding(target_sats - balance))
}

async fn set_lightning_backend(
    profile: &PolarAutomationProfile,
    network_id: &str,
    lightning_node_name: &str,
    bitcoin_node_name: &str,
) -> Result<(), String> {
    let result = execute_tool(
        profile,
        "set_lightning_backend",
        json!({
            "networkId": network_id_argument(network_id),
            "lightningNodeName": lightning_node_name,
            "bitcoinNodeName": bitcoin_node_name,
        }),
    )
    .await;

    match result {
        Ok(_) => Ok(()),
        Err(message) if is_lightning_backend_already_connected_error(&message) => Ok(()),
        Err(message) => Err(message),
    }
}

fn is_lightning_backend_already_connected_error(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    normalized.contains("already connected") && normalized.contains("backend")
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
        add_lightning_node_arguments(network_id, desired_name),
    )
    .await?;

    let after_add = list_networks(profile).await?;
    let created_name = validate_created_lightning_node_after_add(
        desired_name,
        &add_result,
        before_add,
        &after_add,
        network_id,
    )?;

    if created_name != desired_name {
        rename_demo_node(profile, network_id, &created_name, desired_name).await?;
        let after_rename = list_networks(profile).await?;
        if !lightning_node_exists(&after_rename, network_id, desired_name) {
            return Err(format!(
                "Polar created {created_name} while the app requested {desired_name}, and the app could not verify the rename. Retry after Polar finishes updating the network."
            ));
        }
    }

    Ok(desired_name.to_string())
}

async fn ensure_game_treasury_node_shell(
    profile: &PolarAutomationProfile,
    network_id: &str,
    policy: TreasuryShellPolicy,
) -> Result<(), String> {
    let networks = list_networks(profile).await?;
    let desired_name = polar_node_name(DemoNodeId::GameTreasury);
    if lightning_node_exists(&networks, network_id, desired_name) {
        return Ok(());
    }

    if let Some(alice_name) = find_reclaimable_default_alice_node(&networks, network_id) {
        rename_demo_node(profile, network_id, &alice_name, desired_name).await?;
        return Ok(());
    }

    if policy == TreasuryShellPolicy::ReclaimOnly {
        return Err(game_treasury_missing_message());
    }

    let before_add = list_networks(profile).await?;
    let created_name =
        create_lightning_node(profile, network_id, desired_name, &before_add).await?;
    debug_assert_eq!(created_name, desired_name);

    Ok(())
}

fn add_lightning_node_arguments(network_id: &str, desired_name: &str) -> Value {
    json!({
        "networkId": network_id_argument(network_id),
        "implementation": "LND",
        "name": desired_name,
        "nodeName": desired_name,
        "displayName": desired_name,
        "alias": desired_name,
    })
}

fn add_taproot_assets_node_attempts(
    network_id: &str,
    lightning_node_name: &str,
    bitcoin_node_name: &str,
) -> Vec<(&'static str, Value)> {
    vec![
        (
            "add_node",
            add_taproot_assets_node_arguments(
                network_id,
                "tapd",
                lightning_node_name,
                bitcoin_node_name,
            ),
        ),
        (
            "add_node",
            add_taproot_assets_node_arguments(
                network_id,
                "Taproot Assets",
                lightning_node_name,
                bitcoin_node_name,
            ),
        ),
        (
            "add_tap_node",
            add_taproot_assets_node_arguments(
                network_id,
                "tapd",
                lightning_node_name,
                bitcoin_node_name,
            ),
        ),
        (
            "add_taproot_node",
            add_taproot_assets_node_arguments(
                network_id,
                "tapd",
                lightning_node_name,
                bitcoin_node_name,
            ),
        ),
    ]
}

fn add_taproot_assets_node_arguments(
    network_id: &str,
    implementation: &str,
    lightning_node_name: &str,
    bitcoin_node_name: &str,
) -> Value {
    json!({
        "networkId": network_id_argument(network_id),
        "implementation": implementation,
        "type": "taproot",
        "nodeType": "taproot",
        "name": TAPROOT_ASSETS_NODE_NAME,
        "nodeName": TAPROOT_ASSETS_NODE_NAME,
        "displayName": TAPROOT_ASSETS_NODE_NAME,
        "alias": TAPROOT_ASSETS_NODE_NAME,
        "lightningNodeName": lightning_node_name,
        "lndNodeName": lightning_node_name,
        "bitcoinNodeName": bitcoin_node_name,
    })
}

fn remove_node_arguments(network_id: &str, node_name: &str) -> Value {
    let network_id = network_id_argument(network_id);
    json!({
        "networkId": network_id,
        "nodeName": node_name,
        "network": {
            "selected": network_id,
        },
        "node": {
            "selected": node_name,
        },
        "selected": {
            "networkId": network_id,
            "nodeName": node_name,
        },
    })
}

async fn remove_demo_node_by_name(
    profile: &PolarAutomationProfile,
    network_id: &str,
    node_name: &str,
) -> Result<(), String> {
    match remove_node_once(profile, network_id, node_name).await {
        Ok(()) => Ok(()),
        Err(message) if can_ignore_remove_node_error(&message) => Ok(()),
        Err(message) if is_remove_node_post_success_error(&message) => {
            wait_for_removed_node_after_ambiguous_error(profile, network_id, node_name).await
        }
        Err(message) if is_taproot_dependency_remove_error(&message) => {
            remove_taproot_assets_nodes(profile, network_id).await?;
            match remove_node_once(profile, network_id, node_name).await {
                Ok(()) => Ok(()),
                Err(retry_message) if can_ignore_remove_node_error(&retry_message) => Ok(()),
                Err(retry_message) if is_remove_node_post_success_error(&retry_message) => {
                    wait_for_removed_node_after_ambiguous_error(profile, network_id, node_name)
                        .await
                }
                Err(retry_message) => Err(retry_message),
            }
        }
        Err(message) => Err(message),
    }
}

async fn wait_for_removed_node_after_ambiguous_error(
    profile: &PolarAutomationProfile,
    network_id: &str,
    node_name: &str,
) -> Result<(), String> {
    for attempt in 1..=DEMO_NODE_START_ATTEMPTS {
        let networks = list_networks(profile).await?;
        if !non_bitcoin_node_summaries(&networks, network_id)
            .into_iter()
            .filter_map(|node| node.name)
            .any(|name| name.eq_ignore_ascii_case(node_name))
        {
            return Ok(());
        }

        log_to_terminal(&format!(
            "[polar-service] remove-node-ambiguous-wait network={network_id} node={node_name} attempt={attempt}/{DEMO_NODE_START_ATTEMPTS}"
        ));
        if attempt < DEMO_NODE_START_ATTEMPTS {
            wait_for_demo_node_start_delay().await;
        }
    }

    Err(format!(
        "Polar returned an ambiguous remove-node error for {node_name}, and the node is still listed. Retry after Polar finishes updating the network."
    ))
}

async fn remove_node_once(
    profile: &PolarAutomationProfile,
    network_id: &str,
    node_name: &str,
) -> Result<(), String> {
    execute_tool(
        profile,
        "remove_node",
        remove_node_arguments(network_id, node_name),
    )
    .await
    .map(|_| ())
}

async fn remove_taproot_assets_nodes(
    profile: &PolarAutomationProfile,
    network_id: &str,
) -> Result<(), String> {
    let networks = list_networks(profile).await?;
    for node_name in taproot_assets_node_names(&networks, network_id) {
        match remove_node_once(profile, network_id, &node_name).await {
            Ok(()) => {}
            Err(message) if can_ignore_remove_node_error(&message) => {}
            Err(message) => return Err(message),
        }
    }

    Ok(())
}

fn can_ignore_remove_node_error(message: &str) -> bool {
    message.to_ascii_lowercase().contains("not found")
}

fn is_remove_node_post_success_error(message: &str) -> bool {
    message
        .to_ascii_lowercase()
        .contains("cannot read property 'size' of undefined")
}

fn is_taproot_dependency_remove_error(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    message.contains("cannot remove a lightning node")
        && message.contains("taproot assets node connected")
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
    let node_name = polar_node_name(node_id);
    let starting_balance = get_lightning_wallet_balance(profile, network_id, node_name)
        .await
        .unwrap_or(0);
    let target_balance = starting_balance.saturating_add(sats);

    for attempt in 1..=2 {
        let result = execute_tool(
            profile,
            "deposit_funds",
            json!({
                "networkId": network_id_argument(network_id),
                "nodeName": node_name,
                "sats": sats,
            }),
        )
        .await;

        match result {
            Ok(_) => return Ok(()),
            Err(error) if is_transient_bridge_request_error(&error) && attempt == 1 => {
                wait_for_demo_node_ready_delay().await;
                if get_lightning_wallet_balance(profile, network_id, node_name)
                    .await
                    .map(|balance| balance >= target_balance)
                    .unwrap_or(false)
                {
                    return Ok(());
                }
            }
            Err(error) => return Err(error),
        }
    }

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

async fn start_node_if_needed(
    profile: &PolarAutomationProfile,
    network_id: &str,
    node_name: &str,
    backend_name: &str,
) -> Result<(), String> {
    let networks = list_networks(profile).await?;
    if lightning_node_is_started(&networks, network_id, node_name) {
        return Ok(());
    }

    let status = lightning_node_status(&networks, network_id, node_name)
        .unwrap_or_else(|| "not started".to_string());
    request_lightning_node_start(profile, network_id, node_name, &status).await?;
    wait_for_lightning_node_started(profile, network_id, node_name, backend_name).await
}

async fn wait_for_lightning_node_started(
    profile: &PolarAutomationProfile,
    network_id: &str,
    node_name: &str,
    _backend_name: &str,
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
            if attempt % 3 == 0 {
                log_to_terminal(&format!(
                    "[polar-service] node-start-cycle-restart node={node_name} attempt={attempt}"
                ));
                restart_network_for_node_start_recovery(profile, network_id).await?;

                let networks = list_networks(profile).await?;
                for node in DemoNodeId::ALL {
                    let node_name = polar_node_name(node);
                    let status = lightning_node_status(&networks, network_id, node_name)
                        .unwrap_or_else(|| "missing".to_string());
                    log_to_terminal(&format!(
                        "[polar-service] node-start-cycle-check network={network_id} node={node_name} status={status}"
                    ));
                }
                log_to_terminal(&format!(
                    "[polar-service] node-start-cycle-restart-complete node={node_name} attempt={attempt}"
                ));
            } else {
                wait_for_demo_node_start_delay().await;
            }
        }
    }

    Err(format!(
        "Polar demo node {node_name} did not finish starting within {DEMO_NODE_START_TIMEOUT_SECONDS} seconds. Last status: {}.",
        last_status.unwrap_or_else(|| "unknown".to_string())
    ))
}

#[allow(dead_code)]
async fn wait_for_lightning_nodes_started(
    profile: &PolarAutomationProfile,
    network_id: &str,
) -> Result<(), String> {
    for attempt in 1..=DEMO_NODE_START_ATTEMPTS {
        let networks = list_networks(profile).await?;
        let mut not_started_nodes = Vec::new();
        let mut statuses = Vec::new();

        for node_id in DemoNodeId::ALL {
            let node_name = polar_node_name(node_id);
            let status = lightning_node_status(&networks, network_id, node_name)
                .unwrap_or_else(|| "not started".to_string());
            statuses.push(format!("{node_name}={status}"));

            if !lightning_node_is_started(&networks, network_id, node_name) {
                not_started_nodes.push(node_name);
            }
        }

        if not_started_nodes.is_empty() {
            log_to_terminal(&format!(
                "[polar-service] step-5-node-ready attempt={attempt}/{DEMO_NODE_START_ATTEMPTS} statuses={}",
                statuses.join(", ")
            ));
            return Ok(());
        }

        log_to_terminal(&format!(
            "[polar-service] step-5-node-wait attempt={attempt}/{DEMO_NODE_START_ATTEMPTS} not_started_nodes={}; statuses={}",
            not_started_nodes.join(", "),
            statuses.join(", ")
        ));

        if attempt % 3 == 0 {
            log_to_terminal(&format!(
                "[polar-service] step-5-cycle-restart attempt={attempt} network={network_id}"
            ));
            restart_network_for_node_start_recovery(profile, network_id).await?;
        } else if attempt < DEMO_NODE_START_ATTEMPTS {
            wait_for_demo_node_start_delay().await;
        }
    }

    let networks = list_networks(profile).await?;
    let mut statuses = Vec::new();
    for node_id in DemoNodeId::ALL {
        let node_name = polar_node_name(node_id);
        statuses.push(format!(
            "{node_name}={}",
            lightning_node_status(&networks, network_id, node_name)
                .unwrap_or_else(|| "unknown".to_string())
        ));
    }

    Err(format!(
        "Polar demo nodes did not finish starting within {DEMO_NODE_START_TIMEOUT_SECONDS} seconds. Last statuses: {}.",
        statuses.join(", ")
    ))
}

async fn wait_for_required_polar_nodes_started(
    profile: &PolarAutomationProfile,
    network_id: &str,
) -> Result<(), String> {
    wait_for_named_polar_nodes_started(profile, network_id, &required_polar_node_names()).await
}

async fn wait_for_named_polar_nodes_started(
    profile: &PolarAutomationProfile,
    network_id: &str,
    node_names: &[&'static str],
) -> Result<(), String> {
    let mut last_started_count = 0usize;
    let mut no_progress_polls = 0u8;
    let mut restarted_network = false;

    for attempt in 1..=DEMO_NODE_START_ATTEMPTS {
        let networks = list_networks(profile).await?;
        let mut not_started_nodes = Vec::new();
        let mut statuses = Vec::new();
        let mut started_count = 0usize;

        for node_name in node_names.iter().copied() {
            let status = any_node_status(&networks, network_id, node_name)
                .unwrap_or_else(|| "not started".to_string());
            statuses.push(format!("{node_name}={status}"));

            if any_node_is_started(&networks, network_id, node_name) {
                started_count += 1;
            } else {
                not_started_nodes.push(node_name);
            }
        }

        if not_started_nodes.is_empty() {
            log_to_terminal(&format!(
                "[polar-service] required-node-ready attempt={attempt}/{DEMO_NODE_START_ATTEMPTS} statuses={}",
                statuses.join(", ")
            ));
            return Ok(());
        }

        if started_count > last_started_count {
            last_started_count = started_count;
            no_progress_polls = 0;
        } else {
            no_progress_polls = no_progress_polls.saturating_add(1);
        }

        log_to_terminal(&format!(
            "[polar-service] required-node-wait attempt={attempt}/{DEMO_NODE_START_ATTEMPTS} not_started_nodes={}; statuses={}",
            not_started_nodes.join(", "),
            statuses.join(", ")
        ));

        if required_node_restart_due(no_progress_polls, restarted_network) {
            log_to_terminal(&format!(
                "[polar-service] required-node-no-progress-restart network={network_id}"
            ));
            restart_network_for_node_start_recovery(profile, network_id).await?;
            restarted_network = true;
            no_progress_polls = 0;
        } else if attempt < DEMO_NODE_START_ATTEMPTS {
            wait_for_demo_node_start_delay().await;
        }
    }

    let networks = list_networks(profile).await?;
    let statuses = node_names
        .iter()
        .copied()
        .map(|node_name| {
            format!(
                "{node_name}={}",
                any_node_status(&networks, network_id, node_name)
                    .unwrap_or_else(|| "unknown".to_string())
            )
        })
        .collect::<Vec<_>>();

    Err(format!(
        "Polar nodes did not finish starting within {DEMO_NODE_START_TIMEOUT_SECONDS} seconds. Last statuses: {}.",
        statuses.join(", ")
    ))
}

fn required_node_restart_due(no_progress_polls: u8, restarted_network: bool) -> bool {
    no_progress_polls >= 6 && !restarted_network
}

async fn wait_for_lightning_wallet_balance(
    profile: &PolarAutomationProfile,
    network_id: &str,
    node_name: &str,
    node_id: DemoNodeId,
) -> Result<u64, String> {
    let mut last_error = None;

    let attempts = ready_attempts_for_node(node_id);
    for attempt in 1..=attempts {
        match get_lightning_wallet_balance(profile, network_id, node_name).await {
            Ok(balance) => return Ok(balance),
            Err(message) => {
                last_error = Some(message);
                if attempt < attempts {
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

#[allow(dead_code)]
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

    let attempts = ready_attempts_for_node(node_id);
    for attempt in 1..=attempts {
        match verify_demo_node_ready(profile, required_balance_sats, node_id).await {
            Ok(()) => return Ok(()),
            Err(message) => {
                last_error = Some(message);
                if attempt < attempts {
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

async fn wait_for_taproot_assets_node(
    profile: &PolarAutomationProfile,
    network_id: &str,
) -> Result<(), String> {
    for _ in 0..4 {
        wait_for_demo_node_start_delay().await;
    }

    for attempt in 1..=TAPROOT_NODE_START_ATTEMPTS {
        let networks = list_networks(profile).await?;
        if taproot_assets_node_exists(&networks, network_id) {
            log_to_terminal(&format!(
                "[polar-service] taproot-node-ready node={TAPROOT_ASSETS_NODE_NAME} attempt={attempt}"
            ));
            return Ok(());
        }

        log_to_terminal(&format!(
            "[polar-service] taproot-node-wait node={TAPROOT_ASSETS_NODE_NAME} attempt={attempt}/{TAPROOT_NODE_START_ATTEMPTS}"
        ));
        if attempt % 2 == 0 && attempt < TAPROOT_NODE_START_ATTEMPTS {
            log_to_terminal(&format!(
                "[polar-service] taproot-node-cycle-restart attempt={attempt} network={network_id}"
            ));
            restart_network_for_node_start_recovery(profile, network_id).await?;
        } else if attempt < TAPROOT_NODE_START_ATTEMPTS {
            wait_for_demo_node_start_delay().await;
        }
    }

    Err(format!(
        "Polar did not list Taproot Assets node {TAPROOT_ASSETS_NODE_NAME} after creation. Retry Game Treasury (TRAs) after Polar finishes updating the network."
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

    if !node_wallet_balance_matches_app_rules(node_id, balance, required_balance_sats) {
        let target_sats = demo_node_funding_target(node_id, required_balance_sats);
        return Err(format!(
            "{} has {balance} sats available, but the app needs {target_sats} sats for this setup step.",
            node_id.label()
        ));
    }

    Ok(())
}

#[allow(dead_code)]
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

#[allow(dead_code)]
fn wallet_balance_matches_app_rules(balance: u64, required_balance_sats: u64) -> bool {
    node_wallet_balance_matches_app_rules(DemoNodeId::Alice, balance, required_balance_sats)
}

fn node_wallet_balance_matches_app_rules(
    node_id: DemoNodeId,
    balance: u64,
    required_balance_sats: u64,
) -> bool {
    match node_id {
        DemoNodeId::GameTreasury => {
            balance >= demo_node_funding_target(node_id, required_balance_sats)
        }
        DemoNodeId::Alice | DemoNodeId::Bob | DemoNodeId::Carol => {
            balance == demo_node_funding_target(node_id, required_balance_sats)
        }
    }
}

async fn request_lightning_node_start(
    profile: &PolarAutomationProfile,
    network_id: &str,
    node_name: &str,
    status: &str,
) -> Result<(), String> {
    if status.eq_ignore_ascii_case("starting") {
        return Ok(());
    }

    let result = execute_tool(
        profile,
        "start_node",
        json!({
            "networkId": network_id_argument(network_id),
            "nodeName": node_name,
        }),
    )
    .await;

    match result {
        Ok(_) => Ok(()),
        Err(message) if is_lightning_node_already_started_error(&message) => Ok(()),
        Err(message) if is_retryable_node_start_request_error(&message) => {
            log_to_terminal(&format!(
                "[polar-service] node-start-request-deferred node={node_name} error={}",
                redact_sensitive_log_text(&message)
            ));
            Ok(())
        }
        Err(message) => Err(message),
    }
}

fn is_lightning_node_already_started_error(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    normalized.contains("start_node")
        && normalized.contains("currently started")
        && normalized.contains("only stopped or error")
}

fn is_retryable_node_start_request_error(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    normalized.contains("tool execution timed out")
        || normalized.contains("timed out")
        || normalized.contains("timeout")
        || normalized.contains("ports are not available")
        || normalized.contains("only one usage of each socket address")
}

fn demo_node_funding_target(node_id: DemoNodeId, required_balance_sats: u64) -> u64 {
    match node_id {
        DemoNodeId::GameTreasury => DEMO_NODE_FUNDING_SATS.max(required_balance_sats),
        DemoNodeId::Alice | DemoNodeId::Bob | DemoNodeId::Carol => DEMO_NODE_FUNDING_SATS,
    }
}

fn ready_attempts_for_node(node_id: DemoNodeId) -> u16 {
    match node_id {
        DemoNodeId::GameTreasury => GAME_TREASURY_READY_ATTEMPTS,
        DemoNodeId::Alice | DemoNodeId::Bob | DemoNodeId::Carol => DEMO_NODE_READY_ATTEMPTS,
    }
}

fn require_game_treasury_node_name(value: &Value, network_id: &str) -> Result<String, String> {
    find_lightning_node_name(value, network_id, polar_node_name(DemoNodeId::GameTreasury))
        .ok_or_else(game_treasury_missing_message)
}

fn game_treasury_missing_message() -> String {
    format!(
        "{} is missing. Retry Game Treasury so the app can prepare the treasury node before funding it.",
        polar_node_name(DemoNodeId::GameTreasury)
    )
}

fn find_reclaimable_default_alice_node(value: &Value, network_id: &str) -> Option<String> {
    let node_names: Vec<String> = lightning_node_summaries(value, network_id)
        .into_iter()
        .filter_map(|node| node.name)
        .collect();

    let has_treasury = node_names
        .iter()
        .any(|name| name.eq_ignore_ascii_case(polar_node_name(DemoNodeId::GameTreasury)));
    if has_treasury {
        return None;
    }

    let has_user_nodes = node_names
        .iter()
        .any(|name| name.eq_ignore_ascii_case("Bob") || name.eq_ignore_ascii_case("Carol"));
    if has_user_nodes {
        return None;
    }

    node_names
        .into_iter()
        .find(|name| name.eq_ignore_ascii_case("Alice"))
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

async fn wait_for_network_stopped_before_delete(
    profile: &PolarAutomationProfile,
    network_id: &str,
) -> Result<(), String> {
    let mut last_status = "missing".to_string();
    for attempt in 1..=DELETE_NETWORK_STATUS_ATTEMPTS {
        let networks = list_networks(profile).await?;
        let status = find_network_status(&networks, network_id);
        if status
            .as_deref()
            .map(network_status_allows_restart)
            .unwrap_or(true)
        {
            return Ok(());
        }
        last_status = status.unwrap_or_else(|| "missing".to_string());

        log_to_terminal(&format!(
            "[polar-service] delete-network-stop-wait network={network_id} attempt={attempt}/{DELETE_NETWORK_STATUS_ATTEMPTS} status={last_status}"
        ));
        if attempt < DELETE_NETWORK_STATUS_ATTEMPTS {
            wait_for_delete_network_settle_delay().await;
        }
    }

    Err(delete_network_stop_timeout_message(
        network_id,
        &last_status,
    ))
}

fn delete_network_stop_timeout_message(network_id: &str, status: &str) -> String {
    format!(
        "Polar network {network_id} did not stop before deletion. Last status: {status}. Stop the network in Polar, then run Delete all networks again."
    )
}

async fn wait_for_network_removed(
    profile: &PolarAutomationProfile,
    network_id: &str,
) -> Result<(), String> {
    for attempt in 1..=DELETE_NETWORK_STATUS_ATTEMPTS {
        if !network_exists(profile, network_id).await? {
            return Ok(());
        }

        log_to_terminal(&format!(
            "[polar-service] delete-network-removed-wait network={network_id} attempt={attempt}/{DELETE_NETWORK_STATUS_ATTEMPTS}"
        ));
        if attempt < DELETE_NETWORK_STATUS_ATTEMPTS {
            wait_for_delete_network_settle_delay().await;
        }
    }

    Err(format!(
        "Polar bridge reported delete success for network {network_id}, but the network is still listed."
    ))
}

async fn network_exists(
    profile: &PolarAutomationProfile,
    network_id: &str,
) -> Result<bool, String> {
    let networks = list_networks(profile).await?;
    Ok(find_network_status(&networks, network_id).is_some()
        || top_level_network_ids(&networks)
            .iter()
            .any(|id| id.eq_ignore_ascii_case(network_id)))
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
    is_taproot: bool,
    is_bitcoin: bool,
}

fn lightning_node_summaries(value: &Value, network_id: &str) -> Vec<LightningNodeSummary> {
    let mut nodes = Vec::new();
    if let Some(network) = find_network_value(value, network_id) {
        collect_lightning_node_summaries(network, &mut nodes);
    }

    nodes
}

fn non_bitcoin_node_summaries(value: &Value, network_id: &str) -> Vec<LightningNodeSummary> {
    let mut nodes = Vec::new();
    if let Some(network) = find_network_value(value, network_id) {
        collect_non_bitcoin_node_summaries(network, &mut nodes);
    }

    nodes
}

fn network_node_summaries(value: &Value, network_id: &str) -> Vec<LightningNodeSummary> {
    let mut nodes = Vec::new();
    if let Some(network) = find_network_value(value, network_id) {
        collect_network_node_summaries(network, &mut nodes);
    }

    nodes
}

fn bitcoin_node_exists(value: &Value, network_id: &str, node_name: &str) -> bool {
    network_node_summaries(value, network_id)
        .into_iter()
        .filter(|node| node.is_bitcoin)
        .filter_map(|node| node.name)
        .any(|name| name.eq_ignore_ascii_case(node_name))
}

fn any_node_status(value: &Value, network_id: &str, node_name: &str) -> Option<String> {
    network_node_summaries(value, network_id)
        .into_iter()
        .find(|node| {
            node.name
                .as_deref()
                .map(|name| name.eq_ignore_ascii_case(node_name))
                .unwrap_or(false)
        })
        .and_then(|node| node.status)
}

fn any_node_is_started(value: &Value, network_id: &str, node_name: &str) -> bool {
    any_node_status(value, network_id, node_name)
        .map(|status| {
            let status = status.to_ascii_lowercase();
            status.contains("started") || status.contains("running") || status.contains("online")
        })
        .unwrap_or(false)
}

fn taproot_assets_node_exists(value: &Value, network_id: &str) -> bool {
    find_taproot_assets_node_name(value, network_id).is_some()
}

fn taproot_assets_node_names(value: &Value, network_id: &str) -> Vec<String> {
    non_bitcoin_node_summaries(value, network_id)
        .into_iter()
        .filter(|node| node.is_taproot)
        .filter_map(|node| node.name)
        .collect()
}

fn find_taproot_assets_node_name(value: &Value, network_id: &str) -> Option<String> {
    let taproot_names = non_bitcoin_node_summaries(value, network_id)
        .into_iter()
        .filter(|node| node.is_taproot)
        .filter_map(|node| node.name)
        .collect::<Vec<_>>();

    taproot_names
        .iter()
        .find(|name| {
            name.eq_ignore_ascii_case(TAPROOT_ASSETS_NODE_NAME)
                || name.eq_ignore_ascii_case(LEGACY_TAPROOT_ASSETS_NODE_NAME)
        })
        .cloned()
        .or_else(|| taproot_names.into_iter().next())
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

fn collect_non_bitcoin_node_summaries(value: &Value, nodes: &mut Vec<LightningNodeSummary>) {
    match value {
        Value::Object(map) => {
            if looks_like_removable_non_bitcoin_node(value) {
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
                    is_taproot: looks_like_taproot_assets_node(value),
                    is_bitcoin: false,
                });
                return;
            }

            for nested in map.values() {
                collect_non_bitcoin_node_summaries(nested, nodes);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_non_bitcoin_node_summaries(item, nodes);
            }
        }
        _ => {}
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
                    is_taproot: false,
                    is_bitcoin: false,
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

fn collect_network_node_summaries(value: &Value, nodes: &mut Vec<LightningNodeSummary>) {
    match value {
        Value::Object(map) => {
            if looks_like_removable_non_bitcoin_node(value) || looks_like_bitcoin_node(value) {
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
                    is_taproot: looks_like_taproot_assets_node(value),
                    is_bitcoin: looks_like_bitcoin_node(value),
                });
                return;
            }

            for nested in map.values() {
                collect_network_node_summaries(nested, nodes);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_network_node_summaries(item, nodes);
            }
        }
        _ => {}
    }
}

fn looks_like_removable_non_bitcoin_node(value: &Value) -> bool {
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

    if type_text.contains("bitcoin") || type_text.contains("bitcoind") || type_text.contains("core")
    {
        return false;
    }

    type_text.contains("lightning")
        || type_text == "lnd"
        || type_text.contains("c-lightning")
        || type_text.contains("eclair")
        || type_text.contains("litd")
        || type_text.contains("tap")
        || type_text.contains("taproot")
}

fn looks_like_taproot_assets_node(value: &Value) -> bool {
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

    type_text.contains("tap") || type_text.contains("taproot") || type_text.contains("litd")
}

fn looks_like_bitcoin_node(value: &Value) -> bool {
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

    type_text.contains("bitcoin") || type_text.contains("bitcoind") || type_text.contains("core")
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
    let attempts = create_network_attempts(network_name);
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

fn create_network_attempts(network_name: &str) -> Vec<(&'static str, Value)> {
    vec![
        (
            "create_network",
            json!({
                "name": network_name,
                "bitcoinNodes": [{ "name": DEFAULT_BITCOIN_BACKEND_NAME, "implementation": "bitcoind" }],
                "lightningNodes": [],
                "tapNodes": [],
            }),
        ),
        (
            "create_network",
            json!({
                "networkName": network_name,
                "bitcoinNodes": [{ "name": DEFAULT_BITCOIN_BACKEND_NAME, "implementation": "bitcoind" }],
                "lightningNodes": [],
                "tapNodes": [],
            }),
        ),
        (
            "create_network",
            json!({
                "name": network_name,
                "nodes": {
                    "bitcoin": [{ "name": DEFAULT_BITCOIN_BACKEND_NAME, "implementation": "bitcoind" }],
                    "lightning": [],
                    "tap": [],
                },
            }),
        ),
        (
            "create_network",
            json!({
                "networkName": network_name,
                "nodes": {
                    "bitcoin": [{ "name": DEFAULT_BITCOIN_BACKEND_NAME, "implementation": "bitcoind" }],
                    "lightning": [],
                    "tap": [],
                },
            }),
        ),
        (
            "add_network",
            json!({
                "name": network_name,
                "bitcoinNodes": [{ "name": DEFAULT_BITCOIN_BACKEND_NAME, "implementation": "bitcoind" }],
                "lightningNodes": [],
                "tapNodes": [],
            }),
        ),
        (
            "add_network",
            json!({
                "networkName": network_name,
                "bitcoinNodes": [{ "name": DEFAULT_BITCOIN_BACKEND_NAME, "implementation": "bitcoind" }],
                "lightningNodes": [],
                "tapNodes": [],
            }),
        ),
        (
            "add_network",
            json!({
                "name": network_name,
                "nodes": {
                    "bitcoin": [{ "name": DEFAULT_BITCOIN_BACKEND_NAME, "implementation": "bitcoind" }],
                    "lightning": [],
                    "tap": [],
                },
            }),
        ),
        (
            "add_network",
            json!({
                "networkName": network_name,
                "nodes": {
                    "bitcoin": [{ "name": DEFAULT_BITCOIN_BACKEND_NAME, "implementation": "bitcoind" }],
                    "lightning": [],
                    "tap": [],
                },
            }),
        ),
    ]
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
        return find_network_id(networks, &requested).ok_or_else(|| {
            format!(
                "Polar network {requested} is not listed by the current Polar bridge. Return to Server Name, choose the intended Polar server, and retry before the app adds or removes nodes."
            )
        });
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
    if !requested.is_empty() && bitcoin_backend_exists(networks, network_id, &requested) {
        return requested;
    }

    find_bitcoin_backend_name(networks, network_id).unwrap_or_else(|| {
        if requested.is_empty() {
            DEFAULT_BITCOIN_BACKEND_NAME.to_string()
        } else {
            requested
        }
    })
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

#[allow(dead_code)]
fn ensure_named_network_running(networks: &Value, network_name: &str) -> Result<(), String> {
    let network_id = find_network_id_by_name(networks, network_name)
        .ok_or_else(|| format!("Polar server {network_name} is not listed by that name."))?;
    ensure_network_started(networks, &network_id)
}

#[allow(dead_code)]
async fn ensure_network_running(
    profile: &PolarAutomationProfile,
    network_id: &str,
) -> Result<(), String> {
    let networks = list_networks(profile).await?;
    if network_is_started(&networks, network_id) {
        return Ok(());
    }

    start_network(profile, network_id).await?;
    wait_for_network_started(profile, network_id).await
}

async fn start_network(profile: &PolarAutomationProfile, network_id: &str) -> Result<(), String> {
    let attempts = [(
        "start_network",
        json!({ "networkId": network_id_argument(network_id) }),
    )];
    let mut errors = Vec::new();

    for (tool, arguments) in attempts {
        match execute_tool(profile, tool, arguments).await {
            Ok(_) => return Ok(()),
            Err(error) => {
                if network_started_after_ambiguous_start_error(profile, network_id).await {
                    return Ok(());
                }
                errors.push(format!("{tool} failed: {error}"));
            }
        }
    }

    Err(format!(
        "Polar bridge could not start network {network_id}. Start it in Polar, then retry. {}",
        errors.join("; ")
    ))
}

async fn network_started_after_ambiguous_start_error(
    profile: &PolarAutomationProfile,
    network_id: &str,
) -> bool {
    if network_status_probe_is_started(profile, network_id).await {
        return true;
    }

    wait_for_demo_node_start_delay().await;
    network_status_probe_is_started(profile, network_id).await
}

async fn network_status_probe_is_started(
    profile: &PolarAutomationProfile,
    network_id: &str,
) -> bool {
    match list_networks(profile).await {
        Ok(networks) => network_is_started(&networks, network_id),
        Err(error) => {
            log_to_terminal(&format!(
                "[polar-service] network-start-status-probe-skipped network={network_id} error={error}"
            ));
            false
        }
    }
}

async fn restart_network(profile: &PolarAutomationProfile, network_id: &str) -> Result<(), String> {
    stop_network(profile, network_id).await?;
    wait_for_network_stopped(profile, network_id).await?;
    start_network(profile, network_id).await?;
    wait_for_network_started(profile, network_id).await
}

async fn restart_network_for_node_start_recovery(
    profile: &PolarAutomationProfile,
    network_id: &str,
) -> Result<(), String> {
    restart_network(profile, network_id).await?;
    wait_for_demo_node_network_restart_settle_delay().await;
    wait_for_network_started(profile, network_id).await
}

async fn stop_network(profile: &PolarAutomationProfile, network_id: &str) -> Result<(), String> {
    match execute_tool(
        profile,
        "stop_network",
        json!({ "networkId": network_id_argument(network_id) }),
    )
    .await
    {
        Ok(_) => Ok(()),
        Err(error) => {
            if network_stop_started_after_ambiguous_stop_error(profile, network_id).await {
                return Ok(());
            }

            Err(error)
        }
    }
}

async fn network_stop_started_after_ambiguous_stop_error(
    profile: &PolarAutomationProfile,
    network_id: &str,
) -> bool {
    match list_networks(profile).await {
        Ok(networks) => network_is_stopping_or_stopped(&networks, network_id),
        Err(error) => {
            log_to_terminal(&format!(
                "[polar-service] network-stop-status-probe-skipped network={network_id} error={error}"
            ));
            false
        }
    }
}

async fn wait_for_network_stopped(
    profile: &PolarAutomationProfile,
    network_id: &str,
) -> Result<(), String> {
    let mut last_status = "missing".to_string();
    for attempt in 1..=DEMO_NODE_START_ATTEMPTS {
        let networks = list_networks(profile).await?;
        let status = find_network_status(&networks, network_id);
        if status
            .as_deref()
            .map(network_status_allows_restart)
            .unwrap_or(true)
        {
            return Ok(());
        }
        last_status = status.unwrap_or_else(|| "missing".to_string());

        log_to_terminal(&format!(
            "[polar-service] network-stop-wait network={network_id} attempt={attempt}/{DEMO_NODE_START_ATTEMPTS} status={last_status}"
        ));
        if attempt < DEMO_NODE_START_ATTEMPTS {
            wait_for_demo_node_start_delay().await;
        }
    }

    Err(format!(
        "Polar network {network_id} did not stop cleanly before restarting new demo nodes. Last status: {last_status}."
    ))
}

async fn wait_for_network_started(
    profile: &PolarAutomationProfile,
    network_id: &str,
) -> Result<(), String> {
    for attempt in 1..=DEMO_NODE_START_ATTEMPTS {
        let networks = list_networks(profile).await?;
        if network_is_started(&networks, network_id) {
            return Ok(());
        }

        log_to_terminal(&format!(
            "[polar-service] network-start-wait network={network_id} attempt={attempt}/{DEMO_NODE_START_ATTEMPTS}"
        ));
        if attempt < DEMO_NODE_START_ATTEMPTS {
            wait_for_demo_node_start_delay().await;
        }
    }

    Err(format!(
        "Polar network {network_id} did not restart after creating new demo nodes."
    ))
}

async fn wait_for_network_id_by_name(
    profile: &PolarAutomationProfile,
    requested_name: &str,
) -> Result<String, String> {
    for attempt in 1..=DEMO_NODE_START_ATTEMPTS {
        let networks = list_networks(profile).await?;
        if let Some(network_id) = find_network_id_by_name(&networks, requested_name) {
            return Ok(network_id);
        }

        log_to_terminal(&format!(
            "[polar-service] network-create-list-wait network={requested_name} attempt={attempt}/{DEMO_NODE_START_ATTEMPTS}"
        ));
        if attempt < DEMO_NODE_START_ATTEMPTS {
            wait_for_demo_node_start_delay().await;
        }
    }

    Err(format!(
        "Polar bridge created server {requested_name}, but it was not listed by that name within {DEMO_NODE_START_TIMEOUT_SECONDS} seconds."
    ))
}

fn network_is_started(networks: &Value, network_id: &str) -> bool {
    find_network_status(networks, network_id)
        .map(|status| network_status_is_started(&status))
        .unwrap_or(false)
}

fn network_is_stopping_or_stopped(networks: &Value, network_id: &str) -> bool {
    find_network_status(networks, network_id)
        .map(|status| {
            let status = status.to_ascii_lowercase();
            status == "stopping" || network_status_allows_restart(&status)
        })
        .unwrap_or(false)
}

fn network_status_is_started(status: &str) -> bool {
    let status = status.to_ascii_lowercase();
    status == "started" || status == "running"
}

fn network_status_allows_restart(status: &str) -> bool {
    let status = status.to_ascii_lowercase();
    status == "stopped" || status == "error"
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
    polar_mcp_connector::execute_tool(profile, tool, arguments, log_level.into()).await
}

fn clean_network_id(profile: &PolarAutomationProfile) -> String {
    profile.network_id.trim().to_string()
}

fn clean_backend_name(profile: &PolarAutomationProfile) -> String {
    profile.bitcoin_backend_name.trim().to_string()
}

fn polar_node_name(node_id: DemoNodeId) -> &'static str {
    match node_id {
        DemoNodeId::GameTreasury => lightning_service::GAME_TREASURY_NODE_LABEL,
        DemoNodeId::Alice => "Alice",
        DemoNodeId::Bob => "Bob",
        DemoNodeId::Carol => "Carol",
    }
}

#[cfg(test)]
fn bridge_request_timeout_message(method: &str, url: &str) -> String {
    polar_mcp_connector::bridge_request_timeout_message(method, url)
}

fn is_transient_bridge_request_error(message: &str) -> bool {
    polar_mcp_connector::is_transient_bridge_request_error(message)
}

#[cfg(test)]
fn sanitized_log_value(value: &Value) -> Value {
    polar_mcp_connector::sanitized_log_value(value)
}

fn redact_sensitive_log_text(text: &str) -> String {
    polar_mcp_connector::redact_sensitive_log_text(text)
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

fn validate_created_lightning_node_name(
    desired_name: &str,
    response: &Value,
) -> Result<(), String> {
    if let Some(name) = extract_node_name(response) {
        if name == desired_name {
            return Ok(());
        }

        return Err(unexpected_created_node_name_message(desired_name, &name));
    }

    Ok(())
}

fn validate_created_lightning_node_after_add(
    desired_name: &str,
    response: &Value,
    before_add: &Value,
    after_add: &Value,
    network_id: &str,
) -> Result<String, String> {
    if lightning_node_exists(after_add, network_id, desired_name) {
        return Ok(desired_name.to_string());
    }

    let response_name = extract_node_name(response).filter(|name| name != desired_name);
    if let Some(created_name) = find_new_lightning_node_name(before_add, after_add, network_id) {
        if response_name
            .as_deref()
            .map(|name| name == created_name)
            .unwrap_or(true)
        {
            return Ok(created_name);
        }
    }

    validate_created_lightning_node_name(desired_name, response)?;

    Err(format!(
        "Polar did not list the requested LND node {desired_name} after creation. Retry the step after Polar finishes updating the network."
    ))
}

fn unexpected_created_node_name_message(desired_name: &str, created_name: &str) -> String {
    format!(
        "The app asked Polar to create {desired_name}, but Polar created {created_name}. Remove {created_name} in Polar, then retry after updating or restarting the Polar bridge."
    )
}

#[cfg(test)]
fn format_mcp_error(tool: &str, error: &str) -> String {
    polar_mcp_connector::format_mcp_error(tool, error)
}

fn can_skip_channel_cleanup_error(error: &str) -> bool {
    error.contains("Polar MCP tool list_channels could not run")
        && error.contains("automation helper file is missing")
}

#[cfg(test)]
fn extract_mcp_error(value: &Value) -> Option<String> {
    polar_mcp_connector::extract_mcp_error(value)
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
    let mut ids = top_level_network_ids(value);

    if ids.len() == 1 {
        ids.pop()
    } else {
        None
    }
}

fn top_level_network_ids(value: &Value) -> Vec<String> {
    let mut ids = Vec::new();
    collect_top_level_network_ids(value, &mut ids);
    ids.sort();
    ids.dedup();
    ids
}

#[cfg(test)]
fn top_level_network_summaries(value: &Value) -> Vec<String> {
    let mut summaries = Vec::new();
    collect_top_level_network_summaries(value, &mut summaries);
    summaries.sort();
    summaries.dedup();
    summaries
}

fn top_level_network_records(value: &Value) -> Vec<PolarNetworkRecord> {
    let mut records = Vec::new();
    collect_top_level_network_records(value, &mut records);
    records.sort_by(|a, b| a.id.cmp(&b.id));
    records.dedup_by(|a, b| a.id.eq_ignore_ascii_case(&b.id));
    records
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

fn collect_top_level_network_records(value: &Value, records: &mut Vec<PolarNetworkRecord>) {
    match value {
        Value::Object(map) => {
            for key in ["networks", "result", "data"] {
                if let Some(networks) = map.get(key) {
                    collect_top_level_network_records(networks, records);
                    return;
                }
            }

            if let Some(id) = map
                .get("id")
                .or_else(|| map.get("networkId"))
                .and_then(value_as_id_string)
            {
                let name = map
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .unwrap_or("unnamed")
                    .to_string();
                let status = map
                    .get("status")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|status| !status.is_empty())
                    .unwrap_or("unknown")
                    .to_string();
                records.push(PolarNetworkRecord { id, name, status });
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_top_level_network_records(item, records);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
fn collect_top_level_network_summaries(value: &Value, summaries: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for key in ["networks", "result", "data"] {
                if let Some(networks) = map.get(key) {
                    collect_top_level_network_summaries(networks, summaries);
                    return;
                }
            }

            if let Some(id) = map
                .get("id")
                .or_else(|| map.get("networkId"))
                .and_then(value_as_id_string)
            {
                let name = map
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .unwrap_or("unnamed");
                let status = map
                    .get("status")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|status| !status.is_empty())
                    .unwrap_or("unknown");
                summaries.push(format!("{id} {name} ({status})"));
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_top_level_network_summaries(item, summaries);
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
async fn wait_for_demo_node_network_restart_settle_delay() {
    gloo_timers::future::TimeoutFuture::new(DEMO_NODE_NETWORK_RESTART_SETTLE_MS).await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn wait_for_demo_node_network_restart_settle_delay() {
    futures_timer::Delay::new(std::time::Duration::from_millis(
        DEMO_NODE_NETWORK_RESTART_SETTLE_MS.into(),
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

#[cfg(target_arch = "wasm32")]
async fn delete_network_timeout_delay() {
    gloo_timers::future::TimeoutFuture::new(DELETE_NETWORK_TIMEOUT_SECONDS.saturating_mul(1_000))
        .await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn delete_network_timeout_delay() {
    futures_timer::Delay::new(std::time::Duration::from_secs(
        DELETE_NETWORK_TIMEOUT_SECONDS.into(),
    ))
    .await;
}

#[cfg(target_arch = "wasm32")]
async fn wait_for_delete_network_settle_delay() {
    gloo_timers::future::TimeoutFuture::new(DELETE_NETWORK_SETTLE_MS).await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn wait_for_delete_network_settle_delay() {
    futures_timer::Delay::new(std::time::Duration::from_millis(
        DELETE_NETWORK_SETTLE_MS.into(),
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

    #[test]
    fn sanitizes_sensitive_polar_metadata_from_verbose_logs() {
        let value = json!({
            "paths": {
                "adminMacaroon": "C:\\Users\\example\\admin.macaroon",
                "tlsCert": "C:\\Users\\example\\tls.cert"
            },
            "nodes": [
                {
                    "name": "Alice",
                    "token": "abc",
                    "ports": { "grpc": 10001 }
                }
            ]
        });

        let sanitized = sanitized_log_value(&value);

        assert_eq!(sanitized["paths"], "[redacted]");
        assert_eq!(sanitized["nodes"][0]["token"], "[redacted]");
        assert_eq!(sanitized["nodes"][0]["ports"]["grpc"], 10001);
    }

    #[test]
    fn redacts_sensitive_polar_error_text() {
        let error = "failed to read C:\\Users\\example\\admin.macaroon";

        assert_eq!(
            redact_sensitive_log_text(error),
            "[redacted sensitive Polar detail]"
        );
        assert_eq!(
            format_mcp_error("list_networks", error),
            "Polar MCP tool list_networks failed: [redacted sensitive Polar detail]"
        );
    }

    #[test]
    fn bridge_request_timeout_message_names_operation_and_deadline() {
        assert_eq!(
            bridge_request_timeout_message("POST", "http://localhost:37373/api/mcp/execute"),
            "Polar bridge POST http://localhost:37373/api/mcp/execute timed out after 90 seconds."
        );
    }

    #[test]
    fn transient_bridge_request_errors_are_retryable() {
        assert!(is_transient_bridge_request_error(
            "Polar bridge request failed: TypeError: Failed to fetch"
        ));
        assert!(is_transient_bridge_request_error(
            "Polar bridge request failed: connection reset by peer"
        ));
        assert!(!is_transient_bridge_request_error(
            "Polar MCP tool deposit_funds failed: permission denied by policy"
        ));
    }

    #[test]
    fn delete_network_timeout_message_names_network_and_deadline() {
        assert_eq!(
            delete_network_timeout_message("network-8"),
            "Timed out after 12s while deleting Polar network network-8."
        );
    }

    #[test]
    fn delete_network_stop_timeout_message_is_actionable() {
        assert_eq!(
            delete_network_stop_timeout_message("31", "Started"),
            "Polar network 31 did not stop before deletion. Last status: Started. Stop the network in Polar, then run Delete all networks again."
        );
    }

    #[test]
    fn delete_network_preparation_stops_starting_networks() {
        assert_eq!(
            delete_network_preparation_for_status("Starting"),
            DeleteNetworkPreparation::StopThenDelete
        );
        assert_eq!(
            delete_network_preparation_for_status("Started"),
            DeleteNetworkPreparation::StopThenDelete
        );
        assert_eq!(
            delete_network_preparation_for_status("Stopping"),
            DeleteNetworkPreparation::WaitThenDelete
        );
        assert_eq!(
            delete_network_preparation_for_status("Stopped"),
            DeleteNetworkPreparation::DeleteNow
        );
    }

    #[test]
    fn delete_network_error_explains_locked_windows_files() {
        let message = delete_network_error_message(
            "12",
            "Polar MCP tool delete_network failed: EPERM: operation not permitted, lstat 'C:\\Users\\example\\.polar\\networks\\12\\volumes\\lnd\\alice\\logs\\bitcoin\\regtest\\lnd.log'".to_string(),
        );

        assert!(message.contains("Windows still has Polar/LND files locked"));
        assert!(!message.contains("remove_network"));
        assert!(!message.contains("C:\\Users"));
        assert!(!message.contains("channel.db"));
    }

    #[test]
    fn delete_network_retries_transient_polar_helper_errors() {
        assert!(is_retryable_delete_network_error(
            "Polar MCP tool delete_network failed: Cannot read property 'nodes' of undefined"
        ));
        assert!(is_retryable_delete_network_error(
            "Polar MCP tool delete_network failed: no configuration file provided: not found"
        ));
        assert!(is_retryable_delete_network_error(
            "Polar MCP tool delete_network failed: network must be stopped before deletion"
        ));
        assert!(is_retryable_delete_network_error(
            "Polar MCP tool delete_network failed: network is currently running"
        ));
        assert!(is_retryable_delete_network_error(
            "Timed out after 12s while deleting Polar network 12."
        ));
        assert!(!is_retryable_delete_network_error(
            "Polar MCP tool delete_network failed: permission denied by policy"
        ));
    }

    #[test]
    fn delete_network_already_gone_does_not_match_missing_config() {
        assert!(is_network_already_gone_error(
            "Polar MCP tool delete_network failed: network not found"
        ));
        assert!(!is_network_already_gone_error(
            "Polar MCP tool delete_network failed: no configuration file provided: not found"
        ));
    }

    #[test]
    fn delete_all_networks_failure_explains_corrupted_polar_records() {
        let result = PolarDeleteAllResult {
            deleted_count: 0,
            failed_networks: vec![
                PolarDeleteAllNetworkFailure {
                    network: PolarNetworkRecord {
                        id: "12".to_string(),
                        name: "Dioxus Bitcoin Lightning Game".to_string(),
                        status: "Error".to_string(),
                    },
                    error:
                        "Polar MCP tool delete_network failed: no configuration file provided: not found"
                            .to_string(),
                },
                PolarDeleteAllNetworkFailure {
                    network: PolarNetworkRecord {
                        id: "3".to_string(),
                        name: "Bitcoin Lightning Game 991".to_string(),
                        status: "Error".to_string(),
                    },
                    error:
                        "Polar MCP tool delete_network failed: Cannot read property 'nodes' of undefined"
                            .to_string(),
                },
            ],
            remaining_networks: vec![
                PolarNetworkRecord {
                    id: "3".to_string(),
                    name: "Bitcoin Lightning Game 991".to_string(),
                    status: "Error".to_string(),
                },
                PolarNetworkRecord {
                    id: "12".to_string(),
                    name: "Dioxus Bitcoin Lightning Game".to_string(),
                    status: "Error".to_string(),
                },
            ],
        };

        let message = delete_all_networks_failure_message(&result);

        assert!(message.contains("Polar is still listing these networks"));
        assert!(message.contains("Restart Polar and the local Polar bridge"));
        assert!(message.contains("Remaining networks: 3 Bitcoin Lightning Game 991 (Error)"));
    }

    #[test]
    fn delete_all_networks_failure_explains_still_listed_networks_without_delete_errors() {
        let result = PolarDeleteAllResult {
            deleted_count: 2,
            failed_networks: Vec::new(),
            remaining_networks: vec![PolarNetworkRecord {
                id: "7".to_string(),
                name: "existing".to_string(),
                status: "Stopped".to_string(),
            }],
        };

        let message = delete_all_networks_failure_message(&result);

        assert_eq!(
            message,
            "Deleted 2 Polar network(s), but 1 network(s) are still visible to the local Polar bridge. Remaining networks: 7 existing (Stopped)."
        );
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
                                "name": "Alice",
                                "type": "lightning",
                                "implementation": "LND",
                                "status": "Started"
                            },
                            {
                                "name": "Bob",
                                "type": "lightning",
                                "implementation": "LND",
                                "status": "Started"
                            },
                            {
                                "name": "Carol",
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
    fn required_polar_node_names_match_requested_topology() {
        assert_eq!(
            required_polar_node_names(),
            [
                "GAME_BITCOIN",
                "GAME_LND",
                "GAME_TAPROOT",
                "Alice",
                "Bob",
                "Carol"
            ]
        );
        assert_eq!(
            required_polar_base_node_names(),
            ["GAME_BITCOIN", "GAME_LND", "Alice", "Bob", "Carol"]
        );
    }

    #[test]
    fn required_topology_cleanup_uses_exact_node_names() {
        let networks = json!({
            "networks": [{
                "id": 1,
                "name": DEFAULT_NETWORK_NAME,
                "nodes": {
                    "bitcoin": [{ "name": DEFAULT_BITCOIN_BACKEND_NAME }],
                    "lightning": [
                        { "name": "GAME_LND", "type": "lightning" },
                        { "name": "alice", "type": "lightning" },
                        { "name": "Bob", "type": "lightning" },
                        { "name": "Carol", "type": "lightning" }
                    ],
                    "taproot": [{ "name": "GAME_TAPROOT", "type": "taproot" }]
                }
            }]
        });

        assert_eq!(
            first_unexpected_required_topology_node_name(&networks, "1"),
            Some("alice".to_string())
        );
    }

    #[test]
    fn required_node_restart_waits_for_six_no_progress_polls_once() {
        assert!(!required_node_restart_due(5, false));
        assert!(required_node_restart_due(6, false));
        assert!(!required_node_restart_due(6, true));
    }

    #[test]
    fn node_start_timeout_defers_to_readiness_polling() {
        assert!(is_retryable_node_start_request_error(
            "Polar MCP tool start_node failed: Tool execution timed out"
        ));
        assert!(is_retryable_node_start_request_error(
            "ports are not available: Only one usage of each socket address"
        ));
        assert!(!is_retryable_node_start_request_error(
            "Polar MCP tool start_node failed: node does not exist"
        ));
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
    fn collects_all_top_level_network_ids_for_delete_all() {
        let networks = json!({
            "networks": [
                { "id": 29, "name": "autopilot-a" },
                { "networkId": "7", "name": "existing" },
                { "id": 29, "name": "duplicate" }
            ]
        });

        assert_eq!(
            top_level_network_ids(&networks),
            vec!["29".to_string(), "7".to_string()]
        );
    }

    #[test]
    fn collects_top_level_network_records_for_delete_all() {
        let networks = json!({
            "networks": [
                { "id": 29, "name": "autopilot-a", "status": "Started" },
                { "networkId": "7", "name": "existing", "status": "Error" },
                { "id": 29, "name": "duplicate", "status": "Stopped" }
            ]
        });

        assert_eq!(
            top_level_network_records(&networks),
            vec![
                PolarNetworkRecord {
                    id: "29".to_string(),
                    name: "autopilot-a".to_string(),
                    status: "Started".to_string(),
                },
                PolarNetworkRecord {
                    id: "7".to_string(),
                    name: "existing".to_string(),
                    status: "Error".to_string(),
                },
            ]
        );
    }

    #[test]
    fn delete_all_failure_message_includes_failed_and_remaining_networks() {
        let result = PolarDeleteAllResult {
            deleted_count: 2,
            failed_networks: vec![PolarDeleteAllNetworkFailure {
                network: PolarNetworkRecord {
                    id: "7".to_string(),
                    name: "broken".to_string(),
                    status: "Error".to_string(),
                },
                error: "network must be stopped before deletion".to_string(),
            }],
            remaining_networks: vec![PolarNetworkRecord {
                id: "7".to_string(),
                name: "broken".to_string(),
                status: "Error".to_string(),
            }],
        };

        let message = delete_all_networks_failure_message(&result);

        assert!(message.contains("Deleted 2 Polar network(s)"));
        assert!(message.contains("7 broken (Error): network must be stopped"));
        assert!(message.contains("Remaining networks: 7 broken (Error)."));
    }

    #[test]
    fn delete_all_failure_message_explains_corrupted_records_in_mixed_failures() {
        let result = PolarDeleteAllResult {
            deleted_count: 0,
            failed_networks: vec![
                PolarDeleteAllNetworkFailure {
                    network: PolarNetworkRecord {
                        id: "30".to_string(),
                        name: "autopilot".to_string(),
                        status: "Stopping".to_string(),
                    },
                    error: "Polar network 30 did not stop before deletion. Last status: Stopping. Stop the network in Polar, then run Delete all networks again.".to_string(),
                },
                PolarDeleteAllNetworkFailure {
                    network: PolarNetworkRecord {
                        id: "31".to_string(),
                        name: "autopilot-1".to_string(),
                        status: "Error".to_string(),
                    },
                    error:
                        "Polar MCP tool delete_network failed: no configuration file provided: not found"
                            .to_string(),
                },
            ],
            remaining_networks: vec![
                PolarNetworkRecord {
                    id: "30".to_string(),
                    name: "autopilot".to_string(),
                    status: "Stopping".to_string(),
                },
                PolarNetworkRecord {
                    id: "31".to_string(),
                    name: "autopilot-1".to_string(),
                    status: "Error".to_string(),
                },
            ],
        };

        let message = delete_all_networks_failure_message(&result);

        assert!(message.contains("Polar is still listing these networks"));
        assert!(message.contains("Restart Polar and the local Polar bridge"));
        assert!(message.contains("30 autopilot (Stopping)"));
        assert!(message.contains("31 autopilot-1 (Error)"));
    }

    #[test]
    fn summarizes_remaining_top_level_networks_for_delete_all() {
        let networks = json!({
            "networks": [
                { "id": 29, "name": "autopilot-a", "status": "Error" },
                { "networkId": "7", "name": "existing", "status": "Stopped" }
            ]
        });

        assert_eq!(
            top_level_network_summaries(&networks),
            vec![
                "29 autopilot-a (Error)".to_string(),
                "7 existing (Stopped)".to_string()
            ]
        );
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
    fn resolve_backend_name_prefers_discovered_backend_over_stale_default() {
        let networks = json!({
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
                                "name": "backend1",
                                "type": "bitcoin",
                                "implementation": "bitcoind"
                            }
                        ],
                        "lightning": []
                    }
                }
            ]
        });
        let profile = PolarAutomationProfile {
            bridge_url: "http://localhost:37373".to_string(),
            network_id: "1".to_string(),
            bitcoin_backend_name: DEFAULT_BITCOIN_BACKEND_NAME.to_string(),
        };

        assert_eq!(resolve_backend_name(&profile, &networks, "1"), "backend1");
    }

    #[test]
    fn resolve_backend_name_keeps_saved_backend_when_it_exists() {
        let networks = json!({
            "networks": [
                {
                    "id": 1,
                    "name": DEFAULT_NETWORK_NAME,
                    "status": "Started",
                    "nodes": {
                        "bitcoin": [
                            { "name": DEFAULT_BITCOIN_BACKEND_NAME, "type": "bitcoin" },
                            { "name": "backend1", "type": "bitcoin" }
                        ],
                        "lightning": []
                    }
                }
            ]
        });
        let profile = PolarAutomationProfile {
            bridge_url: "http://localhost:37373".to_string(),
            network_id: "1".to_string(),
            bitcoin_backend_name: DEFAULT_BITCOIN_BACKEND_NAME.to_string(),
        };

        assert_eq!(
            resolve_backend_name(&profile, &networks, "1"),
            DEFAULT_BITCOIN_BACKEND_NAME
        );
    }

    #[test]
    fn sends_numeric_network_id_when_polar_uses_numeric_ids() {
        assert_eq!(network_id_argument("1"), json!(1));
    }

    #[test]
    fn resolve_network_id_rejects_stale_requested_network_before_mutation() {
        let networks = json!({
            "networks": [
                {
                    "id": 2,
                    "name": DEFAULT_NETWORK_NAME,
                    "status": "Started",
                    "nodes": {}
                }
            ]
        });
        let profile = PolarAutomationProfile {
            bridge_url: "http://localhost:37373".to_string(),
            network_id: "network-1".to_string(),
            bitcoin_backend_name: DEFAULT_BITCOIN_BACKEND_NAME.to_string(),
        };

        let error = resolve_network_id(&profile, &networks)
            .expect_err("stale requested network ids must not be sent to Polar mutations");

        assert!(error.contains("network-1"));
        assert!(error.contains("not listed"));
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

        assert!(lightning_node_exists(&networks, "1", "Alice"));
        assert!(lightning_node_is_started(&networks, "1", "Alice"));
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
            find_lightning_node_name(&networks, "1", "Alice"),
            Some("Alice".to_string())
        );
        assert_ne!(
            find_lightning_node_name(&networks, "1", "Bob"),
            Some("Alice".to_string())
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
    fn game_treasury_balance_accepts_at_least_required_funding() {
        assert!(node_wallet_balance_matches_app_rules(
            DemoNodeId::GameTreasury,
            DEMO_NODE_FUNDING_SATS,
            250_000
        ));
        assert!(node_wallet_balance_matches_app_rules(
            DemoNodeId::GameTreasury,
            DEMO_NODE_FUNDING_SATS + 1,
            250_000
        ));
        assert!(!node_wallet_balance_matches_app_rules(
            DemoNodeId::GameTreasury,
            DEMO_NODE_FUNDING_SATS - 1,
            250_000
        ));
    }

    #[test]
    fn game_treasury_gets_extended_wallet_readiness_window() {
        assert!(
            ready_attempts_for_node(DemoNodeId::GameTreasury)
                > ready_attempts_for_node(DemoNodeId::Alice)
        );
    }

    #[test]
    fn game_treasury_step_rejects_alice_as_treasury_shell() {
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

        let error = require_game_treasury_node_name(&networks, "1")
            .expect_err("alice must not satisfy treasury setup");

        assert!(error.contains("GAME_LND is missing"));
        assert!(error.contains("Retry Game Treasury"));
        assert!(!error.contains("Retry Server Name"));
    }

    #[test]
    fn game_treasury_step_accepts_exact_treasury_shell() {
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
                                "name": "GAME_LND",
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
            require_game_treasury_node_name(&networks, "1"),
            Ok("GAME_LND".to_string())
        );
    }

    #[test]
    fn backend_already_connected_error_is_idempotent() {
        assert!(is_lightning_backend_already_connected_error(
            "The node 'GAME_LND' is already connected to 'backend1'"
        ));
    }

    #[test]
    fn lightning_node_already_started_error_is_idempotent() {
        assert!(is_lightning_node_already_started_error(
            "Polar MCP tool start_node failed: Cannot start node \"alice\". Node is currently Started. Only Stopped or Error nodes can be started."
        ));
    }

    #[test]
    fn unrelated_lightning_node_start_error_is_not_idempotent() {
        assert!(!is_lightning_node_already_started_error(
            "Polar MCP tool start_node failed: Cannot start node \"alice\". Wallet is locked."
        ));
    }

    #[test]
    fn unrelated_backend_error_is_not_idempotent() {
        assert!(!is_lightning_backend_already_connected_error(
            "The node 'GAME_LND' could not connect to backend1"
        ));
    }

    #[test]
    fn treasury_shell_can_reclaim_default_alice_before_user_nodes_exist() {
        let networks = json!({
            "networks": [
                {
                    "id": 1,
                    "name": DEFAULT_NETWORK_NAME,
                    "status": "Stopped",
                    "nodes": {
                        "bitcoin": [],
                        "lightning": [
                            {
                                "name": "Alice",
                                "type": "lightning",
                                "implementation": "LND",
                                "status": "Stopped"
                            }
                        ]
                    }
                }
            ]
        });

        assert_eq!(
            find_reclaimable_default_alice_node(&networks, "1"),
            Some("Alice".to_string())
        );
    }

    #[test]
    fn create_network_requests_bitcoin_backend_only_for_step_two() {
        for (_, arguments) in create_network_attempts(DEFAULT_NETWORK_NAME) {
            let bitcoin_nodes = arguments
                .get("bitcoinNodes")
                .or_else(|| arguments.pointer("/nodes/bitcoin"))
                .expect("create network attempt includes bitcoin nodes");
            let lightning_nodes = arguments
                .get("lightningNodes")
                .or_else(|| arguments.pointer("/nodes/lightning"));
            let tap_nodes = arguments
                .get("tapNodes")
                .or_else(|| arguments.pointer("/nodes/tap"));

            assert_eq!(bitcoin_nodes.as_array().map(Vec::len), Some(1));
            assert_eq!(
                bitcoin_nodes[0]["name"],
                json!(DEFAULT_BITCOIN_BACKEND_NAME)
            );
            assert_eq!(
                lightning_nodes.and_then(Value::as_array).map(Vec::len),
                Some(0)
            );
            assert_eq!(tap_nodes.and_then(Value::as_array).map(Vec::len), Some(0));
        }
    }

    #[test]
    fn detects_taproot_dependency_remove_node_error() {
        assert!(is_taproot_dependency_remove_error(
            "Polar MCP tool remove_node failed: Cannot remove a Lightning node that has a Taproot Assets node connected to it."
        ));
    }

    #[test]
    fn detects_ambiguous_remove_node_post_success_error() {
        assert!(is_remove_node_post_success_error(
            "Polar MCP tool remove_node failed: Cannot read property 'size' of undefined"
        ));
        assert!(!is_remove_node_post_success_error(
            "Polar MCP tool remove_node failed: wallet locked"
        ));
    }

    #[test]
    fn treasury_shell_does_not_reclaim_alice_after_user_nodes_exist() {
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
                            },
                            {
                                "name": "Bob",
                                "type": "lightning",
                                "implementation": "LND",
                                "status": "Started"
                            }
                        ]
                    }
                }
            ]
        });

        assert_eq!(find_reclaimable_default_alice_node(&networks, "1"), None);
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
    fn add_lightning_node_arguments_request_requested_name() {
        let args = add_lightning_node_arguments("1", "GAME_LND");

        assert_eq!(args["networkId"], json!(1));
        assert_eq!(args["implementation"], json!("LND"));
        assert_eq!(args["name"], json!("GAME_LND"));
        assert_eq!(args["nodeName"], json!("GAME_LND"));
    }

    #[test]
    fn add_lightning_node_arguments_send_all_known_name_fields() {
        let args = add_lightning_node_arguments("1", "Alice");

        assert_eq!(args["name"], json!("Alice"));
        assert_eq!(args["nodeName"], json!("Alice"));
        assert_eq!(args["displayName"], json!("Alice"));
        assert_eq!(args["alias"], json!("Alice"));
    }

    #[test]
    fn remove_node_arguments_send_selected_option_shape() {
        let args = remove_node_arguments("1", "GAME_LND");

        assert_eq!(args["networkId"], json!(1));
        assert_eq!(args["nodeName"], json!("GAME_LND"));
        assert_eq!(args["network"]["selected"], json!(1));
        assert_eq!(args["node"]["selected"], json!("GAME_LND"));
        assert_eq!(args["selected"]["networkId"], json!(1));
        assert_eq!(args["selected"]["nodeName"], json!("GAME_LND"));
    }

    #[test]
    fn generated_fallback_lightning_node_name_is_rejected() {
        let response = json!({
            "success": true,
            "nodeName": "dave"
        });

        let error = validate_created_lightning_node_name("Alice", &response)
            .expect_err("generated fallback node name must not be accepted");

        assert!(error.contains("asked Polar to create Alice"));
        assert!(error.contains("created dave"));
        assert!(error.contains("Remove dave"));
    }

    #[test]
    fn async_created_fallback_lightning_node_name_can_be_repaired() {
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
                                "name": "erin",
                                "type": "lightning",
                                "implementation": "LND",
                                "status": "Started"
                            }
                        ]
                    }
                }
            ]
        });

        let created_name = validate_created_lightning_node_after_add(
            "Alice",
            &json!({ "success": true }),
            &before_add,
            &after_add,
            "1",
        )
        .expect("async fallback node name should be returned for repair");

        assert_eq!(created_name, "erin");
    }

    #[test]
    fn add_node_response_with_existing_nodes_does_not_override_post_add_state() {
        let before_add = json!({
            "networks": [
                {
                    "id": 1,
                    "name": DEFAULT_NETWORK_NAME,
                    "status": "Started",
                    "nodes": {
                        "bitcoin": [],
                        "lightning": [
                            {
                                "name": "Bob",
                                "type": "lightning",
                                "implementation": "LND",
                                "status": "Started"
                            }
                        ]
                    }
                }
            ]
        });
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
                                "name": "Bob",
                                "type": "lightning",
                                "implementation": "LND",
                                "status": "Started"
                            },
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
        let response = json!({
            "success": true,
            "result": {
                "network": {
                    "nodes": {
                        "lightning": [
                            {
                                "name": "Bob",
                                "type": "lightning",
                                "implementation": "LND"
                            },
                            {
                                "name": "Alice",
                                "type": "lightning",
                                "implementation": "LND"
                            }
                        ]
                    }
                }
            }
        });

        assert_eq!(
            validate_created_lightning_node_after_add(
                "Alice",
                &response,
                &before_add,
                &after_add,
                "1",
            ),
            Ok("Alice".to_string())
        );
    }

    #[test]
    fn taproot_assets_node_arguments_connect_to_treasury_and_backend() {
        let arguments = add_taproot_assets_node_arguments(
            "1",
            "tapd",
            "GAME_LND",
            DEFAULT_BITCOIN_BACKEND_NAME,
        );

        assert_eq!(arguments["networkId"], json!(1));
        assert_eq!(arguments["implementation"], json!("tapd"));
        assert_eq!(arguments["nodeName"], json!(TAPROOT_ASSETS_NODE_NAME));
        assert_eq!(arguments["type"], json!("taproot"));
        assert_eq!(arguments["lightningNodeName"], json!("GAME_LND"));
        assert_eq!(arguments["lndNodeName"], json!("GAME_LND"));
        assert_eq!(
            arguments["bitcoinNodeName"],
            json!(DEFAULT_BITCOIN_BACKEND_NAME)
        );
    }

    #[test]
    fn taproot_assets_node_exists_discovers_named_taproot_node() {
        let networks = json!({
            "networks": [
                {
                    "id": 1,
                    "name": DEFAULT_NETWORK_NAME,
                    "nodes": {
                        "bitcoin": [
                            { "name": DEFAULT_BITCOIN_BACKEND_NAME, "type": "bitcoin" }
                        ],
                        "tap": [
                            { "name": TAPROOT_ASSETS_NODE_NAME, "type": "taproot", "implementation": "tapd" }
                        ]
                    }
                }
            ]
        });

        assert!(taproot_assets_node_exists(&networks, "1"));
    }

    #[test]
    fn taproot_assets_node_exists_discovers_legacy_tapd_node() {
        let networks = json!({
            "networks": [
                {
                    "id": 1,
                    "name": DEFAULT_NETWORK_NAME,
                    "nodes": {
                        "tap": [
                            { "name": LEGACY_TAPROOT_ASSETS_NODE_NAME, "type": "taproot", "implementation": "tapd" }
                        ]
                    }
                }
            ]
        });

        assert_eq!(
            find_taproot_assets_node_name(&networks, "1"),
            Some(LEGACY_TAPROOT_ASSETS_NODE_NAME.to_string())
        );
    }

    #[test]
    fn taproot_assets_node_exists_accepts_polar_generated_treasury_tap_name() {
        let networks = json!({
            "networks": [
                {
                    "id": 31,
                    "name": "autopilot-1779139644401",
                    "nodes": {
                        "tap": [
                            {
                                "name": "GAME_LND-tap",
                                "type": "tap",
                                "implementation": "tapd",
                                "lndName": "GAME_LND",
                                "status": "Started"
                            }
                        ]
                    }
                }
            ]
        });

        assert_eq!(
            find_taproot_assets_node_name(&networks, "31"),
            Some("GAME_LND-tap".to_string())
        );
        assert!(taproot_assets_node_exists(&networks, "31"));
    }

    #[test]
    fn mcp_enoent_error_explains_missing_bridge_helper() {
        assert_eq!(
            format_mcp_error(
                "list_channels",
                "ENOENT: no such file or directory, open 'C:\\\\Users\\\\user\\\\polar.json'"
            ),
            "Polar MCP tool list_channels could not run because Polar's automation helper file is missing. Restart Polar and the local Polar bridge, then retry the step."
        );
    }

    #[test]
    fn missing_helper_during_channel_listing_is_optional_cleanup() {
        let error = format_mcp_error(
            "list_channels",
            "ENOENT: no such file or directory, open 'C:\\\\Users\\\\user\\\\polar.json'",
        );

        assert!(can_skip_channel_cleanup_error(&error));
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
            "nodeName": "Alice",
            "confirmed_balance": "250000",
            "unconfirmed_balance": "750000"
        });

        assert_eq!(extract_wallet_balance_sats(&wallet_info), Some(250000));
    }

    #[test]
    fn extracts_wallet_balance_from_text_sats() {
        let wallet_info = json!({
            "success": true,
            "result": "Wallet balance for Alice: 1,000,000 sats"
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
    fn restart_wait_treats_stopping_network_as_busy() {
        let networks = json!({
            "networks": [
                {
                    "id": 31,
                    "name": "autopilot",
                    "status": "Stopping"
                }
            ]
        });

        assert!(network_is_stopping_or_stopped(&networks, "31"));
        assert!(!find_network_status(&networks, "31")
            .as_deref()
            .is_some_and(network_status_allows_restart));
    }

    #[test]
    fn restart_wait_allows_stopped_or_error_network() {
        assert!(network_status_allows_restart("Stopped"));
        assert!(network_status_allows_restart("Error"));
        assert!(!network_status_allows_restart("Starting"));
        assert!(!network_status_allows_restart("Started"));
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
                "name": "Alice",
                "type": "lightning",
                "implementation": "LND",
                "status": "Started"
            },
            {
                "name": "Bob",
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
    polar_mcp_connector::get_json(profile, path, log_level.into()).await
}

#[cfg(not(target_arch = "wasm32"))]
async fn get_json(
    profile: &PolarAutomationProfile,
    path: &str,
    log_level: DemoLogLevel,
) -> Result<Value, String> {
    polar_mcp_connector::get_json(profile, path, log_level.into()).await
}
