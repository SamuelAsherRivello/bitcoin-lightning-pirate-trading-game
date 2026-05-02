# Tasks: Nostr Identity Profile

**Input**: Design documents from `/specs/010-nostr-profile/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/nostr-profile-service.md, quickstart.md

**Tests**: Included because the specification defines mandatory independent tests, validation rules, and storage safety outcomes.

**Organization**: Tasks are grouped by user story so each story can be implemented and tested as an independent increment.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel because it touches different files or does not depend on incomplete tasks.
- **[Story]**: User story label for traceability. Setup, foundational, and polish tasks do not use story labels.
- Every task includes the target file path or command result file.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Add the Nostr service/module surface without changing Play Game behavior yet.

- [ ] T001 Add `nostr-sdk` with browser and desktop compatible feature selection in `packages/ui/Cargo.toml`
- [ ] T002 Create the Nostr profile service module shell and export it from `packages/ui/src/client/services/nostr_profile_service.rs` and `packages/ui/src/client/services/mod.rs`
- [ ] T003 [P] Create the Profile component module shell and export it from `packages/ui/src/client/components/profile/mod.rs` and the inline `components` module in `packages/ui/src/client/mod.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Shared DTOs, validation policy, storage safety, localized copy, and app state needed before any story can be implemented.

**Critical**: No user story work should begin until this phase is complete.

- [ ] T004 Add `NostrIdentity`, `NostrProfile`, `NostrProfileEditRequest`, `NostrAuthorizationSession`, action/status/source enums, and profile error types in `packages/lightning-service/src/client/models.rs`
- [ ] T005 [P] Add username validation tests for trim, empty input, maximum length, control characters, and secret-like values in `packages/lightning-service/src/client/models.rs`
- [ ] T006 Implement username validation helpers and public DTO exports in `packages/lightning-service/src/client/models.rs`, `packages/lightning-service/src/client/mod.rs`, and `packages/lightning-service/src/lib.rs`
- [ ] T007 Re-export Nostr profile DTOs for Dioxus code in `packages/ui/src/client/models.rs`
- [ ] T008 Add non-sensitive Nostr profile snapshot load/save APIs in `packages/ui/src/client/services/storage_service.rs`
- [ ] T009 [P] Add storage safety tests that reject Nostr private keys, seed phrases, signing secrets, bearer tokens, cookies, and relay auth secrets in `packages/ui/tests/tests.rs`
- [ ] T010 Add app-level Nostr profile and prompt signal/context types in `packages/ui/src/client/mod.rs`
- [ ] T011 [P] Add localized Profile, Set Name, username, Submit, Cancel, Nostr QR, stale/offline, and validation copy keys in `packages/ui/assets/i18n/en-US.ftl`, `packages/ui/assets/i18n/es-MX.ftl`, `packages/ui/assets/i18n/fr-FR.ftl`, and `packages/ui/assets/i18n/pt-BR.ftl`

**Checkpoint**: Foundation is ready; user story implementation can start.

---

## Phase 3: User Story 1 - Set Profile Name With Nostr Identity (Priority: P1) MVP

**Goal**: A player can open the Play Game Profile group, authorize with a Nostr identity, submit a username, and see the button update without adding a game profile database.

**Independent Test**: Start with no profile name, select `Set Name ()`, complete Nostr QR authorization, submit `jack`, and confirm Play Game shows `Set Name (jack)` while cancel, invalid input, auth failure, auth timeout, and publish failure preserve the previous username.

### Tests for User Story 1

> Write these tests first and confirm they fail before implementation.

- [ ] T012 [P] [US1] Add service contract tests for `start_nostr_profile_authorization`, `submit_nostr_profile_name`, and `cancel_nostr_profile_edit` in `packages/ui/src/client/services/nostr_profile_service.rs`
- [ ] T013 [P] [US1] Add Play Game UI tests for localized `Profile`, `Set Name ()`, `username:`, `Submit`, and `Cancel` copy in `packages/ui/tests/tests.rs`
- [ ] T014 [P] [US1] Add failure-path tests for invalid username, canceled auth, expired or timed-out auth, publish failure, and overlapping submit attempts in `packages/ui/src/client/services/nostr_profile_service.rs`

### Implementation for User Story 1

