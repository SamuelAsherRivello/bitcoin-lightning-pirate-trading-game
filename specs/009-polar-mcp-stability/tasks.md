# Tasks: Polar MCP Stability

**Input**: Design documents from `specs/009-polar-mcp-stability/`
**Prerequisites**: `plan.md`, `spec.md`

**Tests**: Included because FR-018 requires verification coverage and the plan calls for focused adapter, idempotency, redaction, and runtime checks.

**Organization**: Tasks are grouped by user story so each story can be implemented and verified independently.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel because it touches different files or depends only on completed prerequisite phases.
- **[Story]**: Maps the task to a user story from `spec.md`.
- Every task includes an exact file path.

## Phase 1: Setup

**Purpose**: Establish the local connector dependency and baseline evidence before refactoring behavior.

- [X] T001 Document the Node.js 18+ and `npx -y @lightningpolar/mcp` prerequisite in `README.md`
- [X] T002 Add Polar MCP prerequisite and health-check guidance to `Scripts/Common/InstallDependencies.ps1`
- [X] T003 [P] Add a connector launch/helper script for `npx -y @lightningpolar/mcp` in `Scripts/Common/RunPolarMcp.ps1`
- [X] T004 [P] Add baseline timing and operation-count capture instructions in `specs/009-polar-mcp-stability/plan.md`
- [X] T005 [P] Record the current Polar call inventory for `packages/ui/src/client/services/polar_bridge_service.rs` in `specs/009-polar-mcp-stability/polar-call-inventory.md`

---

## Phase 2: Foundational

**Purpose**: Create the shared connector boundary, typed status models, redaction, and verification helpers that all stories depend on.

**CRITICAL**: No user story work should begin until this phase is complete.

- [X] T006 Add portable connector status and operation DTOs in `packages/lightning-service/src/client/models.rs`
- [X] T007 Add connector failure variants and redacted display text in `packages/lightning-service/src/client/error.rs`
- [X] T008 Create the typed Polar connector boundary in `packages/ui/src/client/services/polar_mcp_connector.rs`
- [X] T009 Register the new connector module in `packages/ui/src/client/services/mod.rs`
- [X] T010 Move shared MCP/bridge request timeout, retry classification, and redaction helpers into `packages/ui/src/client/services/polar_mcp_connector.rs`
- [X] T011 Update `packages/ui/src/client/services/polar_bridge_service.rs` to call the connector boundary for health checks and raw tool execution
- [X] T012 [P] Add connector parsing, missing-tool, retry, timeout, and redaction tests in `packages/ui/tests/polar_mcp_connector_tests.rs`
- [X] T013 [P] Add model serialization tests for connector status and operation DTOs in `packages/lightning-service/src/client/models.rs`

**Checkpoint**: Connector boundary exists, raw Polar access is centralized, and unit tests cover parsing, failure, timeout, retry, and redaction behavior.

---

## Phase 3: User Story 1 - Keep The Same Polar Setup Experience (Priority: P1) MVP

**Goal**: Preserve the existing setup/game/dashboard user experience while making the setup path more idempotent and stable.

**Independent Test**: Run the setup flow from `Bridge URL` through `Unlock Routes`, rerun it on the same network, and confirm the same visible step order, status feedback, and connected result.

### Tests for User Story 1

- [ ] T014 [P] [US1] Add setup step order and route-locking regression tests in `packages/ui/tests/tests.rs`
- [ ] T015 [P] [US1] Add idempotent existing-network and existing-node tests in `packages/ui/tests/polar_bridge_service_tests.rs`
- [ ] T016 [P] [US1] Add already-started node regression coverage in `packages/ui/tests/polar_bridge_service_tests.rs`

### Implementation for User Story 1

- [ ] T017 [US1] Refactor `test_bridge`, `list_networks`, and health-check call sites to the connector boundary in `packages/ui/src/client/services/polar_bridge_service.rs`
- [ ] T018 [US1] Refactor `ensure_server` and network start handling to preserve find/create/start semantics in `packages/ui/src/client/services/polar_bridge_service.rs`
- [ ] T019 [US1] Refactor required-node creation and start checks to treat existing or already-started nodes as success in `packages/ui/src/client/services/polar_bridge_service.rs`
- [ ] T020 [US1] Refactor treasury sats, treasury TRA, user sats, and user TRA setup steps to preserve existing labels and progress events in `packages/ui/src/client/services/lightning_server_functions.rs`
- [ ] T021 [US1] Preserve setup row labels, progress messaging, and route-locking behavior in `packages/ui/src/client/pages/setup.rs`
- [ ] T022 [US1] Preserve Play Game and Network Dashboard post-setup behavior in `packages/ui/src/client/pages/play_game.rs`
- [ ] T023 [US1] Preserve Network Dashboard diagnostics after setup in `packages/ui/src/client/pages/debug_network.rs`
- [ ] T024 [US1] Run and document focused verification commands in `specs/009-polar-mcp-stability/verification-notes.md`

