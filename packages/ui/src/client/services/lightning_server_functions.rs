use crate::client::models::{
    BlockWaitReason, ConnectionStatus, DemoNodeId, GameItemDefinition, LabState, MintTraRequest,
    RouteStatus, SetupProfile, TransferTraRequest, DEFAULT_BITCOIN_BACKEND_NAME,
    DEFAULT_ROUTE_CAPACITY_SATS,
};
use crate::client::services::{polar_bridge_service, storage_service};

pub use polar_bridge_service::{
    PolarLabHealthIssue, PolarServerEnsureResult, PolarServerEnsureStatus,
};

#[derive(Clone, Debug, PartialEq)]
pub struct PolarLabRecovery {
    pub profile: SetupProfile,
    pub lab_state: Option<LabState>,
    pub message: String,
}

pub async fn test_setup(profile: SetupProfile) -> Result<LabState, String> {
    let state = if profile.setup_mode == lightning_service::SetupMode::ServerConfig
        && profile.polar_automation.is_complete()
    {
        lightning_service::validate_setup_profile(&profile).map_err(|error| error.to_string())?;
        let automation =
            polar_bridge_service::resolve_automation_profile(&profile.polar_automation).await?;

        let mut saved_profile = profile;
        saved_profile.polar_automation = automation;
        saved_profile.connection_status = lightning_service::ConnectionStatus::SavedOffline;
        saved_profile.last_verified_at = Some(chrono::Utc::now());

        lightning_service::default_lab_state(saved_profile)
    } else {
        lightning_service::test_setup(profile).map_err(|error| error.to_string())?
    };
    let state = refresh_polar_block_height(state).await?;

    storage_service::save_setup_profile(&state.profile);
    storage_service::save_lab_state_snapshot(&state);
    Ok(state)
}

pub async fn save_setup_preferences(profile: SetupProfile) -> Result<SetupProfile, String> {
    lightning_service::validate_setup_profile(&profile).map_err(|error| error.to_string())?;
    storage_service::save_setup_profile(&profile);
    Ok(profile)
}

pub async fn verify_polar_bridge(profile: SetupProfile) -> Result<SetupProfile, String> {
    lightning_service::validate_setup_profile(&profile).map_err(|error| error.to_string())?;
    polar_bridge_service::test_bridge(&profile.polar_automation).await?;

    storage_service::save_setup_profile(&profile);
    storage_service::clear_lab_state_snapshot();
    Ok(profile)
}

pub async fn ensure_polar_server(profile: SetupProfile) -> Result<PolarServerEnsureResult, String> {
    lightning_service::validate_setup_profile(&profile).map_err(|error| error.to_string())?;
    let mut profile = profile;
    let result = polar_bridge_service::ensure_server(&profile.polar_automation).await?;

    profile.polar_automation = result.profile.clone();
    profile.connection_status = lightning_service::ConnectionStatus::SavedOffline;
    profile.last_verified_at = None;

    storage_service::save_setup_profile(&profile);
    storage_service::clear_lab_state_snapshot();

    Ok(result)
}

pub async fn create_polar_demo_nodes(profile: SetupProfile) -> Result<LabState, String> {
    create_polar_demo_nodes_with_progress(profile, |_| {}).await
}

pub async fn close_polar_demo_channels(profile: SetupProfile) -> Result<SetupProfile, String> {
    close_polar_demo_channels_with_progress(profile, |_| {}).await
}

pub async fn close_polar_demo_channels_with_progress<F>(
    profile: SetupProfile,
    report_progress: F,
) -> Result<SetupProfile, String>
where
    F: FnMut(String),
{
    lightning_service::validate_setup_profile(&profile).map_err(|error| error.to_string())?;
    let mut profile = profile;
    profile.polar_automation = polar_bridge_service::close_demo_channels_with_progress(
        &profile.polar_automation,
        report_progress,
    )
    .await?;

    storage_service::save_setup_profile(&profile);
    Ok(profile)
}

