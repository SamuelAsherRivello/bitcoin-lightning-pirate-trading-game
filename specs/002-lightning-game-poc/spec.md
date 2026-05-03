# Feature Specification: Bitcoin Lightning Game POC

**Feature Branch**: `002-lightning-game-poc`  
**Created**: 2026-05-02  
**Status**: Draft  
**Input**: User description: "Use the chat above and Dioxus and Rust and client/server and the best Rust-specific Bitcoin Lightning API as a service installed as a server service in this project."

## User Scenarios & Testing

### User Story 1 - Complete Regtest Setup (Priority: P1)

A learner opens the app, sees `Home`, learns why the demo is useful, visits `Set Up`, follows the Polar regtest checklist, enters the demo transaction amount, saves the Polar automation profile, and lets the app create Alice, Bob, and Carol from a Polar Bitcoin backend node.

**Why this priority**: Without a clear setup path, the rest of the app cannot teach Lightning operations or run the game.

**Independent Test**: Start with no saved setup, open the app, complete the setup form using a running Polar bridge, let the app reuse or create the named Polar server, create the demo Lightning nodes from the app, refresh the page, and verify `Play Game` and `Debug Network` unlock.

**Acceptance Scenarios**:

1. **Given** no saved connection profile, **When** the learner opens the app, **Then** `Home` and `Set Up` are enabled while `Play Game` and `Debug Network` are visible but disabled.
2. **Given** the learner is on `Home`, **When** they read `Why this demo exists`, **Then** they understand that this is a Polar regtest lab, the app controls all demo nodes, and setup validation rejects hosted, production, mainnet, and other non-regtest profiles.
3. **Given** Polar and its localhost bridge are running, **When** the learner saves the bridge URL and submits a Polar server name, **Then** the app reuses that server if it exists or creates it if it does not, reports the result through a toast, and continues the setup wizard.
4. **Given** a saved profile whose nodes are offline, **When** the learner reloads the app, **Then** the app explains that setup is saved but Polar is not reachable and keeps gameplay locked.
5. **Given** the learner no longer wants the generated demo nodes, **When** they destroy demo nodes from `Set Up`, **Then** the app asks Polar to remove Alice, Bob, and Carol and locks gameplay until nodes are created again.

---

### User Story 2 - Play The Lightning Game (Priority: P2)

A learner uses `Play Game` to control Alice, trade with Bob at the Beach and Carol at the Mountain, and see game actions become real Lightning operations using the configured sats-per-transaction amount.

**Why this priority**: This is the main approachable learning loop: game action first, Lightning concept second.

**Independent Test**: With setup complete, open `Play Game`, open a trade route to Bob, wait for the next block, buy an item from Bob, and verify the game log shows the underlying invoice and payment.

**Acceptance Scenarios**:

1. **Given** setup is complete and no Alice-Bob channel is active, **When** Alice opens a trade route to Bob, **Then** the app starts channel opening and marks the route as under construction until a Bitcoin block confirms it.
2. **Given** a route is under construction, **When** the learner clicks `Wait for Next Block`, **Then** the app advances the local regtest chain and updates the route to active after confirmation.
3. **Given** an active Alice-Bob route, **When** Alice buys an item from Bob, **Then** Bob creates an invoice for the configured sats-per-transaction amount and Alice pays it.
4. **Given** a payment completes, **When** the learner views the action summary, **Then** the summary states that the trade used Lightning and did not need a new Bitcoin block.

---

### User Story 3 - Debug The Network (Priority: P3)

A learner opens `Debug Network` to inspect nodes, channels, balances, invoices, and payments.

**Why this priority**: The app must teach both the game-level idea and the real network-level mechanics.

**Independent Test**: With setup complete and at least one channel/payment created, open `Debug Network` and verify that the channel row, node balances, invoice history, and payment history match the game actions.

**Acceptance Scenarios**:

1. **Given** Alice and Bob have a channel, **When** the learner views `Debug Network`, **Then** the page shows a row with Alice on the left, Bob on the right, a purse-to-purse channel/payment visual, status, capacity, local/remote balances, and action buttons.
2. **Given** Bob clicks `Create Invoice`, **When** Alice has `AutoSend` enabled for that channel, **Then** the app visibly creates the invoice, pays it from Alice, and logs both steps.
3. **Given** a channel is pending, **When** the learner views the row, **Then** the row states that a Bitcoin block is required before Lightning payments can use that channel.
4. **Given** a channel is active, **When** the learner views recent payments, **Then** the page states that completed Lightning payments did not require a new Bitcoin block.

---

### User Story 4 - Read Home Concepts And FAQ (Priority: P4)

A learner opens `Home` to understand why the demo exists, Bitcoin, Lightning, their tradeoffs, and which operations need Bitcoin blocks before or after using the lab.

