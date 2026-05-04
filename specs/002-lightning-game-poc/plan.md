# Implementation Plan: Bitcoin Lightning Game POC

**Branch**: `002-lightning-game-poc` | **Date**: 2026-05-02 | **Spec**: [spec.md](spec.md)  
**Input**: Feature specification from `/specs/002-lightning-game-poc/spec.md`

## Summary

Replace the template pages with a Dioxus 0.7 fullstack learning app that starts with a Home page explaining why the demo exists and Bitcoin/Lightning FAQ concepts, guides a learner through a local Polar regtest setup, creates Alice/Bob/Carol Lightning nodes from an existing Polar Bitcoin backend through Polar's localhost bridge, unlocks a playful Lightning trading game, and exposes a network dashboard view that proves the underlying channel, invoice, payment, and block-confirmation operations.

## Technical Context

**Language/Version**: Rust workspace using existing project toolchain and Dioxus 0.7.7  
**Primary Dependencies**: Dioxus 0.7 fullstack/router; `tonic_lnd` for server-side LND gRPC access; Tokio async runtime; Serde for contracts; existing localization/theme/storage services  
**Storage**: Existing local storage service for non-sensitive setup preferences and Polar automation values; existing native SQLite remains only for template/local app data until replaced by feature data needs  
**Testing**: `.\Scripts\Other\RunTests.ps1`, targeted `cargo test`, web smoke test through `.\Scripts\Common\RunWeb.ps1`, desktop smoke test through `.\Scripts\Common\RunDesktop.ps1` when practical  
**Target Platform**: Windows 11 local development; Dioxus web and desktop clients; local server-side service connecting to Polar LND regtest nodes  
**Project Type**: Dioxus fullstack web/desktop app plus internal Rust service crate  
**Performance Goals**: Setup test should complete in under 5 seconds against running Polar; invoice/payment status refresh should show user feedback within 1 second for local regtest operations; UI animations should remain responsive during async network calls  
**Constraints**: Never require mainnet credentials; do not expose macaroons in browser storage by default; keep all production-spend actions out of scope; keep web and desktop support; no browser SQLite/OPFS for this feature  
**Scale/Scope**: Single-user local learning app controlling one existing Polar Bitcoin backend and three app-created local demo LND nodes: Alice, Bob, Carol

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- Dioxus 0.7 remains the UI contract. New UI must use `Element`, `#[component]`, `use_signal`, `use_resource`, `use_memo`, context, and `Router::<Route>`.
- Shared behavior remains in `packages/ui`; platform entrypoints stay thin in `packages/web` and `packages/desktop`.
- Loading and toast-style feedback must be added for setup testing, connection saves, node creation/destruction, node refreshes, invoice creation, payment attempts, channel opening, and block mining.
- Browser builds keep localStorage snapshots for non-sensitive setup preferences and do not add browser SQLite or OPFS.
- Existing native database creation constraints are preserved until feature data replaces template data in a later implementation task.
- Browser-visible changes require served-web verification when implemented.
- Runtime code must use meaningful `Result` errors and avoid panics/unwraps in LND operations.
- Dioxus code must use 0.7 APIs only.

**Known Constitution Conflict**: The current constitution says to preserve top navigation order `Page01`, `Page02`, `Page03`, but the new feature requires `Home`, `Set Up`, `Play Game`, and `Network Dashboard`. Implementation must update `.specify/memory/constitution.md`, `AGENTS.md`, localization assets, and `Documentation/DioxusFeatureMatrix.md` as part of the first task group before route replacement.

## Project Structure

### Documentation (this feature)

```text
specs/002-lightning-game-poc/
├── plan.md
├── spec.md
├── research.md
├── data-model.md
├── quickstart.md
├── checklists/
│   └── requirements.md
└── contracts/
    └── server-functions.md
```

### Source Code (repository root)