pub async fn create_polar_demo_nodes_with_progress<F>(
    profile: SetupProfile,
    report_progress: F,
) -> Result<LabState, String>
where
    F: FnMut(String),
{
    lightning_service::validate_setup_profile(&profile).map_err(|error| error.to_string())?;
    let mut profile = profile;
    let required_balance_sats = profile
        .sats_per_transaction
        .max(DEFAULT_ROUTE_CAPACITY_SATS);
    profile.polar_automation = polar_bridge_service::create_demo_nodes_with_progress(
        &profile.polar_automation,
        required_balance_sats,
        report_progress,
    )
    .await?;

    let state = lightning_service::test_setup(profile).map_err(|error| error.to_string())?;
    let mut state = refresh_polar_block_height(state).await?;
    state.profile.connection_status = lightning_service::ConnectionStatus::PartiallyConnected;
    state.profile.polar_block_height_confirmed = false;
    storage_service::save_setup_profile(&state.profile);
    storage_service::save_lab_state_snapshot(&state);
    Ok(state)
}

pub async fn confirm_polar_block_height(
    mut profile: SetupProfile,
    block_height: u64,
) -> Result<LabState, String> {
    lightning_service::validate_setup_profile(&profile).map_err(|error| error.to_string())?;
    let polar_observed_block_height = if should_read_polar(&profile) {
        Some(polar_bridge_service::get_blockchain_height(&profile.polar_automation).await?)
    } else {
        None
    };

    profile.connection_status = lightning_service::ConnectionStatus::PartiallyConnected;
    profile.polar_block_height_confirmed = true;
    profile.last_verified_at = None;

    let mut state = storage_service::load_lab_state_snapshot()
        .unwrap_or_else(|| lightning_service::default_lab_state(profile.clone()));
    state.profile = profile;
    state.block_height = block_height;
    state.polar_observed_block_height = polar_observed_block_height;

    storage_service::save_setup_profile(&state.profile);
    storage_service::save_lab_state_snapshot(&state);
    Ok(state)
}

pub async fn destroy_polar_demo_nodes(profile: SetupProfile) -> Result<SetupProfile, String> {
    lightning_service::validate_setup_profile(&profile).map_err(|error| error.to_string())?;
    let mut profile = profile;
    profile.polar_automation =
        polar_bridge_service::destroy_demo_nodes(&profile.polar_automation).await?;

    profile.connection_status = lightning_service::ConnectionStatus::SavedOffline;
    profile.last_verified_at = None;
    profile.polar_automation.network_id.clear();
    storage_service::save_setup_profile(&profile);
    storage_service::clear_lab_state_snapshot();
    Ok(profile)
}

pub async fn delete_created_polar_server(profile: SetupProfile) -> Result<SetupProfile, String> {
    lightning_service::validate_setup_profile(&profile).map_err(|error| error.to_string())?;
    polar_bridge_service::delete_polar_network(&profile.polar_automation).await?;

    let mut profile = profile;
    profile.connection_status = lightning_service::ConnectionStatus::NotConfigured;
    profile.last_verified_at = None;
    profile.polar_automation.network_id.clear();
    profile.polar_automation.bitcoin_backend_name =
        lightning_service::DEFAULT_BITCOIN_BACKEND_NAME.to_string();

    storage_service::save_setup_profile(&profile);
    storage_service::clear_lab_state_snapshot();
    Ok(profile)
}

pub async fn reset_polar_setup_start(mut profile: SetupProfile) -> Result<SetupProfile, String> {
    profile.connection_status = lightning_service::ConnectionStatus::NotConfigured;
    profile.last_verified_at = None;
    profile.polar_automation.network_id.clear();
    profile.polar_automation.bitcoin_backend_name =
        lightning_service::DEFAULT_BITCOIN_BACKEND_NAME.to_string();

    storage_service::save_setup_profile(&profile);
    storage_service::clear_lab_state_snapshot();
    Ok(profile)
}

