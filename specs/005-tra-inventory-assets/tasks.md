# Tasks: TRA Inventory Assets

**Input**: Design documents from `/specs/005-tra-inventory-assets/`  
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/server-functions.md, quickstart.md

**Tests**: No separate test-first workflow was requested. Verification tasks are included in the final phase.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel because it touches different files and does not depend on incomplete tasks.
- **[Story]**: Maps the task to a user story from `spec.md`.
- Every task includes exact repository file paths.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm the feature artifacts and existing TRA API boundary are aligned before implementation continues.

- [X] T001 Review `specs/005-tra-inventory-assets/spec.md`, `specs/005-tra-inventory-assets/plan.md`, and `specs/005-tra-inventory-assets/contracts/server-functions.md` for the current TRA scope before editing code
- [X] T002 [P] Update `Documentation/DioxusFeatureMatrix.md` with the planned TRA setup step, `TraService` boundary, inventory snapshot behavior, and future Taproot Assets adapter work
- [X] T003 [P] Confirm TRA-related user-facing terms are represented in `packages/ui/assets/i18n/en-US.ftl`, `packages/ui/assets/i18n/es-MX.ftl`, `packages/ui/assets/i18n/fr-FR.ftl`, and `packages/ui/assets/i18n/pt-BR.ftl`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Complete shared models, service APIs, adapter boundaries, and persistence assumptions that all user stories depend on.

**Critical**: No user story work should start until this phase is complete.

- [X] T004 Complete TRA DTOs and constants in `packages/lightning-service/src/client/models.rs` for `GameItemDefinition`, `TraItem`, `MintTraRequest`, `TransferTraRequest`, `TraOwnershipStatus`, `TraTransferStatus`, `BOOK_ITEM_ID`, and `MAX_TRA_ITEMS_PER_NODE`
- [X] T005 Complete TRA error variants in `packages/lightning-service/src/client/error.rs` for unavailable items, full inventory, unsupported item type, duplicate name, owner mismatch, and future adapter failures
- [X] T006 Complete the KISS/DRY `TraService` domain API in `packages/lightning-service/src/client/tra_service.rs` for catalog lookup, setup verification, minting, transfer, owner inventory, capacity checks, unique-name checks, and ownership logging
- [X] T007 Wire `tra_service` exports in `packages/lightning-service/src/client/mod.rs` and `packages/lightning-service/src/lib.rs`
- [X] T008 Implement a server-only Taproot Assets adapter in `packages/lightning-service/src/server/tra_client.rs` for Polar/Litd or `tapd` capability verification, mint/discover, transfer, proof status, and owner verification; add any required server-only dependency, feature flag, endpoint, or profile/config wiring in `packages/lightning-service/Cargo.toml` and existing setup profile/config files; then export it from `packages/lightning-service/src/server/mod.rs`
- [X] T009 Wire TRA fields into the default lab snapshot in `packages/lightning-service/src/client/lab_service.rs`
- [X] T010 Re-export TRA DTOs and constants through `packages/ui/src/client/models.rs`
- [X] T011 Add UI-facing async wrappers for `get_tra_item_catalog`, `verify_tra_setup`, `mint_tra`, and `transfer_tra` in `packages/ui/src/client/services/lightning_server_functions.rs`
- [X] T012 Add explicit snapshot-safety checks in `packages/ui/src/client/services/storage_service.rs` so persisted `LabState.tra_items` includes only non-sensitive identity, `item_id`, owner, and status fields and excludes Taproot Assets private keys, macaroons, wallet seeds, or sensitive proof material

**Checkpoint**: TRA domain model, service API, UI wrapper API, and persistence snapshot shape are ready.

---

## Phase 3: User Story 1 - Prepare TRA Inventory During Setup (Priority: P1)

**Goal**: Add a TRA inventory preparation step after Polar Lightning nodes are connected and funded, verify support before continuing, and assign initial mock items to NPCs.

