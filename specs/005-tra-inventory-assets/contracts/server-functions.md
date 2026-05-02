# Contracts: TRA Inventory Assets

The project exposes TRA operations through the same internal async service layer used by existing Lightning lab actions: `packages/ui/src/client/services/lightning_server_functions.rs`. Domain behavior lives in `packages/lightning-service/src/client/tra_service.rs`. Real Taproot Assets calls will be hidden behind a server-only adapter and must not be called directly from browser UI.

## Shared DTOs

Defined in `lightning_service` and re-exported by `ui::client::models`.

```rust
pub struct GameItemDefinition {
    pub item_id: u32,
    pub item_type: String,
    pub display_name: String,
    pub cost_sats: u64,
    pub visual_key: String,
}

pub struct TraItem {
    pub tra_id: String,
    pub asset_id: String,
    pub unique_name: String,
    pub item_id: u32,
    pub owner_node: DemoNodeId,
    pub ownership_status: TraOwnershipStatus,
    pub transfer_status: TraTransferStatus,
}

pub struct MintTraRequest {
    pub owner_node: DemoNodeId,
    pub unique_name: String,
    pub item_id: u32,
}

pub struct TransferTraRequest {
    pub tra_id: String,
    pub from_node: DemoNodeId,
    pub to_node: DemoNodeId,
}
```

## Catalog

```rust
pub async fn get_tra_item_catalog() -> Result<Vec<GameItemDefinition>, String>
```

Behavior:

- Returns supported game item definitions.
- Must include `item_id=1` mapped to Book and `item_id=2` mapped to Apple.
- Must not require Polar to be running.

Errors:

- None expected for current hardcoded catalog; preserve `Result` for API consistency.

## Verify TRA Setup

```rust
pub async fn verify_tra_setup(profile: SetupProfile) -> Result<LabState, String>
```

Behavior:

- Loads current lab state.
- Verifies setup is connected.
- Verifies known TRA items against the game catalog.
- Future real adapter must verify local Taproot Assets capability before mint/transfer.
- Saves updated lab snapshot.

Errors:

- Setup incomplete.
- Local lab or future Taproot Assets adapter unavailable.
- Unsupported item IDs are reflected in item status unless setup cannot proceed.

## Reset TRA Inventory

```rust
pub async fn reset_tra_inventory(profile: SetupProfile) -> Result<LabState, String>
```

Behavior:

- Loads current lab state.
- Clears the app's saved TRA inventory snapshot before setup items are recreated.
- Abandons prior setup item identities for gameplay purposes; a future real adapter may reconcile or leave old local-lab Taproot Assets in place, but the app must not treat those stale identities as valid game inventory unless setup rediscovers and verifies them.
- Must run when the learner submits `Add Tap Root Assets` after resetting back to that step.
- Saves the cleared lab snapshot before new items are minted or discovered.

Errors:

- Setup incomplete.
- Local lab or future Taproot Assets adapter unavailable when reset needs external cleanup or reconciliation.

## Mint TRA

```rust
pub async fn mint_tra(
    profile: SetupProfile,
    request: MintTraRequest,
) -> Result<LabState, String>
```

Behavior:

- Requires connected setup.
- Requires `request.item_id` to resolve to a game catalog entry.
- Requires `request.unique_name` to be unique in the local lab.
- Requires `request.owner_node` to own fewer than 3 items.
- Creates or discovers one TRA-backed item and assigns ownership to the owner.
- Saves updated lab snapshot.

Errors:

- Setup incomplete.
- Unsupported item type.
- Duplicate item name.
- Owner inventory full.
- Future real adapter mint/discovery failure.

## Transfer TRA

```rust
pub async fn transfer_tra(
    profile: SetupProfile,
    request: TransferTraRequest,
) -> Result<LabState, String>
```

Behavior:

- Requires connected setup.
- Requires `request.tra_id` to exist.
- Requires `request.from_node` to match current verified owner.
- Requires `request.to_node` to own fewer than 3 items.
- Requires item type to be supported by the game catalog.
- Transfers ownership, verifies ownership, and saves updated lab snapshot.

Errors:

- Setup incomplete.
- Item unavailable.
- Owner mismatch.
- Recipient inventory full.
- Unsupported item type.
- Future real adapter transfer/proof failure.

## Buy/Sell Integration Contract

Buy item flow:

1. Select a concrete NPC-owned `TraItem` from `LabState.tra_items`.
2. Reject the action unless the selected item is `Verified`, has a supported `item_id`, and the player has an empty inventory slot.
3. Resolve the item price and visuals from the game catalog using the selected item's `item_id`.
4. Create/pay Lightning invoice from NPC for the selected item's catalog price.
5. Call `transfer_tra` from NPC to player with the selected `tra_id`.
6. Only finalize visible inventory after verified transfer.

Sell item flow:

1. Select a concrete Jack-owned `TraItem` from `LabState.tra_items`.
2. Reject the action unless the selected item is `Verified`, has a supported `item_id`, and the NPC has an empty inventory slot.
3. Resolve the item price and visuals from the game catalog using the selected item's `item_id`.
4. Create/pay Lightning invoice from player for the selected item's catalog price.
5. Call `transfer_tra` from player to NPC with the selected `tra_id`.
6. Only finalize visible inventory after verified transfer.

Play Game rendering contract:

- Inventory image slots must be derived from `LabState.tra_items`.
- The item image is selected by `GameItemDefinition.visual_key`, resolved from the backing `TraItem.item_id`.
- The visible inventory slot shows only the catalog icon and display name; item ID and verification status belong in Network Dashboard details, not the compact game inventory.
- A visible item slot must retain the backing `tra_id`; buy/sell must never infer item ownership from payment counts.
- Existing payment history may be shown in the game log, but it is not an ownership source.

Partial completion:

- If payment succeeds but TRA transfer cannot be verified, item status becomes recoverable `Pending` or `Failed`.
- UI must show the partial state and disable duplicate transfer attempts for that item until recovery.
