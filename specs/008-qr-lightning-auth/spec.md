# Feature Specification: QR Lightning Auth Mode

**Feature Branch**: `[008-qr-lightning-auth]`  
**Created**: 2026-05-18  
**Status**: Draft  
**Input**: User description: "This project requires no qr code auth, which is helpful during development. But I'd like to add a mode that uses qr code auth for the player and the players chain transactions. Find the lnauth library and analyze how to add such a mode and what part of what pages of the app to modify. Second goal, all lightening stuff (qr auth, tra work, and lightening payments) will be consolidated into 1 or more libraries in this project so all of it can be used in a bevy project. So keep the dioxus view separate from the services for this project so its more portable"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Choose User Auth Mode (Priority: P1)

As a developer or demo operator, I want setup to start with a `User Auth` step offering development-only `App`, development-testable `Mock LNAuth`, and scalable-feeling `LNAuth`, so that local iteration stays fast while realistic player authorization can be demonstrated when needed.

**Why this priority**: The app must preserve the current development-friendly setup while making the production-oriented user experience clearly center on external wallet authorization.

**Independent Test**: Can be tested by opening setup, selecting `App`, `Mock LNAuth`, or `LNAuth` in the first setup step, and confirming the app clearly shows which mode is active before gameplay unlocks.

**Acceptance Scenarios**:

1. **Given** a fresh setup, **When** the user views setup step 1, **Then** the app shows `User Auth` with exactly three choices: `App`, `Mock LNAuth`, and `LNAuth`.
2. **Given** a fresh setup, **When** the user selects `App`, **Then** setup continues with the current behavior and labels the mode as intended for development rather than production-scale authorization.
3. **Given** a fresh setup, **When** the user selects `Mock LNAuth`, **Then** setup continues with the LNAuth-shaped user experience while QR prompts auto-complete for development and automated testing.
4. **Given** a fresh setup, **When** the user selects `LNAuth`, **Then** setup presents it as the scalable authorization path and prepares Play Game to request wallet login on entry.
5. **Given** the user is deciding whether to use `LNAuth`, **When** they open the info tip next to the `LNAuth` choice, **Then** the app recommends one free mobile wallet that works on both Android and iOS and explains why it is recommended.
6. **Given** an already connected setup, **When** the user switches auth modes, **Then** the app clearly indicates that existing lab state must be revalidated before gameplay continues.

---

### User Story 2 - Authenticate Player On Game Entry (Priority: P1)

As a player, I want Play Game to prompt me with a focused QR modal when `LNAuth` is active and I first arrive there, so that I log in with my compatible Lightning wallet before gameplay actions proceed.

**Why this priority**: Player authentication is the core new capability and must work before transaction approval can be meaningful.

**Independent Test**: Can be tested by choosing `Mock LNAuth`, opening Play Game, observing the centered QR modal, waiting for auto-completion, and confirming gameplay becomes usable without a real wallet. Real `LNAuth` can be tested later by scanning the same modal with Alby Go.

**Acceptance Scenarios**:

1. **Given** `LNAuth` is selected and the player opens Play Game without an active player login, **When** the page loads, **Then** the app darkens the page and shows a centered modal titled `Scan with wallet`.
2. **Given** the login modal is open, **When** the player reads the modal body, **Then** it explains that the QR code is for logging in and shows the QR code centered below the body text.
3. **Given** the player scans and approves the login request, **When** the wallet response is accepted, **Then** the app closes the modal and records the player as authenticated without exposing private keys or wallet secrets.
4. **Given** `Mock LNAuth` is selected and the login modal opens, **When** one second passes, **Then** the app auto-completes the modal as though a wallet responded successfully.
5. **Given** the QR authentication request expires or fails, **When** the player returns to the app, **Then** the modal displays a recoverable failure state and offers a fresh QR request.
6. **Given** the QR modal is open, **When** the user presses `Cancel`, **Then** the modal closes for development convenience and the protected action remains incomplete.

---

### User Story 3 - Approve Sats Sends With QR Modal (Priority: P2)

As an authenticated player, I want a focused QR modal when I send sats, so that value movement feels explicitly authorized rather than automated.

**Why this priority**: The user experience should feel scalable and production-shaped: login happens when entering gameplay, while sats sends require explicit approval at the moment value moves.

**Independent Test**: Can be tested by selecting `Mock LNAuth`, entering Play Game, initiating a sats send, observing the modal detail such as `You are sending 1,000 sats`, waiting for auto-completion, and confirming the send completes from the user's perspective.

