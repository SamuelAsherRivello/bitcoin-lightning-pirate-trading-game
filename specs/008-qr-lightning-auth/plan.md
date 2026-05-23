# Implementation Plan: QR Lightning Auth Mode

**Branch**: `main` | **Date**: 2026-05-18 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `specs/008-qr-lightning-auth/spec.md`

## Summary

Add a `User Auth` selector before the existing Polar bridge flow without replacing the current `Connection` tabs. `App` preserves the current no-QR behavior as a development convenience where the app acts on behalf of the player. `Mock LNAuth` uses the same QR-modal UX as real LNAuth, but auto-completes each QR prompt after one second so the feature can be built and tested without a phone wallet. `LNAuth` is the scalable authorization path: the player authenticates with an external Lightning wallet on Play Game entry, then sees QR-backed approval for player sats sends. The implementation should add portable auth/approval DTOs and policy in `packages/lightning-service`, expose Dioxus-safe async wrappers from `packages/ui/src/client/services/lightning_server_functions.rs`, and keep page code limited to displaying mode/status and opening the QR modal.

Research found no published Rust crate named `lnauth` through `cargo search`. The closest maintained candidate is `lnurl-rs` 0.9.0, whose upstream README lists `lnurl-auth` support and whose docs expose async/blocking clients. For server-side verification, this feature should still keep a thin project-owned auth boundary so `lnurl-rs`, direct `secp256k1` verification, or a different LNURL utility can be swapped without leaking into Dioxus pages.

## Technical Context

**Language/Version**: Rust 2021 workspace; Dioxus 0.7.7 router-based UI.  
**Primary Dependencies**: Existing `dioxus`, `lightning-service`, `serde`, `chrono`, Polar bridge wrappers, optional `tonic_lnd`; planned evaluation of `lnurl-rs` 0.9.0 plus a QR rendering crate behind UI/service adapters.  
**Storage**: Existing `SetupProfile` and `LabState` snapshots through `storage_service`; native SQLite remains only for template data setup through `create_database_if_missing()`; user auth mode, mock-auth status, and QR auth state store only non-sensitive challenge/session status.  
**Testing**: Repository PowerShell test script `.\Scripts\Other\RunTests.ps1`, focused `cargo test -p lightning-service`, and browser-visible web verification through `.\Scripts\Common\RunWeb.ps1` when UI changes land.  
**Target Platform**: Browser and desktop remain supported; QR-authenticated demo mode targets local/regtest/Polar development first and does not introduce mainnet custody.  
**Project Type**: Rust workspace with reusable service crate plus Dioxus web/desktop app.  
**Performance Goals**: QR challenge generation and status updates should be perceptibly instant locally; successful wallet callbacks should update auth state within the spec's 5 second goal.  
**Constraints**: No wallet secrets, macaroons, seed material, or private keys in browser localStorage; no browser SQLite or OPFS worker; Dioxus pages must not own LNURL verification, TRA transfer rules, invoice/payment rules, authorization-event policy, or route/channel policy; the player's external wallet does not need to be created as a Polar node; the QR modal cancel path exists for development convenience and must leave the protected action incomplete.  
**Scale/Scope**: Primary routes are `Home`, `Set Up`, `Play Game`, and `Network Dashboard`; current setup is being reorganized so the Polar workflow runs `Bridge URLs`, `Server Name`, `Create Nodes`, `Game Treasury (Sats)`, `Game Treasury (TRAs)`, `User Nodes (Sats)`, `User Nodes (TRAs)`, `Block Height`, and `Unlock Routes`; current gameplay sends sats through `execute_tra_item_trade` for Buy/Sell and debug sends through invoice/payment helpers.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- PASS: UI work will use Dioxus 0.7 APIs only and introduces no removed APIs such as `cx`, `Scope`, or `use_state`.
- PASS: Shared behavior remains in `packages/ui`; platform entrypoints in `packages/web` and `packages/desktop` stay thin.
- PASS: Authentication, approval, invoice, payment, route/channel, and asset operations will keep visible loading or toast-style feedback in the app.
- PASS: Browser builds keep localStorage snapshots and do not introduce browser SQLite or OPFS worker startup.
- PASS: Native database creation remains in `create_database_if_missing()`; this feature does not require destructive or implicit schema setup.
- PASS: Browser-visible setup/game/dashboard changes can be verified against the served web app.
- PASS: Service additions will use idiomatic Rust results and errors rather than panics or UI-specific string control flow.
- PASS: Dioxus code will continue to use `Element`, `#[component]`, signals/resources/memos/context, `Router::<Route>`, and `asset!`.