**Why this priority**: The concept page should be available even before setup so learners can understand the game vocabulary without connecting a local lab first.

**Independent Test**: Open `Home` from the top navigation before setup is verified and verify that it shows why the demo exists, Bitcoin and Lightning summaries with links, a Bitcoin vs Lightning pros/cons table, and the operation block-dependency table.

**Acceptance Scenarios**:

1. **Given** the learner has not completed setup, **When** they open `Home`, **Then** the page is enabled and does not require a connected lab state.
2. **Given** the learner reads the top Home FAQ sections, **When** they compare Bitcoin and Lightning, **Then** they can identify Bitcoin as the base settlement layer and Lightning as a faster payment layer with channel/liquidity tradeoffs.
3. **Given** the learner reads the operation table, **When** they compare actions, **Then** they can distinguish actions that require a Bitcoin node from actions that require a mined block.

---

### Edge Cases

- Saved setup exists but one node is unreachable.
- Saved setup exists but points to fewer than Alice, Bob, and Carol.
- Polar is running but its localhost MCP bridge is unavailable.
- The browser app is opened from an origin that Polar's localhost bridge CORS policy rejects.
- Polar creates an LND node but does not return the generated node name to the app.
- A learner enters a transaction amount that is zero, negative, non-numeric, or larger than the demo max.
- A channel does not have enough outbound liquidity for the configured transaction amount.
- The app receives an invoice but the expected payer cannot route or pay it.
- The learner tries to open a trade route that already exists or is already pending.
- The learner refreshes during a pending route, invoice, or payment.
- The learner pastes credentials from a non-regtest node.
- Polar is running but the Bitcoin backend is not ready or cannot mine the next block.

## Requirements

### Functional Requirements

- **FR-001**: The app MUST provide exactly four primary user pages, from left to right: `Home`, `Set Up`, `Play Game`, and `Debug Network`.
- **FR-002**: `Play Game` and `Debug Network` MUST remain visible but disabled until setup is saved and successfully verified; `Home` and `Set Up` MUST remain available before setup.
- **FR-003**: Disabled pages MUST explain that the learner must complete `Set Up` before gameplay or network debugging can start.
- **FR-004**: `Home` MUST contain `Why this demo exists` and FAQ/concepts content.
- **FR-005**: `Why this demo exists` MUST explain that this is a Polar regtest Lightning lab, the app controls all demo nodes, and the demo separates game-level actions from network-level mechanics.
- **FR-006**: `Overview` MUST explain through a `Regtest safety check` callout that the app accepts local Polar regtest profiles only and rejects hosted, production, mainnet, and other non-regtest profiles before lab actions unlock.
- **FR-007**: `Setup` MUST present a `Connection` section with `Polar Connection (Networked)` and `Mock Connection (Offline)` tabs, each with a short explanation and its relevant controls inside the selected tab panel.
- **FR-008**: `Setup` MUST let the learner set `Sats per transaction`, defaulting to `1,000` sats.
- **FR-009**: `Sats per transaction` MUST be a whole number between `1` and `100,000` sats for the demo.
- **FR-010**: `Polar Connection (Networked)` MUST provide an `OS Setup` section with four numbered manual rows: install Docker, run Docker, install Polar, and run Polar. Only the words `Docker` and `Polar` in the install rows should be linked.
- **FR-010a**: The Polar bridge URL field MUST include a visible `(i)` hover affordance with 5-10 words of field-specific help.
- **FR-010b**: `Polar Setup` MUST render four numbered rows from the start: save Polar MCP bridge URL, ensure Polar server name, create 3 demo nodes, and complete setup.
- **FR-010c**: Each `Polar Setup` row MUST be a compact form row with label, `(i)` tooltip, prepopulated value field, and buttons.
- **FR-010d**: `Polar Setup` MUST show `SUBMIT` for every row, enable only the current valid row, grey out future rows, show `RESET` next to steps 2, 3, and 4, and return reset focus to the previous step.
- **FR-010e**: The Polar MCP bridge URL row MUST use the app-owned text field. The Polar server name row MUST use an app-owned text field and must ask the bridge to reuse the named server when it exists or create it when it does not.
- **FR-011**: The app MUST let the learner paste the Polar bridge URL in the UI instead of manually editing setup files.
- **FR-012**: The setup guidance MUST only appear inside the selected `Polar Connection (Networked)` tab and should separate local `OS Setup` from app-driven `Polar Setup`.
- **FR-013**: The app MUST model Alice, Bob, and Carol as demo node personas, not production users.
- **FR-014**: The app MUST treat Alice as the player, Bob as the Beach merchant, and Carol as the Mountain merchant.
- **FR-015**: The game MUST use `Town`, `Beach`, `Mountain`, and optionally `Desert` as readable location labels.
- **FR-016**: The game MUST represent a Lightning channel as a `Trade Route` between two node personas.
- **FR-017**: The game MUST represent a pending channel as a trade route under construction until the next Bitcoin block.
- **FR-018**: The game MUST provide `Wait for Next Block` instead of fixed wait-time wording.
- **FR-019**: `Wait for Next Block` MUST explain that mainnet blocks arrive about every 10 minutes on average but regtest mines instantly.
- **FR-020**: Game purchases, sales, tips, or tolls MUST use the configured sats-per-transaction amount.
- **FR-021**: A receive action MUST create a Lightning invoice; it MUST NOT imply that money arrives without a sender paying.
- **FR-022**: A send action MUST pay a known invoice from the selected sender node.
- **FR-023**: AutoSend MUST be visibly marked as a lab/demo feature and MUST use the configured transaction amount as its max per automatic payment.
- **FR-023a**: `Play Game` MUST show `Recent actions` as history rows with the current action title and body on the left, plus zero to three right-side detail pills for steps such as `Invoice Sent`, `Invoice Paid`, `Channel Open Request`, `Block Mined`, and `Channel Open Complete`.
- **FR-024**: `Debug Network` MUST show channel rows with node blocks on each side and a line from wallet/purse to wallet/purse.
- **FR-025**: Each channel row MUST show status, capacity, local/remote balances, whether a block is required, and available actions.
- **FR-026**: `Debug Network` MUST show recent invoices and payments generated by the app.
- **FR-027**: `Home` MUST include FAQ content explaining Bitcoin, Lightning, their pros and cons, and that blocks are needed to enter and exit Lightning channels while payments over active channels do not wait for new blocks.
- **FR-028**: The Home FAQ content MUST include a simplified operation table for create invoice, pay invoice, fund wallet, open channel, close channel, check payment status, and wait/mine block.
- **FR-029**: The app MUST include a `Lab mode` warning explaining that controlling all demo nodes is appropriate for education but not for real player funds.
- **FR-030**: The app MUST present production guidance that a real game would normally request payment from a player's own Lightning wallet rather than spending from it directly.
- **FR-031**: The app MUST provide a `Create Demo Nodes` action that asks Polar to add, name, start, fund, and confirm Alice, Bob, and Carol LND nodes using the configured Bitcoin backend.
- **FR-032**: The app MUST provide a `Destroy Demo Nodes` action that asks Polar to remove Alice, Bob, and Carol and clears the active lab state.

