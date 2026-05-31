# Implementation Plan: TRA Inventory Assets

**Branch**: `005-tap-root-assets-tra` | **Date**: 2026-05-14 | **Spec**: [spec.md](spec.md)  
**Input**: Feature specification from `/specs/005-tra-inventory-assets/spec.md`

## Summary

Add Taproot Assets-backed mock inventory to the existing Lightning learning game. Each node can own at most 3 inventory items, each item is represented by one unique TRA carrying an `item_id`, and the game catalog hardcodes cost and visuals by `item_id` (`item_id=1` is Book). Setup gains a TRA preparation/verification step after Polar Lightning nodes are connected and funded. Play Game must use verified TRA ownership for inventory images, button eligibility, selected item identity, and buy/sell ownership transfer, while keeping sensitive Taproot Assets credentials outside browser storage.

The implementation approach is incremental but not mock-only: preserve the current local `TraService` domain API as the KISS boundary, and implement the server-only Taproot Assets adapter behind it for real Polar/Litd or `tapd` capability verification, mint/discover, transfer, and ownership verification. UI and gameplay call the service through existing `lightning_server_functions` patterns so browser and desktop stay aligned.

## Technical Context

**Language/Version**: Rust workspace using existing project toolchain and Dioxus 0.7.7  
**Primary Dependencies**: Dioxus 0.7 fullstack/router; existing `lightning-service`; existing `tonic_lnd` optional LND boundary; server-only Taproot Assets client boundary using Polar/Litd or `tapd` gRPC/REST integration; any adapter dependency, feature flag, endpoint, and profile/config additions must remain server-only and must be documented in `packages/lightning-service/Cargo.toml` or the existing setup profile model; Serde/Chrono for DTOs  
**Storage**: Existing browser localStorage snapshots for non-sensitive setup/lab state; existing native SQLite remains only for template data; TRA identities and owner summaries are non-sensitive snapshots; Taproot Assets credentials/proofs remain server-side or in local lab tooling  
**Testing**: `cargo test -p lightning-service`, `cargo check -p ui --target wasm32-unknown-unknown`, `cargo check -p desktop`, `.\Scripts\Other\RunTests.ps1`, served-web smoke via `.\Scripts\Common\RunWeb.ps1` when UI changes land  
**Target Platform**: Windows 11 local development; Dioxus web and desktop clients; local Polar regtest with Lightning and Taproot Assets support  
**Project Type**: Dioxus fullstack web/desktop app plus internal Rust service crate  
**Performance Goals**: TRA setup status appears within 1 second of each user action; setup prepares at least 3 TRA-backed mock items within 2 minutes after Lightning nodes are running and funded; final served-web verification records these two timing checks manually; inventory updates render immediately after verified ownership changes; Play Game inventory must not render item images from payment-derived counts or other mock ownership substitutes  
**Constraints**: No mainnet use; no real monetary value; no browser-stored Taproot Assets private keys, macaroons, seeds, or proofs with sensitive material; keep web and desktop support; keep direct LND/TRA access behind `packages/lightning-service`; do not introduce browser SQLite/OPFS  
**Scale/Scope**: Single-user local learning lab; Jack as player, Bob/Carol as NPCs; maximum 3 TRA inventory items per node; first catalog item is Book with `item_id=1`

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- Pass: Dioxus 0.7 remains the UI contract; no plan requires removed APIs such as `cx`, `Scope`, or `use_state`.
- Pass: Shared behavior remains in `packages/ui` and `packages/lightning-service`; `packages/web` and `packages/desktop` remain thin launch surfaces.
- Pass: Setup and trade operations require visible status feedback for TRA verification, minting, ownership checks, transfer attempts, cache reads/writes, and payment attempts.
- Pass: Browser builds continue to use localStorage snapshots and will not introduce browser SQLite or OPFS worker startup.
- Pass: Native database creation remains isolated to `create_database_if_missing()`; this feature does not add destructive schema work.
- Pass: Browser-visible setup and Play Game inventory changes have a served-web verification path.
- Pass: Rust code follows rustfmt, typed `Result` errors, and service boundaries; runtime TRA failures become recoverable states rather than panics.
- Pass: Dioxus UI work will use 0.7 APIs only.
- Pass with note: Constitution navigation says `Set Up`, `Play Game`, `Debug Network`; this feature does not change navigation order.

## Project Structure

