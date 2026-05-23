# Research: Nostr Identity Profile

## Decision: Use Nostr Profile Metadata Instead Of A Custom Game DB

**Decision**: Store the username as Nostr profile metadata tied to the authenticated Nostr public key. The app may keep a non-sensitive local snapshot for fast display, but Nostr relays remain the durable profile source.

**Rationale**: The feature is profile identity, not game-owned account management. Nostr already models public identity metadata by public key, so adding a game database would create a second profile authority, require account linking, and introduce migration/security work that is not needed for a display name.

**Alternatives considered**:

- Custom game DB table: rejected for v1 because it adds ownership and sync complexity for one public profile field.
- Browser localStorage only: rejected because it is local to one browser/device and does not represent the user's Nostr identity.
- Lightning auth profile field: rejected because Lightning auth is for payment/value authorization in this app, while profile identity is a distinct Nostr use case.

## Decision: Use `nostr-sdk` From `rust-nostr`

**Decision**: Use the `nostr-sdk` crate family as the primary Rust dependency for Nostr client, signer, event, and metadata behavior.

**Rationale**: The `rust-nostr` repository lists `nostr-sdk` as the full-featured SDK for building Nostr applications and includes related signer crates such as NIP-07 browser signer and NIP-46 Nostr Connect. The docs.rs page for `nostr-sdk` shows profile metadata construction and `client.set_metadata(&metadata).await?`, and documents wasm32 target support. Those are the exact capabilities needed for browser/desktop Dioxus profile editing.

Sources:

- Rust Nostr Book getting started: https://rust-nostr.org/sdk/getting-started.html
- rust-nostr repository project structure and supported NIPs: https://github.com/rust-nostr/nostr
- `nostr-sdk` docs.rs: https://docs.rs/nostr-sdk/latest/nostr_sdk/

**Alternatives considered**:

- Lower-level `nostr` crate only: useful for protocol primitives, but v1 needs a client and metadata publish flow.
- Hand-rolled Nostr event signing/publishing: rejected because signing, relays, NIPs, and wasm compatibility are easy to get wrong.
- JavaScript-only Nostr client: rejected because the app's reusable service boundary is Rust-first and must preserve desktop support.

## Decision: Keep Nostr Auth Separate From Lightning Auth

**Decision**: Add a separate Nostr authorization session type and QR prompt mode for profile edits, rather than reusing Lightning transaction approval DTOs.

**Rationale**: Lightning auth in this app authorizes payments and value-moving operations. Nostr auth proves or delegates control of a Nostr public identity for profile metadata. Sharing visible modal styling is fine, but sharing business state would couple unrelated trust models and produce confusing UX.

**Alternatives considered**:

- Reuse LNAuth sessions directly: rejected because it would make profile edits look like payment approvals.
- Require Nostr login before Play Game navigation: rejected because profile editing is optional and should not block gameplay.

## Decision: Start With Username/Name Only

**Decision**: Treat `username` as the Nostr metadata `name` field for v1.

**Rationale**: The requested UI is `Set Name (Username)` and a `username:` form. More metadata fields such as picture, about, display name, and NIP-05 can be planned later without changing the initial service boundary.

**Alternatives considered**:

- Full profile editor: rejected because it expands UI, validation, and relay behavior beyond the requested first slice.
- Game-only alias separate from Nostr metadata: rejected because it undermines the no-custom-game-DB goal.
