# Feature Specification: Game View

**Feature Branch**: `[004-game-view]`  
**Created**: 2026-05-06  
**Status**: Draft  
**Input**: User description: "Replace the Play Game page gameplay area with a layered GameView showing player/NPC trade, channel open/close/block wait/buy item controls, inventory boxes, and payment/item animations."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See Player And NPC Trade Scene (Priority: P1)

As a learner, I can see the player on the left and the NPC on the right with wallets, inventories, and a clear trade stage, so the Lightning interaction is represented as a character-to-character exchange.

**Why this priority**: The new visual model replaces the old route/location analogy and is the foundation for every gameplay action.

**Independent Test**: Can be tested by opening Play Game after setup and confirming the top page widgets remain while the gameplay area shows the two-character GameView.

**Acceptance Scenarios**:

1. **Given** setup is connected, **When** the user opens Play Game, **Then** the page shows the player on the left, NPC on the right, node-derived "Player: Name" and "NPC: Name" labels, each character's sats, wallets on both characters, 3 empty player inventory boxes, and 3 NPC book boxes.
2. **Given** setup is connected, **When** the user views the page, **Then** Play Game stats remain in the upper-right widget above the GameView.

---

### User Story 2 - Manage The Lightning Trade (Priority: P1)

As a learner, I can use player-side buttons to open a trade, close a trade, wait for the next block, change location, buy an item, and sell an item, so the gameplay mirrors the core Lightning channel lifecycle and inventory exchange.

**Why this priority**: The feature must preserve the real Lightning lab interactions while changing the visual analogy.

**Independent Test**: Can be tested by using each player-side button and confirming the visible trade state and game log update.

**Acceptance Scenarios**:

1. **Given** no trade is open, **When** the user chooses Open Trade, **Then** the app starts channel opening and draws the channel line once the open request is accepted.
2. **Given** a channel open or close is waiting for confirmation, **When** the user chooses Wait for Block X, **Then** the app mines the next block and updates the trade state.
3. **Given** the trade is active, **When** the user chooses Close Trade, **Then** the app starts closing the channel and waits for block confirmation.
4. **Given** Open Trade and Close Trade both require confirmation, **When** the user views the player controls, **Then** those controls are grouped with Wait for Block X under a "Requires Wait" label.
5. **Given** any current trade state, **When** the user chooses Change Location, **Then** the visible NPC and NPC background advance to the next location without spending currency or changing the player background, player character, player wallet, player sats, or player inventory.

---

### User Story 3 - Buy A Book Over Lightning (Priority: P2)

As a learner, I can buy a book from the NPC, see payment move from player to NPC, then see the item move back from NPC to player.

**Why this priority**: This demonstrates invoice creation, payment, and item transfer after the channel is active.

**Independent Test**: Can be tested after opening and confirming a trade by choosing Buy Item and observing animation, inventory, and game log changes.

**Acceptance Scenarios**:

1. **Given** the trade is active, the NPC has books, and the player has enough currency, **When** the user chooses Buy Item, **Then** the payment animation moves left-to-right and the item animation moves right-to-left.
2. **Given** a purchase succeeds, **When** the animations complete, **Then** one book moves from the NPC inventory to the player inventory.
3. **Given** the trade is active and the player has at least one book, **When** the user chooses Sell Item, **Then** the item animation moves left-to-right, the payment animation moves right-to-left, and one book moves from the player inventory to the NPC inventory.

### Edge Cases

- If setup is not connected, Play Game remains locked and points the user to Set Up.
- If a channel action fails because the lab is stale or unavailable, existing recovery prompts and toasts remain visible.
- If the player already has all 3 books, Buy Item is unavailable.
- If the player does not have enough channel balance for the purchase, Buy Item is unavailable.
- If the player has no books, Sell Item is unavailable.
- If a channel is missing, Close Trade, Wait for Block X, and Buy Item are unavailable until the trade reaches the correct state.
- If the user changes location, the current location's existing channel state determines whether Open Trade or Close Trade is available.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Play Game MUST keep the existing top gameplay widgets and upper-right stats above the gameplay view.
- **FR-002**: Play Game MUST replace the old route/location map with a full-width GameView character trade scene.
- **FR-003**: GameView MUST render player, NPC, left/right backgrounds, node-derived character labels, character sats, wallets, optional channel line, inventory boxes, and animation layers independently.
- **FR-004**: GameView MUST expose player-side controls for Open Trade, Close Trade, Wait for Block X, Change Location, Buy Item, and Sell Item.
- **FR-005**: The NPC side MUST NOT render action buttons.
- **FR-006**: Play Game MUST own Lightning lab calls and pass callbacks into GameView rather than letting GameView call lab services directly.
- **FR-007**: Open Trade MUST start channel opening and show the channel line after the open request is accepted.
- **FR-008**: Wait for Block X MUST advance pending channel open or close confirmation.
- **FR-009**: Close Trade MUST start channel closing for an active trade.
- **FR-010**: Buy Item MUST create and pay the invoice, animate payment left-to-right, animate item right-to-left, and update inventories after success.
- **FR-011**: Sell Item MUST create and pay the reverse invoice, animate item left-to-right, animate payment right-to-left, and update inventories after success.
- **FR-012**: Open Trade, Close Trade, and Wait for Block X MUST be visually grouped under "Requires Wait".
- **FR-013**: Buy Item MUST be unavailable unless the player has enough currency for the purchase.
- **FR-014**: Sell Item MUST be unavailable unless the player has at least one inventory item.
- **FR-015**: Change Location MUST advance the NPC side through Desert, Blizzard, Jungle, and Ocean while the player side background, character, wallet, sats, and inventory stay unchanged.
- **FR-016**: Change Location MUST switch between the two available NPCs and use the selected NPC's existing trade/channel state.
- **FR-017**: The game log MUST remain at the bottom and describe the new trade/channel/payment flow.
- **FR-018**: The feature MUST keep both web and desktop support intact.
- **FR-019**: GameView MUST label the left character as "Player: {Name}" and the right character as "NPC: {Name}" using the connected lab node data.
- **FR-020**: GameView MUST display an "x sats" value beside each character label using the connected lab node's current gameplay balance.

### Key Entities

- **GameView**: The layered gameplay surface that displays characters, wallets, inventories, channel line, animation state, and player controls.
- **Trade**: The Lightning relationship between player and NPC, including missing, opening, active, closing, and closed states.
- **Inventory**: Three visible slots per side; player starts empty, NPC starts with books, and successful purchases move books to the player.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A connected user can identify the player, NPC, inventories, and channel state within 5 seconds of opening Play Game.
- **SC-002**: A user can complete Open Trade, Wait for Block X, Buy Item, Sell Item, Change Location, Open Trade for the next NPC, Close Trade, and Wait for Block X again without leaving Play Game.
- **SC-003**: The GameView renders on both web and desktop builds without blocking the existing setup, toast, recovery, or game log behavior.
- **SC-004**: A successful purchase visibly changes inventory ownership from 0 player books and 3 NPC books toward 3 player books and 0 NPC books.

## Assumptions

- The first available player-to-merchant trade is the v1 NPC trade.
- Inventory is derived from successful local lab payments until the domain model gains explicit item ownership.
- V1 animations are CSS-based layered image animations.
- Placeholder repo-local art is acceptable until final production art is provided.
- The four locations reuse the two available NPC identities.
