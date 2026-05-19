# Research: QR Lightning Auth Mode

## Decision: Treat LNURL-auth as the QR authentication protocol

**Rationale**: The LNURL LUD-04 specification describes a QR-based service challenge with a 32-byte `k1`, wallet-signed `secp256k1` signature, compressed public key, one-time challenge cache, and `register | login | link | auth` actions. That maps directly to "QR auth for the player" and can also be used for approving sensitive actions through the `auth` action.

**Alternatives considered**:

- Custom QR token: simpler locally, but would not work with compatible Lightning wallets.
- Node pubkey login: rejected because LUD-04 explicitly discourages using the plain Lightning node key as identity.
- WebAuthn: strong general authentication, but it does not satisfy the Lightning wallet QR-auth requirement.

## Decision: No `lnauth` Rust crate was found; evaluate `lnurl-rs` behind a project-owned adapter

**Rationale**: `cargo search lnauth --limit 10` returned no published crate. `cargo search lnurl` and `cargo info lnurl-rs` identify `lnurl-rs` 0.9.0 as the practical Rust candidate. Its upstream README says it supports plaintext, TLS, Onion, blocking/async, WASM, and `lnurl-auth`; docs.rs shows async/blocking clients and a dependency set including `bitcoin`, `bech32`, `reqwest`, `ureq`, and `url`. Keeping it behind `packages/lightning-service/src/server/auth_client.rs` avoids binding Dioxus or future Bevy code to one crate's exact types.

**Alternatives considered**:

- `lnurl` 0.2.0: older and documented around withdrawal/service response types rather than broad LNURL-auth support.
- `fedimint-lnurl`: useful lower-level LNURL utilities from a strong project, but not clearly a complete app-level auth server boundary by itself.
- Direct `secp256k1` implementation only: viable for server-side verification, but a thin adapter can still use direct verification internally while letting higher-level code remain stable.

## Decision: Add service-level auth and approval state to `lightning-service`

**Rationale**: Current `SetupMode`, `SetupProfile`, `LabState`, invoice/payment DTOs, route state, and TRA DTOs already live in `packages/lightning-service/src/client/models.rs`, while UI models re-export them. Existing route/channel/payment functions live in `lab_service.rs`, and TRA behavior lives in `tra_service.rs` plus `server/tra_client.rs`. QR auth should follow that same pattern so a Bevy frontend can call the same domain operations without Dioxus pages, route navigation, or toast state.

**Alternatives considered**:

- Implement QR state directly in `setup.rs`: fastest for UI, but violates the portability goal and would duplicate policy for Bevy.
- Put auth in `packages/ui/src/client/services` only: keeps Dioxus pages thinner, but still leaves the reusable library incomplete.
- Create a separate workspace crate immediately: possible later if `lightning-service` becomes too broad, but the existing crate is already the project's Lightning library boundary.

## Decision: Keep `User Auth` before setup, but make the Polar workflow start at `Bridge URL`

**Rationale**: The auth mode affects later gameplay authorization, so it remains a selector before Polar setup. The current code already uses `SetupMode` for lab connection choice (`Polar Connection (Networked)` vs `Mock Connection (Offline)`), so user auth should be a separate `UserAuthMode` rather than another `SetupMode` variant. `App` keeps the current fast development behavior and should be labeled as development-focused, not production-scalable. `Mock LNAuth` exercises the same QR modal and blocking flow as real LNAuth while auto-completing prompts after one second for development and automated testing. `LNAuth` turns on real wallet-backed player QR login and sats-send approvals.

The Polar node workflow itself should start at `Bridge URL`, then `Server Name`, then `Create Nodes`. This reflects the newer stability workaround: create or find all required Polar nodes before funding, minting, or balancing. `Create Nodes` owns the Bitcoin backend, Game Treasury LND node, Taproot Assets node, one player LND node, and two NPC LND nodes. It requests creation, polls for started/ready status, retries readiness checks, and restarts the Polar network once when repeated readiness retries indicate unstable node startup. Later steps operate on already-created nodes.

