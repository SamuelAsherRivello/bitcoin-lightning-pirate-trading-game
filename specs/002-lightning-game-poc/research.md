# Research: Bitcoin Lightning Game POC

## Decision: Use Polar Regtest As The Required First Environment

**Rationale**: Polar already runs on the user's Windows 11 machine with Docker Desktop and provides local Bitcoin Core plus LND nodes. Regtest lets the app mine blocks instantly, reset state, and avoid public testnet faucet/liquidity problems. This directly supports the learning mechanic where `Wait for Next Block` mines a local block.

**Alternatives considered**:

- Public testnet/signet: More realistic but introduces faucet, liquidity, confirmation, and wallet-support friction.
- Hosted LND provider: Useful later, but requires account setup and creates real credential risk earlier than needed.
- WSL2 hand-built LND/Bitcoin Core: Educational, but slower and less appropriate for a pretty Dioxus app POC.

## Decision: Use LND Nodes Alice, Bob, And Carol

**Rationale**: The user already created a Polar network using LND. LND has a broad API surface, common node-manager compatibility, and practical docs. Three nodes support the teaching model: Alice as player, Bob as Beach merchant, Carol as Mountain merchant.

**Alternatives considered**:

- Core Lightning nodes: Good implementation, but the user flow and Polar setup are already LND-oriented.
- Mixed LND/Core Lightning network: Useful as a future interoperability lesson, but too much for v1.
- Location-owned nodes: It weakens the important concept that nodes are controlled by wallets, people, companies, or services.

## Decision: Use A Server-Only Rust LND gRPC Adapter

**Rationale**: LND exposes gRPC and REST APIs. A typed Rust gRPC adapter gives compile-time contracts for node operations and keeps macaroon/TLS handling out of browser code. `cargo search` currently shows `tonic_lnd = "0.5.1"` as the latest matching Rust LND tonic/prost client. `cargo info tonic_lnd` identifies it as an async library implementing LND RPC via tonic and prost with docs and repository links. `voltage-tonic-lnd = "0.4.0"` is a Voltage-maintained fork with feature flags and Rust 1.75 metadata; it remains a fallback if implementation reveals a compatibility advantage.

**Alternatives considered**:

- LND REST with `reqwest`: Easier to inspect manually, but less type-safe and still requires careful macaroon/TLS handling.
- `ldk-node`: Current crate is available and Rust-native, but it embeds/builds a Lightning node rather than controlling the existing Polar LND lab. Better for a future "wallet/node from scratch" project.
- LDK/`rust-lightning`: Powerful and production-used, but too low-level for this learning app's immediate goal.

## Decision: Add `packages/lightning-service`

**Rationale**: A dedicated service crate gives the app a clean boundary:

```text
Dioxus UI -> Dioxus server functions -> lightning-service -> Polar LND nodes
```

The crate can normalize LND details into app operations: test setup, open trade route, wait for next block, create invoice, pay invoice, list channels, list invoices, and get balances.

**Alternatives considered**:

- Put all service code in `packages/ui`: Simpler file count, but mixes UI and privileged node control.
- Add a standalone binary service immediately: More operational setup than needed. A library crate can be used first by Dioxus server functions and later wrapped as a separate binary if useful.

## Decision: Store Sensitive Credentials Server-Side By Default

**Rationale**: Macaroons can authorize spending or administration. The app's teaching model controls all demo nodes, which is acceptable only in lab mode. Browser local storage is appropriate for non-sensitive preferences such as transaction amount and "setup complete" hints, but not ideal for macaroon material.

**Alternatives considered**:

- Store all node connection details in browser local storage: Convenient for a throwaway regtest lab, but teaches a dangerous habit.
- Require environment variables only: Safer but less approachable for a setup page. A local config file gives a better learning path.

## Decision: Use Dioxus Server Functions For UI Operations

**Rationale**: The project is already Dioxus 0.7, and server functions are the natural client/server boundary. UI components can call domain functions like `create_invoice` or `open_trade_route` without knowing LND connection details.

**Alternatives considered**:

- Build a separate HTTP API first: More boilerplate and less aligned with the existing Dioxus fullstack template.
- Direct browser calls to LND REST: Not acceptable by default because it would expose node credentials and CORS/TLS concerns to the browser.

## Decision: Use "Trade Route" As The Channel Analogy

**Rationale**: A channel is between nodes/personas, not places. "Trade Route: Alice <-> Bob" keeps the game language readable while preserving the person-to-person node relationship.

**Alternatives considered**:

- Path to Beach/Mountain: Friendly but risks teaching that channels are location-to-location.
- Credit line: Closer to liquidity but sounds like debt.
- Bridge: Clear pending/open metaphor but too place-oriented.

## Decision: Use `Wait for Next Block`

**Rationale**: Fixed "Wait 10 minutes" wording is inaccurate because Bitcoin blocks arrive about every 10 minutes on average, not exactly every 10 minutes. In regtest, the app can mine instantly. `Wait for Next Block` is both accurate and game-readable.

**Alternatives considered**:

- `Wait 10 Minutes`: Easier to understand but technically misleading.
- `Mine Block`: Accurate in regtest but less friendly in game mode.