pub async fn lock_polar_setup_completion(mut profile: SetupProfile) -> Result<LabState, String> {
    lightning_service::validate_setup_profile(&profile).map_err(|error| error.to_string())?;
    profile.connection_status = lightning_service::ConnectionStatus::SavedOffline;
    profile.last_verified_at = None;

    let mut state = storage_service::load_lab_state_snapshot()
        .unwrap_or_else(|| lightning_service::default_lab_state(profile.clone()));
    state.profile = profile;
    let state = refresh_polar_block_height(state).await?;

    storage_service::save_setup_profile(&state.profile);
    storage_service::save_lab_state_snapshot(&state);
    Ok(state)
}

pub async fn complete_polar_setup(mut profile: SetupProfile) -> Result<LabState, String> {
    lightning_service::validate_setup_profile(&profile).map_err(|error| error.to_string())?;
    profile.connection_status = lightning_service::ConnectionStatus::Connected;
    profile.polar_block_height_confirmed = true;
    profile.last_verified_at = Some(chrono::Utc::now());

    let mut state = storage_service::load_lab_state_snapshot()
        .unwrap_or_else(|| lightning_service::default_lab_state(profile.clone()));
    state.profile = profile;

    storage_service::save_setup_profile(&state.profile);
    storage_service::save_lab_state_snapshot(&state);
    Ok(state)
}

pub async fn get_lab_state(profile: SetupProfile) -> Result<LabState, String> {
    let can_load_snapshot = profile.is_connected()
        || profile.connection_status == lightning_service::ConnectionStatus::PartiallyConnected;

    if !can_load_snapshot {
        return Ok(lightning_service::default_lab_state(profile));
    }

    let state = storage_service::load_lab_state_snapshot()
        .filter(|state| {
            state.profile.is_connected()
                || state.profile.connection_status
                    == lightning_service::ConnectionStatus::PartiallyConnected
        })
        .unwrap_or_else(|| lightning_service::default_lab_state(profile.clone()));

    let state = if state.profile.sats_per_transaction == profile.sats_per_transaction
        && state.profile.setup_mode == profile.setup_mode
    {
        state
    } else {
        lightning_service::default_lab_state(profile)
    };

    let mut refreshed_state = refresh_polar_block_height(state).await?;
    if refreshed_state.profile.is_connected() && !refreshed_state.tra_items.is_empty() {
        refreshed_state = lightning_service::TraService::verify_tra_setup(refreshed_state)
            .map_err(|error| error.to_string())?;
    }
    storage_service::save_lab_state_snapshot(&refreshed_state);
    Ok(refreshed_state)
}

pub async fn recover_if_polar_lab_unhealthy(profile: SetupProfile) -> Option<PolarLabRecovery> {
    if !profile.is_connected() || !should_read_polar(&profile) {
        return None;
    }

    match polar_bridge_service::validate_lab_health_verification_poll(&profile.polar_automation)
        .await
    {
        Ok(report) => {
            if report.profile != profile.polar_automation {
                let mut saved_profile = profile;
                saved_profile.polar_automation = report.profile;
                storage_service::save_setup_profile(&saved_profile);
            }
            None
        }
        Err(issue) => Some(recover_from_polar_lab_issue(profile, issue).await),
    }
}

pub async fn get_lab_state_or_recover(profile: SetupProfile) -> Result<LabState, PolarLabRecovery> {
    if let Some(recovery) = recover_if_polar_lab_unhealthy(profile.clone()).await {
        return Err(recovery);
    }

    match get_lab_state(profile.clone()).await {
        Ok(state) => Ok(state),
        Err(message) => Err(recover_from_polar_lab_issue(
            profile,
            PolarLabHealthIssue::BridgeUnavailable(message),
        )
        .await),
    }
}

