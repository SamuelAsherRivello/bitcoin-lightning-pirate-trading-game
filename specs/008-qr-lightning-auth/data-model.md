# Data Model: QR Lightning Auth Mode

## UserAuthMode

Represents whether the lab uses the existing development-only app-managed path, mock LNAuth, or real LNAuth-backed player identity and scalable authorization.

Fields:

- `mode`: `App`, `MockLNAuth`, or `LNAuth`
- `is_development_only`: true for `App`
- `is_mock`: true for `MockLNAuth`
- `requires_player_auth`: derived boolean
- `requires_authorization_event_approval`: derived boolean for consequential player actions
- `recommended_wallet_tip`: optional display-safe wallet recommendation for `LNAuth`

Validation:

- `App` mode must never require QR challenge completion and must be treated as development-oriented.
- `MockLNAuth` mode must show the same QR modal UX as `LNAuth` and auto-complete after one second unless canceled.
- `LNAuth` mode must not unlock gameplay until player auth is accepted.

## QrAuthorizationModal

Represents the reusable user-facing modal for login and send approval QR prompts.

Fields:

- `modal_id`
- `title`: expected default `Scan with wallet`
- `description`: action-specific body text shown above the QR code
- `qr_payload`
- `qr_kind`: `Login` or `SendSats`
- `amount_sats`
- `status`: `Open`, `MockCompleting`, `Approved`, `Canceled`, `Expired`, `Failed`
- `can_cancel`: true
- `opened_at`
- `auto_complete_after_ms`: `1000` for `MockLNAuth`, absent for real `LNAuth`

Validation:

- The modal must darken the current page and center the QR code.
- Canceling the modal must leave the protected action incomplete.
- Mock auto-completion must not apply after cancel.

## WalletRecommendationTip

Represents the display-safe info-tip content shown next to `LNAuth`.

Fields:

- `wallet_name`: `Alby Go`
- `platforms`: `Android`, `iOS`
- `recommendation_reason`
- `official_links`
- `fallback_note`
- `compatibility_status`: `PendingValidation`, `Validated`, or `Blocked`

Validation:

- The tip must recommend exactly one default wallet.
- The tip must not imply that the user's wallet is imported into Polar.
- The tip must make the wallet recommendation display-safe and free of affiliate or tracking content.
- The tip must not claim full compatibility until Alby Go has completed the exact implemented `LNAuth` flow in mobile testing.

## PlayerAuthSession

Represents a short-lived LNURL-auth challenge lifecycle.

Fields:

- `session_id`
- `challenge_id`
- `lnurl`
- `qr_payload`
- `action`: `login`, `register`, `link`, or `auth`
- `status`: `Created`, `Displayed`, `Approved`, `Expired`, `Rejected`, `Failed`, `Canceled`
- `expires_at`
- `player_identity`
- `failure_reason`

Validation:

- A challenge can be accepted only once.
- A stale challenge must not authenticate the player after a replacement challenge is created.
- Expired, rejected, failed, and canceled sessions are recoverable by creating a fresh challenge.

State transitions:

```text
Created -> Displayed -> Approved
Created -> Displayed -> Expired
Created -> Displayed -> Rejected
Created -> Displayed -> Failed
Created -> Displayed -> Canceled
Expired|Rejected|Failed|Canceled -> Created
```

## PlayerIdentity

Non-sensitive representation of the authenticated player.

Fields:

- `linking_key_fingerprint`
- `display_label`
- `authenticated_at`
- `last_seen_at`

Validation:

- Store only a public key or fingerprint suitable for display/debugging.
- Never store seed phrases, private keys, wallet descriptors, node private material, or macaroons.

## AuthorizationEvent

Represents a key event where a real user would expect to approve value movement or durable state changes.

Fields:

- `event_id`
- `event_kind`: `PlayerLogin`, `SendSats`, `PayInvoice`, `OpenRoute`, `CloseRoute`, `TransferAsset`, `OtherValueMovingAction`
- `summary`
- `requires_qr_approval`
- `risk_level`: `Low`, `ValueMoving`, `DurableStateChange`

