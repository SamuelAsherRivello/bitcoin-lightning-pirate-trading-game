use crate::client::models::{
    BlockWaitReason, DemoNodeId, LabState, SetupProfile, DEFAULT_ROUTE_CAPACITY_SATS,
};
use crate::client::services::{polar_bridge_service, storage_service};

pub use polar_bridge_service::{PolarServerEnsureResult, PolarServerEnsureStatus};

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
    lightning_service::validate_setup_profile(&profile).map_err(|error| error.to_string())?;
    let mut profile = profile;
    profile.polar_automation =
        polar_bridge_service::create_demo_nodes(&profile.polar_automation).await?;

    let mut state = lightning_service::test_setup(profile).map_err(|error| error.to_string())?;
    state.profile.connection_status = lightning_service::ConnectionStatus::PartiallyConnected;
    let state = refresh_polar_block_height(state).await?;
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
    profile.last_verified_at = Some(chrono::Utc::now());

    let mut state = storage_service::load_lab_state_snapshot()
        .unwrap_or_else(|| lightning_service::default_lab_state(profile.clone()));
    state.profile = profile;
    let state = refresh_polar_block_height(state).await?;

    storage_service::save_setup_profile(&state.profile);
    storage_service::save_lab_state_snapshot(&state);
    Ok(state)
}

pub async fn get_lab_state(profile: SetupProfile) -> Result<LabState, String> {
    if !profile.is_connected() {
        return Ok(lightning_service::default_lab_state(profile));
    }

    let state = storage_service::load_lab_state_snapshot()
        .filter(|state| state.profile.is_connected())
        .unwrap_or_else(|| lightning_service::default_lab_state(profile.clone()));

    let state = if state.profile.sats_per_transaction == profile.sats_per_transaction
        && state.profile.setup_mode == profile.setup_mode
    {
        state
    } else {
        lightning_service::default_lab_state(profile)
    };

    refresh_polar_block_height(state).await
}

pub async fn open_trade_route(
    profile: SetupProfile,
    to_node: DemoNodeId,
) -> Result<LabState, String> {
    let state = get_lab_state(profile).await?;
    let state = lightning_service::open_trade_route(
        state,
        DemoNodeId::Alice,
        to_node,
        DEFAULT_ROUTE_CAPACITY_SATS,
    )
    .map_err(|error| error.to_string())?;
    storage_service::save_lab_state_snapshot(&state);
    Ok(state)
}

pub async fn wait_for_next_block(
    profile: SetupProfile,
    affected_route_id: Option<String>,
) -> Result<LabState, String> {
    let state = get_lab_state(profile).await?;
    let polar_height = if should_read_polar(&state.profile) {
        Some(polar_bridge_service::mine_blocks(&state.profile.polar_automation, 1).await?)
    } else {
        None
    };
    let state = lightning_service::wait_for_next_block(
        state,
        BlockWaitReason::ChannelOpenConfirmation,
        affected_route_id,
    )
    .map_err(|error| error.to_string())?;
    let mut state = state;
    if let Some(height) = polar_height {
        state.block_height = height;
        if let Some(action) = state.block_actions.last_mut() {
            action.resulting_height = Some(height);
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

pub async fn reset_lab() -> Result<SetupProfile, String> {
    storage_service::clear_setup_profile();
    storage_service::clear_lab_state_snapshot();
    Ok(SetupProfile::default())
}

async fn refresh_polar_block_height(mut state: LabState) -> Result<LabState, String> {
    if should_read_polar(&state.profile) {
        state.block_height =
            polar_bridge_service::get_blockchain_height(&state.profile.polar_automation).await?;
    }

    Ok(state)
}

fn should_read_polar(profile: &SetupProfile) -> bool {
    profile.setup_mode == lightning_service::SetupMode::ServerConfig
        && profile.polar_automation.is_complete()
}
