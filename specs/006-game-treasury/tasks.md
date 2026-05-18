# Tasks: Game Treasury

**Input**: Design documents from `/specs/006-game-treasury/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/treasury-server-functions.md, quickstart.md

**Tests**: The feature specification does not require TDD. Validation and smoke-check tasks are included in the final phase because browser-visible behavior and service correctness are required by the constitution and plan.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel with other marked tasks in the same phase because it touches different files and does not depend on incomplete tasks
- **[Story]**: Maps to the user story from `specs/006-game-treasury/spec.md`
- All task descriptions include concrete repository paths

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Align documentation, localizable text surface, and shared module structure before implementing treasury behavior.

- [x] T001 Update Game Treasury scope, setup sequence, cache behavior, platform support, and future work in Documentation/DioxusFeatureMatrix.md
- [x] T002 Add localizable setup labels and gameplay treasury copy keys for Game Treasury, User Nodes, NPC Item Transfers, treasury status, history, and impact previews in packages/ui/assets/i18n/en-US/main.ftl
- [x] T003 [P] Add setup component module declarations for Game Treasury and NPC Item Transfers in packages/ui/src/client/components/setup/mod.rs
- [x] T004 [P] Add gameplay component module declaration for Game Treasury UI in packages/ui/src/client/components/game/mod.rs

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Create shared treasury data contracts and service boundaries required by every user story.

**CRITICAL**: No user story work should begin until this phase is complete.

- [x] T005 Add `GameTreasury`, `TreasuryStatus`, `TreasuryResource`, `TreasuryEntry`, `TreasuryImpactPreview`, `SetupStep`, and `NpcItemTransfer` DTOs in packages/lightning-service/src/client/models.rs
- [x] T006 Add treasury-specific domain errors for node setup, funding, item preparation, transfer, readiness, stale state, and sensitive-description rejection in packages/lightning-service/src/client/error.rs
- [x] T007 Add treasury summary, treasury history, and NPC transfer state to the lab/game snapshot model in packages/lightning-service/src/client/lab_service.rs
- [x] T008 Add treasury domain service methods matching specs/006-game-treasury/contracts/treasury-server-functions.md in packages/lightning-service/src/client/tra_service.rs
- [x] T009 Expose UI-facing treasury server function wrappers for setup, summary, impact preview, and event recording in packages/ui/src/client/services/lightning_server_functions.rs
- [x] T010 Persist only non-sensitive treasury summaries, node labels, item identities, transfer results, and history snapshots in packages/ui/src/client/services/storage_service.rs
- [x] T011 Add UI treasury view models for setup state, treasury summary, history entries, transfer results, and action previews in packages/ui/src/client/models.rs

**Checkpoint**: Shared treasury contracts and service boundaries are ready for user story implementation.

---

## Phase 3: User Story 1 - Create Game Treasury During Setup (Priority: P1) MVP

**Goal**: Polar setup creates/verifies a dedicated GAME_TREASURY node after Bridge URL and Server Name, funds it, and assigns initial NPC-bound items to the treasury before user nodes are created.

**Independent Test**: Run setup through the Game Treasury step and confirm that Game Treasury exists, is funded for game activity, owns the initial NPC-bound items, and shows visible progress/failure feedback.

### Implementation for User Story 1

- [x] T012 [US1] Replace the setup step sequence with Bridge URL, Server Name, Game Treasury, User Nodes, NPC Item Transfers, Block Height, and Unlock Routes in packages/ui/src/client/pages/setup.rs
- [x] T013 [P] [US1] Implement reusable Game Treasury setup status UI with loading, success, retry, and recoverable failure states in packages/ui/src/client/components/setup/game_treasury.rs
- [x] T014 [US1] Render the Game Treasury setup step and gate step advancement from setup state in packages/ui/src/client/pages/setup.rs
- [x] T015 [US1] Wire Game Treasury setup actions to `create_or_verify_game_treasury`, `fund_game_treasury`, and `prepare_treasury_items` wrappers in packages/ui/src/client/pages/setup.rs
- [x] T016 [US1] Implement create-or-verify GAME_TREASURY node behavior in packages/lightning-service/src/client/tra_service.rs
- [x] T017 [US1] Implement scenario-based Game Treasury funding behavior in packages/lightning-service/src/client/tra_service.rs
- [x] T018 [US1] Implement creation or verification of treasury-owned items intended for NPC distribution in packages/lightning-service/src/client/tra_service.rs
- [x] T019 [US1] Record treasury entries for setup funding and item preparation in packages/lightning-service/src/client/lab_service.rs
- [x] T020 [US1] Save non-sensitive Game Treasury setup completion and treasury-owned item summaries in packages/ui/src/client/services/storage_service.rs
- [x] T021 [US1] Prevent User Nodes setup from advancing until the Game Treasury step is ready or visibly recoverable in packages/ui/src/client/pages/setup.rs

**Checkpoint**: User Story 1 is independently functional as the MVP setup increment.

---

## Phase 4: User Story 2 - Distribute NPC Items From Treasury (Priority: P2)

**Goal**: Setup creates/verifies Alice, Bob, and Carol as user nodes, then transfers Bob and Carol's starting items from Game Treasury to those NPCs.

**Independent Test**: Complete User Nodes and NPC Item Transfers setup, then confirm Bob and Carol receive the same starting items they currently receive, with each item visibly originating from Game Treasury.

### Implementation for User Story 2

- [x] T022 [US2] Rename or revise the setup node-creation step to User Nodes while preserving Alice, Bob, and Carol creation behavior in packages/ui/src/client/pages/setup.rs
- [x] T023 [P] [US2] Implement reusable NPC item transfer setup UI with per-item transfer progress and recoverable failure states in packages/ui/src/client/components/setup/npc_item_transfers.rs
- [x] T024 [US2] Render NPC Item Transfers setup step and block Block Height progression until transfer state is complete or visibly recoverable in packages/ui/src/client/pages/setup.rs
- [x] T025 [US2] Define Bob and Carol starting item transfer intents from existing item configuration in packages/lightning-service/src/client/tra_service.rs
- [x] T026 [US2] Implement treasury-to-NPC item transfer operations for Bob and Carol in packages/lightning-service/src/client/tra_service.rs
- [x] T027 [US2] Record treasury entries for each setup-time item transfer from Game Treasury to Bob or Carol in packages/lightning-service/src/client/lab_service.rs
- [x] T028 [US2] Persist non-sensitive NPC transfer results and refreshed ownership summaries in packages/ui/src/client/services/storage_service.rs
- [x] T029 [US2] Update Play Game initial inventory loading to use treasury-originated NPC ownership summaries in packages/ui/src/client/pages/play_game.rs

**Checkpoint**: User Stories 1 and 2 establish the revised startup economy and NPC inventory distribution.

---

## Phase 5: User Story 3 - Use Treasury Readiness in Gameplay (Priority: P3)

**Goal**: Gameplay actions reflect treasury readiness, show impact previews, and refresh treasury state after treasury-affecting actions complete, fail, or are cancelled.

**Independent Test**: Set treasury resources below and above a required action threshold, then confirm gameplay enables, blocks, or explains each action with accurate treasury impact feedback.

### Implementation for User Story 3

- [x] T030 [P] [US3] Implement reusable Game Treasury summary and history gameplay UI in packages/ui/src/client/components/game/game_treasury.rs
- [x] T031 [US3] Add treasury summary, status, and recent history display to Play Game in packages/ui/src/client/pages/play_game.rs
- [ ] T032 [US3] Add treasury impact previews for treasury-dependent buy, sell, reward, or cost actions in packages/ui/src/client/pages/play_game.rs
- [ ] T033 [US3] Gate treasury-dependent gameplay actions when treasury sats, required items, or freshness are insufficient in packages/ui/src/client/pages/play_game.rs
- [ ] T034 [US3] Refresh treasury state after gameplay actions complete, fail, or are cancelled in packages/ui/src/client/pages/play_game.rs
- [ ] T035 [US3] Implement treasury readiness and impact-preview calculations in packages/lightning-service/src/client/tra_service.rs
- [x] T036 [US3] Record gameplay treasury entries for rewards, costs, trades, and inventory movements in packages/lightning-service/src/client/lab_service.rs
- [x] T037 [US3] Persist non-sensitive treasury history snapshots after gameplay updates in packages/ui/src/client/services/storage_service.rs

**Checkpoint**: All user stories are independently functional with setup and gameplay treasury behavior.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Keep web/desktop behavior aligned, preserve user-visible status feedback, and update project docs.

- [ ] T038 [P] Review setup copy for player-facing, localizable, non-technical treasury language in packages/ui/src/client/pages/setup.rs
- [ ] T039 [P] Review Play Game copy for player-facing, localizable, non-technical balance, item, history, and preview explanations in packages/ui/src/client/pages/play_game.rs
- [ ] T040 Ensure all treasury setup, cache read/write, transfer, refresh, and gameplay operations preserve visible loading or toast-style feedback in packages/ui/src/client/pages/setup.rs
- [x] T041 Ensure treasury snapshots exclude sensitive wallet, node credential, macaroon, seed, proof, and transport details in packages/ui/src/client/services/storage_service.rs
- [ ] T042 Align desktop launch compatibility for shared treasury DTOs and server functions in packages/desktop/src/main.rs
- [ ] T043 Align web launch compatibility for shared treasury DTOs and server functions in packages/web/src/main.rs
- [ ] T044 Run `cargo test -p lightning-service` to validate treasury domain behavior
- [ ] T045 Run `cargo check -p ui --target wasm32-unknown-unknown` to validate browser UI compatibility
- [ ] T046 Run `cargo check -p desktop` to validate desktop compatibility
- [ ] T047 Run `.\Scripts\Common\RunWeb.ps1` and perform the setup/gameplay smoke from specs/006-game-treasury/quickstart.md

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies; can start immediately.
- **Foundational (Phase 2)**: Depends on Phase 1 and blocks all user stories.
- **User Story 1 (Phase 3)**: Depends on Phase 2 and is the MVP.
- **User Story 2 (Phase 4)**: Depends on User Story 1 because NPC item transfers require a ready Game Treasury and treasury-owned items.
- **User Story 3 (Phase 5)**: Depends on Phase 2 and can begin after treasury contracts exist, but final acceptance should integrate with User Stories 1 and 2.
- **Polish (Phase 6)**: Depends on desired user stories being complete.

### User Story Dependencies

- **User Story 1 (P1)**: No dependency on other user stories after foundational work.
- **User Story 2 (P2)**: Depends on User Story 1 because NPC items must originate from Game Treasury.
- **User Story 3 (P3)**: Can begin after foundational work, but complete acceptance requires the treasury setup and item transfer flows.

### Within Each User Story

- Shared DTOs and service methods before UI wiring.
- Setup state before setup progression gates.
- Treasury ownership changes before persistence updates.
- Core behavior before documentation polish and validation.

### Parallel Opportunities

- T003 and T004 can run in parallel during module setup.
- T013 can run while T016-T018 service behavior is implemented, then integrate in T015.
- T023 can run while T025-T027 transfer service behavior is implemented, then integrate in T024.
- T030 can run while T035-T036 gameplay treasury calculations are implemented, then integrate in T031-T034.
- T038 and T039 can run in parallel during copy polish.

---

## Parallel Example: User Story 1

```text
Task: "T013 [P] [US1] Implement reusable Game Treasury setup status UI with loading, success, retry, and recoverable failure states in packages/ui/src/client/components/setup/game_treasury.rs"
Task: "T016 [US1] Implement create-or-verify GAME_TREASURY node behavior in packages/lightning-service/src/client/tra_service.rs"
Task: "T017 [US1] Implement scenario-based Game Treasury funding behavior in packages/lightning-service/src/client/tra_service.rs"
```

## Parallel Example: User Story 2

```text
Task: "T023 [P] [US2] Implement reusable NPC item transfer setup UI with per-item transfer progress and recoverable failure states in packages/ui/src/client/components/setup/npc_item_transfers.rs"
Task: "T025 [US2] Define Bob and Carol starting item transfer intents from existing item configuration in packages/lightning-service/src/client/tra_service.rs"
```

## Parallel Example: User Story 3

```text
Task: "T030 [P] [US3] Implement reusable Game Treasury summary and history gameplay UI in packages/ui/src/client/components/game/game_treasury.rs"
Task: "T035 [US3] Implement treasury readiness and impact-preview calculations in packages/lightning-service/src/client/tra_service.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup.
2. Complete Phase 2: Foundational treasury contracts and service boundaries.
3. Complete Phase 3: Game Treasury setup step.
4. Stop and validate that setup creates/verifies, funds, and seeds the Game Treasury before User Nodes.

### Incremental Delivery

1. Add User Story 1 to make Game Treasury a real setup participant.
2. Add User Story 2 to distribute NPC starting items from Game Treasury.
3. Add User Story 3 to surface treasury readiness and impact during gameplay.
4. Complete polish tasks to keep web/desktop support, documentation, localization, safe persistence, and served-web validation aligned.

### Notes

- `setup-plan.ps1 -Json` reported branch `main`; the plan is still generated under `specs/006-game-treasury/`.
- `/speckit-plan` updated `AGENTS.md` to point at `specs/006-game-treasury/plan.md`.
- Do not store credentials, secrets, macaroons, seeds, or sensitive proofs in browser snapshots.