pub async fn resume_polar_setup_after_restart(
    profile: SetupProfile,
) -> Result<LabState, PolarLabRecovery> {
    if profile.setup_mode != lightning_service::SetupMode::ServerConfig
        || !profile.polar_automation.is_complete()
    {
        return Ok(lightning_service::default_lab_state(profile));
    }

    match polar_bridge_service::validate_lab_health(&profile.polar_automation).await {
        Ok(report) => {
            let mut saved_profile = profile;
            let was_connected = saved_profile.connection_status == ConnectionStatus::Connected;
            let block_height_was_confirmed = saved_profile.polar_block_height_confirmed;
            saved_profile.polar_automation = report.profile;
            if was_connected {
                saved_profile.connection_status = ConnectionStatus::Connected;
                saved_profile.last_verified_at = Some(chrono::Utc::now());
            } else {
                saved_profile.last_verified_at = None;
            }
            let recovery_profile = saved_profile.clone();

            let mut state = storage_service::load_lab_state_snapshot()
                .unwrap_or_else(|| lightning_service::default_lab_state(saved_profile.clone()));
            state.profile = saved_profile;
            if let Some(block_height) = report.block_height.filter(|_| was_connected) {
                state = reconcile_polar_block_height(state, block_height).map_err(|error| {
                    recovery_for_issue(
                        recovery_profile.clone(),
                        PolarLabHealthIssue::BridgeUnavailable(error),
                    )
                })?;
            } else if !block_height_was_confirmed {
                if let Some(block_height) = report.block_height {
                    state.block_height = block_height;
                    state.polar_observed_block_height = Some(block_height);
                }
            }

            if state.profile.is_connected() && !state.tra_items.is_empty() {
                state =
                    lightning_service::TraService::verify_tra_setup(state).map_err(|error| {
                        recovery_for_issue(
                            recovery_profile.clone(),
                            PolarLabHealthIssue::BridgeUnavailable(error.to_string()),
                        )
                    })?;
            }

            storage_service::save_setup_profile(&state.profile);
            storage_service::save_lab_state_snapshot(&state);
            Ok(state)
        }
        Err(issue) => Err(recover_from_polar_lab_issue(profile, issue).await),
    }
}

pub async fn open_trade_route(
    profile: SetupProfile,
    to_node: DemoNodeId,
) -> Result<LabState, String> {
    let state = get_lab_state(profile).await?;
    let route_capacity_sats = state.profile.sats_per_transaction.saturating_mul(3);
    let state =
        lightning_service::open_trade_route(state, DemoNodeId::Alice, to_node, route_capacity_sats)
            .map_err(|error| error.to_string())?;
    storage_service::save_lab_state_snapshot(&state);
    Ok(state)
}

pub async fn close_trade_route(
    profile: SetupProfile,
    to_node: DemoNodeId,
) -> Result<LabState, String> {
    let state = get_lab_state(profile).await?;
    let state = lightning_service::close_trade_route(state, DemoNodeId::Alice, to_node)
        .map_err(|error| error.to_string())?;
    storage_service::save_lab_state_snapshot(&state);
    Ok(state)
}

pub async fn wait_for_next_block(
    profile: SetupProfile,
    affected_route_id: Option<String>,
) -> Result<LabState, String> {
    let state = get_lab_state(profile).await?;
    let reason = affected_route_id
        .as_ref()
        .and_then(|route_id| {
            state
                .trade_routes
                .iter()
                .find(|route| &route.route_id == route_id)
        })
        .map(|route| {
            if route.status == RouteStatus::Closing {
                BlockWaitReason::ChannelCloseConfirmation
            } else {
                BlockWaitReason::ChannelOpenConfirmation
            }
        })
        .unwrap_or(BlockWaitReason::ChannelOpenConfirmation);
    let polar_height = if should_read_polar(&state.profile) {
        Some(polar_bridge_service::mine_blocks(&state.profile.polar_automation, 1).await?)
    } else {
        None
    };
    let state = lightning_service::wait_for_next_block(state, reason, affected_route_id)
        .map_err(|error| error.to_string())?;
    let mut state = state;
    if let Some(height) = polar_height {
        state.polar_observed_block_height = Some(height);
        if let Some(action) = state.block_actions.last_mut() {
            action.resulting_height = Some(state.block_height);
        }
    }
    storage_service::save_lab_state_snapshot(&state);
    Ok(state)
}