- [ ] T015 [US1] Implement mock/local Nostr authorization session creation, cancellation, and expiration state transitions in `packages/ui/src/client/services/nostr_profile_service.rs`
- [ ] T016 [US1] Implement username submit validation, approved-session enforcement, expired-session rejection, mock publish success, and previous-profile preservation on failure in `packages/ui/src/client/services/nostr_profile_service.rs`
- [ ] T017 [US1] Implement `nostr-sdk` metadata publish support for the username/name field while preserving the mock/local adapter boundary in `packages/ui/src/client/services/nostr_profile_service.rs`
- [ ] T018 [US1] Persist successful non-sensitive profile snapshots only after mock or real publish success in `packages/ui/src/client/services/storage_service.rs`
- [ ] T019 [US1] Implement the `ProfileNamePrompt` component with localized `username:`, `Submit`, validation, loading, and `Cancel` states in `packages/ui/src/client/components/profile/profile_name_prompt.rs`
- [ ] T020 [US1] Add the localized `Profile` button group and exact `Set Name (Username)` label behavior to `packages/ui/src/client/pages/play_game.rs`
- [ ] T021 [US1] Wire Play Game prompt submit, cancel, toast/status feedback, and state updates to the Nostr profile service in `packages/ui/src/client/pages/play_game.rs`

**Checkpoint**: User Story 1 is independently functional and testable as the MVP with durable Nostr metadata publish support or an explicitly selected mock/local adapter in development mode.

---

## Phase 4: User Story 2 - Reuse Existing Nostr Profile Name (Priority: P2)

**Goal**: A returning player can see the username already associated with the authorized Nostr identity from relay metadata or a safe local snapshot.

**Independent Test**: Authenticate with a Nostr identity that has existing metadata and confirm Play Game renders `Set Name (existing-name)`; if relay lookup fails with a snapshot available, confirm gameplay is not blocked and stale/offline status is visible.

### Tests for User Story 2

- [ ] T022 [P] [US2] Add service tests for snapshot-first profile summary and relay-refresh status in `packages/ui/src/client/services/nostr_profile_service.rs`
- [ ] T023 [P] [US2] Add service tests proving profile summaries are scoped by Nostr public key and identity switches do not display the previous identity's username in `packages/ui/src/client/services/nostr_profile_service.rs`
- [ ] T024 [P] [US2] Add Play Game tests for returning-user label `Set Name (existing-name)`, identity-switch label refresh, and stale/offline status copy in `packages/ui/tests/tests.rs`

### Implementation for User Story 2

- [ ] T025 [US2] Implement `get_nostr_profile_summary` with local snapshot fallback, Nostr public key scoping, and relay refresh status in `packages/ui/src/client/services/nostr_profile_service.rs`
- [ ] T026 [US2] Implement `nostr-sdk` metadata read support for profile name lookup in `packages/ui/src/client/services/nostr_profile_service.rs`
- [ ] T027 [US2] Integrate profile summary loading and identity-switch refresh into the Play Game route without blocking route render in `packages/ui/src/client/pages/play_game.rs`
- [ ] T028 [US2] Add visible stale/offline status or toast behavior when relay lookup fails but a local snapshot exists in `packages/ui/src/client/pages/play_game.rs`

**Checkpoint**: User Stories 1 and 2 both work independently.

---

## Phase 5: User Story 3 - Keep Nostr Auth Separate From Lightning Auth (Priority: P3)

**Goal**: Nostr profile authorization uses its own prompt state and copy, while existing Lightning buy/sell authorization keeps its current behavior.

**Independent Test**: Start profile editing and verify the QR modal refers to Nostr identity/profile authorization only; run Buy/Sell and verify Lightning approval runs without opening the Nostr profile prompt.

### Tests for User Story 3

- [ ] T029 [P] [US3] Add model/service tests proving Nostr profile actions do not reuse Lightning transaction approval DTOs in `packages/lightning-service/src/client/models.rs`
- [ ] T030 [P] [US3] Add Play Game interaction tests proving Buy/Sell does not open the Nostr prompt and Profile editing does not open the Lightning prompt in `packages/ui/tests/tests.rs`

### Implementation for User Story 3

- [ ] T031 [US3] Extend the QR authorization modal to support localized Nostr identity/profile copy without changing Lightning approval copy in `packages/ui/src/client/components/auth/qr_authorization_modal.rs`
- [ ] T032 [US3] Route Nostr profile authorization through separate prompt/session state from Lightning authorization in `packages/ui/src/client/mod.rs`
- [ ] T033 [US3] Audit and adjust Play Game Buy/Sell handlers so they keep using the Lightning authorization path only in `packages/ui/src/client/pages/play_game.rs`

**Checkpoint**: All user stories are independently functional.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Documentation and verification across web and desktop.

