use chrono::Utc;

use super::error::LightningError;
use super::models::{
    ActionLogEntry, ConnectionStatus, DemoNodeId, GameItemDefinition, LabState, MintTraRequest,
    NpcItemTransfer, TraItem, TraOwnershipStatus, TraTransferStatus, TransferTraRequest,
    TreasuryEntry, TreasuryEntryDirection, TreasuryImpactPreview, TreasuryResource, TreasuryStatus,
    APPLE_ITEM_ID, BOOK_ITEM_ID, DEFAULT_ROUTE_CAPACITY_SATS, DEFAULT_SATS_PER_TRANSACTION,
    GAME_TREASURY_NODE_LABEL, MAX_TRA_ITEMS_PER_NODE,
};

pub struct TraService;

impl TraService {
    pub fn item_catalog() -> Vec<GameItemDefinition> {
        vec![
            GameItemDefinition {
                item_id: BOOK_ITEM_ID,
                item_type: "book".to_string(),
                display_name: "Book".to_string(),
                cost_sats: DEFAULT_SATS_PER_TRANSACTION,
                visual_key: "book".to_string(),
            },
            GameItemDefinition {
                item_id: APPLE_ITEM_ID,
                item_type: "apple".to_string(),
                display_name: "Apple".to_string(),
                cost_sats: DEFAULT_SATS_PER_TRANSACTION,
                visual_key: "apple".to_string(),
            },
        ]
    }

    pub fn initial_setup_items() -> Vec<MintTraRequest> {
        vec![
            MintTraRequest {
                owner_node: DemoNodeId::Bob,
                unique_name: "Book".to_string(),
                item_id: BOOK_ITEM_ID,
            },
            MintTraRequest {
                owner_node: DemoNodeId::Bob,
                unique_name: "Book 2".to_string(),
                item_id: BOOK_ITEM_ID,
            },
            MintTraRequest {
                owner_node: DemoNodeId::Carol,
                unique_name: "Apple".to_string(),
                item_id: APPLE_ITEM_ID,
            },
            MintTraRequest {
                owner_node: DemoNodeId::Carol,
                unique_name: "Apple 2".to_string(),
                item_id: APPLE_ITEM_ID,
            },
        ]
    }

    pub fn prepare_game_treasury(mut state: LabState) -> Result<LabState, LightningError> {
        ensure_setup_started(&state)?;

        let funded_sats = state
            .profile
            .sats_per_transaction
            .saturating_mul(10)
            .max(DEFAULT_ROUTE_CAPACITY_SATS);
        state.profile.game_treasury_ready = true;
        state.profile.game_treasury_funded_sats = funded_sats;
        state.game_treasury.node_label = GAME_TREASURY_NODE_LABEL.to_string();
        state.game_treasury.status = TreasuryStatus::Ready;
        state.game_treasury.spendable_sats = funded_sats;
        state.game_treasury.last_updated_at = Some(Utc::now());
        crate::upsert_game_treasury_node(&mut state, funded_sats);

        push_treasury_entry(
            &mut state,
            "Game Treasury funded for local game activity.",
            TreasuryEntryDirection::Increase,
            Some(funded_sats),
            None,
            None,
            "Polar setup: Game Treasury",
        )?;
        push_log(
            &mut state,
            "Game Treasury ready",
            "The house node is funded and ready for treasury-owned TRA setup.",
            &["Game Treasury"],
        );

        Ok(state)
    }