pub async fn create_invoice(
    profile: SetupProfile,
    creator_node: DemoNodeId,
    expected_payer_node: Option<DemoNodeId>,
    memo: String,
) -> Result<LabState, String> {
    let amount_sats = profile.sats_per_transaction;
    let state = get_lab_state(profile).await?;
    let state = lightning_service::create_invoice(
        state,
        creator_node,
        expected_payer_node,
        amount_sats,
        memo,
    )
    .map_err(|error| error.to_string())?;
    storage_service::save_lab_state_snapshot(&state);
    Ok(state)
}

pub async fn pay_latest_invoice(
    profile: SetupProfile,
    payer_node: DemoNodeId,
) -> Result<LabState, String> {
    let state = get_lab_state(profile).await?;
    let invoice_id = state
        .recent_invoices
        .first()
        .map(|invoice| invoice.invoice_id.clone())
        .ok_or_else(|| "Create an invoice before trying to pay it.".to_string())?;
    let state = lightning_service::pay_invoice(state, payer_node, invoice_id)
        .map_err(|error| error.to_string())?;
    storage_service::save_lab_state_snapshot(&state);
    Ok(state)
}

pub async fn create_invoice_and_maybe_autosend(
    profile: SetupProfile,
    creator_node: DemoNodeId,
    candidate_payer_node: DemoNodeId,
    autosend_enabled: bool,
    memo: String,
) -> Result<LabState, String> {
    let amount_sats = profile.sats_per_transaction;
    let state = get_lab_state(profile).await?;
    let state = lightning_service::create_invoice_and_maybe_autosend(
        state,
        creator_node,
        candidate_payer_node,
        amount_sats,
        memo,
        autosend_enabled,
    )
    .map_err(|error| error.to_string())?;
    storage_service::save_lab_state_snapshot(&state);
    Ok(state)
}

pub async fn create_invoice_and_maybe_autosend_for_amount(
    profile: SetupProfile,
    creator_node: DemoNodeId,
    candidate_payer_node: DemoNodeId,
    amount_sats: u64,
    autosend_enabled: bool,
    memo: String,
) -> Result<LabState, String> {
    let state = get_lab_state(profile).await?;
    let state = lightning_service::create_invoice_and_maybe_autosend(
        state,
        creator_node,
        candidate_payer_node,
        amount_sats,
        memo,
        autosend_enabled,
    )
    .map_err(|error| error.to_string())?;
    storage_service::save_lab_state_snapshot(&state);
    Ok(state)
}

pub async fn get_tra_item_catalog() -> Result<Vec<GameItemDefinition>, String> {
    Ok(lightning_service::TraService::item_catalog())
}

pub fn initial_tra_setup_items() -> Vec<MintTraRequest> {
    lightning_service::TraService::initial_setup_items()
}

pub async fn verify_tra_setup(profile: SetupProfile) -> Result<LabState, String> {
    let state = get_lab_state(profile).await?;
    let state = lightning_service::TraService::verify_tra_setup(state)
        .map_err(|error| error.to_string())?;
    storage_service::save_lab_state_snapshot(&state);
    Ok(state)
}

pub async fn reset_tra_inventory(profile: SetupProfile) -> Result<LabState, String> {
    let state = get_lab_state(profile).await?;
    let state = lightning_service::TraService::reset_tra_inventory(state)
        .map_err(|error| error.to_string())?;
    storage_service::save_lab_state_snapshot(&state);
    Ok(state)
}