**Acceptance Scenarios**:

1. **Given** `LNAuth` is active and the player initiates a sats send, **When** the approval is required, **Then** the app darkens the page and shows the same centered QR modal pattern titled `Scan with wallet`.
2. **Given** the send approval modal is open, **When** the modal body is shown, **Then** it states the action detail above the QR code, such as `You are sending 1,000 sats`.
3. **Given** `LNAuth` is active and the send approval modal is pending, **When** approval has not succeeded, **Then** the app prevents the send from completing.
4. **Given** `Mock LNAuth` is active and the send approval modal opens, **When** one second passes, **Then** the app auto-completes the approval as though a wallet responded successfully.
5. **Given** `LNAuth` is active and approval fails, expires, or is canceled, **When** the player returns to the app, **Then** the send remains incomplete and the app shows a clear recovery path.
6. **Given** `App` mode is active, **When** the same game action is initiated, **Then** the current fast development behavior remains available without QR approval and remains labeled as development-focused.

---

### User Story 4 - Reuse Lightning Capabilities Outside The App UI (Priority: P2)

As a developer building another Rust game, I want authentication, payment, route/channel, and asset-transfer behavior to be available through portable service boundaries, so that the same Lightning game logic can be reused outside this app's current UI.

**Why this priority**: Portability is a stated goal and protects the feature from becoming tied to one frontend.

**Independent Test**: Can be tested by reviewing the service contracts and exercising Lightning operations without depending on route components, page state, toast state, or visual rendering.

**Acceptance Scenarios**:

1. **Given** the portable service layer is used by a non-UI caller, **When** the caller requests a QR authentication challenge, **Then** it receives data needed to present the challenge without any page-specific dependency.
2. **Given** the portable service layer is used by a non-UI caller, **When** the caller creates invoices, sends payments, opens or closes routes, or transfers assets, **Then** the operation result is represented as data and errors suitable for another game frontend.
3. **Given** the app UI consumes the same service layer, **When** Lightning actions run, **Then** UI-only feedback remains outside the portable service layer.

### Edge Cases