The resulting Polar order is `Bridge URL`, `Server Name`, `Create Nodes`, `Game Treasury (Sats)`, `Game Treasury (TRAs)`, `User Nodes (Sats)`, `User Nodes (TRAs)`, `Block Height`, and `Unlock Routes`. `Game Treasury (Sats)` funds or tops up treasury sats. `Game Treasury (TRAs)` mints or verifies treasury-owned TRA inventory. `User Nodes (Sats)` rebalances sats between Game Treasury and user nodes until user targets are met. `User Nodes (TRAs)` rebalances TRA ownership between Game Treasury and user nodes until user inventory targets are met.

**Alternatives considered**:

- Put `User Auth` after Polar node creation: rejected because the mode changes whether later actions need player approval and would make setup state harder to explain.
- Put `User Auth` inside Play Game only: rejected because setup and later protected service calls need to know the selected auth policy before gameplay starts, even though route locking itself should remain based on setup connection status.
- Keep node creation split across Game Treasury/User Nodes/TRA steps: rejected because Polar startup is more stable when all node shells are created and verified before later funding, minting, and rebalancing operations.
- Require exact Game Treasury depletion during user rebalancing: rejected because setup must work against both fresh and existing networks where Game Treasury and user nodes may already hold sats or TRAs.
- Replace the current setup mode selector: rejected because `App` mode must remain a first-class development path, even though it is not the scalable production-style authorization model.
- Build real `LNAuth` first: rejected because the feature can be made user-complete and regression-tested with `Mock LNAuth` before requiring user-assisted mobile wallet testing.
- Add `Mock LNAuth` as `SetupMode::BrowserRegtestOnly`: rejected because connection mode and auth mode are orthogonal in the current codebase.

## Decision: Keep external player wallet separate from Polar lab nodes

**Rationale**: The player's real wallet should not need to exist in Polar. Polar is a fake/local lab backend for creating Alice, Bob, Carol, Game Treasury, channels, regtest invoices/payments, and TRA state. `LNAuth` should use the external wallet only for identity and approval signatures. In local/regtest mode, QR approval for a sats send means "I authorize the app to perform this Polar lab payment on my behalf", not "my production wallet pays this regtest invoice."

**Alternatives considered**:

- Import or create the user's wallet in Polar: rejected because it mixes real wallet identity with fake lab infrastructure and creates an unsafe custody expectation.
- Require a production wallet to pay Polar invoices: rejected because production Lightning and Polar regtest are different networks.
- Keep QR auth only as initial login: rejected because the user wants any sats send to require QR again.

## Decision: Promote Alby Go as the LNAuth testing wallet

**Rationale**: The user selected Alby Go for testing. Current Alby documentation says Alby Go is available on iOS and Android, is open source, works with Alby Hub or any NWC wallet service, and can authorize new app connections to Alby Hub from a phone. That fits the desired mobile approval UX and makes it the primary wallet to test first.

**Compatibility note**: The Alby Go pages found during planning describe NWC and app-connection authorization, but do not explicitly state that Alby Go itself completes LNURL-auth callbacks. Implementation must validate the exact `LNAuth` QR flow with Alby Go. If it cannot complete the chosen flow, the project should document that blocker and either adapt the flow through Alby Hub/NWC where appropriate or choose a fallback wallet for the visible recommendation.

**Alternatives considered**:

- ZEUS: appears in the LUD-04 auth list and its docs state full LNURL support, but the user wants Alby Go for testing.
- Breez: also appears in the LUD-04 auth list and has Android/iOS distribution, but is not the selected test wallet.
- Phoenix: appears in the LUD-04 auth list, but is not the selected test wallet.
- Wallet of Satoshi: historically widely used and appears in LNURL lists for some specs, but it ended U.S. service/app-store availability, so it is a poor default for this project's expected U.S. context.
- Listing many wallets: rejected because the UI requirement is to reduce ambiguity by promoting one default.

