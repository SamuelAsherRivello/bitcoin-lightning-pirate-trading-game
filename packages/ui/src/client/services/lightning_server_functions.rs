use crate::client::models::{
    AuthAction, BlockWaitReason, ConnectionStatus, DemoNodeId, GameItemDefinition, GameTreasury,
    LabState, MintTraRequest, NodeStatus, PlayerAuthSession, RouteStatus, SetupProfile,
    TraOwnershipStatus, TraTransferStatus, TransferTraRequest, TreasuryImpactPreview,
    APPLE_ITEM_ID, BOOK_ITEM_ID, DEFAULT_BITCOIN_BACKEND_NAME, DEFAULT_ROUTE_CAPACITY_SATS,
};
use crate::client::services::{polar_bridge_service, storage_service};

pub use polar_bridge_service::{
    PolarDeleteAllProgress, PolarDeleteAllResult, PolarLabHealthIssue, PolarServerEnsureResult,
    PolarServerEnsureStatus,
};

#[derive(Clone, Debug, PartialEq)]
pub struct PolarLabRecovery {
    pub profile: SetupProfile,
    pub lab_state: Option<LabState>,
    pub message: String,
}

pub async fn begin_player_auth(
    profile: SetupProfile,
    action: AuthAction,
) -> Result<PlayerAuthSession, String> {
    lightning_service::begin_player_auth(&profile, action).map_err(|error| error.to_string())
}

pub async fn display_player_auth_session(session: PlayerAuthSession) -> PlayerAuthSession {
    lightning_service::display_player_auth_session(session)
}

pub async fn approve_mock_player_auth_session(
    session: PlayerAuthSession,
) -> Result<PlayerAuthSession, String> {
    lightning_service::approve_mock_player_auth_session(session).map_err(|error| error.to_string())
}

pub async fn complete_player_auth(
    session: PlayerAuthSession,
    linking_key_fingerprint: String,
) -> Result<PlayerAuthSession, String> {
    lightning_service::complete_player_auth(session, linking_key_fingerprint)
        .map_err(|error| error.to_string())
}

pub async fn cancel_player_auth_session(session: PlayerAuthSession) -> PlayerAuthSession {
    lightning_service::cancel_player_auth_session(session)
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
    let state = refresh_polar_node_names(state).await?;

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
    let previous_state = storage_service::load_lab_state_snapshot();

    profile.polar_automation = result.profile.clone();
    profile.connection_status = lightning_service::ConnectionStatus::SavedOffline;
    profile.polar_block_height_confirmed = false;
    profile.game_treasury_ready = false;
    profile.game_treasury_funded_sats = 0;
    profile.last_verified_at = None;

    if let Some(mut state) =
        previous_state.filter(|state| setup_snapshot_matches_polar_network(state, &profile))
    {
        profile.game_treasury_ready = state.profile.game_treasury_ready;
        profile.game_treasury_funded_sats = state.profile.game_treasury_funded_sats;
        profile.polar_block_height_confirmed = state.profile.polar_block_height_confirmed;
        state.profile = profile.clone();
        storage_service::save_setup_profile(&profile);
        storage_service::save_lab_state_snapshot(&state);
    } else {
        storage_service::save_setup_profile(&profile);
        storage_service::clear_lab_state_snapshot();
    }

    Ok(result)
}

pub async fn create_polar_demo_nodes(profile: SetupProfile) -> Result<LabState, String> {
    create_required_polar_nodes_with_progress(profile, |_| {}).await
}

pub async fn create_required_polar_nodes(profile: SetupProfile) -> Result<LabState, String> {
    create_required_polar_nodes_with_progress(profile, |_| {}).await
}

