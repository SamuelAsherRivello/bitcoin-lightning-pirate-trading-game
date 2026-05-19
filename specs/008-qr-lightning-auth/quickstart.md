# Quickstart: QR Lightning Auth Mode

## Planning Notes

1. Keep `packages/lightning-service` as the reusable Rust library boundary for QR auth, route/channel work, invoices, payments, and TRA operations.
2. Add `UserAuthMode`, auth/session DTOs, and QR modal DTOs before adding UI so service tests can exercise `App`, `Mock LNAuth`, and `LNAuth` transitions without Dioxus.
3. Add serde defaults for new `SetupProfile` and `LabState` fields so existing localStorage/native snapshots keep loading.
4. Add an `auth_client` adapter boundary in `packages/lightning-service/src/server` before choosing the final dependency implementation.
5. Evaluate `lnurl-rs` 0.9.0 behind the adapter; do not expose crate-specific types through public app/service contracts.
6. Extend Dioxus wrappers in `packages/ui/src/client/services/lightning_server_functions.rs` after the portable service contract exists.
7. Add a QR prompt signal/context in `AppLayout` and render the QR modal from `PageHeader`, beside the existing toast and operation prompt regions.
8. Update `Set Up`, `Play Game`, `Network Dashboard`, and `Home` as UI consumers of the service state.
9. Keep browser localStorage limited to non-sensitive auth mode and status snapshots.

## Suggested Verification

```powershell
.\Scripts\Other\RunTests.ps1
```

Focused service checks during implementation:

```powershell
cargo test -p lightning-service
```

Browser-visible verification when UI changes are implemented:

```powershell
.\Scripts\Common\RunWeb.ps1
```

Verify in the served app:

- Set Up starts with `User Auth`, showing exactly `App`, `Mock LNAuth`, and `LNAuth`.
- Set Up labels `App` as the development convenience path and `LNAuth` as the scalable authorization path.
- Set Up labels `Mock LNAuth` as the automated-wallet development path.
- Set Up shows an info icon next to `LNAuth` that promotes Alby Go as the primary Android/iOS test wallet.
- Set Up keeps `User Auth` separate from the Polar workflow, then uses this Polar order: `Bridge URL`, `Server Name`, `Create Nodes`, `Game Treasury (Sats)`, `Game Treasury (TRAs)`, `User Nodes (Sats)`, `User Nodes (TRAs)`, `Block Height`, `Unlock Routes`.
- `Create Nodes` finds or creates the Bitcoin backend, Game Treasury, Taproot Assets node, two NPC nodes, and one player node before later funding or inventory steps run.
- `Create Nodes` requests creation first, then checks all required node statuses; after several readiness retries, it restarts the Polar network once and checks again.
- `User Nodes (Sats)` balances sats to or from Game Treasury until the user nodes have the right balances, while allowing Game Treasury to keep extra sats.
- `User Nodes (TRAs)` balances TRA ownership to or from Game Treasury until the user nodes have the right TRA inventory, while allowing Game Treasury to keep extra TRAs.
- User Auth is separate from the existing `Polar Connection (Networked)` / `Mock Connection (Offline)` connection tabs.
- Play Game shows the darkened QR modal on first arrival when `Mock LNAuth` or `LNAuth` is selected and player auth is incomplete.
- Play Game shows the login QR modal after its existing sat/TRA refresh has completed or produced a connected lab state.
- The QR modal title is `Scan with wallet`, the action description appears above the QR code, the QR code is centered, and `Cancel` leaves the protected action incomplete.
- `Mock LNAuth` login and send QR modals auto-complete after one second unless canceled.
- Open Trade, Close Trade, Buy Item, Sell Item, and direct sats-send actions preserve current behavior in development-only `App` mode.
- `Mock LNAuth` and `LNAuth` gate Play Game login and the Buy/Sell sats sends before completion.
- `LNAuth` does not prompt for low-risk actions such as navigation, dashboard refreshes, inventory inspection, channel/TRA display, or educational content.
- Network Dashboard shows auth mode, player identity fingerprint, auth session state, and approval history without exposing secrets.
- Home explains `App` and `LNAuth` at a concept level, including why Alby Go is the test wallet and why the external wallet is not a Polar node.
- Mobile QA verifies the exact `LNAuth` QR flow with Alby Go, or records a compatibility blocker and fallback decision.

Implementation order:

1. Finish and test `App` mode with the new User Auth selector.
2. Finish and test `Mock LNAuth` end-to-end, including modal overlay, cancel, login auto-complete, sats-send auto-complete, service tests, and browser QA.
3. Implement real `LNAuth` last using the same modal/service path, then pause for Alby Go mobile validation.

Current status:

- `App` mode remains the no-QR development path.
- `Mock LNAuth` has the shared QR modal, one-second login completion, one-second Buy/Sell send approval, cancel handling, approval history recording, and a trade-service guard that requires approved authorization before value-moving trades run.
- `LNAuth` uses the same modal surface but still needs the real Alby Go callback validation step before protected sends can complete.

## Dependency Caution

Before implementation, inspect the final dependency tree for `lnurl-rs` or any QR rendering crate. Prefer dependency features that keep WASM/browser and desktop support intact, and keep protocol verification behind the `lightning-service` adapter so another Rust game can consume the same behavior.
