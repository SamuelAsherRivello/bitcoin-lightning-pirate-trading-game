# Implementation Plan: Nostr Identity Profile

**Branch**: `010-nostr-profile` | **Date**: 2026-05-23 | **Spec**: [spec.md](spec.md)  
**Input**: Feature specification from `specs/010-nostr-profile/spec.md`

## Summary

Add a Nostr identity-backed profile slice to Play Game without introducing a custom game profile database. The first user-facing surface is a `Profile` button group with `Set Name (Username)`, where the username is empty until known. Clicking opens a username prompt with `Submit` and `Cancel`; submit requires Nostr QR identity authorization and then publishes or stages Nostr profile metadata for the authenticated public key. The implementation keeps Nostr auth separate from Lightning/LNAuth payment approvals, uses Nostr profile metadata as the durable profile source, and stores only non-sensitive local snapshots for display.

## Technical Context

**Language/Version**: Rust 2021 workspace; Dioxus 0.7.7 shared UI.  
**Primary Dependencies**: Existing `dioxus`, `serde`, `chrono`, QR prompt/modal infrastructure, `lightning-service`; planned `nostr-sdk` from the maintained `rust-nostr` crate family for Nostr client, signer, event, and metadata behavior.  
**Storage**: Nostr relays are the primary durable profile store through Nostr metadata events; browser/local snapshots may cache only non-sensitive public identity/profile summaries. No new game profile database table.  
**Testing**: Focused service tests in `packages/lightning-service` or `packages/ui` for profile DTOs, validation, and storage safety; `cargo check -p ui --target wasm32-unknown-unknown`; `cargo check -p web --target wasm32-unknown-unknown`; `cargo check -p desktop`; browser-visible Play Game verification when UI is implemented.  
**Target Platform**: Browser and desktop remain supported. Real mobile QR signing may require a Nostr auth bridge or NIP-46/Nostr Connect-compatible signer path; v1 can use a mock/local adapter while preserving the real boundary.  
**Project Type**: Rust workspace with reusable service DTOs and Dioxus web/desktop UI.  
**Performance Goals**: Opening the Profile prompt should be immediate; profile name display should prefer cached summary on first render and refresh from relays asynchronously; successful local/mock auth and profile update should reflect in UI within 2 seconds.  
**Constraints**: No Nostr private keys, seed phrases, signing secrets, bearer tokens, cookies, or relay auth secrets in browser storage, docs, logs, screenshots, or commits. Do not introduce browser SQLite or OPFS. Keep Lightning auth/payment approvals separate from Nostr identity/profile authorization. Preserve visible loading/toast feedback for auth, relay lookup, and publish attempts.  
**Scale/Scope**: Initial scope is Play Game profile username only: `Profile` group, `Set Name (Username)` button, username prompt, Nostr identity authorization, metadata publish/read contract, safe local snapshot, and dashboard/home docs only if needed for discoverability.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- PASS: UI work will use Dioxus 0.7 APIs only and introduces no removed APIs such as `cx`, `Scope`, or `use_state`.
- PASS: Shared behavior remains in `packages/ui` and reusable identity/profile DTOs can live in `packages/lightning-service`; web and desktop entrypoints stay thin.
- PASS: Nostr auth, relay lookup, publish attempts, cache reads, and cache writes will keep visible loading or toast-style feedback.
- PASS: Browser builds keep localStorage snapshots and do not introduce browser SQLite or OPFS worker startup.
- PASS: First-time native template database setup remains in `create_database_if_missing()`; this feature adds no profile database schema.
- PASS: Browser-visible changes have a practical served-web verification path through the real Play Game route.
- PASS: The feature follows Rust formatting, naming, ownership, and typed error-handling standards.
- PASS: Dioxus code will use `Element`, `#[component]`, signals/resources/memos/context, router-safe components, and existing prompt/modal patterns.

## Phase 0 Research Summary

See [research.md](research.md).

Key decisions:

- Nostr profile metadata is the right durable source for this feature because profile name is public, identity-scoped metadata and does not justify a custom game-owned database.
- Use `nostr-sdk` as the Rust crate starting point because the `rust-nostr` repository describes it as the full-featured SDK, docs show metadata publishing, and docs.rs documents wasm32 target support.
- Keep Nostr identity auth separate from existing Lightning auth. Nostr authorizes profile identity/actions; Lightning authorizes payment and value-moving actions.

## Phase 1 Design Summary

See [data-model.md](data-model.md), [contracts/nostr-profile-service.md](contracts/nostr-profile-service.md), and [quickstart.md](quickstart.md).

## Project Structure

### Documentation (this feature)

```text
specs/010-nostr-profile/
├── plan.md
├── spec.md
├── research.md
├── data-model.md
├── quickstart.md
└── contracts/
    └── nostr-profile-service.md
```

### Source Code (repository root)

```text
packages/
├── lightning-service/
│   └── src/client/
│       ├── models.rs              # NostrIdentity, NostrProfile, NostrProfileEditRequest DTOs
│       └── lab_service.rs         # Profile validation/policy helpers if kept portable
└── ui/
    └── src/client/
        ├── mod.rs                 # App-level Nostr profile/prompt context if needed
        ├── models.rs              # Re-export Nostr profile DTOs for Dioxus code
        ├── pages/play_game.rs     # Profile group and Set Name button
        ├── components/auth/       # Reuse or extend QR prompt surface for Nostr auth
        ├── components/profile/    # Username prompt component if extracted
        └── services/
            ├── lightning_server_functions.rs # Dioxus-safe async wrappers
            ├── nostr_profile_service.rs      # UI/service adapter over nostr-sdk or mock bridge
            └── storage_service.rs            # Non-sensitive profile snapshot persistence
```

**Structure Decision**: Put portable Nostr profile DTOs and validation rules beside existing service-owned game models so Play Game does not own identity policy. Keep Nostr transport/signing/relay operations behind `packages/ui/src/client/services` initially because the auth and QR prompt orchestration must work in browser and desktop. If the Nostr adapter becomes reusable outside Dioxus, move the pure client boundary into `packages/lightning-service` behind feature flags.

## Implementation Sequence

1. Add portable DTOs: `NostrIdentity`, `NostrProfile`, `NostrProfileEditRequest`, `NostrAuthorizationSession`, and profile status/error enums.
2. Add username validation policy and tests: trim, non-empty, max length, display-safe characters, no secret-like values.
3. Add storage snapshot support for public key, npub, username, status, and timestamp; update sensitivity tests to reject private key/seed/token fields.
4. Add Dioxus prompt state and Profile group in Play Game with exact `Profile`, `Set Name ()`, `username:`, `Submit`, and `Cancel` visible copy.
5. Add mock/local Nostr auth adapter that exercises QR prompt UX without real mobile signing, preserving the final real Nostr contract.
6. Integrate `nostr-sdk` for metadata read/publish behind a service wrapper. Use `Client::set_metadata` or equivalent metadata-event flow for username/name.
7. Add browser-visible verification: set name, cancel, invalid name, auth cancel, publish failure, reload with snapshot, and identity switch.
8. Update `Documentation/DioxusFeatureMatrix.md` when implementation changes current storage/auth/route behavior.

## Verification Plan

Focused checks:

```powershell
cargo test -p lightning-service nostr
cargo test -p ui nostr
cargo check -p ui --target wasm32-unknown-unknown
cargo check -p web --target wasm32-unknown-unknown
cargo check -p desktop
```

Browser-visible implementation check:

```powershell
.\Scripts\Common\RunWeb.ps1
```

Verify the Play Game route shows `Profile`, displays `Set Name ()` before profile data exists, opens the username prompt, validates bad names, preserves state on cancel, updates to `Set Name (alice)` after successful mock/real Nostr authorization, and does not trigger Lightning payment approval.

## Complexity Tracking

No constitution violations are currently required.
