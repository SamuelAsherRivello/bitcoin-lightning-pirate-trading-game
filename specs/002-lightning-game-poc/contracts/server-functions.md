# Server Function Contracts: Bitcoin Lightning Game POC

These contracts describe the domain operations exposed from the Dioxus UI to the server-side Lightning service. Exact Rust signatures may evolve during implementation, but the behavior and data boundaries should stay stable.

## Security Boundary

- Server functions are the only UI-accessible entrypoint for LND operations.
- Direct LND endpoints, TLS certificates, and macaroons must not be required for the main Polar automation flow.
- Every operation must reject non-regtest profiles in v1.
- Every spending operation must be visibly tied to a user action or the explicit AutoSend lab setting.

## `test_setup`

**Purpose**: Verify that Alice, Bob, and Carol are reachable and match regtest expectations.

**Input**:

- Optional setup profile id or inline regtest-only setup payload.
- `sats_per_transaction`.

**Output**:

- Overall connection status.
- Per-node status, pubkey, alias, balances when available.
- User-readable warnings.

**Errors**:

- Missing profile.
- Node unreachable.
- Non-regtest node detected.
- Invalid transaction amount.
- Polar bridge unavailable.

## `save_setup_preferences`

**Purpose**: Save non-sensitive setup preferences such as transaction amount and selected setup mode.

**Input**:

- `sats_per_transaction`.
- `network_name`.
- `setup_mode`.
- Polar bridge URL for Polar automation mode. Network and backend values are discovered through the bridge and saved when available.

**Output**:

- Saved setup summary.

**Errors**:

- Invalid amount.
- Missing or non-local Polar bridge URL when using networked mode.

## `create_polar_demo_nodes`

**Purpose**: Ask Polar to create Alice, Bob, and Carol LND nodes from an existing Bitcoin backend node.

**Input**:

- Polar bridge URL.
- Optional saved network name or id.
- Optional saved Bitcoin backend node name.
- `sats_per_transaction`.

**Output**:

- Created demo nodes.
- Updated setup status.
- Initial lab state.

**Errors**:

- Polar bridge unavailable.
- Polar tool execution failed.
- Polar created a node without returning a name the app can rename.

## `destroy_polar_demo_nodes`

**Purpose**: Ask Polar to remove Alice, Bob, and Carol and clear active lab state.

**Input**:

- Polar bridge URL.
- Optional saved network name or id.

**Output**:

- Saved setup profile with gameplay locked until demo nodes are created again.

**Errors**:

- Polar bridge unavailable.
- Polar remove-node tool failed for a node that should exist.

## `get_lab_state`

**Purpose**: Load current node, channel, invoice, payment, and block status for game/debug views.

**Input**:

- Setup profile id or active setup selection.

**Output**:

- Demo nodes.
- Trade routes.
- Recent invoices.
- Recent payments.
- Current block height if available.

**Errors**:

- Setup incomplete.
- One or more nodes unreachable.

## `open_trade_route`

**Purpose**: Start channel opening between two demo nodes.

**Input**:

- `from_node`.
- `to_node`.
- `capacity_sats`.
- Optional push amount.

**Output**:

- Trade route id.
- Channel point if known.
- Status `UnderConstruction`.
- `requires_next_block = true`.

**Errors**:

- Route already active or pending.
- Source node lacks confirmed on-chain funds.
- Peer connection failed.
- Channel open rejected by LND.

## `wait_for_next_block`

**Purpose**: Advance local regtest chain and refresh route/node state.

**Input**:

- Reason for waiting.
- Optional affected trade route id.
- Block count, default `1`.

**Output**:

- New block height.
- Updated affected route state.
- User-facing explanation that regtest mined instantly.

**Errors**:

- Bitcoin backend cannot mine.
- Affected route still pending after mining.

## `create_invoice`

**Purpose**: Create a Lightning invoice for a receive action.

**Input**:

- `creator_node`.
- Optional expected payer node.
- `amount_sats`.
- `memo`.

**Output**:

- Invoice id.
- Payment request.
- Status `Created`.

**Errors**:

- Creator node unreachable.
- Invalid amount.
- LND invoice creation failed.

## `pay_invoice`

**Purpose**: Pay a known invoice from a selected node.

**Input**:

- `payer_node`.
- Invoice id or payment request.

**Output**:

- Payment id.
- Status.
- Route summary if available.
- Updated route balances.

**Errors**:

- Payer node unreachable.
- No active route or insufficient outbound liquidity.
- Payment failed or timed out.

## `create_invoice_and_maybe_autosend`

**Purpose**: Support the debug channel-row flow where one side creates an invoice and the other side pays automatically if AutoSend is enabled.

**Input**:

- `creator_node`.
- `candidate_payer_node`.
- `amount_sats`.
- `memo`.
- `autosend_enabled`.

**Output**:

- Created invoice.
- Optional payment attempt.
- Updated route state.
- Explicit note when AutoSend was skipped.

**Errors**:

- Same as `create_invoice` and `pay_invoice`.
- AutoSend amount exceeds configured demo limit.

## `get_operation_faq`

**Purpose**: Return the simplified operation table used by the FAQ/concepts view.

**Input**: None.

**Output**:

- Operation rows with needs-Bitcoin-node and needs-mined-block flags.