- QR authentication is unavailable because the player has no compatible wallet.
- The recommended mobile wallet is unavailable on the user's device or regional app store.
- QR challenge expires while the player is still viewing setup or gameplay.
- Wallet approval succeeds after the app has already generated a replacement challenge.
- `Mock LNAuth` auto-completion fires after the user already canceled the modal.
- Player authentication is lost or invalidated while gameplay is already unlocked.
- A chain-impacting action is started while another approval or Lightning operation is pending.
- A player tries to send sats in `LNAuth` mode after the original login QR has succeeded but before the per-send QR approval succeeds.
- A low-risk navigation, inventory inspection, dashboard refresh, channel open/close display, TRA ownership display, or educational explanation is performed in `LNAuth` mode and should not trigger a QR prompt.
- The player's real Lightning wallet is on production Lightning while the app's Polar lab is a local/regtest simulation.
- The service layer returns a recoverable failure while the UI is on Play Game or Network Dashboard.
- A saved local setup was created in `App` mode and is later reopened after `LNAuth` is selected.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST add `User Auth` as the first setup step with exactly three choices: `App`, `Mock LNAuth`, and `LNAuth`.
- **FR-002**: The system MUST preserve the existing no-QR development behavior under `App` mode and label it as development-focused rather than production-scalable.
- **FR-003**: The system MUST provide `Mock LNAuth` mode that uses the same QR modal UX as `LNAuth` but auto-completes each QR prompt after one second for development and automated testing.
- **FR-004**: The system MUST provide `LNAuth` mode as the scalable authorization path that requires player authentication before gameplay actions are considered usable.
- **FR-005**: The system MUST keep `User Auth` independent from the existing lab connection mode, so `App`/`Mock LNAuth`/`LNAuth` does not replace `Polar Connection (Networked)`/`Mock Connection (Offline)`.
- **FR-006**: The system MUST keep the subsequent Polar setup order as `Bridge URL`, `Server Name`, `Create Nodes`, `Game Treasury (Sats)`, `Game Treasury (TRAs)`, `User Nodes (Sats)`, `User Nodes (TRAs)`, `Block Height`, and `Unlock Routes`.
- **FR-006a**: The `Create Nodes` step MUST find or create all Polar nodes required by later steps: the Bitcoin backend, Game Treasury LND node, `GAME_TAPROOT` Taproot Assets node, two NPC LND nodes, and one player LND node.
- **FR-006b**: The `Create Nodes` step MUST request creation before checking readiness, restart the Polar network after node topology changes such as creates, renames, or cleanup removals, poll all required nodes for started/ready status, and restart the Polar network once if repeated readiness retries show unstable or not-started nodes.
- **FR-006c**: The `User Nodes (Sats)` step MUST rebalance sats between Game Treasury and the player/NPC user nodes until user-node sats targets are met, while allowing Game Treasury to retain extra sats after rebalancing so existing networks remain usable.
- **FR-006d**: The `User Nodes (TRAs)` step MUST rebalance TRA ownership between Game Treasury and the player/NPC user nodes until user-node TRA targets are met, while allowing Game Treasury to retain extra TRAs after rebalancing so existing networks remain usable.
- **FR-007**: The system MUST show the active authentication mode and authentication status wherever users make setup or gameplay decisions affected by that mode.
- **FR-008**: The system MUST show an info icon next to `LNAuth` with a tip recommending Alby Go as the primary mobile wallet for testing on both Android and iOS.
- **FR-009**: The `LNAuth` wallet tip MUST explain that Alby Go is recommended for testing because it is available on Android and iOS, is open source, works with Alby Hub or another NWC wallet service, and can approve Alby app connections from mobile.
- **FR-010**: The implementation MUST verify the chosen `LNAuth` QR flow against Alby Go during testing; if Alby Go cannot complete the exact LNURL-auth callback flow, the app MUST surface that as a documented compatibility blocker before choosing a fallback wallet.
- **FR-011**: The system MUST show all QR login and QR approval requests in a reusable modal overlay that floats above the current page, darkens the background, and centers the QR code.
- **FR-012**: The QR modal MUST include a title, body text above the QR code, the centered QR code, and a `Cancel` button.
- **FR-013**: The QR modal title for login and send approvals SHOULD be `Scan with wallet`.
- **FR-014**: The QR modal body MUST describe the specific action, such as `Log in to start playing` for login or `You are sending 1,000 sats` for a send approval.
- **FR-015**: The QR modal `Cancel` button MUST close the modal for development convenience and leave the protected login or send action incomplete.
- **FR-016**: The system MUST trigger the login QR modal when a user first arrives on Play Game in `LNAuth` or `Mock LNAuth` mode without an active player login.
- **FR-017**: The system MUST trigger the Play Game login modal after the existing route-entry lab refresh has produced a connected lab state, so normal loading feedback remains visible first.
- **FR-018**: The system MUST generate a fresh player authentication challenge when `LNAuth` mode needs player login.
- **FR-019**: The system MUST accept a successful wallet approval as proof of player identity without storing wallet secrets, private keys, seed phrases, or credential material in user-visible local preferences.
- **FR-020**: The system MUST treat expired, rejected, mismatched, canceled, or failed authentication attempts as recoverable states with clear retry behavior.
- **FR-021**: The system MUST require a fresh QR-backed player approval before completing player Buy Item or Sell Item sats sends when `LNAuth` or `Mock LNAuth` mode is active.
- **FR-022**: The system MUST avoid QR prompts for low-risk actions such as navigation, dashboard refreshes, educational content, channel display, TRA display, and read-only state inspection.
- **FR-023**: The system MUST leave existing fast transaction behavior available when `App` mode is active.
- **FR-024**: The system MUST record enough non-sensitive authentication and transaction status for the user to understand what happened in setup, gameplay history, and network diagnostics.
- **FR-025**: The system MUST keep QR authentication, asset transfer, route/channel work, invoice creation, and payment behavior behind portable service boundaries that are independent of page components, route navigation, toast messages, and visual rendering.
- **FR-026**: The system MUST expose service-level results and errors that another Rust game frontend can consume without depending on this app's UI.
- **FR-027**: The system MUST continue to provide visible loading or status feedback during authentication, transaction approval, invoice creation, payment attempts, route/channel changes, and asset transfers.
- **FR-028**: The system MUST ensure saved local preferences contain only non-sensitive setup choices and status snapshots.
- **FR-029**: The system MUST treat the player's external Lightning wallet as an authentication and approval wallet, not as a Polar node that must be created inside the local lab.

### Key Entities