**Independent Test**: Run setup from a fresh local lab and confirm the TRA step verifies the environment, creates or discovers required item assets, assigns them to NPC owners, and blocks progression when verification fails.

### Implementation for User Story 1

- [X] T013 [US1] Add a `TraInventory` wizard state after `BlockHeight` and before `Complete` in `packages/ui/src/client/pages/setup.rs`
- [X] T014 [P] [US1] Add `Add Tap Root Assets` setup copy, TRA inventory labels, ownership status labels, partial-completion messages, and failure strings to `packages/ui/assets/i18n/en-US.ftl`, `packages/ui/assets/i18n/es-MX.ftl`, `packages/ui/assets/i18n/fr-FR.ftl`, and `packages/ui/assets/i18n/pt-BR.ftl`
- [X] T015 [P] [US1] Add reusable TRA setup status UI to `packages/ui/src/client/components/setup/mod.rs`
- [X] T016 [US1] Wire the TRA setup step in `packages/ui/src/client/pages/setup.rs` to call `verify_tra_setup` and show visible loading, success, failure, and recovery feedback
- [X] T017 [US1] Add initial NPC item preparation in `packages/ui/src/client/pages/setup.rs` using `mint_tra` for unique TRA-backed items including `Book` with `item_id=1`
- [X] T018 [US1] Prevent setup completion in `packages/ui/src/client/pages/setup.rs` until TRA setup has verified capability and every initial setup item is verified
- [X] T019 [US1] Ensure submitting `Add Tap Root Assets` in `packages/ui/src/client/pages/setup.rs` clears the previous setup inventory snapshot, abandons prior setup item identities for gameplay unless rediscovered and verified, and recreates the initial TRA items from scratch
- [X] T020 [US1] Update setup reset/restart paths in `packages/ui/src/client/pages/setup.rs` so TRA state returns to the correct wizard step after stale Polar bridge, missing demo nodes, or Polar reset recovery

**Checkpoint**: User Story 1 is independently functional when setup can prepare verified TRA inventory before Play Game unlocks.

---

## Phase 4: User Story 2 - See Unique Mock Items Owned By NPCs (Priority: P1)

**Goal**: Display TRA-backed inventories with unique names, `item_id`-derived type, owner, and ownership status.

**Independent Test**: Open Play Game after TRA setup and confirm every visible inventory slot maps to a unique TRA-backed mock item name, status, and catalog entry.

### Implementation for User Story 2

- [X] T021 [US2] Replace payment-derived book counts with concrete `LabState.tra_items` inventory derivation in `packages/ui/src/client/pages/play_game.rs`
- [X] T022 [US2] Add catalog lookup helpers in `packages/ui/src/client/pages/play_game.rs` so `item_id=1` resolves to Book cost and visuals
- [X] T023 [US2] Extend `GameInventorySlot` in `packages/ui/src/client/components/game/game_view.rs` to carry `tra_id`, TRA item name, `item_id`, owner status, transfer status, and visual key
- [X] T024 [US2] Update inventory rendering in `packages/ui/src/client/components/game/game_view.rs` to display unique item names and unsupported, pending, missing, failed, or verified ownership states
- [X] T025 [P] [US2] Update game inventory styling in `packages/web/assets/main.css` for TRA item labels, status states, and compact 3-slot layout
- [X] T026 [P] [US2] Update game inventory styling in `packages/desktop/assets/main.css` for TRA item labels, status states, and compact 3-slot layout
- [X] T027 [US2] Update buy/sell button availability in `packages/ui/src/client/pages/play_game.rs` to use TRA owner capacity, ownership status, and catalog price instead of payment-derived book counts

**Checkpoint**: User Story 2 is independently functional when NPC inventories show unique TRA-backed items and catalog-derived Book behavior.

---

## Phase 5: User Story 3 - Transfer Ownership When Buying Or Selling (Priority: P1)

**Goal**: Coordinate Lightning payment with TRA ownership transfer for buy and sell flows, then verify ownership before finalizing visible inventory.