    pub fn prepare_game_treasury_items(mut state: LabState) -> Result<LabState, LightningError> {
        ensure_game_treasury_ready(&state)?;
        state.game_treasury.status = TreasuryStatus::CreatingItems;
        state.game_treasury.owned_items.clear();
        state
            .tra_items
            .retain(|item| item.owner_node != DemoNodeId::GameTreasury);
        for request in Self::initial_setup_items() {
            validate_supported_item_id(request.item_id)?;
            let definition = Self::catalog_item(request.item_id)
                .ok_or(LightningError::UnsupportedTraItemType)?;
            let tra_id = format!("tra-treasury-{}", state.tra_items.len() + 1);
            state.tra_items.push(TraItem {
                asset_id: format!("regtest-treasury-asset-{}", state.tra_items.len() + 1),
                tra_id: tra_id.clone(),
                unique_name: request.unique_name.clone(),
                item_id: request.item_id,
                owner_node: DemoNodeId::GameTreasury,
                ownership_status: TraOwnershipStatus::Verified,
                transfer_status: TraTransferStatus::None,
            });
            state.game_treasury.owned_items.push(TreasuryResource {
                resource_id: tra_id,
                resource_type: "Item".to_string(),
                display_name: request.unique_name,
                item_id: Some(request.item_id),
                owner: GAME_TREASURY_NODE_LABEL.to_string(),
                estimated_value_sats: Some(definition.cost_sats),
            });
        }
        state.game_treasury.inventory_value_sats = state
            .game_treasury
            .owned_items
            .iter()
            .filter_map(|item| item.estimated_value_sats)
            .sum();
        state.game_treasury.status = TreasuryStatus::Ready;
        state.game_treasury.last_updated_at = Some(Utc::now());
        push_treasury_entry(
            &mut state,
            "Game Treasury prepared the starting items for Bob and Carol.",
            TreasuryEntryDirection::NoChange,
            None,
            None,
            None,
            "Polar setup: Treasury items",
        )?;

        Ok(state)
    }

    pub fn transfer_npc_starting_items(mut state: LabState) -> Result<LabState, LightningError> {
        ensure_connected(&state)?;
        ensure_game_treasury_ready(&state)?;

        let requests = Self::initial_setup_items();
        for (index, request) in requests.into_iter().enumerate() {
            let definition = Self::catalog_item(request.item_id)
                .ok_or(LightningError::UnsupportedTraItemType)?;
            let tra_id = state
                .tra_items
                .iter()
                .find(|item| {
                    item.owner_node == DemoNodeId::GameTreasury
                        && item.item_id == request.item_id
                        && item.unique_name == request.unique_name
                })
                .map(|item| item.tra_id.clone())
                .ok_or(LightningError::GameTreasuryItemUnavailable)?;
            state = Self::transfer_tra(
                state,
                TransferTraRequest {
                    tra_id,
                    from_node: DemoNodeId::GameTreasury,
                    to_node: request.owner_node,
                },
            )?;
            let entry_id = push_treasury_entry(
                &mut state,
                &format!(
                    "Game Treasury transferred {} to {}.",
                    request.unique_name,
                    request.owner_node.label()
                ),
                TreasuryEntryDirection::TransferOut,
                None,
                Some(request.item_id),
                Some(request.unique_name.clone()),
                "Polar setup: User Nodes (TRAs)",
            )?;
            state.npc_item_transfers.push(NpcItemTransfer {
                transfer_id: format!("npc-transfer-{}", index + 1),
                item_id: request.item_id,
                item_name: definition.display_name,
                source: GAME_TREASURY_NODE_LABEL.to_string(),
                destination: request.owner_node,
                status: TraTransferStatus::Succeeded,
                entry_id: Some(entry_id),
            });
        }
        state.game_treasury.owned_items.clear();
        state.game_treasury.inventory_value_sats = 0;
        state.game_treasury.status = TreasuryStatus::Ready;
        state.game_treasury.last_updated_at = Some(Utc::now());

        Ok(state)
    }

    pub fn treasury_summary(state: &LabState) -> super::models::GameTreasury {
        let mut treasury = state.game_treasury.clone();
        treasury.recent_entries = state
            .game_treasury
            .recent_entries
            .iter()
            .take(10)
            .cloned()
            .collect();
        treasury
    }

