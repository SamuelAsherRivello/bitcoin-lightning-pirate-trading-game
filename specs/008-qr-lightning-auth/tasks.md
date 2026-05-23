# Tasks: QR Lightning Auth Mode

**Input**: Design documents from `/specs/008-qr-lightning-auth/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/lightning-auth-service.md, quickstart.md

**Tests**: Include focused Rust unit tests for service behavior and browser-visible verification tasks for Dioxus UI changes, matching the repository test-first guidance.

**Organization**: Tasks are grouped by user story so each story can be implemented and tested independently after foundational work is complete.

## Phase 1: Setup

**Purpose**: Confirm the existing feature context and prepare dependency decisions before changing source code.

- [X] T001 Review the QR auth and Polar setup plan in specs/008-qr-lightning-auth/plan.md
- [X] T002 Review the Dioxus 0.7 workflow constraints in .codex/rules/dioxus-0.7-workflow.md
- [X] T003 Inspect current setup wizard and service boundaries in packages/ui/src/client/pages/setup.rs
- [X] T004 [P] Inspect current Lightning DTOs and lab behavior in packages/lightning-service/src/client/models.rs
- [X] T005 [P] Inspect current portable lab operations in packages/lightning-service/src/client/lab_service.rs
- [X] T006 [P] Inspect current TRA ownership operations in packages/lightning-service/src/client/tra_service.rs
- [X] T007 [P] Inspect current Dioxus service wrappers in packages/ui/src/client/services/lightning_server_functions.rs
- [X] T008 [P] Inspect current local snapshot safety checks in packages/ui/src/client/services/storage_service.rs
- [X] T009 Decide final QR rendering crate and LNURL-auth adapter approach in packages/lightning-service/Cargo.toml

---

## Phase 2: Foundational

**Purpose**: Add shared DTOs and stabilize the Polar setup workflow before user-story work begins.

**Critical**: No user story work should begin until this phase is complete.

- [X] T010 Add UserAuthMode, WalletRecommendationTip, PlayerIdentity, PlayerAuthSession, AuthorizationEvent, TransactionApproval, QrAuthorizationModal, LightningOperationResult, and PolarSetupStep DTOs in packages/lightning-service/src/client/models.rs
- [X] T011 Add serde defaults for new SetupProfile and LabState auth fields in packages/lightning-service/src/client/models.rs
- [X] T012 [P] Add unit tests for DTO serde defaults and old snapshot compatibility in packages/lightning-service/src/client/models.rs
- [X] T013 Re-export new auth DTOs from packages/lightning-service/src/client/mod.rs
- [X] T014 Re-export new auth DTOs from packages/ui/src/client/models.rs
- [X] T015 Add non-sensitive auth/session/approval snapshot validation coverage in packages/ui/src/client/services/storage_service.rs
- [X] T016 [P] Add storage safety tests for forbidden auth secret strings in packages/ui/src/client/services/storage_service.rs
- [X] T017 Rename the setup wizard step model to include CreateNodes, UserNodesSats, and UserNodesTras in packages/ui/src/client/pages/setup.rs
- [X] T018 Update setup wizard labels, order, submit focus targets, and reset focus targets for Bridge URL, Server Name, Create Nodes, Game Treasury (Sats), Game Treasury (TRAs), User Nodes (Sats), User Nodes (TRAs), Block Height, and Unlock Routes in packages/ui/src/client/pages/setup.rs
- [X] T019 Add setup wizard order tests for the nine-step Polar flow in packages/ui/src/client/pages/setup.rs
- [X] T020 Split Polar node topology creation from value balancing in packages/ui/src/client/services/polar_bridge_service.rs
- [X] T021 Add create_required_nodes wrapper behavior for Bitcoin backend, Game Treasury, `GAME_TAPROOT`, Alice, Bob, and Carol readiness in packages/ui/src/client/services/lightning_server_functions.rs
- [X] T022 Add one-time Polar network restart recovery after repeated node readiness retries in packages/ui/src/client/services/polar_bridge_service.rs
- [X] T023 Add tests for Create Nodes readiness and one-time restart behavior in packages/ui/src/client/services/polar_bridge_service.rs
- [X] T024 Implement Game Treasury sats top-up after Create Nodes in packages/ui/src/client/services/lightning_server_functions.rs
- [X] T025 Implement Game Treasury TRA verification/minting after Create Nodes in packages/ui/src/client/services/lightning_server_functions.rs
- [ ] T026 Implement User Nodes (Sats) rebalancing to and from Game Treasury in packages/ui/src/client/services/lightning_server_functions.rs
- [ ] T027 Implement User Nodes (TRAs) rebalancing to and from Game Treasury in packages/ui/src/client/services/lightning_server_functions.rs
- [ ] T028 Add tests for user-node sats and TRA rebalancing with extra Game Treasury value allowed in packages/ui/src/client/services/lightning_server_functions.rs
- [X] T029 Update setup autopilot status text and execution order for the nine-step Polar flow in packages/ui/src/client/pages/setup.rs
- [X] T030 Update setup reset helpers and recovery messages for Create Nodes, User Nodes (Sats), and User Nodes (TRAs) in packages/ui/src/client/pages/setup.rs
- [X] T031 Update Dioxus feature matrix setup and Polar health notes in Documentation/DioxusFeatureMatrix.md

**Checkpoint**: The Polar setup flow is stable and follows the required nine-step order.

---

## Phase 3: User Story 1 - Choose User Auth Mode (Priority: P1)

**Goal**: Set Up exposes `App`, `Mock LNAuth`, and `LNAuth` as a separate user-auth selector without replacing the existing connection mode.

**Independent Test**: Open Set Up, choose each auth mode, confirm active mode/status is visible, and confirm the Polar connection tabs remain separate.

### Tests for User Story 1

- [X] T032 [P] [US1] Add service tests for UserAuthMode defaults and mode transitions in packages/lightning-service/src/client/models.rs
- [ ] T033 [P] [US1] Add setup selector persistence tests in packages/ui/src/client/services/storage_service.rs

### Implementation for User Story 1

- [X] T034 [US1] Add User Auth selector state and persistence wiring in packages/ui/src/client/pages/setup.rs
- [X] T035 [US1] Render App, Mock LNAuth, and LNAuth options with active status in packages/ui/src/client/pages/setup.rs
- [X] T036 [US1] Add LNAuth Alby Go info tip using FieldHelpIcon in packages/ui/src/client/pages/setup.rs
- [X] T037 [US1] Invalidate or flag connected lab state for revalidation after auth mode changes in packages/ui/src/client/pages/setup.rs
- [X] T038 [US1] Persist non-sensitive user auth mode and status snapshots in packages/ui/src/client/services/storage_service.rs
- [X] T039 [US1] Show auth mode and setup status in route header or setup summary in packages/ui/src/client/components/page_header.rs
- [X] T040 [US1] Add Home FAQ copy explaining App, Mock LNAuth, LNAuth, Alby Go, and why the external wallet is not a Polar node in packages/ui/src/client/pages/home.rs
- [X] T041 [US1] Verify Set Up selector behavior in the served web app using Scripts/Common/RunWeb.ps1

**Checkpoint**: User Story 1 is independently functional and testable.

---

## Phase 4: User Story 2 - Authenticate Player On Game Entry (Priority: P1)

**Goal**: Play Game prompts unauthenticated `Mock LNAuth` and `LNAuth` players with the reusable QR modal after connected lab refresh.

**Independent Test**: Choose `Mock LNAuth`, open Play Game, observe the centered modal, wait for one-second auto-completion, and confirm gameplay becomes usable without a real wallet.

### Tests for User Story 2

- [X] T042 [P] [US2] Add PlayerAuthSession lifecycle tests for created, displayed, approved, expired, failed, and canceled states in packages/lightning-service/src/client/lab_service.rs
- [X] T043 [P] [US2] Add mock auto-complete cancellation tests in packages/lightning-service/src/client/lab_service.rs

### Implementation for User Story 2

- [X] T044 [US2] Add auth_client adapter boundary for LNURL-auth challenge and verification in packages/lightning-service/src/server/auth_client.rs
- [X] T045 [US2] Wire auth_client module exports in packages/lightning-service/src/server/mod.rs
- [X] T046 [US2] Implement begin_player_auth and complete_player_auth service functions in packages/lightning-service/src/client/lab_service.rs
- [X] T047 [US2] Add Dioxus-safe auth wrappers in packages/ui/src/client/services/lightning_server_functions.rs
- [X] T048 [US2] Add QR prompt context signal and provider beside toast and operation prompt state in packages/ui/src/client/mod.rs
- [X] T049 [US2] Create reusable QR authorization modal component in packages/ui/src/client/components/auth/qr_authorization_modal.rs
- [X] T050 [US2] Render QrAuthorizationModal from PageHeader near ToastRegion and OperationPromptRegion in packages/ui/src/client/components/page_header.rs
- [X] T051 [US2] Add QR modal styles for web and desktop in packages/ui/assets/main.css
- [X] T052 [US2] Trigger Play Game login QR after connected route-entry lab refresh in packages/ui/src/client/pages/play_game.rs
- [X] T053 [US2] Implement Mock LNAuth one-second login auto-complete and cancel handling in packages/ui/src/client/pages/play_game.rs
- [ ] T054 [US2] Add recoverable login failure and retry state handling in packages/ui/src/client/pages/play_game.rs
- [ ] T055 [US2] Verify login modal behavior in the served web app using Scripts/Common/RunWeb.ps1

**Checkpoint**: User Story 2 is independently functional and testable.

---

## Phase 5: User Story 3 - Approve Sats Sends With QR Modal (Priority: P2)

**Goal**: Buy/Sell sats sends require QR approval in `Mock LNAuth` and `LNAuth`, while `App` mode keeps the current fast path.

**Independent Test**: Select `Mock LNAuth`, enter Play Game, initiate Buy Item or Sell Item, observe `You are sending 1,000 sats`, wait for auto-completion, and confirm the send completes only after approval.

### Tests for User Story 3

- [X] T056 [P] [US3] Add authorization policy tests for App, Mock LNAuth, and LNAuth modes in packages/lightning-service/src/client/lab_service.rs
- [ ] T057 [P] [US3] Add tests proving low-risk reads do not require QR approval in packages/lightning-service/src/client/lab_service.rs
- [ ] T058 [P] [US3] Add tests proving canceled or failed send approval leaves the trade incomplete in packages/lightning-service/src/client/tra_service.rs

### Implementation for User Story 3

- [X] T059 [US3] Implement authorize_player_operation and approval result handling in packages/lightning-service/src/client/lab_service.rs
- [X] T060 [US3] Bind TransactionApproval to operation summary, amount, and player identity in packages/lightning-service/src/client/models.rs
- [X] T061 [US3] Add approval wrapper functions in packages/ui/src/client/services/lightning_server_functions.rs
- [X] T062 [US3] Gate execute_tra_item_trade before invoice payment and TRA transfer in packages/ui/src/client/services/lightning_server_functions.rs
- [X] T063 [US3] Show send approval QR modal from Buy Item and Sell Item flows in packages/ui/src/client/pages/play_game.rs
- [X] T064 [US3] Implement Mock LNAuth one-second send approval auto-complete and cancel handling in packages/ui/src/client/pages/play_game.rs
- [X] T065 [US3] Record non-sensitive approval history in LabState and storage snapshots in packages/lightning-service/src/client/models.rs
- [X] T066 [US3] Verify App mode Buy/Sell behavior remains unchanged in packages/ui/src/client/pages/play_game.rs
- [ ] T067 [US3] Verify send approval behavior in the served web app using Scripts/Common/RunWeb.ps1

**Checkpoint**: User Story 3 is independently functional and testable.

---

## Phase 6: User Story 4 - Reuse Lightning Capabilities Outside The App UI (Priority: P2)

**Goal**: Authentication, approvals, payments, route/channel work, and TRA behavior are available through portable service boundaries independent of Dioxus pages.

**Independent Test**: Exercise service functions from Rust tests without instantiating Dioxus components, routes, toast state, or page state.

### Tests for User Story 4

- [ ] T068 [P] [US4] Add portable service tests for auth challenge creation without Dioxus dependencies in packages/lightning-service/src/client/lab_service.rs
- [ ] T069 [P] [US4] Add portable service tests for payment and TRA operation result envelopes in packages/lightning-service/src/client/tra_service.rs

### Implementation for User Story 4

- [ ] T070 [US4] Refactor QR auth policy so page components call service wrappers rather than owning policy in packages/lightning-service/src/client/lab_service.rs
- [ ] T071 [US4] Refactor payment and invoice result mapping into portable DTOs in packages/lightning-service/src/client/lab_service.rs
- [ ] T072 [US4] Refactor TRA transfer result mapping into portable DTOs in packages/lightning-service/src/client/tra_service.rs
- [ ] T073 [US4] Keep Dioxus-only toast, prompt, and route handling in packages/ui/src/client/services/lightning_server_functions.rs
- [ ] T074 [US4] Add Network Dashboard auth mode, player fingerprint, active session, and recent approval display in packages/ui/src/client/pages/debug_network.rs
- [ ] T075 [US4] Document portable service boundaries and Bevy reuse notes in Documentation/DioxusFeatureMatrix.md
- [ ] T076 [US4] Verify portable service tests for packages/lightning-service/Cargo.toml with cargo test -p lightning-service

**Checkpoint**: User Story 4 is independently functional and testable.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Finish validation, documentation, and compatibility checks across all stories.

- [ ] T077 [P] Update quickstart validation notes after implementation in specs/008-qr-lightning-auth/quickstart.md
- [ ] T078 [P] Update implementation status and any final constraints in specs/008-qr-lightning-auth/plan.md
- [ ] T079 [P] Add browser-visible QA notes for Set Up, Play Game, Network Dashboard, and Home in Documentation/DioxusFeatureMatrix.md
- [X] T080 Run focused UI wasm check for packages/ui/Cargo.toml with cargo check -p ui --target wasm32-unknown-unknown
- [X] T081 Run focused web wasm check for packages/web/Cargo.toml with cargo check -p web --target wasm32-unknown-unknown
- [X] T082 Run focused desktop check for packages/desktop/Cargo.toml with cargo check -p desktop
- [X] T083 Run service tests for packages/lightning-service/Cargo.toml with cargo test -p lightning-service
- [X] T084 Run full repository test script with Scripts/Other/RunTests.ps1
- [ ] T085 Verify served web app flows with Scripts/Common/RunWeb.ps1
- [ ] T086 Record Alby Go real-wallet validation result or documented compatibility blocker in specs/008-qr-lightning-auth/quickstart.md

---

## Dependencies & Execution Order

### Phase Dependencies

- Phase 1 Setup has no dependencies.
- Phase 2 Foundational depends on Phase 1 and blocks all user stories.
- Phase 3 User Story 1 depends on Phase 2.
- Phase 4 User Story 2 depends on Phase 2 and can proceed after User Story 1 auth mode persistence is available.
- Phase 5 User Story 3 depends on User Story 2 login/session behavior.
- Phase 6 User Story 4 depends on the portable DTOs from Phase 2 and can proceed alongside User Story 3 after auth and approval contracts exist.
- Phase 7 Polish depends on the implemented stories selected for release.

### User Story Dependencies

- User Story 1 is the MVP user-facing selector and should land before the modal stories.
- User Story 2 needs the mode and persisted auth state from User Story 1.
- User Story 3 needs User Story 2 session and modal behavior.
- User Story 4 can be implemented incrementally after foundational DTOs, with final verification after User Stories 2 and 3.

### Parallel Opportunities

- T004, T005, T006, T007, and T008 can run in parallel during inspection.
- T012 and T016 can run in parallel after T010 and T015 are clear.
- T020 through T028 touch related setup services and should be coordinated, but tests in T023 and T028 can be written before implementation.
- T032 and T033 can run in parallel for User Story 1.
- T042 and T043 can run in parallel for User Story 2.
- T056, T057, and T058 can run in parallel for User Story 3.
- T068 and T069 can run in parallel for User Story 4.
- T077, T078, and T079 can run in parallel during polish.

---

## Parallel Example: User Story 2

```text
Task: "Add PlayerAuthSession lifecycle tests for created, displayed, approved, expired, failed, and canceled states in packages/lightning-service/src/client/lab_service.rs"
Task: "Add mock auto-complete cancellation tests in packages/lightning-service/src/client/lab_service.rs"
```

---

## Implementation Strategy

### MVP First

1. Complete Phase 1 and Phase 2 so the Polar setup order and shared DTOs are stable.
2. Complete Phase 3 User Story 1.
3. Validate Set Up independently in the served app.

### Incremental Delivery

1. Deliver User Story 1 for auth mode selection.
2. Deliver User Story 2 for login modal behavior with Mock LNAuth.
3. Deliver User Story 3 for per-send approval.
4. Deliver User Story 4 for portable non-Dioxus service reuse.

### Validation Gates

1. Run `cargo test -p lightning-service` after service DTO and policy changes.
2. Run `cargo check -p ui --target wasm32-unknown-unknown` after Dioxus UI changes.
3. Run `cargo check -p web --target wasm32-unknown-unknown` and `cargo check -p desktop` before browser/desktop handoff.
4. Run `Scripts/Common/RunWeb.ps1` for browser-visible modal, setup, and dashboard behavior.