**Independent Test**: Complete setup, open a trade with an NPC, buy one item, sell it back or sell another owned item, and confirm both visible inventories and TRA ownership status change.

### Implementation for User Story 3

- [X] T028 [US3] Update Buy Item flow in `packages/ui/src/client/pages/play_game.rs` to select a concrete NPC-owned `tra_id`, create/pay the NPC invoice, call `transfer_tra` from NPC to Alice, and finalize inventory only after verified ownership
- [X] T029 [US3] Update Sell Item flow in `packages/ui/src/client/pages/play_game.rs` to select a concrete Alice-owned `tra_id`, create/pay the Alice invoice, call `transfer_tra` from Alice to NPC, and finalize inventory only after verified ownership
- [X] T030 [US3] Add recoverable partial-completion handling in `packages/ui/src/client/pages/play_game.rs` when payment succeeds but TRA transfer or verification fails
- [X] T031 [US3] Add TRA transfer details to action log entries in `packages/lightning-service/src/client/tra_service.rs` so the game log distinguishes payment from item transfer
- [X] T032 [US3] Ensure duplicate transfer attempts are disabled for pending or failed item transfers in `packages/ui/src/client/pages/play_game.rs`

**Checkpoint**: User Story 3 is independently functional when buy and sell move TRA ownership with visible payment and item-transfer feedback.

---

## Phase 6: User Story 4 - Recover From Stale Or Missing TRA State (Priority: P2)

**Goal**: Reverify saved TRA state after app or Polar restart and block trading for missing or unsupported items until repaired.

**Independent Test**: Complete setup, restart the app, and confirm setup or Play Game verifies existing item ownership before enabling buy or sell actions.

### Implementation for User Story 4

- [X] T033 [US4] Extend lab resume flow in `packages/ui/src/client/services/lightning_server_functions.rs` to call TRA verification when loading a connected `LabState`
- [X] T034 [US4] Add stale, missing, and unsupported TRA recovery messages in `packages/ui/src/client/pages/setup.rs`
- [X] T035 [US4] Add Play Game recovery gating in `packages/ui/src/client/pages/play_game.rs` so missing or unsupported TRA items block only affected item trades
- [X] T036 [US4] Implement real-adapter recovery mapping in `packages/lightning-service/src/server/tra_client.rs` for missing asset, unsupported metadata, proof pending, and transfer failed states

**Checkpoint**: User Story 4 is independently functional when stale saved TRA state is reverified or visibly blocked for recovery.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Finish documentation, validation, and repository-wide checks.

- [X] T037 [P] Add TRA service unit tests in `packages/lightning-service/src/client/tra_service.rs` for capacity, catalog lookup, reset-and-recreate setup inventory, mint, transfer, and owner mismatch behavior
- [X] T038 [P] Add Taproot Assets adapter tests or documented fake-adapter checks in `packages/lightning-service/src/server/tra_client.rs` for capability verification, missing asset, unsupported metadata, proof pending, and transfer failed behavior
- [X] T039 [P] Add setup wizard tests in `packages/ui/src/client/pages/setup.rs` for the `Add Tap Root Assets` step order and reset-to-step recreation behavior
- [X] T040 [P] Add Play Game helper tests in `packages/ui/src/client/pages/play_game.rs` for TRA inventory derivation from `LabState.tra_items`, unsupported item gating, selected `tra_id` transfer targeting, and catalog price lookup
- [X] T041 [P] Update `specs/005-tra-inventory-assets/quickstart.md` with any final setup button labels, status messages, and known Polar/Litd or `tapd` requirements discovered during implementation
- [X] T042 [P] Update `Documentation/DioxusFeatureMatrix.md` with implemented TRA routes, service functions, cache behavior, platform support, and suggested future work
- [X] T043 Run `cargo fmt` from the repository root using `Cargo.toml`
- [X] T044 Run `cargo test -p lightning-service` from the repository root using `packages/lightning-service/Cargo.toml`
- [X] T045 Run `cargo check -p ui --target wasm32-unknown-unknown` from the repository root using `packages/ui/Cargo.toml`
- [X] T046 Run `cargo check -p desktop` from the repository root using `packages/desktop/Cargo.toml`
- [ ] T047 Serve the web app with `.\Scripts\Common\RunWeb.ps1` and verify setup, Play Game inventory display, buy flow, sell flow, recovery feedback, the 1-second visible status-feedback goal, and the 2-minute initial TRA setup goal against `specs/005-tra-inventory-assets/quickstart.md`
  - Note: The served app responded with HTTP 200 on `http://localhost:8080`; full setup/buy/sell timing verification remains unchecked because it requires an active local Polar/Litd or `tapd` lab session.
