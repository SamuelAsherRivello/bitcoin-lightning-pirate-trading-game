# Data Model: Game Treasury

## GameTreasury

Represents the game-bank summary shown to users.

**Fields**:

- `node_label`: Display name, always `GAME_TREASURY` for this feature.
- `status`: Current `TreasuryStatus`.
- `spendable_sats`: Current spendable game balance.
- `inventory_value_sats`: Estimated value of treasury-owned inventory.
- `owned_items`: Non-sensitive list of treasury-owned `TreasuryResource` items.
- `recent_entries`: Newest-first list of recent `TreasuryEntry` records.
- `last_updated_at`: Timestamp for the latest successful summary refresh.

**Validation rules**:

- `node_label` must be present and equal to the player-facing treasury name.
- Spendable and inventory values must not be negative.
- Failed or loading state must not be presented as a ready zero balance.

## TreasuryStatus

Represents readiness for setup and gameplay.

**States**:

- `NotStarted`: Treasury setup has not begun.
- `CreatingNode`: Treasury node creation or verification is running.
- `Funding`: Treasury funding is running.
- `CreatingItems`: Treasury-owned item creation or verification is running.
- `Ready`: Treasury is usable.
- `Refreshing`: Treasury summary is being refreshed.
- `Degraded`: Treasury exists but has a recoverable issue.
- `Failed`: Treasury setup or refresh failed and needs user action.

**State transitions**:

- `NotStarted` -> `CreatingNode` -> `Funding` -> `CreatingItems` -> `Ready`.
- Any active state may transition to `Failed` with a recoverable error.
- `Ready` may transition to `Refreshing`, then back to `Ready` or `Degraded`.

## TreasuryResource

Represents a spendable amount or item that contributes to treasury understanding.

**Fields**:

- `resource_id`: Stable non-sensitive identifier.
- `resource_type`: `Sats` or `Item`.
- `display_name`: Player-facing resource name.
- `item_id`: Optional game catalog item identifier for item resources.
- `owner`: Current owner label, such as Game Treasury, Bob, or Carol.
- `estimated_value_sats`: Optional displayed value.

**Validation rules**:

- Item resources must include an item identifier and owner.
- Sats resources must not include an item identifier.

## TreasuryEntry

Records a treasury-impacting event.

**Fields**:

- `entry_id`: Stable non-sensitive identifier.
- `created_at`: Event time.
- `description`: Plain-language explanation.
- `direction`: `Increase`, `Decrease`, `TransferOut`, `TransferIn`, or `NoChange`.
- `amount_sats`: Optional sats amount.
- `item_id`: Optional item identifier.
- `item_name`: Optional item display name.
- `source`: Optional source participant.
- `destination`: Optional destination participant.
- `related_action`: Setup or gameplay action that caused the entry.

**Validation rules**:

- Each entry must include either an amount, an item, or an explicit no-change reason.
- Source and destination are required for item transfers.
- Descriptions must avoid technical credentials, wallet secrets, and transport details.

## TreasuryImpactPreview

Explains a pending gameplay action before the player commits.

**Fields**:

- `action_label`: Player-facing action name.
- `can_execute`: Whether the treasury can support the action.
- `blocking_reason`: Optional visible reason when blocked.
- `expected_sats_delta`: Optional sats change.
- `expected_item_movements`: List of expected item source/destination movements.
- `requires_refresh`: Whether treasury state must be refreshed first.

**Validation rules**:

- If `can_execute` is false, `blocking_reason` must be present.
- If `requires_refresh` is true, the action must not commit until refreshed or explicitly retried.

## SetupStep

Represents the ordered setup flow.

**Values**:

1. Bridge URL
2. Server Name
3. Game Treasury
4. User Nodes
5. NPC Item Transfers
6. Block Height
7. Unlock Routes

**Validation rules**:

- User Nodes cannot complete before Game Treasury is ready.
- NPC Item Transfers cannot complete before User Nodes exist.
- Block Height cannot proceed until NPC item transfer state is complete or explicitly recoverable.

## NpcItemTransfer

Represents a setup-time movement of a starting item from Game Treasury to Bob or Carol.

**Fields**:

- `transfer_id`: Stable non-sensitive identifier.
- `item_id`: Game catalog item identifier.
- `item_name`: Player-facing item name.
- `source`: Always Game Treasury for this feature.
- `destination`: Bob or Carol.
- `status`: Pending, transferring, complete, failed, or needs retry.
- `entry_id`: Optional linked treasury entry.

**Validation rules**:

- Source must be Game Treasury.
- Destination must be a configured NPC for v1.
- Completed transfer must have a linked ownership summary and treasury entry.