## Project Structure

### Documentation (this feature)

```text
specs/008-qr-lightning-auth/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── lightning-auth-service.md
└── checklists/
    └── requirements.md
```

### Source Code (repository root)

```text
packages/
├── lightning-service/
│   ├── Cargo.toml                         # Add auth/lnurl/QR-adjacent dependencies when implementation starts
│   └── src/
│       ├── lib.rs                         # Re-export portable Lightning game modules
│       ├── client/
│       │   ├── models.rs                  # UserAuthMode, PlayerAuthSession, PlayerIdentity, AuthorizationEvent, modal/approval DTOs
│       │   ├── lab_service.rs             # Apply auth/authorization-event policy to existing lab operations
│       │   └── tra_service.rs             # Keep TRA rules service-owned and UI-independent
│       └── server/
│           ├── auth_client.rs             # LNURL-auth challenge/verification adapter boundary
│           ├── lnd_client.rs              # Existing LND observer/payment boundary
│           └── tra_client.rs              # Existing Taproot Assets adapter boundary
├── ui/
│   └── src/client/
│       ├── models.rs                      # Re-export service DTOs only
│       ├── mod.rs                         # Add app-level QR prompt signal/context beside existing toast/operation prompt
│       ├── pages/
│       │   ├── setup.rs                   # Add User Auth selector and LNAuth info tip before existing Polar steps
│       │   ├── play_game.rs               # Trigger login/send QR modal and gate Buy/Sell sends
│       │   ├── debug_network.rs           # Show auth sessions/approvals and gate debug send actions later
│       │   └── home.rs                    # Add concise concept/FAQ copy for QR auth mode
│       ├── components/
│       │   └── auth/qr_authorization_modal.rs # Reusable darkened overlay modal for login/send QR
│       └── services/
│           ├── lightning_server_functions.rs # Dioxus-facing async wrappers over portable services
│           └── storage_service.rs             # Persist non-sensitive auth mode/status snapshots only
├── web/
│   └── assets/main.css                    # QR/auth panel styling if needed
└── desktop/
    └── assets/main.css                    # Keep desktop visual parity with web assets
```

**Structure Decision**: Add serializable auth state to `lightning-service` first because `SetupProfile` and `LabState` live there and are re-exported by `packages/ui/src/client/models.rs`. Keep Dioxus integration in `packages/ui/src/client/services`, and limit pages to displaying state, opening QR prompts, and invoking service wrappers. This preserves a reusable library boundary for a future Bevy project.

## Page Modification Map

- `Set Up`: add `User Auth` as a connection-level selector before the nested `Environment` / `Polar` setup panels. Do not remove or repurpose `SetupMode::{ServerConfig, BrowserRegtestOnly}`; those remain the Polar-vs-offline lab connection modes. `User Auth` has exactly three buttons: `App`, `Mock LNAuth`, and `LNAuth`. Add an info icon next to `LNAuth` using the existing `FieldHelpIcon` pattern; the tip should promote Alby Go as the primary Android/iOS test wallet and explain that it works with Alby Hub or another NWC wallet service. The Polar wizard visual order is `Bridge URL`, `Server Name`, `Create Nodes`, `Game Treasury (Sats)`, `Game Treasury (TRAs)`, `User Nodes (Sats)`, `User Nodes (TRAs)`, `Block Height`, `Unlock Routes`.

## Polar Setup Reorganization

The Polar setup flow must separate topology creation from value balancing:

1. `Bridge URLs`: verify the Polar bridge URL exactly as before; when `LNAuth` mode is selected, also verify the LNAuth bridge URL before continuing.
2. `Server Name`: find/create/start the named Polar network exactly as before.
3. `Create Nodes`: find or create all required Polar nodes before any funding, minting, or user rebalancing: Bitcoin backend, Game Treasury LND node, `GAME_TAPROOT` Taproot Assets node, Alice/player LND node, Bob NPC LND node, and Carol NPC LND node. This step inspects first; if the required node types already exist and the visible network nodes are started/running, it returns ready and reports extras without cleanup. If required nodes are missing, it creates only the missing required nodes. For Taproot Assets, setup finds by proper node type first: an existing generated Taproot Assets node such as `GAME_LND-tap` or `tapd` is accepted and preserved even though the preferred name is `GAME_TAPROOT`. Only when no Taproot node exists does setup create one with the preferred `GAME_TAPROOT` name. It restarts the Polar network after successful topology changes such as creates or cleanup removals, starts only nodes that are not already started/running, then polls every required node for started/ready status. If repeated readiness retries still show unstable or not-started nodes, restart the Polar network once and re-check readiness.
4. `Game Treasury (Sats)`: fund or top up Game Treasury sats after all node shells exist.
5. `Game Treasury (TRAs)`: create/verify treasury-owned TRA inventory after `GAME_TAPROOT` exists.
6. `User Nodes (Sats)`: transfer sats to or from Game Treasury until the player/NPC user nodes have the target sats balances. The step must work with fresh or existing networks, so Game Treasury may retain extra sats after rebalancing.
7. `User Nodes (TRAs)`: transfer TRAs to or from Game Treasury until the player/NPC user nodes have the target TRA inventory. The step must work with fresh or existing networks, so Game Treasury may retain extra TRAs after rebalancing.
8. `Block Height`: save the app block-height baseline.
9. `Unlock Routes`: revalidate health and mark the setup connected.

Implementation implication: the current `User Nodes` and `NPC Item Transfers` concepts should be split. Node creation belongs only in `Create Nodes`; user sats balancing belongs in `User Nodes (Sats)`; user TRA inventory balancing belongs in `User Nodes (TRAs)`. The old one-way `NPC Item Transfers` label should not remain as the user-facing step name because the new flow must be able to rebalance from existing networks, not only distribute starting items from an empty state.

Polar lifecycle implication: the live Polar MCP tool schema documents `rename_node` as temporarily stopping a running network, `set_lightning_backend` as restarting the affected Lightning node when the network is running, and `remove_node` as stopping the removed node. Setup recovery must therefore prefer readiness checks over topology normalization. Do not rename, remove, rebackend, stop, or restart a manually prepared network when the required exact-name nodes and backend relationships are already usable.
- `Play Game`: preserve the existing route-entry `preview_tra_setup` refresh. After the route is connected and state exists, if `Mock LNAuth` or `LNAuth` is selected and no active player login exists, show the QR modal. Gate the current user-facing sats sends: Buy Item and Sell Item both call `execute_tra_item_trade`, which creates/pays an invoice and transfers TRA ownership. In `Mock LNAuth` and `LNAuth`, the QR/send approval must complete before this wrapper performs the payment/transfer. Navigation, inventory inspection, route panels, game treasury panel, channel/TRA display, and educational copy should not trigger QR approval. Existing `App` mode behavior remains the development fast path.
- `Network Dashboard`: add rows or panels for current auth mode, player identity fingerprint, active auth session, and recent approval attempts. Existing route, invoice, payment, and TRA tables stay intact. Debug send/payment actions can be gated after Play Game buy/sell is feature-complete.
- `Home`: add FAQ/concept copy explaining `App`, `Mock LNAuth`, and `LNAuth`, why the external wallet is not a Polar node, and why the demo keeps secrets out of browser storage.
- `PageHeader`/route locking: keep the current connection-based locking using `SetupProfile::is_connected()`. Do not block Play Game navigation just because LNAuth login is missing; Play Game owns the login QR modal on first arrival.

## QR Modal UX

Every QR request uses the same modal prompt. The current code already has `OperationPromptRegion` rendered from `PageHeader`, but QR auth needs separate state because it includes a QR payload and cancel means "leave this protected auth/send action incomplete" rather than the existing long-running Polar cancel semantics.

- The modal floats over the current page and darkens the background.
- The modal content is centered and focuses on the QR code.
- Title: `Scan with wallet`.
- Body text appears above the QR code and describes the exact action, such as `Log in to start playing` or `You are sending 1,000 sats`.
- The QR code is centered below the body text.
- A `Cancel` button is present for development convenience; canceling closes the modal and leaves the login or send incomplete.
- `Mock LNAuth` shows the same modal, waits one second, then completes the prompt as if a wallet responded successfully, unless the user canceled first.

Implementation detail: add a new QR prompt signal/context in `AppLayout` and render `QrAuthorizationModal` from `PageHeader` near `ToastRegion` and `OperationPromptRegion`. Reuse overlay CSS patterns where appropriate, but keep the data model separate from `OperationPrompt`.

## Wallet And Polar Reconciliation