    pub fn preview_treasury_impact(
        state: &LabState,
        action_label: String,
        amount_sats: Option<u64>,
    ) -> TreasuryImpactPreview {
        let missing_treasury = !state.profile.game_treasury_ready;
        let insufficient_sats = amount_sats
            .map(|amount| state.game_treasury.spendable_sats < amount)
            .unwrap_or(false);
        let can_execute = !missing_treasury && !insufficient_sats;
        let blocking_reason = if missing_treasury {
            Some("Game Treasury is not ready yet.".to_string())
        } else if insufficient_sats {
            Some("Game Treasury does not have enough sats for this action.".to_string())
        } else {
            None
        };

        TreasuryImpactPreview {
            action_label,
            can_execute,
            blocking_reason,
            expected_sats_delta: amount_sats.map(|amount| -(amount as i64)),
            expected_item_movements: Vec::new(),
            requires_refresh: state.game_treasury.status != TreasuryStatus::Ready,
        }
    }

    pub fn verify_tra_setup(mut state: LabState) -> Result<LabState, LightningError> {
        ensure_connected(&state)?;

        for item in &mut state.tra_items {
            item.ownership_status = if Self::is_supported_item_id(item.item_id) {
                TraOwnershipStatus::Verified
            } else {
                TraOwnershipStatus::Unsupported
            };
        }

        Ok(state)
    }

    pub fn reset_tra_inventory(mut state: LabState) -> Result<LabState, LightningError> {
        ensure_connected(&state)?;

        state.tra_items.clear();
        push_log(
            &mut state,
            "Reset TRA inventory",
            "The app cleared the local TRA inventory snapshot before recreating setup items.",
            &["TRA Inventory Reset"],
        );

        Ok(state)
    }

    pub fn mint_tra(
        mut state: LabState,
        request: MintTraRequest,
    ) -> Result<LabState, LightningError> {
        ensure_connected(&state)?;
        validate_supported_item_id(request.item_id)?;
        validate_unique_name(&state, &request.unique_name)?;
        ensure_inventory_capacity(&state, request.owner_node)?;

        let tra_id = format!("tra-{}", state.tra_items.len() + 1);
        let item = TraItem {
            asset_id: format!("regtest-asset-{}", state.tra_items.len() + 1),
            tra_id,
            unique_name: request.unique_name,
            item_id: request.item_id,
            owner_node: request.owner_node,
            ownership_status: TraOwnershipStatus::Verified,
            transfer_status: TraTransferStatus::None,
        };

        let summary = format!("Minted TRA {}", item.unique_name);
        let detail = format!(
            "{} now owns TRA item {} with item_id={}.",
            item.owner_node.label(),
            item.unique_name,
            item.item_id
        );
        state.tra_items.push(item);
        push_log(&mut state, &summary, &detail, &["TRA Minted"]);

        Ok(state)
    }

    pub fn transfer_tra(
        mut state: LabState,
        request: TransferTraRequest,
    ) -> Result<LabState, LightningError> {
        ensure_connected(&state)?;
        ensure_inventory_capacity(&state, request.to_node)?;

        let item = state
            .tra_items
            .iter_mut()
            .find(|item| item.tra_id == request.tra_id)
            .ok_or(LightningError::TraItemUnavailable)?;

        if item.owner_node != request.from_node {
            return Err(LightningError::TraOwnerMismatch);
        }

        validate_supported_item_id(item.item_id)?;

        item.owner_node = request.to_node;
        item.ownership_status = TraOwnershipStatus::Verified;
        item.transfer_status = TraTransferStatus::Succeeded;

        let summary = format!("Transferred TRA {}", item.unique_name);
        let detail = format!(
            "{} transferred {} to {}. The game catalog resolves item_id={} for cost and visuals.",
            request.from_node.label(),
            item.unique_name,
            request.to_node.label(),
            item.item_id
        );
        push_log(&mut state, &summary, &detail, &["TRA Transferred"]);

        Ok(state)
    }

    pub fn owner_inventory(state: &LabState, owner_node: DemoNodeId) -> Vec<TraItem> {
        state
            .tra_items
            .iter()
            .filter(|item| item.owner_node == owner_node)
            .cloned()
            .collect()
    }