**Checkpoint**: User Story 1 is independently testable as the MVP: same app experience, more idempotent setup, no new user workflow.

---

## Phase 4: User Story 2 - Use A Standard Polar Automation Connector (Priority: P1)

**Goal**: Make the maintained Polar MCP package the documented and validated connector path for networked Polar interactions.

**Independent Test**: Follow the setup documentation, launch the connector, verify health, then confirm app operations flow through the connector contract and recover clearly when it is unavailable.

### Tests for User Story 2

- [ ] T025 [P] [US2] Add connector-unavailable recovery tests in `packages/ui/tests/polar_mcp_connector_tests.rs`
- [X] T026 [P] [US2] Add required-tool discovery and unsupported-tool tests in `packages/ui/tests/polar_mcp_connector_tests.rs`

### Implementation for User Story 2

- [X] T027 [US2] Add connector health verification and required-tool validation in `packages/ui/src/client/services/polar_mcp_connector.rs`
- [ ] T028 [US2] Surface connector health and recovery guidance through setup service results in `packages/ui/src/client/services/lightning_server_functions.rs`
- [ ] T029 [US2] Show connector health and recovery guidance on the Set Up page in `packages/ui/src/client/pages/setup.rs`
- [ ] T030 [US2] Add connector health and last-operation diagnostics to Network Dashboard in `packages/ui/src/client/pages/debug_network.rs`
- [ ] T031 [US2] Update English localized connector health and recovery copy in `packages/ui/assets/i18n/en-US.ftl`
- [ ] T032 [US2] Update feature/platform documentation for Polar MCP usage in `Documentation/DioxusFeatureMatrix.md`

**Checkpoint**: The app has a standard connector dependency path, health validation, required-tool checks, and user-facing recovery guidance.

---

## Phase 5: User Story 3 - Make Polar Operations Faster (Priority: P2)

**Goal**: Reduce redundant reads and unnecessary waits while keeping correctness, progress feedback, and error visibility.

**Independent Test**: Compare warm-network setup and dashboard refresh timing before and after the refactor, and confirm fewer repeated full-network reads.

### Tests for User Story 3

- [ ] T033 [P] [US3] Add state-snapshot reuse tests in `packages/ui/tests/polar_bridge_service_tests.rs`
- [ ] T034 [P] [US3] Add bounded polling stop-condition tests in `packages/ui/tests/polar_bridge_service_tests.rs`

### Implementation for User Story 3

- [ ] T035 [US3] Add a short-lived `PolarStateSnapshot` helper in `packages/ui/src/client/services/polar_bridge_service.rs`
- [ ] T036 [US3] Reuse validated state snapshots during setup operations in `packages/ui/src/client/services/polar_bridge_service.rs`
- [ ] T037 [US3] Replace repeated ad hoc polling loops with a bounded wait helper in `packages/ui/src/client/services/polar_bridge_service.rs`
- [ ] T038 [US3] Add operation counting or lightweight instrumentation for before/after comparison in `packages/ui/src/client/services/polar_bridge_service.rs`
- [ ] T039 [US3] Document measured warm-network timing and call-count results in `specs/009-polar-mcp-stability/verification-notes.md`

**Checkpoint**: Warm-network setup and dashboard refreshes have measured speed improvements and fewer redundant state reads.

---

## Phase 6: User Story 4 - Preserve Existing Safety Boundaries (Priority: P2)

**Goal**: Keep connector access local-only, avoid new destructive behavior, and prevent secret-like data from being persisted or displayed.

**Independent Test**: Review saved preferences, displayed errors, logs, and destructive-operation paths after connector failures and successful setup.

### Tests for User Story 4

- [X] T040 [P] [US4] Add local-only connector URL validation tests in `packages/ui/tests/polar_mcp_connector_tests.rs`
- [ ] T041 [P] [US4] Add secret-redaction and saved-snapshot safety tests in `packages/ui/tests/storage_service_tests.rs`
- [ ] T042 [P] [US4] Add bounded destructive-operation regression tests in `packages/ui/tests/polar_bridge_service_tests.rs`

### Implementation for User Story 4

- [X] T043 [US4] Enforce localhost-only connector access in `packages/ui/src/client/services/polar_mcp_connector.rs`
- [ ] T044 [US4] Ensure connector status persistence remains non-sensitive in `packages/ui/src/client/services/storage_service.rs`
- [ ] T045 [US4] Keep demo reset/delete operations bounded to existing app-owned actions in `packages/ui/src/client/services/polar_bridge_service.rs`
- [ ] T046 [US4] Redact connector failure details before toast or dashboard display in `packages/ui/src/client/components/toast.rs`
- [ ] T047 [US4] Add secret-safety review notes for saved setup data, logs, and displayed errors in `specs/009-polar-mcp-stability/verification-notes.md`