pub async fn mint_tra(profile: SetupProfile, request: MintTraRequest) -> Result<LabState, String> {
    let state = get_lab_state(profile).await?;
    let state = lightning_service::TraService::mint_tra(state, request)
        .map_err(|error| error.to_string())?;
    storage_service::save_lab_state_snapshot(&state);
    Ok(state)
}

pub async fn transfer_tra(
    profile: SetupProfile,
    request: TransferTraRequest,
) -> Result<LabState, String> {
    let state = get_lab_state(profile).await?;
    let state = lightning_service::TraService::transfer_tra(state, request)
        .map_err(|error| error.to_string())?;
    storage_service::save_lab_state_snapshot(&state);
    Ok(state)
}

pub async fn reset_lab() -> Result<SetupProfile, String> {
    storage_service::clear_setup_profile();
    storage_service::clear_lab_state_snapshot();
    Ok(SetupProfile::default())
}

async fn recover_from_polar_lab_issue(
    profile: SetupProfile,
    issue: PolarLabHealthIssue,
) -> PolarLabRecovery {
    let recovery = recovery_for_issue(profile, issue);

    storage_service::save_setup_profile(&recovery.profile);
    storage_service::clear_lab_state_snapshot();

    recovery
}

fn recovery_for_issue(mut profile: SetupProfile, issue: PolarLabHealthIssue) -> PolarLabRecovery {
    let message = recovery_message(&issue);
    let lab_state = match issue {
        PolarLabHealthIssue::BridgeUnavailable(_) => {
            profile.connection_status = ConnectionStatus::NotConfigured;
            profile.last_verified_at = None;
            None
        }
        PolarLabHealthIssue::NetworkMissing { .. }
        | PolarLabHealthIssue::NetworkStopped { .. }
        | PolarLabHealthIssue::BitcoinBackendMissing { .. } => {
            profile.connection_status = ConnectionStatus::SavedOffline;
            profile.last_verified_at = None;
            profile.polar_automation.network_id.clear();
            profile.polar_automation.bitcoin_backend_name =
                DEFAULT_BITCOIN_BACKEND_NAME.to_string();
            None
        }
        PolarLabHealthIssue::DemoNodeMissing { .. }
        | PolarLabHealthIssue::DemoNodeStopped { .. } => {
            profile.connection_status = ConnectionStatus::SavedOffline;
            profile.last_verified_at = None;
            None
        }
    };

    PolarLabRecovery {
        profile,
        lab_state,
        message,
    }
}

