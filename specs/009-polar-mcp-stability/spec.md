# Feature Specification: Polar MCP Stability

**Feature Branch**: `[009-polar-mcp-stability]`  
**Created**: 2026-05-19  
**Status**: Draft  
**Input**: User description: "add this jamaljsr/polar-mcp so you can improve the stability and speed of all existing interactions between rust and polar. Include in the plan the installation and refactor of the project so the same user experience exists with better stability and speed."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Keep The Same Polar Setup Experience (Priority: P1)

As a developer or demo operator, I want the existing Polar setup steps to behave the same from the app user's perspective while becoming faster and less fragile, so that I can prepare the local Lightning lab without learning a new workflow.

**Why this priority**: The requested improvement is explicitly a stability and speed refactor, not a visible workflow redesign.

**Independent Test**: Can be tested by running the setup flow from Bridge URL through Unlock Routes and confirming the same step labels, decisions, progress messages, and final connected result are still present.

**Acceptance Scenarios**:

1. **Given** the user opens Set Up, **When** they use the Polar connection path, **Then** the visible setup order remains `Bridge URL`, `Server Name`, `Create Nodes`, `Game Treasury (Sats)`, `Game Treasury (TRAs)`, `User Nodes (Sats)`, `User Nodes (TRAs)`, `Block Height`, and `Unlock Routes`.
2. **Given** an existing Polar network already has some required resources, **When** the user reruns setup, **Then** the app reuses valid existing resources instead of failing or duplicating them.
3. **Given** Polar reports that a network or node is already started, **When** the app asks for the started state, **Then** the app treats the result as success and continues.
4. **Given** a setup step performs a long-running Polar action, **When** the action is in progress, **Then** the app continues to show visible progress or status feedback.
5. **Given** setup completes successfully, **When** the user opens Play Game or Network Dashboard, **Then** the existing gameplay and diagnostics surfaces remain usable without new navigation or setup requirements.

---

### User Story 2 - Use A Standard Polar Automation Connector (Priority: P1)

As a maintainer, I want the project to depend on a maintained Polar automation connector rather than project-specific assumptions about Polar internals, so that future Polar versions are easier to support and failures are easier to diagnose.

**Why this priority**: Standardizing the Polar integration is the main way to improve stability and reduce one-off repair work.

**Independent Test**: Can be tested by following the documented installation path, verifying the connector health check, and confirming that each app operation goes through the same connector contract.

**Acceptance Scenarios**:

1. **Given** a fresh development machine with Polar installed, **When** the maintainer follows the project setup instructions, **Then** the Polar automation connector can be installed or launched without manual source checkout.
2. **Given** Polar is running locally, **When** the app verifies the automation connector, **Then** it reports a clear healthy or unavailable status before attempting setup mutations.
3. **Given** the connector is unavailable, **When** the user starts setup, **Then** the app explains the missing dependency and recovery step instead of showing a generic failure.
4. **Given** the connector reports a tool-level failure, **When** the app surfaces the error, **Then** the message names the failed operation without exposing secrets or local credential material.

---

### User Story 3 - Make Polar Operations Faster (Priority: P2)

As a developer iterating on the demo, I want repeated Polar reads and dependent operations to finish with fewer redundant waits, so that setup and gameplay testing feels responsive.

**Why this priority**: Speed matters after the stable connector is in place, but correctness and unchanged user experience come first.

**Independent Test**: Can be tested by timing repeated setup and dashboard refresh flows before and after the refactor and confirming reduced unnecessary polling while preserving correctness.

**Acceptance Scenarios**:

1. **Given** a setup step needs the current network state more than once, **When** the step runs, **Then** it avoids redundant full-state reads where a recent state snapshot is still valid.
2. **Given** multiple nodes need the same state transition, **When** the app performs that transition, **Then** it batches or sequences work in a way that reduces total wait time without hiding failures.
3. **Given** a poll is waiting for a known Polar state change, **When** the desired state appears, **Then** the app stops waiting immediately and advances the setup step.
4. **Given** a transient connector failure occurs, **When** retrying is appropriate, **Then** the app retries with bounded attempts and clear progress feedback.

---

### User Story 4 - Preserve Existing Safety Boundaries (Priority: P2)

As a maintainer, I want the refactor to keep the app's current local-only and secret-safety boundaries, so that better automation does not widen the risk surface.

**Why this priority**: Polar controls local Lightning lab resources, so automation must stay local and must not persist or reveal credentials.

**Independent Test**: Can be tested by reviewing saved setup data, generated logs, and failure messages after connector failures and successful setup operations.

**Acceptance Scenarios**:

1. **Given** the connector is configured, **When** the app saves preferences, **Then** only non-sensitive local setup choices and status snapshots are saved.
2. **Given** the app performs Polar automation, **When** it connects to the connector, **Then** it uses a local endpoint and does not expose local services to the public internet.
3. **Given** the app logs a Polar failure, **When** the message includes details from the connector, **Then** secret-like content is redacted before display or logs.
4. **Given** a destructive Polar operation would remove user-created networks or nodes outside the demo's bounded setup contract, **When** that operation is requested, **Then** the app blocks or requires an explicit app-owned safety path.