pub async fn create_polar_demo_nodes_with_progress<F>(
    profile: SetupProfile,
    report_progress: F,
) -> Result<LabState, String>
where
    F: FnMut(String),
{
    create_required_polar_nodes_with_progress(profile, report_progress).await
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

pub async fn create_required_polar_nodes_with_progress<F>(
    profile: SetupProfile,
    report_progress: F,
) -> Result<LabState, String>
where
    F: FnMut(String),
{
    lightning_service::validate_setup_profile(&profile).map_err(|error| error.to_string())?;
    let mut profile = profile;
    if let Some(mut state) = matching_setup_snapshot(&profile) {
        if setup_has_user_nodes(&state) {
            state.profile = profile;
            state.profile.connection_status =
                lightning_service::ConnectionStatus::PartiallyConnected;
            state.profile.polar_block_height_confirmed = false;
            state.profile.last_verified_at = None;
            let mut state = refresh_polar_block_height(state).await?;
            state = refresh_polar_node_names(state).await?;
            storage_service::save_setup_profile(&state.profile);
            storage_service::save_lab_state_snapshot(&state);
            return Ok(state);
        }
    }
    let required_balance_sats = profile
        .sats_per_transaction
        .max(DEFAULT_ROUTE_CAPACITY_SATS);
    profile.polar_automation = polar_bridge_service::create_required_nodes_with_progress(
        &profile.polar_automation,
        required_balance_sats,
        report_progress,
    )
    .await?;

    let previous_state = storage_service::load_lab_state_snapshot();
    let mut state = lightning_service::test_setup(profile).map_err(|error| error.to_string())?;
    if let Some(previous_state) = previous_state {
        if previous_state.profile.game_treasury_ready {
            state.game_treasury = previous_state.game_treasury;
            state.tra_items = previous_state.tra_items;
        }
    }
    let mut state = refresh_polar_block_height(state).await?;
    state = refresh_polar_node_names(state).await?;
    state.profile.connection_status = lightning_service::ConnectionStatus::PartiallyConnected;
    state.profile.polar_block_height_confirmed = false;
    storage_service::save_setup_profile(&state.profile);
    storage_service::save_lab_state_snapshot(&state);
    Ok(state)
}

pub async fn prepare_user_node_sats(profile: SetupProfile) -> Result<LabState, String> {
    lightning_service::validate_setup_profile(&profile).map_err(|error| error.to_string())?;
    let mut profile = profile;
    let required_balance_sats = profile
        .sats_per_transaction
        .max(DEFAULT_ROUTE_CAPACITY_SATS);
    if should_read_polar(&profile) {
        profile.polar_automation = polar_bridge_service::fund_demo_user_nodes(
            &profile.polar_automation,
            required_balance_sats,
        )
        .await?;
        profile.connection_status = lightning_service::ConnectionStatus::PartiallyConnected;
        profile.polar_block_height_confirmed = false;
        profile.last_verified_at = None;
    }

    let mut state = get_lab_state(profile).await?;
    for node in &mut state.nodes {
        if DemoNodeId::ALL.contains(&node.node_id) {
            node.wallet_balance_sats = 1_000_000;
        }
    }
    storage_service::save_setup_profile(&state.profile);
    storage_service::save_lab_state_snapshot(&state);
    Ok(state)
}

pub async fn prepare_game_treasury(profile: SetupProfile) -> Result<LabState, String> {
    lightning_service::validate_setup_profile(&profile).map_err(|error| error.to_string())?;
    let mut profile = profile;
    if let Some(mut state) = matching_setup_snapshot(&profile) {
        if state.profile.game_treasury_ready {
            profile.game_treasury_ready = true;
            profile.game_treasury_funded_sats = state.profile.game_treasury_funded_sats;
            profile.connection_status = lightning_service::ConnectionStatus::PartiallyConnected;
            profile.last_verified_at = None;
            state.profile = profile;
            let state = refresh_polar_block_height(state).await?;
            storage_service::save_setup_profile(&state.profile);
            storage_service::save_lab_state_snapshot(&state);
            return Ok(state);
        }
    }
    let required_balance_sats = profile
        .sats_per_transaction
        .max(DEFAULT_ROUTE_CAPACITY_SATS);
    if should_read_polar(&profile) {
        profile.polar_automation = polar_bridge_service::create_game_treasury_node(
            &profile.polar_automation,
            required_balance_sats,
        )
        .await?;
        profile.connection_status = lightning_service::ConnectionStatus::PartiallyConnected;
        profile.last_verified_at = None;
        storage_service::save_setup_profile(&profile);
    }

    let mut state = storage_service::load_lab_state_snapshot()
        .unwrap_or_else(|| lightning_service::default_lab_state(profile.clone()));
    state.profile = profile;
    let state = lightning_service::TraService::prepare_game_treasury(state)
        .map_err(|error| error.to_string())?;
    storage_service::save_setup_profile(&state.profile);
    storage_service::save_lab_state_snapshot(&state);
    Ok(state)
}

pub async fn prepare_game_treasury_tras(profile: SetupProfile) -> Result<LabState, String> {
    lightning_service::validate_setup_profile(&profile).map_err(|error| error.to_string())?;
    if !profile.game_treasury_ready {
        return Err("Complete Game Treasury (Sats) before creating treasury TRAs.".to_string());
    }
    let mut profile = profile;
    if let Some(mut state) = matching_setup_snapshot(&profile) {
        if setup_has_treasury_tras(&state) {
            profile.connection_status = lightning_service::ConnectionStatus::PartiallyConnected;
            profile.polar_block_height_confirmed = false;
            profile.last_verified_at = None;
            state.profile = profile;
            let state = refresh_polar_block_height(state).await?;
            storage_service::save_setup_profile(&state.profile);
            storage_service::save_lab_state_snapshot(&state);
            return Ok(state);
        }
    }
    if should_read_polar(&profile) {
        profile.polar_automation =
            polar_bridge_service::ensure_taproot_assets_node(&profile.polar_automation).await?;
        profile.connection_status = lightning_service::ConnectionStatus::PartiallyConnected;
        profile.last_verified_at = None;
        storage_service::save_setup_profile(&profile);
    }

    let mut state = storage_service::load_lab_state_snapshot()
        .unwrap_or_else(|| lightning_service::default_lab_state(profile.clone()));
    state.profile = profile;
    let state = lightning_service::TraService::prepare_game_treasury_items(state)
        .map_err(|error| error.to_string())?;
    storage_service::save_setup_profile(&state.profile);
    storage_service::save_lab_state_snapshot(&state);
    Ok(state)
}

pub async fn transfer_npc_starting_items(profile: SetupProfile) -> Result<LabState, String> {
    prepare_user_node_tras(profile).await
}

pub async fn prepare_user_node_tras(mut profile: SetupProfile) -> Result<LabState, String> {
    if let Some(mut state) = matching_setup_snapshot(&profile) {
        if setup_has_npc_transfers(&state) {
            profile.connection_status = lightning_service::ConnectionStatus::PartiallyConnected;
            profile.polar_block_height_confirmed = false;
            profile.last_verified_at = None;
            state.profile = profile;
            let mut state = refresh_polar_block_height(state).await?;
            state = refresh_polar_node_names(state).await?;
            storage_service::save_setup_profile(&state.profile);
            storage_service::save_lab_state_snapshot(&state);
            return Ok(state);
        }
    }

    if should_read_polar(&profile) {
        let report = polar_bridge_service::validate_lab_health(&profile.polar_automation)
            .await
            .map_err(|issue| recovery_message(&issue))?;
        profile.polar_automation = report.profile;
    }

    let state = get_lab_state(profile).await?;
    if !setup_has_treasury_tras(&state) {
        return Err(
            "Complete Game Treasury (TRAs) before transferring starting items to NPCs.".to_string(),
        );
    }
    let state = lightning_service::TraService::transfer_npc_starting_items(state)
        .map_err(|error| error.to_string())?;
    storage_service::save_setup_profile(&state.profile);
    storage_service::save_lab_state_snapshot(&state);
    Ok(state)
}

pub async fn get_game_treasury_summary(profile: SetupProfile) -> Result<GameTreasury, String> {
    let state = get_lab_state(profile).await?;
    Ok(lightning_service::TraService::treasury_summary(&state))
}

pub async fn preview_treasury_impact(
    profile: SetupProfile,
    action_label: String,
    amount_sats: Option<u64>,
) -> Result<TreasuryImpactPreview, String> {
    let state = get_lab_state(profile).await?;
    Ok(lightning_service::TraService::preview_treasury_impact(
        &state,
        action_label,
        amount_sats,
    ))
}

pub async fn confirm_polar_block_height(
    mut profile: SetupProfile,
    block_height: u64,
) -> Result<LabState, String> {
    lightning_service::validate_setup_profile(&profile).map_err(|error| error.to_string())?;
    let should_read_polar = should_read_polar(&profile);
    let polar_observed_block_height = if should_read_polar {
        let report = polar_bridge_service::validate_lab_health(&profile.polar_automation)
            .await
            .map_err(|issue| recovery_message(&issue))?;
        profile.polar_automation = report.profile;
        report.block_height
    } else {
        None
    };

    let mut state = storage_service::load_lab_state_snapshot()
        .ok_or_else(|| "Complete User Nodes (TRAs) before setting Block Height.".to_string())?;

    profile.connection_status = if should_read_polar {
        lightning_service::ConnectionStatus::PartiallyConnected
    } else {
        lightning_service::ConnectionStatus::SavedOffline
    };
    profile.last_verified_at = None;
    state.profile = profile.clone();

    if !setup_ready_for_block_height(&state) {
        return Err("Complete Game Treasury (Sats), Game Treasury (TRAs), User Nodes (Sats), and User Nodes (TRAs) before setting Block Height.".to_string());
    }

    profile.polar_block_height_confirmed = true;
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

pub async fn delete_all_polar_networks(
    profile: SetupProfile,
) -> Result<(SetupProfile, usize), String> {
    delete_all_polar_networks_with_progress(profile, |_| {})
        .await
        .map(|(profile, result)| (profile, result.deleted_count))
}

pub async fn count_polar_networks(profile: SetupProfile) -> Result<usize, String> {
    lightning_service::validate_setup_profile(&profile).map_err(|error| error.to_string())?;
    polar_bridge_service::count_polar_networks(&profile.polar_automation).await
}

pub async fn delete_all_polar_networks_with_progress<F>(
    profile: SetupProfile,
    report_progress: F,
) -> Result<(SetupProfile, PolarDeleteAllResult), String>
where
    F: FnMut(PolarDeleteAllProgress),
{
    lightning_service::validate_setup_profile(&profile).map_err(|error| error.to_string())?;
    let result = polar_bridge_service::delete_all_polar_networks_with_progress(
        &profile.polar_automation,
        report_progress,
    )
    .await?;

    let mut profile = profile;
    profile.connection_status = lightning_service::ConnectionStatus::NotConfigured;
    profile.last_verified_at = None;
    profile.polar_automation.network_id.clear();
    profile.polar_automation.bitcoin_backend_name =
        lightning_service::DEFAULT_BITCOIN_BACKEND_NAME.to_string();

    storage_service::save_setup_profile(&profile);
    storage_service::clear_lab_state_snapshot();
    Ok((profile, result))
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
    let state = refresh_polar_node_names(state).await?;

    storage_service::save_setup_profile(&state.profile);
    storage_service::save_lab_state_snapshot(&state);
    Ok(state)
}

pub async fn complete_polar_setup(mut profile: SetupProfile) -> Result<LabState, String> {
    lightning_service::validate_setup_profile(&profile).map_err(|error| error.to_string())?;
    if should_read_polar(&profile) {
        let report = polar_bridge_service::validate_lab_health(&profile.polar_automation)
            .await
            .map_err(|issue| recovery_message(&issue))?;
        profile.polar_automation = report.profile;
    }

    let mut state = storage_service::load_lab_state_snapshot().ok_or_else(|| {
        "Complete all seven Polar setup steps before unlocking routes.".to_string()
    })?;
    state.profile = profile.clone();

    if !setup_ready_for_unlock(&state) {
        return Err("Complete Game Treasury (Sats), Game Treasury (TRAs), User Nodes (Sats), User Nodes (TRAs), and Block Height before unlocking routes.".to_string());
    }

    profile.connection_status = lightning_service::ConnectionStatus::Connected;
    profile.polar_block_height_confirmed = true;
    profile.last_verified_at = Some(chrono::Utc::now());

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

    let mut state = storage_service::load_lab_state_snapshot()
        .filter(|state| {
            state.profile.is_connected()
                || state.profile.connection_status
                    == lightning_service::ConnectionStatus::PartiallyConnected
        })
        .unwrap_or_else(|| lightning_service::default_lab_state(profile.clone()));

    let state = if setup_snapshot_matches_profile(&state, &profile) {
        state.profile = profile;
        state
    } else {
        lightning_service::default_lab_state(profile)
    };

    let mut refreshed_state = refresh_polar_block_height(state).await?;
    refreshed_state = refresh_polar_node_names(refreshed_state).await?;
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

pub async fn execute_tra_item_trade(
    profile: SetupProfile,
    creator_node: DemoNodeId,
    candidate_payer_node: DemoNodeId,
    amount_sats: u64,
    memo: String,
    transfer_request: TransferTraRequest,
) -> Result<LabState, String> {
    let state = get_lab_state(profile).await?;
    let state = lightning_service::create_invoice_and_maybe_autosend(
        state,
        creator_node,
        candidate_payer_node,
        amount_sats,
        memo,
        true,
    )
    .map_err(|error| error.to_string())?;
    let state = lightning_service::TraService::transfer_tra(state, transfer_request)
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

pub async fn preview_tra_setup(profile: SetupProfile) -> Result<LabState, String> {
    let can_load_snapshot = profile.is_connected()
        || profile.connection_status == lightning_service::ConnectionStatus::PartiallyConnected;

    if !can_load_snapshot {
        return Ok(lightning_service::default_lab_state(profile));
    }

    let mut state = storage_service::load_lab_state_snapshot()
        .filter(|state| {
            state.profile.is_connected()
                || state.profile.connection_status
                    == lightning_service::ConnectionStatus::PartiallyConnected
        })
        .unwrap_or_else(|| lightning_service::default_lab_state(profile.clone()));

    let state = if setup_snapshot_matches_profile(&state, &profile) {
        state.profile = profile;
        state
    } else {
        lightning_service::default_lab_state(profile)
    };

    let mut refreshed_state = refresh_polar_block_height(state).await?;
    if refreshed_state.profile.is_connected() && !refreshed_state.tra_items.is_empty() {
        refreshed_state = lightning_service::TraService::verify_tra_setup(refreshed_state)
            .map_err(|error| error.to_string())?;
    }
    Ok(refreshed_state)
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
    fn partial_polar_setup_steps_still_require_polar_reads() {
        let mut profile = connected_profile();
        profile.connection_status = ConnectionStatus::PartiallyConnected;

        assert!(should_read_polar(&profile));
    }

    #[test]
    fn mock_setup_steps_do_not_require_polar_reads() {
        let mut profile = connected_profile();
        profile.setup_mode = lightning_service::SetupMode::BrowserRegtestOnly;

        assert!(!should_read_polar(&profile));
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

    #[test]
    fn block_height_guard_requires_npc_transfers() {
        let mut state = lightning_service::default_lab_state(connected_profile());
        state =
            lightning_service::TraService::prepare_game_treasury(state).expect("treasury prepared");
        state = lightning_service::TraService::prepare_game_treasury_items(state)
            .expect("treasury items prepared");

        assert!(!setup_ready_for_block_height(&state));

        let state = lightning_service::TraService::transfer_npc_starting_items(state)
            .expect("npc item transfers");

        assert!(setup_ready_for_block_height(&state));
        assert!(!setup_ready_for_unlock(&state));
    }

    #[test]
    fn unlock_guard_requires_confirmed_block_height() {
        let mut state = lightning_service::default_lab_state(connected_profile());
        state =
            lightning_service::TraService::prepare_game_treasury(state).expect("treasury prepared");
        state = lightning_service::TraService::prepare_game_treasury_items(state)
            .expect("treasury items prepared");
        state = lightning_service::TraService::transfer_npc_starting_items(state)
            .expect("npc item transfers");

        assert!(!setup_ready_for_unlock(&state));

        state.profile.polar_block_height_confirmed = true;

        assert!(setup_ready_for_unlock(&state));
    }

    #[test]
    fn snapshot_match_rejects_different_polar_network_identity() {
        let state = lightning_service::default_lab_state(connected_profile());
        let mut profile = state.profile.clone();

        assert!(setup_snapshot_matches_profile(&state, &profile));

        profile.polar_automation.network_id = "different-network".to_string();

        assert!(!setup_snapshot_matches_profile(&state, &profile));
    }

    #[test]
    fn snapshot_match_rejects_different_treasury_state() {
        let state = lightning_service::default_lab_state(connected_profile());
        let mut profile = state.profile.clone();

        assert!(setup_snapshot_matches_profile(&state, &profile));

        profile.game_treasury_ready = !profile.game_treasury_ready;

        assert!(!setup_snapshot_matches_profile(&state, &profile));
    }

    #[test]
    fn polar_network_snapshot_match_allows_regressed_step_flags() {
        let mut state = lightning_service::default_lab_state(connected_profile());
        state.profile.game_treasury_ready = true;
        state.profile.game_treasury_funded_sats = 50_000;
        state.profile.polar_block_height_confirmed = true;

        let mut regressed_profile = state.profile.clone();
        regressed_profile.connection_status = ConnectionStatus::SavedOffline;
        regressed_profile.game_treasury_ready = false;
        regressed_profile.game_treasury_funded_sats = 0;
        regressed_profile.polar_block_height_confirmed = false;

        assert!(setup_snapshot_matches_polar_network(
            &state,
            &regressed_profile
        ));
        assert!(!setup_snapshot_matches_profile(&state, &regressed_profile));
    }

    #[test]
    fn polar_network_snapshot_match_rejects_new_network_name() {
        let state = lightning_service::default_lab_state(connected_profile());
        let mut profile = state.profile.clone();
        profile.network_name = "fresh-network".to_string();

        assert!(!setup_snapshot_matches_polar_network(&state, &profile));
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

async fn refresh_polar_node_names(mut state: LabState) -> Result<LabState, String> {
    if !should_read_polar(&state.profile) {
        return Ok(state);
    }

    let names =
        polar_bridge_service::read_network_node_names(&state.profile.polar_automation).await?;
    state.profile.polar_automation.bitcoin_backend_name = names.bitcoin_backend_name;

    let treasury_name = names
        .game_treasury_name
        .unwrap_or_else(|| "Not listed by Polar".to_string());
    state.game_treasury.node_label = treasury_name.clone();
    if let Some(node) = state
        .nodes
        .iter_mut()
        .find(|node| node.node_id == DemoNodeId::GameTreasury)
    {
        node.alias = treasury_name;
    }

    let user_node_names = names.user_node_names;
    for (node_id, node_name) in &user_node_names {
        if let Some(node) = state.nodes.iter_mut().find(|node| node.node_id == *node_id) {
            node.alias = node_name.clone();
        }
    }

    for node_id in DemoNodeId::ALL {
        if !user_node_names
            .iter()
            .any(|(listed_node_id, _)| *listed_node_id == node_id)
        {
            if let Some(node) = state.nodes.iter_mut().find(|node| node.node_id == node_id) {
                node.alias = "Not listed by Polar".to_string();
            }
        }
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

fn setup_snapshot_matches_profile(state: &LabState, profile: &SetupProfile) -> bool {
    let snapshot = &state.profile;
    snapshot.sats_per_transaction == profile.sats_per_transaction
        && snapshot.setup_mode == profile.setup_mode
        && snapshot.network_name == profile.network_name
        && snapshot.polar_automation.bridge_url == profile.polar_automation.bridge_url
        && snapshot.polar_automation.network_id == profile.polar_automation.network_id
        && snapshot.polar_automation.bitcoin_backend_name
            == profile.polar_automation.bitcoin_backend_name
        && snapshot.game_treasury_ready == profile.game_treasury_ready
        && snapshot.game_treasury_funded_sats == profile.game_treasury_funded_sats
}

fn matching_setup_snapshot(profile: &SetupProfile) -> Option<LabState> {
    storage_service::load_lab_state_snapshot()
        .filter(|state| setup_snapshot_matches_polar_network(state, profile))
}

fn setup_snapshot_matches_polar_network(state: &LabState, profile: &SetupProfile) -> bool {
    let snapshot = &state.profile;
    snapshot.sats_per_transaction == profile.sats_per_transaction
        && snapshot.setup_mode == profile.setup_mode
        && snapshot.network_name == profile.network_name
        && snapshot.polar_automation.bridge_url == profile.polar_automation.bridge_url
        && snapshot.polar_automation.network_id == profile.polar_automation.network_id
}

fn setup_ready_for_block_height(state: &LabState) -> bool {
    state.profile.game_treasury_ready
        && setup_has_user_nodes(state)
        && setup_has_npc_transfers(state)
}

fn setup_ready_for_unlock(state: &LabState) -> bool {
    setup_ready_for_block_height(state) && state.profile.polar_block_height_confirmed
}

fn setup_has_user_nodes(state: &LabState) -> bool {
    DemoNodeId::ALL.into_iter().all(|node_id| {
        state.nodes.iter().any(|node| {
            node.node_id == node_id && node.status == NodeStatus::Online && node.pubkey.is_some()
        })
    })
}

fn setup_has_treasury_tras(state: &LabState) -> bool {
    let treasury_books = setup_verified_tra_count(state, DemoNodeId::GameTreasury, BOOK_ITEM_ID);
    let treasury_apples = setup_verified_tra_count(state, DemoNodeId::GameTreasury, APPLE_ITEM_ID);

    treasury_books >= 2 && treasury_apples >= 2
}

fn setup_has_npc_transfers(state: &LabState) -> bool {
    let bob_books = setup_verified_tra_count(state, DemoNodeId::Bob, BOOK_ITEM_ID);
    let carol_apples = setup_verified_tra_count(state, DemoNodeId::Carol, APPLE_ITEM_ID);
    let bob_book_transfers =
        setup_successful_npc_transfer_count(state, DemoNodeId::Bob, BOOK_ITEM_ID);
    let carol_apple_transfers =
        setup_successful_npc_transfer_count(state, DemoNodeId::Carol, APPLE_ITEM_ID);

    bob_books >= 2 && carol_apples >= 2 && bob_book_transfers >= 2 && carol_apple_transfers >= 2
}

fn setup_verified_tra_count(state: &LabState, owner_node: DemoNodeId, item_id: u32) -> usize {
    state
        .tra_items
        .iter()
        .filter(|item| {
            item.owner_node == owner_node
                && item.item_id == item_id
                && item.ownership_status == TraOwnershipStatus::Verified
        })
        .count()
}

fn setup_successful_npc_transfer_count(
    state: &LabState,
    destination: DemoNodeId,
    item_id: u32,
) -> usize {
    state
        .npc_item_transfers
        .iter()
        .filter(|transfer| {
            transfer.destination == destination
                && transfer.item_id == item_id
                && transfer.status == TraTransferStatus::Succeeded
        })
        .count()
}