### Documentation (this feature)

```text
specs/005-tra-inventory-assets/
├── plan.md
├── spec.md
├── research.md
├── data-model.md
├── quickstart.md
├── checklists/
│   └── requirements.md
├── contracts/
│   └── server-functions.md
└── tasks.md
```

### Source Code (repository root)

```text
packages/
├── lightning-service/
│   └── src/
│       ├── client/
│       │   ├── models.rs          # TRA DTOs, catalog DTOs, LabState inventory snapshot
│       │   ├── error.rs           # TRA domain errors
│       │   ├── lab_service.rs     # Default LabState remains source of game snapshot
│       │   └── tra_service.rs     # KISS/DRY TRA domain service API
│       └── server/
│           ├── lnd_client.rs      # Existing LND boundary remains separate
│           └── tra_client.rs      # Server-only Taproot Assets adapter boundary
├── ui/
│   └── src/client/
│       ├── models.rs
│       ├── pages/
│       │   ├── setup.rs           # Add TRA setup step/status
│       │   └── play_game.rs       # TRA-backed inventory display and buy/sell calls
│       ├── components/
│       │   ├── setup/             # TRA setup controls/status
│       │   └── game/              # TRA-backed inventory item slots/detail states
│       └── services/
│           ├── lightning_server_functions.rs # UI-facing async TRA wrappers
│           └── storage_service.rs            # Non-sensitive TRA snapshot persistence
├── web/
│   └── src/main.rs
└── desktop/
    └── src/main.rs
```

**Structure Decision**: Keep all reusable TRA domain rules in `packages/lightning-service/src/client/tra_service.rs`, with server-only `tra_client.rs` handling real Taproot Assets capability checks, mint/discover, transfer, proof, and owner verification. `packages/ui` owns Dioxus setup/game UI and uses `lightning_server_functions` wrappers. The platform crates remain unchanged except for normal build compatibility.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| None | N/A | N/A |

## Phase 0: Research Summary

See [research.md](research.md). Key decisions:

- Use TRA as the app-facing name for Taproot Assets in this feature.
- Use one-of-one collectible-style assets for inventory instances.
- Store stable game type data such as `item_id=1` in or attached to each TRA; keep price and visuals in the game catalog.
- Put TRA preparation after Polar Lightning node connection/funding and before TRA-backed trading unlocks.
- Keep the KISS `TraService` API as the domain boundary and hide concrete Polar/Litd/`tapd` calls behind a server-only adapter.
- Add a dedicated Polar setup step named "Add Tap Root Assets"; resetting back to this step and submitting again clears the app's saved setup inventory, abandons prior setup item identities for gameplay purposes, and recreates the initial TRA inventory from scratch. The real adapter may reconcile or leave old lab assets in place, but stale identities must not remain valid game inventory unless rediscovered and verified by setup.

## Phase 1: Design Summary

See [data-model.md](data-model.md), [contracts/server-functions.md](contracts/server-functions.md), and [quickstart.md](quickstart.md).

- `TraItem` represents one unique mock inventory instance backed by one TRA.
- `GameItemDefinition` maps `item_id` to hardcoded type, display name, cost, visuals, and trade rules.
- Play Game inventory slots are view models derived from concrete verified `TraItem` instances; they carry `tra_id`, item name, `item_id`, owner, ownership status, transfer status, and catalog visual key.
- `TraService` exposes catalog lookup, setup verification, minting, transfer, and owner inventory operations.
- Setup will mint or discover initial NPC items, verify support, and block progression on unsupported or stale ownership.
- Buy/sell will select a concrete verified `tra_id`, coordinate Lightning payment and TRA ownership transfer, then verify ownership before finalizing visible inventory.
- Existing payment-derived book-count helpers in `PlayGame` are implementation debt and must be removed before this feature is considered complete.

## Post-Design Constitution Check

- Passes Dioxus 0.7 API constraint; Dioxus changes are routed through existing page/component/service patterns.
- Passes shared package constraint by keeping domain logic in `packages/lightning-service` and UI in `packages/ui`.
- Passes local storage/credential constraint by storing only non-sensitive TRA identity/owner summaries in snapshots.
- Passes feedback constraint with required setup/trade loading and failure states.
- Passes web/desktop constraint with shared DTOs and no platform-specific UI dependency.
- Passes verification constraint with service tests, wasm UI check, desktop check, and served-web smoke path.
