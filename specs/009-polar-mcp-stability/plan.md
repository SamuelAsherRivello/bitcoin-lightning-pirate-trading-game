# Implementation Plan: Polar MCP Stability

**Branch**: `main` | **Date**: 2026-05-19 | **Spec**: [spec.md](spec.md)  
**Input**: Feature specification from `specs/009-polar-mcp-stability/spec.md`

## Summary

Install and standardize on the maintained Polar MCP server (`@lightningpolar/mcp`) for networked Polar automation, then refactor the app so every Rust-to-Polar read and mutation travels through one typed, local-only adapter. The user experience must remain the same: the Set Up order, Play Game behavior, Network Dashboard diagnostics, route locking, progress feedback, and mock/offline path stay intact. The internal change targets fewer redundant network reads, idempotent operations, bounded retries, clearer connector health diagnostics, and safer redacted errors.

Current research from `jamaljsr/polar-mcp` shows the package is published as `@lightningpolar/mcp`, requires Node.js 18+, can be launched with `npx -y @lightningpolar/mcp`, communicates with Polar through the local `localhost:37373` bridge, and dynamically discovers 40+ Polar tools including network management, node management, Bitcoin operations, Lightning operations, Taproot Assets operations, asset channels, and asset payments.

## Technical Context

**Language/Version**: Rust 2021 workspace; Dioxus 0.7.7 shared UI; Node.js 18+ for the Polar MCP server.  
**Primary Dependencies**: Existing `dioxus`, `serde`, `serde_json`, `gloo_net`, `lightning-service`; external Polar MCP package `@lightningpolar/mcp` launched by `npx` or installed globally as `polar-mcp`.  
**Storage**: Existing local setup snapshots through `storage_service`; persist only non-sensitive connector URL, selected network identity, operation status, and validated lab snapshots.  
**Testing**: Focused Rust tests for adapter parsing/retry/idempotency; `cargo test -p ui polar` or narrower filters; `cargo check -p ui --target wasm32-unknown-unknown`; `cargo check -p web --target wasm32-unknown-unknown`; `cargo check -p desktop`; live web verification when setup UI or runtime behavior changes.  
**Target Platform**: Browser and desktop remain supported; networked Polar automation remains local/regtest only.  
**Project Type**: Rust workspace with shared service/adapters plus Dioxus web/desktop app.  
**Performance Goals**: Reduce redundant full-network reads by at least 30% within refactored setup steps; improve warm-network setup completion by at least 25% on the same machine; keep connector health checks perceptibly immediate when Polar is running.  
**Constraints**: Do not expose Polar, Lightning RPC, Bitcoin RPC, or connector access outside localhost; do not persist wallet secrets, macaroons, private keys, seed material, cookies, API tokens, or database credentials; preserve visible loading/status feedback; do not require Polar MCP for mock/offline mode; avoid destructive cleanup outside the app-owned demo contract.  
**Scale/Scope**: All existing networked Rust-to-Polar interactions: bridge health, list networks, ensure server, create/start nodes, fund sats, create/verify Taproot Assets node, mint/list/send assets, open/list/close channels, create/pay invoices, mine blocks, dashboard refresh, and setup recovery.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- PASS: UI work will preserve Dioxus 0.7 APIs and avoid removed APIs such as `cx`, `Scope`, or `use_state`.
- PASS: Shared behavior remains in `packages/ui` services and `packages/lightning-service`; web and desktop entrypoints stay thin.
- PASS: Connector checks, setup actions, payments, invoice work, route/channel work, and asset operations will keep visible progress or toast-style feedback.
- PASS: Browser builds keep localStorage snapshots and do not introduce browser SQLite or OPFS worker startup.
- PASS: Native template database setup remains unrelated and stays in `create_database_if_missing()`.
- PASS: Browser-visible changes have a practical served-web verification path.
- PASS: Service changes will use typed results and redacted errors rather than UI-specific string control flow where new boundaries are added.
- PASS: Connector access remains local-only and no secret-bearing data is persisted or shown.

## Installation Plan

1. Add a documented dependency prerequisite for Node.js 18+ and the Polar MCP package.
2. Prefer the no-global-install launch path in setup docs and scripts:

```powershell
npx -y @lightningpolar/mcp
```

3. Keep the global install option as an advanced fallback:

```powershell
yarn global add @lightningpolar/mcp
polar-mcp
```

4. Update repository setup docs/scripts so a developer can verify Polar is running and the local bridge is healthy:

```powershell
curl.exe http://localhost:37373/health
```

5. Do not change global Codex, Claude, SSH, Git, firewall, or operating-system service config. Project scripts may document how to run the connector, but automatic global configuration is out of scope.