    pub fn catalog_item(item_id: u32) -> Option<GameItemDefinition> {
        Self::item_catalog()
            .into_iter()
            .find(|item| item.item_id == item_id)
    }

    fn is_supported_item_id(item_id: u32) -> bool {
        Self::catalog_item(item_id).is_some()
    }
}

fn ensure_connected(state: &LabState) -> Result<(), LightningError> {
    if matches!(
        state.profile.connection_status,
        ConnectionStatus::Connected | ConnectionStatus::PartiallyConnected
    ) {
        Ok(())
    } else {
        Err(LightningError::SetupIncomplete)
    }
}

fn ensure_setup_started(state: &LabState) -> Result<(), LightningError> {
    if matches!(
        state.profile.connection_status,
        ConnectionStatus::SavedOffline
            | ConnectionStatus::PartiallyConnected
            | ConnectionStatus::Connected
    ) && !state.profile.polar_automation.network_id.trim().is_empty()
    {
        Ok(())
    } else {
        Err(LightningError::SetupIncomplete)
    }
}

fn ensure_game_treasury_ready(state: &LabState) -> Result<(), LightningError> {
    if state.profile.game_treasury_ready && state.game_treasury.status == TreasuryStatus::Ready {
        Ok(())
    } else {
        Err(LightningError::GameTreasuryNotReady)
    }
}

fn validate_supported_item_id(item_id: u32) -> Result<(), LightningError> {
    if TraService::is_supported_item_id(item_id) {
        Ok(())
    } else {
        Err(LightningError::UnsupportedTraItemType)
    }
}

fn validate_unique_name(state: &LabState, unique_name: &str) -> Result<(), LightningError> {
    if state
        .tra_items
        .iter()
        .any(|item| item.unique_name.eq_ignore_ascii_case(unique_name))
    {
        return Err(LightningError::DuplicateTraItemName);
    }

    Ok(())
}

fn ensure_inventory_capacity(
    state: &LabState,
    owner_node: DemoNodeId,
) -> Result<(), LightningError> {
    if owner_node == DemoNodeId::GameTreasury {
        return Ok(());
    }

    let item_count = state
        .tra_items
        .iter()
        .filter(|item| item.owner_node == owner_node)
        .count();

    if item_count >= MAX_TRA_ITEMS_PER_NODE {
        return Err(LightningError::TraInventoryFull);
    }

    Ok(())
}

fn push_log(state: &mut LabState, summary: &str, network_detail: &str, details: &[&str]) {
    state.action_log.insert(
        0,
        ActionLogEntry {
            id: format!("log-{}", state.action_log.len() + 1),
            summary: summary.to_string(),
            network_detail: network_detail.to_string(),
            details: details.iter().map(|detail| (*detail).to_string()).collect(),
            created_at: Utc::now(),
        },
    );
}

fn push_treasury_entry(
    state: &mut LabState,
    description: &str,
    direction: TreasuryEntryDirection,
    amount_sats: Option<u64>,
    item_id: Option<u32>,
    item_name: Option<String>,
    related_action: &str,
) -> Result<String, LightningError> {
    if looks_sensitive(description) || looks_sensitive(related_action) {
        return Err(LightningError::SensitiveTreasuryDetail);
    }

    let entry_id = format!(
        "treasury-entry-{}",
        state.game_treasury.recent_entries.len() + 1
    );
    state.game_treasury.recent_entries.insert(
        0,
        TreasuryEntry {
            entry_id: entry_id.clone(),
            created_at: Utc::now(),
            description: description.to_string(),
            direction,
            amount_sats,
            item_id,
            item_name,
            source: Some(GAME_TREASURY_NODE_LABEL.to_string()),
            destination: None,
            related_action: related_action.to_string(),
        },
    );
    state.game_treasury.recent_entries.truncate(10);
    Ok(entry_id)
}