Validation:

- Navigation, dashboard refresh, inventory inspection, channel display, TRA display, and educational content are low-risk and must not require QR approval.
- Initial v1 user-facing prompts are Play Game login and sats sends.
- Channel opens/closes and durable ownership transfers are extension points unless they directly include a player sats send.

## TransactionApproval

Represents a QR-backed approval gate for an authorization event.

Fields:

- `approval_id`
- `operation_kind`: `SendSats`, `PayInvoice`, `OpenRoute`, `CloseRoute`, `TransferAsset`, `ChannelFunding`, `OtherPlayerChainAction`
- `operation_summary`
- `player_identity`
- `status`: `NotRequired`, `Required`, `Pending`, `Approved`, `Expired`, `Rejected`, `Failed`
- `created_at`
- `expires_at`
- `approved_at`
- `failure_reason`

Validation:

- `LNAuth` mode requires `Approved` before completing authorization events.
- `MockLNAuth` mode requires the same gate but auto-produces `Approved` after the modal's one-second mock completion.
- `App` mode returns `NotRequired` and preserves the fast path.
- Approval is bound to the requested operation summary and authenticated player.

## LightningOperationResult

Portable result envelope for authentication, route/channel work, invoice creation, payment, and asset transfer.

Fields:

- `operation_id`
- `operation_kind`
- `status`: `Succeeded`, `ApprovalRequired`, `Pending`, `RecoverableFailure`, `Failed`
- `updated_lab_state`
- `auth_session`
- `approval`
- `message`
- `error_code`

Validation:

- Results must be consumable without Dioxus route, toast, page, or component types.
- UI adapters may translate messages into toasts/prompts, but service results stay UI-neutral.

## PolarSetupStep

Represents one ordered step in the Polar lab setup workflow.

Fields:

- `step_id`: `BridgeUrl`, `ServerName`, `CreateNodes`, `GameTreasurySats`, `GameTreasuryTras`, `UserNodesSats`, `UserNodesTras`, `BlockHeight`, or `UnlockRoutes`
- `order`
- `status`: `Locked`, `Current`, `Complete`, `NeedsRetry`, or `Failed`
- `readiness_summary`
- `last_error`

Validation:

- The user-facing order must be exactly `Bridge URL`, `Server Name`, `Create Nodes`, `Game Treasury (Sats)`, `Game Treasury (TRAs)`, `User Nodes (Sats)`, `User Nodes (TRAs)`, `Block Height`, and `Unlock Routes`.
- `Create Nodes` completes only after all required Polar nodes are found or created and report started/ready status: Bitcoin backend, Game Treasury, Taproot Assets, one player node, and two NPC nodes.
- `Create Nodes` may restart the Polar network once after repeated readiness retries, then must report success or a recoverable failure.
- `User Nodes (Sats)` completes when target user-node sats balances are met, regardless of extra sats remaining in Game Treasury.
- `User Nodes (TRAs)` completes when target user-node TRA inventory is met, regardless of extra TRAs remaining in Game Treasury.

## SetupProfile Extensions

Existing setup profile should gain non-sensitive auth fields.

Fields:

- `user_auth_mode`
- `player_identity`
- `last_auth_status`

Validation:

- New fields must use serde defaults so older setup-profile snapshots continue to load.
- `user_auth_mode` is independent from the existing `setup_mode` connection field.
- Browser storage may persist mode and display-safe status only.
- Changing user auth mode invalidates connected gameplay until the setup/auth requirements are revalidated.

## LabState Extensions

Existing lab state should expose auth and approval summaries for gameplay and diagnostics.

Fields:

- `player_auth_session`
- `recent_transaction_approvals`
- `auth_warnings`

Validation:

- New fields must use serde defaults so older lab snapshots continue to load.
- Recent approval history must be bounded.
- Auth/session fields must not include wallet secrets.
