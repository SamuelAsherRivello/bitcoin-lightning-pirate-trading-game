# Feature Specification: Game Treasury

**Feature Branch**: `[006-game-treasury]`  
**Created**: 2026-05-15  
**Status**: Draft  
**Input**: User description: "game treasury"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Create Game Treasury During Setup (Priority: P1)

As a player setting up the local game lab, I want setup to create and fund a dedicated GAME_TREASURY node so the game has a clear house or bank that funds player activity and holds items before they are distributed.

**Why this priority**: The treasury is now part of the required startup flow. Gameplay and NPC inventory distribution depend on the treasury existing before user nodes and later route unlocks are completed.

**Independent Test**: Can be fully tested by running setup through the new Game Treasury step and confirming that a GAME_TREASURY node exists, is funded with enough sats for game activity, and holds the initial treasury-owned items intended for NPC distribution.

**Acceptance Scenarios**:

1. **Given** the bridge URL and server name setup steps are complete, **When** the player starts the Game Treasury setup step, **Then** setup creates a Lightning node named GAME_TREASURY.
2. **Given** the GAME_TREASURY node has been created, **When** the setup step continues, **Then** the treasury is funded with enough sats to support the configured game activity for the other players.
3. **Given** treasury-funded gameplay items are required for NPCs, **When** the Game Treasury setup step completes, **Then** the treasury owns the initial items that will soon be transferred to NPCs.
4. **Given** treasury setup is still running or fails, **When** the player views the step, **Then** visible setup feedback explains the current action or failure without advancing silently.

---

### User Story 2 - Distribute NPC Items From Treasury (Priority: P2)

As a player, I want setup to transfer the NPC starting items from the Game Treasury to Bob and Carol so NPC inventory starts from an explicit game-bank distribution instead of appearing automatically.

**Why this priority**: This makes initial NPC inventory part of the same value-flow model used during gameplay. Items originate in the treasury, then move to NPCs through visible transfers.

**Independent Test**: Can be tested by completing the user-node creation step, then running the item-transfer step and confirming Bob and Carol receive the same starting items they currently have, with transfers recorded as coming from Game Treasury.

**Acceptance Scenarios**:

1. **Given** Alice, Bob, and Carol user nodes exist, **When** setup reaches the NPC item transfer step, **Then** the game transfers Bob's and Carol's configured starting items from Game Treasury to those NPCs.
2. **Given** an NPC starting item transfer succeeds, **When** the setup step is reviewed, **Then** the player can see which item moved from Game Treasury to which NPC.
3. **Given** an NPC starting item transfer fails, **When** setup displays the result, **Then** setup keeps the failure recoverable and does not mark item distribution as complete.

---

### User Story 3 - Use Treasury Readiness in Gameplay (Priority: P3)

As a player, I want gameplay actions to reflect treasury readiness so I know whether the game economy can fund, reward, or settle the action I am trying to take.

**Why this priority**: This prevents confusing game states where the player attempts an action that cannot be supported by the available game economy.

**Independent Test**: Can be tested by setting treasury resources below and above a required action threshold, then confirming that the game clearly enables, blocks, or explains each action.

**Acceptance Scenarios**:

1. **Given** the treasury has enough available sats or items for a game action, **When** the player views that action, **Then** the action is shown as available with a clear treasury impact preview.
2. **Given** the treasury does not have enough available sats or required items for a game action, **When** the player views that action, **Then** the action is blocked or marked unavailable with an explanation of what is missing.
3. **Given** the treasury state is uncertain or outdated, **When** the player attempts a treasury-dependent action, **Then** the game refreshes or prompts for refresh before committing the action.

---

### Edge Cases