fn looks_sensitive(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    [
        "macaroon", "seed", "private", "xprv", "proof", "password", "secret",
    ]
    .iter()
    .any(|marker| value.contains(marker))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConnectionStatus, PolarAutomationProfile, SetupProfile};

    fn connected_state() -> LabState {
        let mut profile = SetupProfile::default();
        profile.connection_status = ConnectionStatus::Connected;
        crate::default_lab_state(profile)
    }

    fn setup_started_state() -> LabState {
        let mut profile = SetupProfile::default();
        profile.connection_status = ConnectionStatus::PartiallyConnected;
        profile.polar_automation = PolarAutomationProfile {
            bridge_url: "http://localhost:37373".to_string(),
            network_id: "1".to_string(),
            bitcoin_backend_name: crate::DEFAULT_BITCOIN_BACKEND_NAME.to_string(),
        };
        crate::default_lab_state(profile)
    }

    #[test]
    fn book_item_id_is_catalogued() {
        let book = TraService::catalog_item(BOOK_ITEM_ID).expect("book catalog entry");

        assert_eq!(book.display_name, "Book");
        assert_eq!(book.visual_key, "book");
    }

    #[test]
    fn apple_item_id_is_catalogued() {
        let apple = TraService::catalog_item(APPLE_ITEM_ID).expect("apple catalog entry");

        assert_eq!(apple.display_name, "Apple");
        assert_eq!(apple.visual_key, "apple");
    }

    #[test]
    fn initial_setup_items_assign_books_and_apples_to_different_npcs() {
        let items = TraService::initial_setup_items();

        assert_eq!(items.len(), 4);
        assert_eq!(
            items
                .iter()
                .filter(|item| item.owner_node == DemoNodeId::Bob && item.item_id == BOOK_ITEM_ID)
                .count(),
            2
        );
        assert_eq!(
            items
                .iter()
                .filter(|item| {
                    item.owner_node == DemoNodeId::Carol && item.item_id == APPLE_ITEM_ID
                })
                .count(),
            2
        );
    }

    #[test]
    fn prepare_game_treasury_funds_sats_without_creating_items() {
        let state =
            TraService::prepare_game_treasury(setup_started_state()).expect("prepare treasury");

        assert!(state.profile.game_treasury_ready);
        assert!(state.profile.game_treasury_funded_sats >= DEFAULT_ROUTE_CAPACITY_SATS);
        assert_eq!(state.game_treasury.node_label, GAME_TREASURY_NODE_LABEL);
        assert_eq!(
            state.game_treasury.spendable_sats,
            state.profile.game_treasury_funded_sats
        );
        assert!(state.game_treasury.owned_items.is_empty());
        assert!(state.tra_items.is_empty());
    }

    #[test]
    fn prepare_game_treasury_items_assigns_initial_items_to_treasury() {
        let state =
            TraService::prepare_game_treasury(setup_started_state()).expect("prepare treasury");
        let state = TraService::prepare_game_treasury_items(state).expect("prepare treasury items");

        assert_eq!(state.game_treasury.owned_items.len(), 4);
        assert!(state
            .game_treasury
            .owned_items
            .iter()
            .all(|item| item.owner == GAME_TREASURY_NODE_LABEL));
        assert_eq!(
            state
                .tra_items
                .iter()
                .filter(|item| item.owner_node == DemoNodeId::GameTreasury)
                .count(),
            4
        );
    }

    #[test]
    fn mint_tra_creates_verified_item_for_owner() {
        let state = TraService::mint_tra(
            connected_state(),
            MintTraRequest {
                owner_node: DemoNodeId::Bob,
                unique_name: "Book".to_string(),
                item_id: BOOK_ITEM_ID,
            },
        )
        .expect("mint TRA");

        assert_eq!(state.tra_items.len(), 1);
        assert_eq!(state.tra_items[0].owner_node, DemoNodeId::Bob);
        assert_eq!(
            state.tra_items[0].ownership_status,
            TraOwnershipStatus::Verified
        );
    }

    #[test]
    fn transfer_tra_moves_verified_owner() {
        let state = TraService::mint_tra(
            connected_state(),
            MintTraRequest {
                owner_node: DemoNodeId::Bob,
                unique_name: "Book".to_string(),
                item_id: BOOK_ITEM_ID,
            },
        )
        .expect("mint TRA");
        let tra_id = state.tra_items[0].tra_id.clone();

        let state = TraService::transfer_tra(
            state,
            TransferTraRequest {
                tra_id,
                from_node: DemoNodeId::Bob,
                to_node: DemoNodeId::Alice,
            },
        )
        .expect("transfer TRA");

        assert_eq!(state.tra_items[0].owner_node, DemoNodeId::Alice);
        assert_eq!(
            state.tra_items[0].transfer_status,
            TraTransferStatus::Succeeded
        );
    }

    #[test]
    fn reset_tra_inventory_clears_existing_items() {
        let state = TraService::mint_tra(
            connected_state(),
            MintTraRequest {
                owner_node: DemoNodeId::Bob,
                unique_name: "Book".to_string(),
                item_id: BOOK_ITEM_ID,
            },
        )
        .expect("mint TRA");

        let state = TraService::reset_tra_inventory(state).expect("reset TRA inventory");

        assert!(state.tra_items.is_empty());
        assert_eq!(state.action_log[0].summary, "Reset TRA inventory");
    }

    #[test]
    fn reset_then_recreate_uses_fresh_initial_items() {
        let state = TraService::mint_tra(
            connected_state(),
            MintTraRequest {
                owner_node: DemoNodeId::Bob,
                unique_name: "Book".to_string(),
                item_id: BOOK_ITEM_ID,
            },
        )
        .expect("mint TRA");
        let state = TraService::reset_tra_inventory(state).expect("reset TRA inventory");
        let state = TraService::mint_tra(
            state,
            MintTraRequest {
                owner_node: DemoNodeId::Carol,
                unique_name: "Book".to_string(),
                item_id: BOOK_ITEM_ID,
            },
        )
        .expect("recreate TRA");

        assert_eq!(state.tra_items.len(), 1);
        assert_eq!(state.tra_items[0].tra_id, "tra-1");
        assert_eq!(state.tra_items[0].owner_node, DemoNodeId::Carol);
    }

    #[test]
    fn transfer_tra_rejects_owner_mismatch() {
        let state = TraService::mint_tra(
            connected_state(),
            MintTraRequest {
                owner_node: DemoNodeId::Bob,
                unique_name: "Book".to_string(),
                item_id: BOOK_ITEM_ID,
            },
        )
        .expect("mint TRA");
        let tra_id = state.tra_items[0].tra_id.clone();

        let error = TraService::transfer_tra(
            state,
            TransferTraRequest {
                tra_id,
                from_node: DemoNodeId::Carol,
                to_node: DemoNodeId::Alice,
            },
        )
        .expect_err("owner mismatch should fail");

        assert!(matches!(error, LightningError::TraOwnerMismatch));
    }

    #[test]
    fn setup_can_prepare_tra_before_final_connected_status() {
        let mut profile = SetupProfile::default();
        profile.connection_status = ConnectionStatus::PartiallyConnected;
        let state = crate::default_lab_state(profile);

        let state = TraService::verify_tra_setup(state).expect("verify TRA setup");

        assert!(state.action_log.is_empty());
    }

    #[test]
    fn mint_tra_enforces_inventory_capacity() {
        let mut state = connected_state();
        for index in 1..=MAX_TRA_ITEMS_PER_NODE {
            state = TraService::mint_tra(
                state,
                MintTraRequest {
                    owner_node: DemoNodeId::Bob,
                    unique_name: format!("Book {index}"),
                    item_id: BOOK_ITEM_ID,
                },
            )
            .expect("mint TRA inside capacity");
        }

        let error = TraService::mint_tra(
            state,
            MintTraRequest {
                owner_node: DemoNodeId::Bob,
                unique_name: "Book 4".to_string(),
                item_id: BOOK_ITEM_ID,
            },
        )
        .expect_err("fourth item should fail");

        assert!(matches!(error, LightningError::TraInventoryFull));
    }
}
