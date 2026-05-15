# Feature Specification: TRA Inventory Assets

**Feature Branch**: `[005-tra-inventory-assets]`  
**Created**: 2026-05-14  
**Status**: Draft  
**Input**: User description: "the game has 3 inventory items max per node. These items are mock. Now create a solution that uses taproot and adds that to polar during polar setup. Choose the appropriate step or add new step. verify proper setup before continuing the step. then give items to some NPC's. Then when buying an item or selling the item transfer ownership. First do research on taproot (let's call them tap root assets "TRA") so there is one tra per inventory item. Make each of them a unique name. like "Book" for one of them. Each TRA needs to have some data in it to express what type it is, for example item_id=1 means Book. The cost and visuals are hardcoded in the game based on that item_id."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Prepare TRA Inventory During Setup (Priority: P1)

As a learner, I can complete Polar setup and see a dedicated Taproot Assets inventory step prepare the game's mock items before Play Game unlocks, so item trading has a visible ownership backing instead of being only local UI state.

**Why this priority**: The game cannot truthfully transfer inventory ownership until the local lab proves Taproot Assets are available and each mock item has a distinct asset record.

**Independent Test**: Can be tested by running setup from a fresh local lab and confirming the TRA step verifies the environment, creates or discovers the required item assets, assigns them to NPC owners, and blocks progression when verification fails.

**Acceptance Scenarios**:

1. **Given** Polar setup has connected and funded the local lab nodes, **When** the learner reaches the "Add Tap Root Assets" step, **Then** the app verifies Taproot Assets support before creating or assigning inventory items.
2. **Given** Taproot Assets support cannot be verified, **When** the learner tries to continue, **Then** setup remains on the TRA inventory step with visible status feedback and a clear recovery message.
3. **Given** verification succeeds, **When** the TRA inventory step completes, **Then** each NPC receives no more than 3 mock inventory items and each item is represented by one unique TRA.
4. **Given** the learner resets back to "Add Tap Root Assets", **When** the learner submits that step again, **Then** the app recreates the initial TRA inventory from scratch instead of reusing the previous setup items.

---

### User Story 2 - See Unique Mock Items Owned By NPCs (Priority: P1)

As a learner, I can view NPC inventories containing uniquely named mock items, so I know which specific item can be bought, sold, or transferred.

**Why this priority**: Unique names make ownership transfer understandable and testable, especially when multiple NPCs hold different items.

**Independent Test**: Can be tested by opening Play Game after setup and confirming every visible inventory slot maps to a unique TRA-backed mock item name.

**Acceptance Scenarios**:

1. **Given** TRA setup has completed, **When** the learner opens Play Game, **Then** NPC inventories show up to 3 item slots per node with unique names such as "Book".
2. **Given** an inventory item is shown, **When** the learner inspects the item details, **Then** the UI identifies the current owner, the unique item name, the item type ID carried by the TRA, and the TRA-backed ownership status.
3. **Given** a node already has 3 items, **When** another item would be assigned to that node, **Then** the app prevents the extra assignment and reports that the node inventory is full.
4. **Given** a TRA carries `item_id=1`, **When** the item is displayed or priced in the game, **Then** the app treats it as a Book and applies the game catalog's hardcoded Book cost and visuals.
5. **Given** TRA setup has completed, **When** the learner opens Play Game or refreshes the browser on Play Game, **Then** the app refreshes current sat balances and verifies TRA inventory before item trading is enabled.

---

### User Story 3 - Transfer Ownership When Buying Or Selling (Priority: P1)

As a learner, I can buy an item from an NPC or sell an item to an NPC and see ownership transfer with the payment, so the game demonstrates both Lightning payment flow and TRA item ownership flow.

**Why this priority**: The core learning moment is that payment and item ownership are separate but coordinated transfers.

**Independent Test**: Can be tested by completing setup, opening a trade with an NPC, buying one item, then selling it back or selling another owned item, and confirming both visible inventories and TRA ownership status change.

**Acceptance Scenarios**:

1. **Given** the player has an active trade with an NPC, the NPC owns an item, and the player has an empty inventory slot, **When** the learner buys the item, **Then** payment completes and the item's ownership changes from the NPC to the player.
2. **Given** the player owns an item and the current NPC has enough available sats plus an empty inventory slot, **When** the learner sells the item, **Then** payment completes and the item's ownership changes from the player to the NPC.
3. **Given** payment succeeds but item ownership transfer cannot be verified, **When** the trade result is shown, **Then** the app clearly reports the partial completion and keeps the item in a recoverable pending state instead of silently changing ownership.
4. **Given** multiple transferable items are visible, **When** the learner buys from an NPC or sells from the player, **Then** the app selects the rightmost transferable item from that owner's inventory.

