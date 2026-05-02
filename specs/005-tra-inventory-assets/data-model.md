# Data Model: TRA Inventory Assets

## GameItemDefinition

Represents a hardcoded game catalog entry keyed by `item_id`.

| Field | Type | Rules |
|-------|------|-------|
| `item_id` | `u32` | Stable game type identifier. `1` means Book and `2` means Apple. |
| `item_type` | `String` | Machine-friendly type label such as `book`. |
| `display_name` | `String` | Learner-facing item type label. |
| `cost_sats` | `u64` | Hardcoded trade price used for buy/sell invoices. |
| `visual_key` | `String` | Hardcoded key for selecting inventory art/CSS. |

Validation:

- Every supported `TraItem.item_id` must resolve to exactly one catalog entry.
- The initial supported catalog includes Book (`item_id=1`) and Apple (`item_id=2`).
- Cost and visuals come only from this catalog, not from TRA ownership state.

## TraItem

Represents one mock inventory instance backed by one TRA.

| Field | Type | Rules |
|-------|------|-------|
| `tra_id` | `String` | App-local stable ID for referencing the item in UI/service calls. |
| `asset_id` | `String` | Taproot Assets identity or mock local identity until real adapter is wired. |
| `unique_name` | `String` | Human-readable unique instance name, e.g. `Book`. Must be unique within the local lab. |
| `item_id` | `u32` | Game type ID carried by the TRA. `1` means Book. |
| `owner_node` | `DemoNodeId` | Current owner: Jack, Bob, or Carol. |
| `ownership_status` | `TraOwnershipStatus` | Verification state for the current owner. |
| `transfer_status` | `TraTransferStatus` | Latest transfer lifecycle state. |

Validation:

- One `TraItem` represents exactly one TRA.
- A node can own at most 3 verified or pending TRA items.
- Unsupported `item_id` values cannot be bought or sold.
- Two owners cannot both be shown as verified owners of the same `tra_id`.

## TraOwnershipStatus

| State | Meaning |
|-------|---------|
| `Verified` | The local lab confirms this owner controls the TRA item. |
| `Pending` | Transfer/proof is in progress and ownership is not final. |
| `Missing` | Saved item identity cannot be found in the local lab. |
| `Unsupported` | The item has missing, malformed, or unknown `item_id`. |
| `Failed` | Ownership verification failed and needs recovery. |

## TraTransferStatus

| State | Meaning |
|-------|---------|
| `None` | No transfer has been attempted for this item. |
| `Pending` | Transfer has started but ownership is not verified. |
| `Succeeded` | Transfer completed and ownership is verified. |
| `Failed` | Transfer failed or could not be verified. |

## MintTraRequest

| Field | Type | Rules |
|-------|------|-------|
| `owner_node` | `DemoNodeId` | Initial owner; must have fewer than 3 items. |
| `unique_name` | `String` | Must not collide case-insensitively with existing TRA item names. |
| `item_id` | `u32` | Must resolve to a supported `GameItemDefinition`. |

## TransferTraRequest

| Field | Type | Rules |
|-------|------|-------|
| `tra_id` | `String` | Existing item to transfer. |
| `from_node` | `DemoNodeId` | Must match current verified owner. |
| `to_node` | `DemoNodeId` | Must have fewer than 3 items. |

## GameInventorySlotView

Represents one visible inventory slot in Play Game. This is a UI view model, not a separate source of ownership.

| Field | Type | Rules |
|-------|------|-------|
| `slot_index` | `usize` | Position from `0` to `2` for each node inventory. |
| `tra_id` | `Option<String>` | Present only when the slot is occupied by a real `TraItem`. |
| `unique_name` | `Option<String>` | Copied from the backing `TraItem` for display. |
| `item_id` | `Option<u32>` | Copied from the backing `TraItem`; used to resolve catalog visuals and cost. |
| `visual_key` | `Option<String>` | Resolved from `GameItemDefinition` by `item_id`. |
| `owner_node` | `Option<DemoNodeId>` | Must match the side being rendered when occupied. |
| `ownership_status` | `Option<TraOwnershipStatus>` | Must come from the backing `TraItem`. |
| `transfer_status` | `Option<TraTransferStatus>` | Must come from the backing `TraItem`. |

Validation:

- Occupied slots must be derived from `LabState.tra_items`, not from `recent_payments`, hardcoded book counts, or animation state.
- Item images must be selected by resolving `item_id` through `GameItemDefinition`.
- Buy and sell actions must pass the selected slot's `tra_id` into `TransferTraRequest`.
- Empty slots carry no `tra_id` and cannot be used as buy/sell transfer targets.

## LabState Extension

`LabState` gains `tra_items: Vec<TraItem>` as the non-sensitive inventory ownership snapshot.

Rules:

- Browser snapshots may store `tra_id`, `asset_id`, `unique_name`, `item_id`, owner, and status.
- Browser snapshots must not store Taproot Assets private keys, macaroons, wallet seeds, or sensitive proof material.
- On app restart, saved TRA items must be reverified or marked for recovery before trading is enabled.

## State Transitions

### Setup Mint/Discover

1. No item exists.
2. Setup verifies TRA support.
3. Setup mints or discovers item identity.
4. Setup assigns owner.
5. Item becomes `Verified` or enters `Missing`/`Unsupported`/`Failed`.

### Buy

1. NPC slot contains a concrete `TraItem` with `Verified` ownership; player has capacity; trade is active.
2. Player pays NPC invoice for the selected item's catalog price.
3. TRA transfer starts from NPC to player using the selected `tra_id`.
4. Ownership verification succeeds: owner becomes player, status `Verified`, transfer `Succeeded`.
5. If payment succeeds but TRA verification fails: item remains recoverable with `Pending` or `Failed`, and duplicate transfer is disabled.

### Sell

1. Player slot contains a concrete `TraItem` with `Verified` ownership; NPC has capacity; trade is active.
2. NPC pays player invoice for the selected item's catalog price.
3. TRA transfer starts from player to NPC using the selected `tra_id`.
4. Ownership verification succeeds: owner becomes NPC, status `Verified`, transfer `Succeeded`.
5. If payment succeeds but TRA verification fails: item remains recoverable with `Pending` or `Failed`, and duplicate transfer is disabled.
