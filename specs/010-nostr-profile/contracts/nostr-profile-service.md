# Contract: Nostr Profile Service

This contract describes the Dioxus-facing service boundary for the Nostr profile feature. Function names are descriptive and can be adapted to existing project naming during implementation.

## `get_nostr_profile_summary`

Reads the current non-sensitive profile summary for Play Game.

Request:

```rust
pub struct GetNostrProfileSummaryRequest {
    pub preferred_relays: Vec<String>,
    pub allow_local_snapshot: bool,
}
```

Response:

```rust
pub struct GetNostrProfileSummaryResponse {
    pub identity: Option<NostrIdentity>,
    pub profile: Option<NostrProfile>,
    pub is_loading_from_relay: bool,
}
```

Rules:

- Must not block Play Game route rendering.
- May return local snapshot first and refresh relay state asynchronously.
- Must not expose secret material.

## `start_nostr_profile_authorization`

Starts QR-backed Nostr authorization for a profile action.

Request:

```rust
pub struct StartNostrProfileAuthorizationRequest {
    pub action: NostrProfileAction,
    pub draft_username: Option<String>,
}

pub enum NostrProfileAction {
    Login,
    SetProfileName,
}
```

Response:

```rust
pub struct StartNostrProfileAuthorizationResponse {
    pub session: NostrAuthorizationSession,
}
```

Rules:

- The QR payload must describe Nostr identity/profile authorization, not Lightning payment approval.
- Canceling the session leaves profile state unchanged.
- Mock/local mode must exercise the same modal state transitions.

## `submit_nostr_profile_name`

Validates and publishes a username for the authenticated Nostr identity.

Request:

```rust
pub struct SubmitNostrProfileNameRequest {
    pub session_id: String,
    pub username: String,
    pub preferred_relays: Vec<String>,
}
```

Response:

```rust
pub struct SubmitNostrProfileNameResponse {
    pub identity: NostrIdentity,
    pub profile: NostrProfile,
}
```

Rules:

- Must reject empty, whitespace-only, too-long, or control-character usernames before publishing.
- Must require an approved Nostr authorization session.
- Must publish Nostr metadata using the authenticated public key/signing path.
- Must update local non-sensitive snapshot only after successful publish or explicit mock success.
- Must preserve the previous username on failure.

## `cancel_nostr_profile_edit`

Cancels the active prompt/session.

Request:

```rust
pub struct CancelNostrProfileEditRequest {
    pub session_id: Option<String>,
}
```

Response:

```rust
pub struct CancelNostrProfileEditResponse {
    pub profile_unchanged: bool,
}
```

Rules:

- Must not publish metadata.
- Must close the prompt and leave the Profile button value unchanged.

## UI Contract

Play Game must render:

- Group label: `Profile`
- Button label when no name exists: `Set Name ()`
- Button label when name exists: `Set Name (alice)`
- Prompt label: `username:`
- Prompt buttons: `Submit`, `Cancel`

The Profile prompt must be independent from Lightning buy/sell approval prompts.