### Edge Cases

- Polar is not installed or is not running when setup starts.
- The automation connector package cannot be downloaded or launched.
- The connector health endpoint is reachable but tool discovery fails.
- The connector exposes a different tool shape than the app expects.
- Polar returns numeric network identifiers in one response and string identifiers in another.
- A network, node, channel, asset, or invoice changes in the Polar UI while the app is polling.
- A node is already started, stopped, or restarting when the app requests a transition.
- A long-running operation exceeds the normal timeout but later succeeds in Polar.
- A connector error includes a local file path, macaroon path, token-like string, or other sensitive text.
- A saved setup profile references a network that no longer exists.
- The user is in mock/offline connection mode and should not be required to install the Polar connector.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST preserve the existing user-facing setup, gameplay, and dashboard flows while improving the internal Polar automation path.
- **FR-002**: The system MUST provide a documented path to install or launch the standard Polar automation connector required for networked Polar interactions.
- **FR-003**: The system MUST verify the connector is healthy before performing Polar mutations.
- **FR-004**: The system MUST provide actionable recovery messages when Polar or the connector is unavailable.
- **FR-005**: The system MUST route all networked Polar reads and mutations through one consistent automation boundary.
- **FR-006**: The system MUST keep mock/offline app behavior available without requiring Polar or the connector.
- **FR-007**: The system MUST preserve the existing Polar setup order and route-locking behavior.
- **FR-008**: The system MUST treat already-satisfied setup conditions as success rather than failure, including existing networks, existing nodes, already-started nodes, already-funded wallets, already-open routes, and already-created assets.
- **FR-009**: The system MUST reduce redundant full-network reads during setup and dashboard refreshes where a recent validated state can be reused.
- **FR-010**: The system MUST use bounded retry and polling behavior for transient connector or Polar readiness failures.
- **FR-011**: The system MUST stop polling as soon as the desired state is observed.
- **FR-012**: The system MUST keep visible progress or status feedback during connector checks, setup mutations, funding, route work, invoice/payment operations, asset work, and dashboard refreshes.
- **FR-013**: The system MUST surface connector/tool failures with operation names and recovery guidance.
- **FR-014**: The system MUST redact secret-like values from connector errors, logs, saved snapshots, and user-facing messages.
- **FR-015**: The system MUST keep connector access local-only unless a future feature explicitly approves another access model.
- **FR-016**: The system MUST avoid destructive Polar cleanup outside the app's bounded demo setup contract.
- **FR-017**: The system MUST provide diagnostics that show connector health, selected network, required node readiness, and last Polar operation status.
- **FR-018**: The system MUST include verification coverage for successful setup, connector-unavailable recovery, idempotent reruns, and at least one gameplay or dashboard Polar read after setup.

### Key Entities

- **Polar Automation Connector**: The local tool bridge that lets the app inspect and mutate Polar networks through a stable command surface.
- **Connector Health Status**: The current availability and readiness of the local connector.
- **Polar Operation**: A named read or mutation such as listing networks, starting a network, starting a node, funding a wallet, opening a channel, creating an invoice, paying an invoice, or managing assets.
- **Polar State Snapshot**: A recent validated view of the current network, nodes, balances, channels, invoices, and assets.
- **Operation Progress Event**: A non-sensitive status update shown while a setup, gameplay, or dashboard operation is running.
- **Connector Failure**: A recoverable unavailable, timeout, schema mismatch, or tool-level error with redacted details and recovery guidance.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can complete the Polar setup flow with the same visible step order and end state as before the refactor.
- **SC-002**: On a warm existing demo network, rerunning setup completes with no false failure for already-created or already-started resources in 95% of tested runs.
- **SC-003**: Repeated network-state reads during a single setup step are reduced by at least 30% compared with the pre-refactor path for that step.
- **SC-004**: Warm-network setup completion time improves by at least 25% compared with the pre-refactor baseline on the same machine and Polar network.
- **SC-005**: Connector-unavailable failures identify the missing or unhealthy connector and provide a recovery step in 100% of tested unavailable scenarios.
- **SC-006**: Secret-safety review of saved setup data, logs, and displayed errors finds no wallet secrets, macaroons, seed material, private keys, cookies, API tokens, or credential material.
- **SC-007**: The mock/offline connection path remains usable without installing or launching Polar automation.
- **SC-008**: At least one full Play Game or Network Dashboard flow after setup confirms that faster setup does not break downstream Polar-backed state.

## Assumptions

- The target connector is the Polar MCP server published as `@lightningpolar/mcp`, from the `jamaljsr/polar-mcp` repository.
- The connector runs locally and communicates with the Polar app's local bridge.
- The feature improves the app's Rust-to-Polar automation path; it does not change the app's public routes, visual setup order, or Lightning game rules.
- The app may add project setup documentation or scripts for connector installation, but mock/offline mode remains independent of that dependency.
- Existing QR auth work remains separate; this feature is about Polar interaction stability and speed for setup, gameplay support operations, and dashboard diagnostics.