## Decision: Use one QR modal pattern for login and send approval

**Rationale**: A floating modal that darkens the page and centers the QR code keeps the user's attention on the authorization action regardless of which page initiated it. The modal should include a title, action-specific body text above the QR, a centered QR code, and a cancel button. The cancel button is a development convenience and should leave the login or send incomplete.

The current app already renders global overlays from `PageHeader` through `ToastRegion` and `OperationPromptRegion`. QR auth should follow that app-level pattern with its own prompt state and component instead of overloading `OperationPrompt`, because the QR prompt needs a QR payload and its cancel behavior is immediate action cancellation rather than "request cancel and wait for Polar to undo".

**Alternatives considered**:

- Inline QR panels inside setup or gameplay: rejected because the user wants every QR need to appear as a focused overlay.
- Toast-only approval: rejected because QR scanning needs a large visible target and explicit action context.
- No cancel button: closer to production UX, but rejected because the project needs a development escape hatch.
- Reuse `OperationPrompt` directly: rejected because the existing prompt is tuned for long-running Polar operations and lacks QR/modal-specific payload fields.

## Decision: Model v1 authorization events as operation gates, not as UI button state

**Rationale**: `LNAuth` mode should feel scalable by asking for approval when the user would expect to authorize value movement. For v1, the user-facing QR prompts are Play Game login and player sats sends. In the current code, Play Game Buy/Sell routes both through `execute_tra_item_trade`, which creates and autosends an invoice and then transfers TRA ownership. That wrapper is the first practical sats-send gate. Channel open/close and durable TRA ownership transfer remain service extension points, but should not create visible QR prompts unless they directly include a player sats send. The operation gate belongs in service logic so `Play Game`, `Network Dashboard`, and future Bevy callers get the same behavior. UI buttons can show pending state, but the service should return `ApprovalRequired`, `ApprovalPending`, `ApprovalExpired`, `Canceled`, or `ApprovalRejected` results before completing protected sends.

**Alternatives considered**:

- Disable buttons until auth is complete only: insufficient because approvals may be per-event and can expire.
- Let each page decide which actions require approval: duplicates security policy and risks inconsistent behavior.
- Require QR approval for channel/chain actions immediately: deferred because the user-facing v1 should focus on login and sats sends, with channel/TRA as service extension points.
- Require QR approval on every click: rejected because it would make `LNAuth` noisy and less production-like; low-risk reads should not need wallet authorization.

## Decision: Store only non-sensitive auth snapshots locally

**Rationale**: The project constitution and current `storage_service` pattern require localStorage to hold only non-sensitive preferences and snapshots. LNURL-auth uses public linking keys and signatures; the app should persist at most auth mode, masked/fingerprinted player identity, challenge status, expiration timestamps, and recent approval summaries. Challenges and used `k1` values should be short-lived and treated as server/service state.

**Alternatives considered**:

- Persist full callback payloads for debugging: rejected because signatures and identifiers should not be over-collected.
- Persist wallet secrets or app-generated private keys: explicitly out of scope and unsafe.

## Sources

- [LNURL LUD-04 auth specification](https://github.com/lnurl/luds/blob/luds/04.md)
- [LNURL LUD wallet matrix](https://github.com/lnurl/luds)
- [lnurl-rs docs.rs crate page](https://docs.rs/lnurl-rs/latest/lnurl/)
- [lnurl-rs GitHub README](https://github.com/benthecarman/lnurl-rs)
- [Breez SDK LNURL-auth guide](https://sdk-doc-spark.breez.technology/guide/lnurl_auth.html)
- [Alby Go product page](https://getalby.com/alby-go)
- [Alby Go guide](https://guides.getalby.com/user-guide/alby-go)
