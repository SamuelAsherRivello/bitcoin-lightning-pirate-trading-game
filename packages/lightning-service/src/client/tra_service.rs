use chrono::Utc;

use super::error::LightningError;
use super::models::{
    ActionLogEntry, ConnectionStatus, DemoNodeId, GameItemDefinition, LabState, MintTraRequest,
    TraItem, TraOwnershipStatus, TraTransferStatus, TransferTraRequest, APPLE_ITEM_ID,
    BOOK_ITEM_ID, DEFAULT_SATS_PER_TRANSACTION, MAX_TRA_ITEMS_PER_NODE,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConnectionStatus, SetupProfile};

    fn connected_state() -> LabState {
        let mut profile = SetupProfile::default();
        profile.connection_status = ConnectionStatus::Connected;
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