**Checkpoint**: The refactor keeps the local-only and secret-safety boundaries intact.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Final documentation, formatting, broad checks, and browser-visible verification.

- [ ] T048 [P] Update any changed setup or connector copy in `packages/web/assets/main.css`
- [ ] T049 [P] Mirror any changed setup or connector copy styling in `packages/desktop/assets/main.css`
- [X] T050 Run `cargo fmt --check` and record the result in `specs/009-polar-mcp-stability/verification-notes.md`
- [X] T051 Run `cargo test -p lightning-service` and record the result in `specs/009-polar-mcp-stability/verification-notes.md`
- [X] T052 Run `cargo test -p ui polar` and record the result in `specs/009-polar-mcp-stability/verification-notes.md`
- [X] T053 Run `cargo test -p ui setup` and record the result in `specs/009-polar-mcp-stability/verification-notes.md`
- [X] T054 Run `cargo check -p ui --target wasm32-unknown-unknown` and record the result in `specs/009-polar-mcp-stability/verification-notes.md`
- [X] T055 Run `cargo check -p web --target wasm32-unknown-unknown` and record the result in `specs/009-polar-mcp-stability/verification-notes.md`
- [X] T056 Run `cargo check -p desktop` and record the result in `specs/009-polar-mcp-stability/verification-notes.md`
- [ ] T057 Verify the served web setup flow with `Scripts/Common/RunWeb.ps1` and record the result in `specs/009-polar-mcp-stability/verification-notes.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies.
- **Foundational (Phase 2)**: Depends on Phase 1 and blocks all user stories.
- **User Story 1 (Phase 3)**: Depends on Phase 2 and is the MVP.
- **User Story 2 (Phase 4)**: Depends on Phase 2; can proceed alongside US1 after connector boundary exists, but should be validated against US1 before handoff.
- **User Story 3 (Phase 5)**: Depends on US1 because speed work must preserve the completed setup behavior.
- **User Story 4 (Phase 6)**: Depends on Phase 2; can proceed alongside US2/US3 where file ownership does not conflict.
- **Polish (Phase 7)**: Depends on all desired user stories being complete.

### User Story Dependencies

- **US1**: First user-facing increment; no dependency on US2-US4 after foundation.
- **US2**: Builds on the connector boundary; should not change visible setup order.
- **US3**: Requires US1 behavior to be stable before performance optimization.
- **US4**: Can be implemented after foundation, but final safety review should happen after US2 and US3.

### Parallel Opportunities

- T003, T004, and T005 can run in parallel after T001-T002 are understood.
- T012 and T013 can run in parallel with T010-T011 after the DTO and connector boundary are sketched.
- US1 test tasks T014-T016 can run in parallel.
- US2 test tasks T025-T026 can run in parallel.
- US3 test tasks T033-T034 can run in parallel.
- US4 test tasks T040-T042 can run in parallel.
- Styling tasks T048-T049 can run in parallel if connector UI copy changes require CSS.

---

## Parallel Example: User Story 1

```text
Task: "T014 [P] [US1] Add setup step order and route-locking regression tests in packages/ui/tests/tests.rs"
Task: "T015 [P] [US1] Add idempotent existing-network and existing-node tests in packages/ui/tests/polar_bridge_service_tests.rs"
Task: "T016 [P] [US1] Add already-started node regression coverage in packages/ui/tests/polar_bridge_service_tests.rs"
```

---

## Parallel Example: User Story 2

```text
Task: "T025 [P] [US2] Add connector-unavailable recovery tests in packages/ui/tests/polar_mcp_connector_tests.rs"
Task: "T026 [P] [US2] Add required-tool discovery and unsupported-tool tests in packages/ui/tests/polar_mcp_connector_tests.rs"
```

---

## Implementation Strategy

### MVP First

1. Complete Phase 1 setup documentation and baseline inventory.
2. Complete Phase 2 connector boundary and focused tests.
3. Complete Phase 3 US1 so the same Polar setup experience works through the centralized boundary.
4. Stop and validate US1 independently before optimizing speed or expanding diagnostics.

### Incremental Delivery

1. Foundation: documented connector plus centralized raw Polar execution.
2. US1: same setup/game/dashboard behavior with better idempotency.
3. US2: standardized connector health and recovery UX.
4. US3: measured speed improvements with state snapshot reuse and bounded waits.
5. US4: final local-only, redaction, persistence, and destructive-operation safety hardening.

### Notes

- Keep implementation scoped to Polar automation stability and speed.
- Do not change QR auth behavior as part of this feature.
- Do not expose Polar, Bitcoin RPC, Lightning RPC, local databases, or connector endpoints outside localhost.
- Do not persist or print secrets while collecting verification evidence.