fn recovery_message(issue: &PolarLabHealthIssue) -> String {
    match issue {
        PolarLabHealthIssue::BridgeUnavailable(_) => {
            "Cannot reach Polar bridge. Open Polar, then return to Set Up.".to_string()
        }
        PolarLabHealthIssue::NetworkMissing { .. } => {
            "Polar server is missing. Recreate or reuse it in Set Up.".to_string()
        }
        PolarLabHealthIssue::NetworkStopped { .. } => {
            "Polar network stopped. Start it again in Set Up.".to_string()
        }
        PolarLabHealthIssue::BitcoinBackendMissing { .. } => {
            "Polar Bitcoin backend is missing. Recheck the server in Set Up.".to_string()
        }
        PolarLabHealthIssue::DemoNodeMissing { node_id, .. } => {
            format!(
                "Polar demo node {} is missing. Recreate Alice, Bob, and Carol in Set Up.",
                node_id.label()
            )
        }
        PolarLabHealthIssue::DemoNodeStopped { node_id, .. } => {
            format!(
                "Polar demo node {} is stopped. Recreate or start demo nodes in Set Up.",
                node_id.label()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::models::{PolarAutomationProfile, DEFAULT_NETWORK_NAME};

    fn connected_profile() -> SetupProfile {
        let mut profile = SetupProfile::default();
        profile.connection_status = ConnectionStatus::Connected;
        profile.network_name = DEFAULT_NETWORK_NAME.to_string();
        profile.polar_automation = PolarAutomationProfile {
            bridge_url: "http://localhost:37373".to_string(),
            network_id: "1".to_string(),
            bitcoin_backend_name: DEFAULT_BITCOIN_BACKEND_NAME.to_string(),
        };
        profile
    }

    #[test]
    fn bridge_failure_resumes_at_bridge_step() {
        let recovery = recovery_for_issue(
            connected_profile(),
            PolarLabHealthIssue::BridgeUnavailable("offline".to_string()),
        );

        assert_eq!(
            recovery.profile.connection_status,
            ConnectionStatus::NotConfigured
        );
        assert_eq!(recovery.profile.polar_automation.network_id, "1");
        assert!(recovery.lab_state.is_none());
    }

    #[test]
    fn stopped_network_resumes_at_server_step() {
        let recovery = recovery_for_issue(
            connected_profile(),
            PolarLabHealthIssue::NetworkStopped {
                network_id: "1".to_string(),
                status: "Stopped".to_string(),
            },
        );

        assert_eq!(
            recovery.profile.connection_status,
            ConnectionStatus::SavedOffline
        );
        assert!(recovery.profile.polar_automation.network_id.is_empty());
        assert_eq!(
            recovery.profile.polar_automation.bitcoin_backend_name,
            DEFAULT_BITCOIN_BACKEND_NAME
        );
        assert!(recovery.lab_state.is_none());
    }

    #[test]
    fn missing_demo_node_resumes_at_demo_node_step() {
        let recovery = recovery_for_issue(
            connected_profile(),
            PolarLabHealthIssue::DemoNodeMissing {
                network_id: "1".to_string(),
                node_id: DemoNodeId::Alice,
            },
        );

        assert_eq!(
            recovery.profile.connection_status,
            ConnectionStatus::SavedOffline
        );
        assert_eq!(recovery.profile.polar_automation.network_id, "1");
        assert!(recovery.lab_state.is_none());
    }

    #[test]
    fn polar_poll_preserves_user_block_height_baseline() {
        let mut state = lightning_service::default_lab_state(connected_profile());
        state.profile.polar_block_height_confirmed = true;
        state.block_height = 0;
        state.polar_observed_block_height = Some(300);

        let state = reconcile_polar_block_height(state, 300).expect("same polar height");
        assert_eq!(state.block_height, 0);
        assert_eq!(state.polar_observed_block_height, Some(300));

        let state = reconcile_polar_block_height(state, 301).expect("next polar height");
        assert_eq!(state.block_height, 1);
        assert_eq!(state.polar_observed_block_height, Some(301));
    }
}

async fn refresh_polar_block_height(mut state: LabState) -> Result<LabState, String> {
    if should_read_polar(&state.profile) {
        let polar_height = polar_bridge_service::get_blockchain_height_verification_poll(
            &state.profile.polar_automation,
        )
        .await?;
        state = if state.profile.polar_block_height_confirmed {
            reconcile_polar_block_height(state, polar_height)?
        } else {
            state.block_height = polar_height;
            state.polar_observed_block_height = Some(polar_height);
            state
        };
    }

    Ok(state)
}

fn reconcile_polar_block_height(
    mut state: LabState,
    polar_height: u64,
) -> Result<LabState, String> {
    let previous_observed_height = state
        .polar_observed_block_height
        .unwrap_or(state.block_height);

    if polar_height > previous_observed_height {
        let block_delta = polar_height - previous_observed_height;
        let next_app_height = state.block_height.saturating_add(block_delta);
        state = lightning_service::apply_external_block_height(state, next_app_height)
            .map_err(|error| error.to_string())?;
    }

    state.polar_observed_block_height = Some(polar_height);
    Ok(state)
}

fn should_read_polar(profile: &SetupProfile) -> bool {
    profile.setup_mode == lightning_service::SetupMode::ServerConfig
        && profile.polar_automation.is_complete()
}