```text
packages/
├── lightning-service/             # Rust service crate with client-safe DTOs and server-only LND boundary
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── client/
│       │   ├── error.rs           # Domain error mapping
│       │   ├── lab_service.rs     # Alice/Bob/Carol state operations
│       │   └── models.rs          # Shared service DTOs
│       └── server/
│           ├── config.rs          # Local profile loading and credential validation
│           └── lnd_client.rs      # tonic_lnd adapter boundary
├── ui/
│   ├── src/client/
│   │   ├── app.rs                 # Router and app shell
│   │   ├── models.rs              # UI-facing game/network models
│   │   ├── pages/
│   │   │   ├── setup.rs
│   │   │   ├── play_game.rs
│   │   │   ├── debug_network.rs
│   │   │   └── home.rs
│   │   ├── components/
│   │   │   ├── setup/
│   │   │   ├── game/
│   │   │   └── network/
│   │   └── services/
│   │       ├── setup_profile_service.rs
│   │       └── lightning_server_functions.rs
│   └── assets/
│       ├── i18n/
│       └── styling/
├── web/
│   └── src/main.rs
└── desktop/
    └── src/main.rs
```

**Structure Decision**: Add `packages/lightning-service` as a workspace member, keep client-safe DTOs and deterministic lab state operations under `src/client`, and keep local profile loading plus all direct LND calls under `src/server`. `packages/ui` owns routes, visual state, localization, and Dioxus server-function wrappers. `packages/web` and `packages/desktop` remain launch surfaces.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| Add `packages/lightning-service` crate | The app needs a server-only boundary around LND credentials and typed Lightning operations | Putting LND calls in UI code risks browser credential exposure and mixes network operations with presentation |
| Replace `Page01/Page02/Page03` route set | The product concept requires `Home`, `Set Up`, `Play Game`, and `Network Dashboard` | Keeping placeholder page names would make the learning flow unclear |

## Phase 0: Research Summary

See [research.md](research.md). Key decisions:

- Use Polar regtest as the required first environment.
- Use LND nodes Alice, Bob, and Carol.
- Use `tonic_lnd` as the first Rust LND gRPC client.
- Keep LDK/`ldk-node` out of v1 because the app should control existing Polar LND nodes rather than embed a new node.
- Use Dioxus server functions to cross the client/server boundary.

## Phase 1: Design Summary

See [data-model.md](data-model.md) and [contracts/server-functions.md](contracts/server-functions.md).

- `SetupProfile` defines the selected connection tab, configured sats-per-transaction amount, and UI-entered Polar regtest bridge URL for the local POC.
- `PolarAutomationProfile` defines the local Polar bridge URL used by the app-owned `Polar Connection (Networked)` form, plus discovered network/backend values saved after bridge discovery.
- `TradeRoute` maps game language to LND channels.
- `InvoiceRequest`, `PaymentAttempt`, and `BlockWaitAction` drive the learning loop.
- Server functions expose domain operations: test setup, get lab state, open route, wait for next block, create invoice, pay invoice, autosend.

## Implementation Notes

- First implementation lets learners paste the throwaway Polar bridge URL directly in the `Polar Connection (Networked)` tab instead of manually editing setup files.
- Polar automation must stay visibly labeled for local regtest use only and must never be presented as a production wallet permission model.
- Use exact page labels requested by the user: `Home`, `Set Up`, `Play Game`, `Network Dashboard`.
- Use plain readable personas and locations: Alice, Bob, Carol; Town, Desert, Beach, Mountain.
- Treat receive as `Create Invoice`; do not imply receive happens without a payer.
- Treat send as `Pay Invoice`.
- Treat channel open as `Open Trade Route`; pending as `Under Construction`; confirmation as `Wait for Next Block`.
- The FAQ should live on `Home` with the `Why this demo exists` content, include short Bitcoin and Lightning summaries with learn-more links, compare Bitcoin and Lightning pros/cons, and preserve the "LND Operation vs Needs Block" table in simplified form.

## Post-Design Constitution Check

- Passes Dioxus 0.7 API and shared UI constraints.
- Requires an intentional constitution update for the route names before implementation.
- Passes storage constraint by keeping sensitive credentials server-side and non-sensitive setup preferences local.
- Passes platform constraint by keeping both web and desktop support.
- Passes verification constraint with quickstart validation steps for web, desktop, and Polar-backed operations.