- Treasury values are unavailable while the game is loading, refreshing, or recovering from a failed setup state.
- The GAME_TREASURY node already exists from a previous setup attempt.
- The GAME_TREASURY node exists but does not have enough sats for configured game activity.
- Treasury item creation succeeds for some items but not all items intended for NPC distribution.
- User nodes exist before treasury setup is complete because setup is resumed from a saved or partial state.
- An NPC item transfer from treasury to Bob or Carol is interrupted or partially complete.
- A treasury-affecting action succeeds in gameplay but the latest treasury summary is not yet refreshed.
- A treasury-affecting action fails or is cancelled after a player saw a preview.
- Multiple game actions affect the treasury in quick succession.
- Inventory-backed value and spendable balance move in opposite directions during the same trade.
- A saved game snapshot contains treasury entries that no longer match the current game state.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Polar setup MUST include the following ordered steps: Bridge URL, Server Name, Game Treasury, User Nodes, NPC Item Transfers, Block Height, and Unlock Routes.
- **FR-002**: The Game Treasury setup step MUST create or verify a Lightning node named GAME_TREASURY.
- **FR-003**: The Game Treasury setup step MUST fund the GAME_TREASURY node with enough sats to support configured game activity for the other players.
- **FR-004**: The Game Treasury setup step MUST create or verify treasury-owned items that are intended for later NPC distribution.
- **FR-005**: The User Nodes setup step MUST create or verify Alice, Bob, and Carol after the Game Treasury step is complete.
- **FR-006**: The NPC Item Transfers setup step MUST transfer the configured starting items for Bob and Carol from Game Treasury to those NPCs.
- **FR-007**: Bob and Carol MUST end setup with the same starting items they currently receive in the game, but those items MUST originate from Game Treasury transfers.
- **FR-008**: Each setup step that creates, funds, or transfers treasury resources MUST provide visible loading, success, and recoverable failure feedback.
- **FR-009**: The system MUST provide a player-visible game treasury view from the game experience.
- **FR-010**: The treasury view MUST show the current spendable game balance, inventory value summary, and overall treasury status.
- **FR-011**: The treasury view MUST distinguish loading, ready, refreshing, degraded, and failed states with user-visible feedback.
- **FR-012**: The system MUST record treasury-impacting game events, including setup funding, setup item creation, NPC starting-item transfers, rewards, costs, trades, and inventory value movements.
- **FR-013**: Each treasury history entry MUST include a plain-language description, direction of change, amount or item affected, related game action, and time.
- **FR-014**: Users MUST be able to review recent treasury history in newest-first order.
- **FR-015**: Treasury-dependent gameplay actions MUST show whether the treasury can support the action before the player commits.
- **FR-016**: Treasury-dependent gameplay actions MUST provide a preview of the expected treasury impact before commitment.
- **FR-017**: The system MUST prevent or clearly block treasury-dependent actions when available resources are insufficient or uncertain.
- **FR-018**: The system MUST refresh treasury state after a treasury-impacting action completes, fails, or is cancelled.
- **FR-019**: The treasury MUST avoid presenting technical wallet, node, credential, or transport details as player-facing explanations.
- **FR-020**: The treasury MUST preserve the learning distinction between spendable sats, inventory-backed value, and historical movement.

### Key Entities *(include if feature involves data)*

- **Game Treasury**: The player-facing summary of game economy resources, including spendable balance, inventory value, status, and latest update time.
- **GAME_TREASURY node**: The dedicated game-bank Lightning node created during setup to fund activity and temporarily hold items before distribution.
- **Treasury Entry**: A readable record of a single treasury-impacting event, including description, change direction, affected value or item, related action, and time.
- **Treasury Impact Preview**: A pre-commit summary of how a pending game action is expected to affect treasury resources.
- **Treasury Status**: The user-visible readiness state for treasury data, such as loading, ready, refreshing, degraded, or failed.
- **Treasury Resource**: A spendable amount or inventory-backed item value that contributes to treasury understanding.
- **User Node**: A gameplay participant node created after Game Treasury setup, including Alice, Bob, and Carol.
- **NPC Item Transfer**: A setup-time movement of an item from Game Treasury to Bob or Carol to establish starting NPC inventory.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 95% of setup attempts show the Game Treasury step as a distinct step between Server Name and User Nodes.
- **SC-002**: 95% of successful setup runs create or verify the GAME_TREASURY node before Alice, Bob, and Carol are created or verified.
- **SC-003**: 100% of successful setup runs show Bob and Carol receiving their configured starting items through Game Treasury transfers.
- **SC-004**: 95% of players can identify the current spendable treasury balance within 5 seconds of opening the treasury.
- **SC-005**: 90% of treasury-affecting game actions produce a visible treasury update or pending-refresh state within 2 seconds of completion.
- **SC-006**: 90% of players can correctly explain whether a proposed game action will increase, decrease, or not affect the treasury after viewing its preview.
- **SC-007**: 100% of blocked treasury-dependent actions provide a visible reason before the player commits the action.
- **SC-008**: Recent treasury history shows at least the 10 most recent treasury-impacting events when that many events exist.
- **SC-009**: Treasury loading or failure states never appear as zero-balance ready states during user testing.

## Assumptions

- The treasury represents game-economy resources for the local learning game, not real user funds or production financial balances.
- The first version changes startup/setup flow and focuses on player-visible summaries, previews, and recent history rather than administrative accounting tools.
- The GAME_TREASURY node is a required local lab participant named exactly Game Treasury for player-facing setup and explanation.
- The amount of sats considered "enough" for the treasury is determined by the configured game scenario and existing player activity requirements.
- Only items soon intended for NPC distribution are initially assigned to the treasury during setup.
- Bob and Carol keep their current starting item configuration, but item ownership is established through treasury transfers.
- Existing game actions that affect Lightning payments, rewards, item purchases, sales, or inventory transfers are the initial sources of treasury changes.
- Treasury history is intended to support learning and debugging gameplay outcomes, not legal, tax, or financial reporting.
- Mobile-specific layout optimization is not required beyond preserving existing responsive behavior.