`LNAuth` should not require the user's real wallet to exist on Polar. Polar remains the local lab backend that creates and operates Game Treasury, Alice, Bob, Carol, regtest channels, invoices, payments, and TRA state. The player's external wallet supplies identity and approval signatures. In the local demo, a QR approval for "send sats" authorizes the app to execute the corresponding Polar/regtest payment on behalf of the authenticated player; it is not a production Lightning payment from the user's wallet into the Polar network.

This keeps development practical and avoids trying to bridge production wallet liquidity into a fake Polar instance. A future production mode can add a separate adapter where the user's wallet actually pays or signs spend requests, but that should be modeled separately from the Polar lab backend.

## Authorization Event Policy

`App` mode is explicitly a development convenience and should not be described as production-scalable. `Mock LNAuth` is the feature-completion path for development and automated QA because it exercises the same modal UX without requiring a phone. `LNAuth` should feel scalable by prompting for QR approval only at consequential moments:

- Initial player login.
- Player sats send or invoice payment.
- Future extension points: channel open/close, durable TRA ownership transfer, or other player-affecting operation that moves value, changes custody, or creates durable network state.

For v1 user-facing behavior, trigger the login modal on Play Game entry and trigger send approval for player sats sends. Do not trigger QR approval for navigation, route entry after login, dashboard refresh, inventory inspection, channel display, TRA display, educational content, theme/language choices, or other low-risk reads. This keeps LNAuth from feeling noisy while still matching a real authorization mental model.

## Implementation Sequence

1. Add `UserAuthMode` and auth/session/approval DTOs to `packages/lightning-service/src/client/models.rs` with `#[serde(default)]` fields on `SetupProfile` and `LabState` so old snapshots still load.
2. Update `storage_service` sensitivity assertions to include auth/session/approval fields and prevent secret-like strings from being persisted.
3. Reorganize the Polar wizard and service boundaries around `Create Nodes`, `Game Treasury (Sats)`, `Game Treasury (TRAs)`, `User Nodes (Sats)`, and `User Nodes (TRAs)` before adding QR-auth prompts, because QR-auth work should sit on top of the stable setup flow.
4. Implement and test `App` mode unchanged except for the new visible `User Auth` selector and development labeling.
5. Implement `Mock LNAuth` feature-complete: setup selector persistence, Play Game login modal after `preview_tra_setup`, one-second auto-complete, Buy/Sell send modal, one-second auto-complete, cancel behavior, service-level auth/approval results, and browser-visible QA.
6. After `Mock LNAuth` passes service tests and user-perspective browser tests, implement real `LNAuth` using the same service and modal contract.
7. Defer final real-wallet validation until the end because it requires user-assisted Alby Go mobile testing.

## Latest Implementation Status

As of the latest codebase pass, the app already has the `User Auth` selector, current nine-step Polar setup order, portable auth/session DTOs, and snapshot safety checks. This implementation pass adds the app-level QR prompt context, reusable centered QR modal, QR rendering dependency, Mock LNAuth login auto-completion on Play Game entry, Mock LNAuth Buy/Sell send approval auto-completion, and a service-level guard in `execute_tra_item_trade` so LNAuth-mode trades cannot create/pay invoices or transfer TRA ownership without an approved `TransactionApproval`.

Real `LNAuth` now uses the same modal surface and a repo-owned `lnauth-bridge` callback service. The bridge creates phone-reachable LNURL-auth sessions, verifies wallet callback signatures, and lets the Dioxus app poll until the player identity or send approval is confirmed. In real `LNAuth`, protected sends execute only after the scanned wallet signs with the same linking key used for login.

## Wallet Recommendation

Promote ZEUS in the `LNAuth` info tip as the primary test wallet. Current ZEUS documentation says it is available on Android, iOS, and F-Droid and lists full LNURL support including auth. Phone scanning still requires the local `lnauth-bridge` callback URL to be reachable from the phone.

Suggested tip text:

```text
Testing wallet: ZEUS. It works on Android and iOS and its docs list LNURL auth support. Use it to scan LNAuth login and key-event approval QR codes. Your wallet stays outside Polar; Polar only runs the local lab nodes.
```

Fallback rule: if mobile testing shows ZEUS cannot complete the exact local LNURL-auth callback flow, document the blocker and choose a fallback wallet or HTTPS/tunnel setup rather than silently shipping a broken recommendation.

## Complexity Tracking

No constitution violations are currently required.