- [ ] T034 [P] Update feature/platform documentation for Nostr profile storage, auth boundary, localized Play Game UI, and metadata read/publish behavior in `Documentation/DioxusFeatureMatrix.md`
- [ ] T035 [P] Update implementation notes and any final mock-vs-real signer decisions in `specs/010-nostr-profile/quickstart.md`
- [ ] T036 Run `cargo test -p lightning-service nostr` and record the result in `specs/010-nostr-profile/quickstart.md`
- [ ] T037 Run `cargo test -p ui nostr` and record the result in `specs/010-nostr-profile/quickstart.md`
- [ ] T038 Run `cargo check -p ui --target wasm32-unknown-unknown` and record the result in `specs/010-nostr-profile/quickstart.md`
- [ ] T039 Run `cargo check -p web --target wasm32-unknown-unknown` and record the result in `specs/010-nostr-profile/quickstart.md`
- [ ] T040 Run `cargo check -p desktop` and record the result in `specs/010-nostr-profile/quickstart.md`
- [ ] T041 Time a successful Play Game set-name flow after Nostr QR authorization and record whether it completes under 30 seconds in `specs/010-nostr-profile/quickstart.md`
- [ ] T042 Run `.\Scripts\Common\RunWeb.ps1` and verify set name, cancel, invalid name, auth cancel, auth timeout, publish failure, reload with snapshot, identity switch, and Buy/Sell auth separation in the browser, then record the result in `specs/010-nostr-profile/quickstart.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies; can start immediately.
- **Foundational (Phase 2)**: Depends on Setup completion; blocks all user stories.
- **User Stories (Phases 3-5)**: Depend on Foundational completion.
- **Polish (Phase 6)**: Depends on the implemented user stories selected for the release.

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational; no dependency on US2 or US3.
- **User Story 2 (P2)**: Can start after Foundational; integrates with the same summary state as US1 but remains independently testable with a seeded snapshot or mocked relay response.
- **User Story 3 (P3)**: Can start after Foundational; validates auth separation and can run after or alongside US1 as long as prompt state exists.

### Within Each User Story

- Tests should be written and observed failing before implementation.
- DTOs, validation, storage, and localization precede service behavior.
- Service behavior precedes Play Game integration.
- UI prompt state precedes browser-visible verification.
- Each story should reach its checkpoint before moving to the next priority if working sequentially.

---

## Parallel Opportunities

- T003 can run in parallel with T001-T002.
- T005, T009, and T011 can run in parallel once the target files are assigned.
- T012, T013, and T014 can run in parallel because they target separate test scopes.
- T022, T023, and T024 can run in parallel.
- T029 and T030 can run in parallel.
- T034 and T035 can run in parallel after the implementation shape is stable.

## Parallel Example: User Story 1

```text
Task: "T012 [P] [US1] Add service contract tests for Nostr profile operations in packages/ui/src/client/services/nostr_profile_service.rs"
Task: "T013 [P] [US1] Add Play Game UI tests for localized Profile group and prompt copy in packages/ui/tests/tests.rs"
Task: "T014 [P] [US1] Add failure-path tests including expired auth in packages/ui/src/client/services/nostr_profile_service.rs"
```

## Parallel Example: User Story 2

```text
Task: "T022 [P] [US2] Add service tests for summary and relay-refresh status in packages/ui/src/client/services/nostr_profile_service.rs"
Task: "T023 [P] [US2] Add identity-switch scoping tests in packages/ui/src/client/services/nostr_profile_service.rs"
Task: "T024 [P] [US2] Add Play Game returning-user and identity-switch tests in packages/ui/tests/tests.rs"
```

## Parallel Example: User Story 3

```text
Task: "T029 [P] [US3] Add DTO separation tests in packages/lightning-service/src/client/models.rs"
Task: "T030 [P] [US3] Add Play Game auth separation tests in packages/ui/tests/tests.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1 setup.
2. Complete Phase 2 foundational DTOs, validation, storage, localization, and app state.
3. Complete Phase 3 User Story 1, including real `nostr-sdk` metadata publish support or an explicitly selected mock/local adapter in development mode.
4. Stop and validate the MVP with `cargo test -p lightning-service nostr`, `cargo test -p ui nostr`, focused wasm checks, and Play Game browser verification.

### Incremental Delivery

1. Deliver US1 to prove profile editing through Nostr identity authorization and metadata publishing.
2. Deliver US2 to recover existing profile data from relay metadata or local snapshot with identity-scoped display.
3. Deliver US3 to harden the separation between Nostr identity auth and Lightning payment auth.
4. Complete polish tasks and full verification before handoff.

### Notes

- Do not add a custom game profile database table or destructive migration for profile names.
- Do not persist Nostr private keys, seed phrases, signing secrets, bearer tokens, cookies, or relay auth secrets.
- Preserve web and desktop support.
- Keep visible loading, toast, and error feedback around auth, relay lookup, publish, cache read, and cache write operations.