---

### User Story 4 - Recover From Stale Or Missing TRA State (Priority: P2)

As a learner, I can rerun setup verification after restarting Polar or the app, so the game can resync known mock item ownership before allowing more trades.

**Why this priority**: Local regtest labs are frequently restarted, and stale ownership state would make the inventory lesson confusing.

**Independent Test**: Can be tested by completing setup, restarting the app, and confirming the TRA inventory step or Play Game verifies existing item ownership before enabling buy or sell actions.

**Acceptance Scenarios**:

1. **Given** TRA item records already exist, **When** setup verification runs again, **Then** the app reuses the existing records instead of creating duplicate items with the same name.
2. **Given** a saved item record cannot be found in the local lab, **When** verification runs, **Then** the app marks the item as missing and blocks trading that item until setup repairs or recreates the mock inventory.

### Edge Cases

- If a node already holds 3 inventory items, additional buys or assignments to that node are unavailable.
- If the player has no empty inventory slot, buying is unavailable even if the player can pay.
- If the NPC has no empty inventory slot, selling to that NPC is unavailable even if the NPC can pay.
- If a TRA item name collides with an existing game item, setup must choose a different unique name rather than overwriting ownership.
- If a TRA has a missing, malformed, or unknown `item_id`, the item must be shown as unsupported and cannot be bought or sold until repaired by setup.
- If the local lab supports Lightning payments but not Taproot Assets, Play Game must remain available only for non-TRA behavior when explicitly allowed by the current feature gate; otherwise the TRA step blocks item trading.
- If the ownership transfer is pending confirmation or proof delivery, the UI must show the item as pending and disable repeat transfer of that item.
- If setup is rerun after a Polar reset, stale local item ownership must not be treated as valid until verified against the local lab.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST define inventory capacity as a maximum of 3 items per node, including the player and every NPC.
- **FR-002**: The system MUST treat inventory items as mock educational items with no real-world value or production wallet meaning.
- **FR-003**: The system MUST represent each inventory item with exactly one TRA in the local lab.
- **FR-004**: The system MUST give every TRA-backed inventory item a unique human-readable name; the initial item set MUST include an item named "Book".
- **FR-005**: Each TRA-backed inventory item MUST include game-readable type data containing an `item_id`.
- **FR-006**: The game item catalog MUST map `item_id=1` to the Book item type.
- **FR-006A**: The game item catalog MUST map `item_id=2` to the Apple item type.
- **FR-007**: Item cost and visual presentation MUST be determined by the hardcoded game item catalog for the item's `item_id`, not by mutable TRA ownership state.
- **FR-008**: Setup MUST reject or flag TRA-backed items whose `item_id` is missing, malformed, duplicated in an invalid way, or not recognized by the game item catalog.
- **FR-009**: Setup MUST include a dedicated step named "Add Tap Root Assets" after the local Polar Lightning nodes are connected and funded, and before TRA-backed item trading is enabled.
- **FR-010**: The TRA setup step MUST verify that the local lab exposes the required Taproot Assets capabilities before it creates, discovers, assigns, or transfers TRA-backed items.
- **FR-011**: The TRA setup step MUST keep visible loading, success, failure, and recovery status feedback while verification, minting, assignment, and ownership checks are running.
- **FR-012**: The system MUST assign initial TRA-backed mock items to one or more NPCs during setup, while respecting the 3-item capacity per NPC.
- **FR-012B**: Initial TRA setup MUST assign 2 Book TRA items to one NPC and 2 Apple TRA items to the other NPC.
- **FR-012A**: If the learner resets back to "Add Tap Root Assets" and submits again, the system MUST clear the previous setup inventory snapshot and recreate the initial TRA inventory from scratch.
- **FR-012C**: If the learner resets the final Polar unlock step, the system MUST return to "Add Tap Root Assets", focus that step's primary control, and recreate TRA inventory on the next submit without forcing the learner to re-enter the block-height step.
- **FR-013**: The system MUST persist enough non-sensitive item identity, `item_id`, and owner state to reconnect visible inventory slots to verified local lab ownership after app restart.
- **FR-014**: The system MUST NOT persist Taproot Assets private keys, macaroons, wallet seeds, or other sensitive credentials in browser-accessible storage.
- **FR-015**: Play Game MUST display each visible inventory item with its unique name, item type, current owner, and whether TRA ownership is verified, pending, missing, unsupported, or failed.
- **FR-015A**: Play Game MUST refresh current sat balances and TRA inventory ownership when the learner arrives on Play Game or reloads the page on Play Game, with visible status feedback while the refresh is pending.
- **FR-016**: Buying an item MUST require an active trade, a payable price from the hardcoded game catalog, NPC ownership of the item, player capacity below 3 items, and a verified TRA transfer path.
- **FR-017**: Selling an item MUST require a payable price from the hardcoded game catalog, current NPC available sats, current NPC capacity below 3 items, player ownership of the item, and a verified TRA transfer path.
- **FR-018**: A completed buy MUST transfer payment from player to NPC and transfer TRA-backed item ownership from NPC to player.
- **FR-019**: A completed sell MUST transfer payment from NPC to player and transfer TRA-backed item ownership from player to NPC.
- **FR-020**: The system MUST verify ownership after every buy or sell before finalizing the visible inventory update.
- **FR-021**: If payment and TRA transfer results diverge, the system MUST show a recoverable partial-completion state and prevent duplicate transfer attempts for the affected item.
- **FR-022**: The system MUST prevent two nodes from being shown as verified owners of the same TRA-backed item at the same time.
- **FR-023**: The game log MUST describe item ownership changes using learner-friendly language that distinguishes payment from item transfer.
- **FR-024**: The feature MUST keep both web and desktop support intact.