- **User Auth Mode**: The selected setup behavior: `App`, `Mock LNAuth`, or `LNAuth`.
- **Wallet Recommendation Tip**: The info-tip content attached to `LNAuth`, including Alby Go as the promoted test wallet, supported mobile platforms, and compatibility expectations.
- **QR Authorization Modal**: The floating modal overlay that darkens the current page and presents a title, action description, centered QR code, and cancel button.
- **Player Auth Session**: A short-lived authentication attempt with challenge status, expiration, and resulting player identity when approved.
- **Player Identity**: The non-sensitive identifier associated with a successfully authenticated player.
- **Authorization Event**: A key player action that requires QR approval in `LNAuth` or mock approval in `Mock LNAuth`, initially player login on Play Game entry and player sats sends.
- **Transaction Approval**: A required approval gate for authorization events in `LNAuth` and `Mock LNAuth` modes.
- **Lightning Operation Result**: A portable result for authentication, route/channel work, invoice creation, payment, or asset transfer, including success data or recoverable error details.
- **Local Setup Snapshot**: Non-sensitive saved setup state used to resume the app without storing wallet secrets.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can choose `App`, `Mock LNAuth`, or `LNAuth` in the `User Auth` setup step in under 30 seconds.
- **SC-002**: In `LNAuth` mode, at least 95% of successful wallet approvals update the app authentication state within 5 seconds of the app receiving the wallet response.
- **SC-003**: In `Mock LNAuth` mode, 100% of QR modal prompts auto-complete successfully after about one second unless canceled.
- **SC-004**: In `LNAuth` and `Mock LNAuth` modes, 100% of configured authorization events are blocked until player approval or mock approval succeeds.
- **SC-005**: In `App` mode, existing setup and gameplay flows remain available without adding QR approval steps and are visibly marked as development-oriented.
- **SC-006**: Authentication and transaction failures are recoverable without restarting the app in at least 95% of tested failure scenarios.
- **SC-007**: Portable Lightning service behavior can be exercised without instantiating any page, route, or visual component.
- **SC-008**: Review of saved local setup data confirms no wallet secrets, private keys, seed phrases, or credential material are stored in user-visible preferences.
- **SC-009**: At least 90% of first-time `LNAuth` testers can identify Alby Go as the recommended test wallet from the info tip without consulting external documentation.
- **SC-010**: The implemented `LNAuth` flow either completes successfully with Alby Go in mobile testing or produces a documented compatibility blocker with a clear fallback decision.

## Assumptions

- `App` mode is the current solution where the app performs user auth and lab actions on behalf of the player; it is useful for development but is not the production-scalable user authorization model.
- `Mock LNAuth` mode is the primary development and automated-test target for the QR-modal user experience because it exercises the modal, blocking, status, and completion paths without requiring a real wallet.
- `LNAuth` mode is the scalable user authorization model for this feature.
- `User Auth Mode` is separate from the existing lab connection mode and from the ordered Polar node workflow. A user may choose `Mock LNAuth` with the current mock/offline connection, or `LNAuth` with the Polar networked connection, as implementation support allows.
- The first `LNAuth` or `Mock LNAuth` QR prompt appears on Play Game entry when no active player login exists; later QR prompts are initially reserved for player sats sends.
- The current Play Game Buy/Sell paths are the first sats-send gates because they already call a single service wrapper that creates/pays the invoice and transfers the TRA.
- Channel opens/closes and TRA ownership transfers remain service-extensible authorization events, but v1 user-facing QR prompts should focus on Play Game login and sats sends unless those operations directly include a sats send.
- QR authentication represents Lightning wallet-based proof of player identity using a standard challenge and signature flow.
- The player's external Lightning wallet is not expected to be created in Polar. Polar remains the local lab backend for Alice, Bob, Carol, Game Treasury, channels, mock/regtest invoices, and TRA state.
- Polar setup must support both fresh and existing networks. Existing user nodes may already hold sats or TRAs, so setup balances user-node targets by transferring to or from Game Treasury instead of assuming every node starts empty.
- In the local/regtest demo, per-send QR approval authorizes the app to execute the corresponding Polar lab payment on behalf of the authenticated player; it does not require the player's production wallet to pay a regtest invoice.
- "Player chain transactions" means player-affecting actions with on-chain or channel consequences, including route/channel open or close work and asset-transfer flows that depend on those transactions.
- Alby Go is the promoted wallet for testing because the user selected it and current public documentation lists it for iOS and Android, describes it as open source, and says it works with Alby Hub or other NWC wallet services.
- Initial scope targets local/regtest demo behavior and avoids mainnet wallet custody.
- Player identity can be represented by a non-sensitive public identifier suitable for display and local status snapshots.
- Another Rust game frontend should be able to consume the service layer through data models and callable operations, while owning its own UI and rendering.
