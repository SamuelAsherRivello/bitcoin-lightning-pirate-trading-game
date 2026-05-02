# Feature Specification: Nostr Identity Profile

**Feature Branch**: `010-nostr-profile`  
**Created**: 2026-05-23  
**Status**: Draft  
**Input**: User description: "Create a new feature that uses Nostr identities. The QR code will be used to auth as a separate feature. Using that, allow the user to use Nostr identities with that auth so they can set custom profile info. For now, create a Profile button group in Play Game with Set Name (Username), where username is empty until set. Clicking opens a prompt with a username form plus Submit and Cancel. This must use the Nostr identities. Confirm this is the best way to offer a profile without a custom game DB, confirm the best Rust crate, then make a plan."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Set Profile Name With Nostr Identity (Priority: P1)

As a player on Play Game, I can open a Profile control and set my display username through my authenticated Nostr identity, so the game can show player profile information without creating a separate game-owned profile database.

**Why this priority**: This is the smallest useful Nostr profile slice: it proves identity-backed profile edits, visible gameplay integration, and the no-custom-game-DB approach.

**Independent Test**: Can be tested by selecting `Set Name ()`, completing Nostr QR authorization, entering a username, submitting, and seeing the button update to `Set Name (jack)` without adding a game DB table.

**Acceptance Scenarios**:

1. **Given** Play Game is available and no profile name is set, **When** the player views the Profile group, **Then** the group shows a `Set Name ()` button.
2. **Given** the player clicks `Set Name ()`, **When** the Nostr QR auth succeeds and the player submits `jack`, **Then** the app publishes or stages Nostr profile metadata for the authenticated public key and updates the button to `Set Name (jack)`.
3. **Given** the player clicks `Cancel` in the username prompt, **When** the prompt closes, **Then** the existing profile name remains unchanged.

---

### User Story 2 - Reuse Existing Nostr Profile Name (Priority: P2)

As a returning player, I can see the username already associated with my Nostr identity, so I do not need to re-enter profile information every session.

**Why this priority**: A profile identity is only useful if the app can recover current metadata from Nostr relays or a safe local snapshot after authorization.

**Independent Test**: Can be tested by authenticating with a Nostr identity that already has metadata and confirming the Profile button renders the recovered name.

**Acceptance Scenarios**:

1. **Given** an authenticated Nostr public key has a profile metadata event with a name, **When** Play Game loads, **Then** the button shows `Set Name (existing-name)`.
2. **Given** relay lookup is unavailable but a non-sensitive local snapshot exists, **When** Play Game loads, **Then** the app may show the snapshot with visible stale/offline status rather than blocking gameplay.

---

### User Story 3 - Keep Nostr Auth Separate From Lightning Auth (Priority: P3)

As a player, I can use Nostr identity auth for profile edits without changing the existing Lightning authorization modes for game payments and trades.

**Why this priority**: Nostr profile identity and Lightning send approval solve different problems. Keeping them separate avoids coupling profile edits to Polar/LNAuth payment flows.

**Independent Test**: Can be tested by using profile name editing without triggering a Lightning payment approval, and by using existing Buy/Sell flows without requiring a profile edit.

**Acceptance Scenarios**:

1. **Given** the player opens the Profile prompt, **When** the Nostr QR auth flow begins, **Then** the modal copy and state refer to Nostr identity/profile authorization, not Lightning payment authorization.
2. **Given** the player buys or sells an item, **When** the existing Lightning approval flow runs, **Then** the Nostr profile prompt does not appear.

### Edge Cases

- If the user submits an empty or whitespace-only username, the prompt stays open with a visible validation message.
- If the username is too long for the game UI, the app rejects it before publishing metadata.
- If Nostr QR auth is canceled or times out, no profile metadata is changed.
- If relay publishing fails after auth succeeds, the app shows an error and does not claim the username was saved.
- If multiple profile edits are submitted quickly, the app prevents overlapping publish attempts.
- If the authenticated public key changes, the displayed username is scoped to the new Nostr identity.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Play Game MUST add a `Profile` button group.
- **FR-002**: The Profile group MUST include a button labeled exactly `Set Name (Username)`, where `Username` is the current Nostr profile name and is empty until set, yielding `Set Name ()`.
- **FR-003**: Clicking the profile-name button MUST open a prompt with a `username:` form field and `Submit` and `Cancel` buttons.
- **FR-004**: Submitting a username MUST require a Nostr identity authorization tied to the current Nostr public key before the profile change is accepted.
- **FR-005**: Canceling the prompt MUST leave the existing profile name unchanged.
- **FR-006**: The app MUST validate username length, empty values, and display safety before publishing or accepting a name.
- **FR-007**: The app MUST use Nostr profile metadata as the primary durable profile store instead of adding a custom game database for profile names.
- **FR-008**: The app MAY cache non-sensitive Nostr profile summary data locally for fast display, but MUST NOT store Nostr private keys, seed phrases, signing secrets, or bearer tokens in browser storage.
- **FR-009**: Nostr identity auth MUST remain a separate feature boundary from existing Lightning auth and payment approvals.
- **FR-010**: The service layer MUST expose Dioxus-safe profile operations so Play Game displays state and prompts without owning Nostr signing, relay, or metadata rules.
- **FR-011**: The implementation SHOULD use the Rust `nostr-sdk` crate family for Nostr client, signer, event, and metadata behavior unless implementation research finds a concrete blocker.

### Key Entities

- **NostrIdentity**: The authenticated Nostr public identity. Key attributes include public key, display form such as npub, auth status, and last verified time.
- **NostrProfile**: Profile metadata associated with a Nostr identity. Initial v1 attributes include username/name, source, relay publish status, and last updated time.
- **NostrProfileEditRequest**: A pending user request to change profile metadata. Key attributes include target username, prompt status, validation errors, and associated identity.
- **NostrAuthorizationSession**: QR-backed authorization state for identity/profile actions. Key attributes include challenge/payload, status, expiration, and public key result.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A player can complete the set-name flow from Play Game in under 30 seconds after a successful Nostr QR auth.
- **SC-002**: The Profile button displays `Set Name ()` before any username is known and `Set Name (Username)` after a valid profile name is saved or recovered.
- **SC-003**: Cancel, invalid username, auth failure, and relay publish failure paths leave the previous username unchanged 100% of the time.
- **SC-004**: The feature adds no custom game database table or destructive schema migration for profile names.
- **SC-005**: Browser storage safety checks prevent Nostr private keys, seed phrases, signing secrets, and authorization tokens from being persisted.

## Assumptions

- Nostr is the right profile substrate because public profile metadata is already identity-scoped and relay-backed, which avoids a game-owned user table for simple profile fields.
- `nostr-sdk` is the preferred Rust crate family because it is the maintained high-level Rust Nostr SDK, supports metadata operations, and documents wasm support.
- The first implementation can use a mock or local Nostr auth bridge if real mobile signing requires additional infrastructure, but the service contracts must preserve the real Nostr identity boundary.
- Username maps to the Nostr metadata `name` field for v1. Richer fields such as picture/about can be added later.
- Relays and QR signing availability may vary in local development; the app must keep Play Game usable when profile editing is unavailable.