## Refactor Plan

1. Inventory every current call in `packages/ui/src/client/services/polar_bridge_service.rs` that directly executes a Polar tool or reads the bridge.
2. Extract one connector boundary with typed methods for health, tool execution, response parsing, retries, timeouts, redaction, and progress reporting.
3. Keep `lightning_server_functions.rs` as the Dioxus-facing orchestration layer, but move raw connector details out of page-facing flows.
4. Add typed operation models for common calls: `list_networks`, `create_network`, `start_network`, `add_node`, `start_node`, `deposit_funds`, `mine_blocks`, `open_channel`, `list_channels`, `create_invoice`, `pay_invoice`, Taproot Assets calls, and health checks.
5. Add a short-lived `PolarStateSnapshot` object used within a setup step to avoid repeated full-network reads after a successful mutation returns enough information or after a fresh validated read already exists.
6. Make all idempotent setup operations check desired state first and treat already-satisfied conditions as success.
7. Replace ad hoc polling loops with a bounded wait helper that stops immediately when the target condition is observed and reports progress on each visible wait.
8. Preserve the current setup labels, route locking, Play Game behavior, Network Dashboard content, and mock/offline path.
9. Keep deletion or destructive Polar operations behind existing app-owned reset/delete actions only; do not expand cleanup behavior as part of this refactor.

## Project Structure

### Documentation (this feature)

```text
specs/009-polar-mcp-stability/
├── plan.md
├── spec.md
└── checklists/
    └── requirements.md
```

### Source Code (repository root)

```text
Scripts/
├── Common/
│   └── InstallDependencies.ps1           # Add/document Node/Polar MCP prerequisite checks if implementation chooses script support
└── Other/
    └── RunTests.ps1                      # Existing verification script remains the broad check

packages/
├── ui/
│   └── src/client/
│       ├── services/
│       │   ├── polar_bridge_service.rs   # Refactor orchestration to use the connector boundary
│       │   ├── lightning_server_functions.rs
│       │   └── storage_service.rs
│       └── pages/
│           ├── setup.rs                  # Preserve visible setup flow; surface connector status if needed
│           └── debug_network.rs          # Preserve diagnostics; add connector health/operation status if useful
└── lightning-service/
    └── src/
        ├── client/models.rs              # Add portable connector status/operation DTOs if needed
        └── client/error.rs               # Add typed, redacted connector failure cases if needed
```

**Structure Decision**: Keep the connector boundary under `packages/ui/src/client/services` because current browser and desktop Polar automation already lives there and depends on Dioxus/web-compatible HTTP execution. Move only portable status/result DTOs into `packages/lightning-service` when they are useful to non-Dioxus callers. Do not move raw MCP transport details into page components.

## Compatibility Notes

- The Polar MCP server dynamically discovers tools from the running Polar app, so the adapter must validate required tools at startup and produce a clear unsupported-tool message if a required tool is missing.
- The server package exposes a stdio MCP interface, while the running Polar app exposes the local HTTP bridge used by current app code. Implementation must verify the exact runtime path before replacing existing calls; if the app continues using Polar's local HTTP bridge directly, the project still standardizes on the same tool names, health check, and package-managed bridge expectations documented by the Polar MCP project.
- Node.js and package installation are development prerequisites only. The browser app must not try to install packages or start local processes.

## Verification Plan

### Baseline Capture

Before changing performance-sensitive behavior, capture a baseline for the current warm-network path:

```powershell
$env:LOG_SERVICE_CALLS_TO_TERMINAL = "true"
.\Scripts\Common\RunWeb.ps1 -NoOpen
```

Then run the networked setup path on an existing Polar demo network and record:

- Total elapsed time from `Bridge URL` submit through `Unlock Routes`.
- Count of `list_networks` calls.
- Count of mutating Polar tool calls.
- Any retry or already-started messages.

Save baseline and after-refactor results in `specs/009-polar-mcp-stability/verification-notes.md`.

1. Unit-test connector response parsing, missing-tool errors, timeout messages, retry classification, and secret redaction.
2. Unit-test idempotent outcomes for existing network, existing nodes, already-started nodes, already-funded wallets, and already-created assets.
3. Compare logged operation counts for at least one warm-network setup path before and after the snapshot/bounded-wait refactor.
4. Run focused checks:

```powershell
cargo test -p ui polar
cargo check -p ui --target wasm32-unknown-unknown
cargo check -p web --target wasm32-unknown-unknown
cargo check -p desktop
```

5. Run the served web app and verify the real setup path when implementation changes user-visible setup, connector health messaging, or dashboard diagnostics.

## Complexity Tracking

No constitution violations are currently required.
