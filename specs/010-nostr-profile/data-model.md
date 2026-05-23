# Data Model: Nostr Identity Profile

## NostrIdentity

Represents the Nostr identity authorized for profile operations.

Fields:

- `public_key`: Hex or canonical internal public key string.
- `npub`: Bech32 public key display string.
- `status`: `Unauthenticated`, `PendingAuth`, `Authenticated`, `AuthFailed`, or `Canceled`.
- `authenticated_at`: Optional timestamp.
- `last_error`: Optional non-sensitive error summary.

Validation:

- Public identity fields must not contain private keys, seed phrases, bearer tokens, or signing secrets.
- `npub` must be derived from the public key or validated as a public bech32 key.

## NostrProfile

Represents public metadata for a Nostr identity.

Fields:

- `public_key`: Public key this profile belongs to.
- `username`: Optional profile name shown in `Set Name (Username)`.
- `source`: `Relay`, `LocalSnapshot`, `PendingPublish`, or `Mock`.
- `publish_status`: `Unknown`, `NotPublished`, `Publishing`, `Published`, or `Failed`.
- `updated_at`: Optional timestamp.
- `relay_urls`: Relays used for read/publish, if known.
- `last_error`: Optional non-sensitive error summary.

Validation:

- Username is trimmed before validation.
- Empty or whitespace-only username is invalid.
- Username should use a conservative UI-safe limit, proposed max 32 Unicode scalar values for v1.
- Username must not include control characters.
- Username must fit the Profile button without layout overlap.

## NostrProfileEditRequest

Represents a pending profile edit from the prompt.

Fields:

- `draft_username`: Current form input.
- `status`: `Editing`, `Validating`, `AwaitingNostrAuth`, `Publishing`, `Succeeded`, `Canceled`, or `Failed`.
- `validation_error`: Optional user-facing validation message.
- `identity_public_key`: Optional target public key once auth completes.
- `requested_at`: Timestamp.

State transitions:

- `Editing` -> `Canceled` when Cancel is clicked.
- `Editing` -> `Validating` when Submit is clicked.
- `Validating` -> `Editing` when validation fails.
- `Validating` -> `AwaitingNostrAuth` when input is valid and identity auth is required.
- `AwaitingNostrAuth` -> `Publishing` when Nostr auth succeeds.
- `AwaitingNostrAuth` -> `Failed` or `Canceled` when auth fails/cancels.
- `Publishing` -> `Succeeded` after relay publish or accepted mock publish.
- `Publishing` -> `Failed` after relay publish failure.

## NostrAuthorizationSession

Represents QR-backed authorization for Nostr identity/profile actions.

Fields:

- `session_id`: Local opaque identifier.
- `action`: `Login` or `SetProfileName`.
- `qr_payload`: URI or payload rendered in the QR modal.
- `status`: `Pending`, `Scanned`, `Approved`, `Rejected`, `Expired`, `Canceled`, or `Failed`.
- `public_key`: Optional public key result.
- `expires_at`: Timestamp.
- `last_error`: Optional non-sensitive error summary.

Validation:

- QR payloads and logs must not include private keys or signing secrets.
- Expired or canceled sessions cannot publish profile changes.

## Local Snapshot

The browser/native app may persist only:

- Public key/npub.
- Username.
- Profile source/status.
- Last updated timestamp.
- Non-sensitive relay URLs.

It must not persist:

- Nostr private keys.
- Seed phrases.
- Signing secret material.
- Authorization bearer tokens.
- Cookies.
- Relay auth secrets.