### Key Entities *(include if feature involves data)*

- **TRA Item**: A mock inventory item represented by one Taproot Asset in the local lab. Key attributes include unique name, asset identity, embedded `item_id`, current owner, ownership status, and transfer status.
- **Game Item Catalog**: The hardcoded in-game catalog that maps an `item_id` to item type, display name, cost, visuals, and trade rules. `item_id=1` represents Book.
- **Inventory Owner**: A game node that can own up to 3 TRA Items. Owners include the player and NPC nodes.
- **TRA Setup Step**: The "Add Tap Root Assets" setup milestone that verifies Taproot Assets support, creates or discovers required mock items, assigns initial NPC ownership, and confirms ownership before enabling trading.
- **Item Trade**: A buy or sell interaction that coordinates a Lightning payment with a TRA ownership transfer and records whether both sides completed.
- **Ownership Verification**: The current proof that a specific owner controls a specific TRA Item in the local lab.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A fresh local setup can prepare at least 3 uniquely named TRA-backed mock items and assign them to NPC inventories in under 2 minutes after the Lightning nodes are already running and funded.
- **SC-002**: 100% of visible inventory slots across player and NPC nodes respect the 3-item capacity limit during setup, buying, selling, and recovery.
- **SC-003**: A learner can buy one TRA-backed item and see verified ownership move from NPC to player without leaving Play Game.
- **SC-004**: A learner can sell one TRA-backed item and see verified ownership move from player to NPC without leaving Play Game.
- **SC-005**: After app restart, verified ownership state for all known mock items is restored or flagged for recovery before buy and sell controls are enabled.
- **SC-006**: All TRA setup and trade operations show visible status feedback within 1 second of starting or failing.
- **SC-007**: 100% of supported visible TRA-backed items resolve to a known game catalog entry by `item_id`, including `item_id=1` resolving to Book.

## Assumptions

- "TRA" means Taproot Assets used by the local learning app, not a production asset or mainnet token.
- One-of-one collectible-style assets are the best fit for unique inventory items because the game needs one distinct ownership record per item.
- The TRA inventory preparation step belongs after Polar node connection and funding because Taproot Assets need a running, synced local lab before item ownership can be verified.
- The first mock inventory set can be small, with at least "Book" plus additional unique names assigned to NPCs.
- `item_id` is the stable game type identifier carried by the TRA; unique TRA names distinguish individual item instances.
- Item cost and visuals are intentionally hardcoded in the app so TRA data proves item type and ownership, not game balance or rendering rules.
- Setup may reuse existing TRA-backed items when their identity and owner can be verified in the local lab.
- Browser and desktop clients may display item state, but sensitive Taproot Assets credentials remain server-side or in the local lab tooling.