### Key Entities

- **Setup Profile**: The locally saved configuration needed to connect to the local lab and define the demo transaction amount.
- **Polar Automation Profile**: The local Polar bridge URL plus discovered network/backend values used to create or destroy demo Lightning nodes.
- **Demo Node**: Alice, Bob, or Carol, each representing a controlled LND node in the regtest lab.
- **Location**: A readable game place such as Town, Beach, Mountain, or Desert.
- **Trade Route**: Game-facing representation of a Lightning channel between two demo nodes.
- **Invoice Request**: A request created by one node to receive the configured sats-per-transaction amount.
- **Payment Attempt**: An attempt by one node to pay an invoice.
- **Block Wait Action**: A game action that advances regtest by mining the next block.
- **Operation FAQ Row**: A teaching row that maps a Lightning or Bitcoin action to whether it needs a Bitcoin node and whether it needs a mined block.

## Success Criteria

### Measurable Outcomes

- **SC-001**: A first-time learner can identify the required setup path from the locked home state in under 30 seconds.
- **SC-002**: A learner with Docker Desktop and Polar already installed can complete the setup checklist and save a verified profile in under 10 minutes.
- **SC-003**: After setup, a learner can complete one channel-open-and-confirm flow and one Lightning payment flow in under 5 minutes.
- **SC-004**: The app clearly distinguishes block-required actions from instant Lightning actions in every relevant game and debug state.
- **SC-005**: A learner can refresh the app after saving setup and resume with the same transaction amount and connection state.
- **SC-006**: A learner can open `Debug Network` after a game trade and identify the invoice, payment, channel, and balance movement caused by that trade.
- **SC-007**: A learner can open `Home` before setup and identify at least one operation that needs a mined block and one operation that does not.

## Assumptions

- The first implementation targets Polar regtest, not public testnet, signet, or mainnet.
- The learner manually installs Docker Desktop and Polar outside the app.
- The app attempts to create the named Polar server when the bridge reports that it does not already exist, falling back to a clear error if the bridge does not expose a compatible create-network tool.
- Polar v4 or newer exposes a localhost MCP bridge that can add, rename, start, fund, and remove nodes for this local lab flow.
- The first implementation controls all demo nodes as a lab tool and must be explicit that this is not a production wallet permission model.
- The project keeps web and desktop support.
- The service layer may require a local server process for secure credential handling.
- Browser-only credential persistence is acceptable only for throwaway regtest use with clear warnings.
