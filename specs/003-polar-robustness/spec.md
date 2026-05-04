# Feature Specification: Polar Lab Robustness

**Feature Branch**: `003-polar-robustness`  
**Created**: 2026-05-03  
**Status**: Draft  
**Input**: User description: "Add Polar lab robustness observer and recovery behavior."

## User Scenarios & Testing

### User Story 1 - Recover From Broken Polar Setup (Priority: P1)

A learner can playfully stop the Polar network, delete the Polar network, or remove one of the app-created demo nodes without leaving the app unlocked against stale state.

**Why this priority**: The current learning loop depends on local Polar resources that can be changed outside the app.

**Independent Test**: Complete setup, stop the Polar network or remove `alice`, `bob`, or `carol` in Polar, return to the app or refresh any route, and verify gameplay/debug routes lock with a blocking prompt and the setup wizard resumes at the right step.

**Acceptance Scenarios**:

1. **Given** setup is complete, **When** the Polar bridge is no longer reachable, **Then** the app shows a blocking prompt, locks gameplay/debug, preserves setup fields, and resumes setup at the bridge URL step.
2. **Given** setup is complete, **When** the saved Polar network is stopped, **Then** the app shows a blocking prompt, navigates to `Home`, locks gameplay/debug, and resumes setup at the server step.
3. **Given** setup is complete, **When** the saved Polar network is deleted, **Then** the app shows a blocking prompt, navigates to `Home`, clears the discovered network id, and resumes setup at the server-name step.
4. **Given** setup is complete, **When** a demo node is deleted, **Then** the app shows a blocking prompt, navigates to `Home`, clears the active lab snapshot, and resumes setup at the demo-node step.
5. **Given** setup is complete and Polar remains healthy, **When** the learner refreshes on `Home`, `Set Up`, `Play Game`, or `Network Dashboard`, **Then** the current route stays active and the saved passing setup remains connected.
6. **Given** a trade route is under construction on `Play Game`, **When** the learner mines the next block directly in Polar, **Then** the app detects the external block from its normal page polling, marks the route active, and enables `Buy Item` without requiring the learner to press `Wait for Next Block`.

---

### User Story 2 - Prefer Event-Ready Observation (Priority: P2)

The app records an event-first observer boundary for native/server LND streams while keeping browser builds on safe Polar bridge checks.

**Why this priority**: LND exposes useful server-streaming APIs, but Polar network/node lifecycle changes still need bridge classification.

**Independent Test**: Build web and desktop targets and verify the observer types compile without storing credentials in browser state.

**Acceptance Scenarios**:

1. **Given** the app is built for browser, **When** lab health is checked, **Then** it uses Polar bridge request/response checks only.
2. **Given** native/server observer support is extended later, **When** LND streams emit channel, invoice, payment, peer, node, or block events, **Then** the app can map them to normalized lab events.

### Edge Cases

- Polar bridge is unavailable while the saved profile still says connected.
- Polar network exists but is stopped.
- Polar network was deleted and the saved numeric id no longer exists.
- One or more app-owned demo nodes are missing or stopped.
- Bitcoin backend cannot be found in the saved network.
- A health check fails while the learner is already on a locked route.

## Requirements

### Functional Requirements

- **FR-001**: The app MUST classify Polar lab health before using saved connected lab state on gameplay and debug routes.
- **FR-002**: The app MUST distinguish bridge unavailable, network missing, network stopped, Bitcoin backend missing, demo node missing, and demo node stopped states.
- **FR-003**: The app MUST recover broken Polar lab state by updating saved setup state, clearing stale lab snapshots when needed, showing a blocking prompt, and navigating to `Home`.
- **FR-004**: Recovery MUST preserve non-sensitive setup preferences such as bridge URL, server name, sats-per-transaction, and setup mode whenever possible.
- **FR-005**: The setup wizard MUST resume at the step that can repair the classified failure.
- **FR-006**: Browser builds MUST NOT store LND credentials or introduce browser SQLite/OPFS.
- **FR-007**: Native/server observer types MUST model LND stream events without requiring browser support.
- **FR-008**: Documentation MUST describe the new health recovery and observer extension point.
- **FR-009**: `packages/lightning-service` observer and Lightning domain boundaries MUST remain theoretically reusable in future Rust projects by avoiding Dioxus UI dependencies, route-specific behavior, and app-specific toast/navigation concerns.
- **FR-010**: The app MUST validate a saved connected Polar setup after browser/app refresh on every primary route without changing routes when the setup still passes.
- **FR-011**: Happy-path setup, game, and debug progress MUST use subtle toasts; broken-lab recovery and reset flows MUST use the queued blocking prompt surface.
- **FR-012**: Prompt messages MUST remain onscreen for at least 0.25 seconds per message and support cancel requests for reset operations that can restore the previous app state after the external Polar call completes.
- **FR-013**: Resetting Polar setup step 4 after the app has been unlocked MUST first show an "Are you sure?" blocking prompt because this action locks a properly running app.
- **FR-014**: Submitting Polar setup step 3 MUST use the blocking prompt with a red cancel button because demo-node creation can take long enough that subtle toast progress is easy to miss.
- **FR-015**: Polar setup step 3 MUST NOT confirm success until polling verifies that the server is running, `alice`, `bob`, and `carol` are running with exact lowercase app-owned names, and each node reports the exact fresh demo wallet balance required by the app rules.
- **FR-016**: Polar setup step 2 MUST match the typed server name to a Polar network name only, ignoring unrelated running servers and unrelated numeric ids; it MUST create the named server when no name match exists and verify the named server is running before passing the step.

### Key Entities

- **Lab Health Issue**: A classified reason the saved Polar lab cannot be trusted.
- **Lab Recovery**: The setup profile, lab snapshot policy, prompt message, and route fallback used after a health issue.
- **Lab Observer Event**: A normalized future event from LND streams or Polar health classification.

## Success Criteria

### Measurable Outcomes

- **SC-001**: A stopped Polar network locks gameplay/debug and resumes setup at the server step within one app interaction.
- **SC-002**: A deleted demo node locks gameplay/debug and resumes setup at the demo-node step within one app interaction.
- **SC-003**: A deleted Polar network clears stale discovered ids and does not leave gameplay/debug unlocked.
- **SC-004**: Web, desktop, and unit-test checks pass after the recovery behavior is implemented.

## Assumptions

- Polar's HTTP bridge remains request/response only for app purposes: `/health`, `/api/mcp/tools`, and `/api/mcp/execute`.
- LND streaming APIs are a native/server extension point, not browser state.
- Connected Polar setup uses lightweight app-level polling on every primary route in addition to refresh-time, route-entry, and action-triggered checks.
