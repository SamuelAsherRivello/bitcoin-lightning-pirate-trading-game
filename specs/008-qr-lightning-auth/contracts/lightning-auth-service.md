# Contract: Portable Lightning Auth And Operation Service

This contract describes service-level behavior for `packages/lightning-service`. Dioxus pages and future game frontends call wrappers around these operations and render their own UI.

## Auth Challenge

### `begin_player_auth(profile, action) -> PlayerAuthSession`

Creates a short-lived player authentication challenge.

Inputs:

- `profile`: current setup profile
- `action`: `login`, `register`, `link`, or `auth`

Returns:

- `PlayerAuthSession` with `session_id`, challenge payload, QR payload, expiration, and `Created` or `Displayed` status

Errors:

- `AuthModeNotEnabled`
- `AuthServiceUnavailable`
- `InvalidCallbackBaseUrl`

Rules:

- Do not include wallet secrets in the result.
- A new challenge invalidates older pending challenges for the same player setup context.
- `Mock LNAuth` creates the same session shape with a mock payload and auto-completion metadata.

## Auth Callback

### `complete_player_auth(session_id, callback_payload) -> LightningOperationResult`

Verifies a wallet response for a pending challenge.

Inputs:

- `session_id`
- callback payload containing challenge id, public key, signature, and action metadata

Returns:

- `LightningOperationResult` with `Succeeded` and `PlayerIdentity` when verification passes
- `RecoverableFailure` for expired, mismatched, rejected, or invalid signatures

Rules:

- Verify that the challenge is known, unused, and not expired.
- Verify the signature against the challenge and public key.
- Remove or mark the challenge used after a successful verification.

## Transaction Approval

### `authorize_player_operation(profile, lab_state, operation) -> LightningOperationResult`

Applies LNAuth approval policy before a configured authorization event completes.

Inputs:

- `profile`
- `lab_state`
- `operation`: event kind, player identity, affected send/route/item/payment fields, and display-safe summary

Returns:

- `Succeeded` or `NotRequired` equivalent when no QR approval is needed
- `ApprovalRequired` with a new approval challenge when QR approval is required
- `RecoverableFailure` when auth is missing, expired, or mismatched

Rules:

- `App` mode must preserve the current fast path.
- `Mock LNAuth` mode must use the same service state and approval contract as real `LNAuth`, but produce a mock success after the QR modal has been open for one second unless canceled.
- `LNAuth` mode must require an authenticated player before creating approvals.
- Approval must be bound to the player and operation.
- Low-risk reads, navigation, dashboard refreshes, channel/TRA display, and educational content must not require QR approval.
- The initial implementation gates the Play Game Buy/Sell wrapper before it creates/pays invoices and transfers TRA ownership.

## QR Modal Contract

### `build_qr_modal(session_or_approval) -> QrAuthorizationModal`

Returns the UI-neutral data needed to render the QR modal.

Rules:

- The Dioxus UI owns the visual overlay, darkened background, and centered QR rendering.
- The service owns modal title, action description, QR payload, expiry, status, and mock auto-completion metadata.
- Login body text should explain the login action, such as `Log in to start playing`.
- Send body text should include the value movement detail, such as `You are sending 1,000 sats`.
- Canceling the modal returns a canceled service result and leaves the protected operation incomplete.

## Lightning Operations

The existing operations should call the approval contract when required:

- `open_trade_route`
- `close_trade_route`
- `create_invoice`
- `pay_invoice`
- `create_invoice_and_maybe_autosend`
- `transfer_tra`

Rules:

- Service functions return data-only results and errors.
- Dioxus wrappers may translate results into prompts, toasts, navigation, and page state.
- Bevy or another frontend should be able to call the same service functions without importing Dioxus.
- `App` mode is for development convenience and must not be presented as the production-scale authorization model.
- V1 user-facing QR prompts are required for Play Game login and player sats sends. Other listed operations remain service extension points unless they perform a player sats send.
- The current Play Game Buy/Sell service path is `execute_tra_item_trade`; it is the first wrapper that should be split or guarded so approval happens before invoice payment and TRA transfer.

## Polar Setup Operations

The setup service should expose or wrap operations that match the visual Polar order:

- `verify_bridge_url`
- `ensure_server_name`
- `create_required_nodes`
- `prepare_game_treasury_sats`
- `prepare_game_treasury_tras`
- `rebalance_user_node_sats`
- `rebalance_user_node_tras`
- `confirm_block_height`
- `unlock_routes`

Rules:

- `create_required_nodes` owns topology readiness only. It finds or creates the Bitcoin backend, Game Treasury LND node, `GAME_TAPROOT` Taproot Assets node, one player LND node, and two NPC LND nodes, then verifies all required node statuses before value setup begins.
- `create_required_nodes` requests creation before readiness checks, restarts the Polar network after node topology changes, retries readiness checks, and restarts the Polar network at most once more for startup instability before returning success or a recoverable failure.
- `prepare_game_treasury_sats` funds or tops up Game Treasury after required nodes exist.
- `prepare_game_treasury_tras` creates or verifies treasury-owned TRA inventory after `GAME_TAPROOT` exists.
- `rebalance_user_node_sats` transfers sats to or from Game Treasury until the user-node sats targets are met; exact Game Treasury balance is not a completion criterion.
- `rebalance_user_node_tras` transfers TRAs to or from Game Treasury until the user-node TRA targets are met; exact Game Treasury inventory is not a completion criterion.

## Persistence Contract

Persistable local fields:

- selected user auth mode
- player public identity fingerprint/display label
- auth status
- challenge expiration/status summaries
- bounded approval history summaries

## Polar Lab Boundary

Rules:

- The player's external Lightning wallet is used for LNAuth identity and approval only.
- The player's wallet is not created as a Polar LND node.
- In local/regtest mode, an approved sats send authorizes the app to execute a Polar lab payment with the configured lab nodes.
- A future production payment adapter must be introduced separately if the user's wallet will actually pay real invoices.

Forbidden local fields:

- private keys
- seed phrases
- wallet descriptors
- macaroons
- TLS private material
- full credential payloads
- database dumps or backup archives