- [X] T048 Stop the served web app with `.\Scripts\Other\StopWeb.ps1`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 Setup**: No dependencies.
- **Phase 2 Foundational**: Depends on Phase 1 and blocks all user stories.
- **Phase 3 US1**: Depends on Phase 2.
- **Phase 4 US2**: Depends on Phase 2; can run in parallel with US1 if the team coordinates `setup.rs` versus `play_game.rs` edits.
- **Phase 5 US3**: Depends on Phase 4 for TRA inventory display helpers and button availability.
- **Phase 6 US4**: Depends on Phase 2 and can begin after US1/US2 state shapes are stable.
- **Phase 7 Polish**: Depends on desired story phases being complete.

### User Story Dependencies

- **US1 Prepare TRA Inventory During Setup**: MVP setup slice; no dependency on other user stories after foundational tasks.
- **US2 See Unique Mock Items Owned By NPCs**: Requires foundational TRA models and wrapper APIs; does not require buy/sell transfer completion.
- **US3 Transfer Ownership When Buying Or Selling**: Requires US2 inventory derivation and display behavior.
- **US4 Recover From Stale Or Missing TRA State**: Requires foundational TRA states and can be completed after setup/display flows are stable.

### Within Each User Story

- Domain/model work before UI calls.
- Service wrappers before page integration.
- Setup and Play Game state gating before visual polish.
- Verification and documentation after implementation.

---

## Parallel Opportunities

- T002 and T003 can run in parallel with T001.
- T004, T005, T008, and T010 can run in parallel because they touch different files.
- T014 and T015 can run in parallel during US1.
- T025 and T026 can run in parallel during US2.
- T037 through T042 can run in parallel during polish when their file ownership does not overlap.

---

## Parallel Example: User Story 2

```text
Task: "Update game inventory styling in packages/web/assets/main.css for TRA item labels, status states, and compact 3-slot layout"
Task: "Update game inventory styling in packages/desktop/assets/main.css for TRA item labels, status states, and compact 3-slot layout"
```

```text
Task: "Replace payment-derived book counts with LabState.tra_items inventory derivation in packages/ui/src/client/pages/play_game.rs"
Task: "Extend GameInventorySlot in packages/ui/src/client/components/game/game_view.rs to carry TRA item name, item_id, owner status, transfer status, and visual key"
```

---

## Implementation Strategy

### MVP First

1. Complete Phase 1 and Phase 2.
2. Complete Phase 3 (US1) so setup can prepare verified TRA inventory.
3. Stop and validate setup independently using `specs/005-tra-inventory-assets/quickstart.md`.

### Incremental Delivery

1. Add US1 setup preparation.
2. Add US2 inventory display and catalog resolution.
3. Add US3 buy/sell ownership transfer.
4. Add US4 recovery behavior.
5. Run Phase 7 verification.

### Notes

- Keep direct Taproot Assets and LND access behind `packages/lightning-service`.
- Do not store Taproot Assets private keys, macaroons, wallet seeds, or sensitive proof material in browser storage.
- Keep all Dioxus UI changes on 0.7 APIs.
- Keep web and desktop support intact.
