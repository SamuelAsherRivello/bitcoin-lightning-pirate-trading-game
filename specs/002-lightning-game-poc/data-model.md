# Data Model: Bitcoin Lightning Game POC

## SetupProfile

Represents learner-facing setup state.

**Fields**:

- `sats_per_transaction`: Whole number, default `1,000`, valid range `1..=100,000`.
- `network_name`: Expected Polar network name, default `Dioxus Bitcoin Lightning Game`.
- `setup_mode`: `ServerConfig` for the visible `Polar Connection (Networked)` tab or `BrowserRegtestOnly` for the visible `Mock Connection (Offline)` tab.
- `polar_automation`: UI-entered Polar bridge URL plus app-discovered Polar network and Bitcoin backend values.
- `polar_block_height_confirmed`: Whether the learner has accepted or edited the Polar Setup `Block Height` row before final unlock.
- `last_verified_at`: Optional timestamp of the last successful setup test.
- `connection_status`: `NotConfigured`, `SavedOffline`, `Connected`, `PartiallyConnected`, `Invalid`.

**Validation**:

- Transaction amount must be a positive whole number within the demo range.
- Setup is complete only when the app has created Alice, Bob, and Carol or mock mode is connected.
- Polar Connection setup requires the learner to confirm a whole-number Block Height before final unlock; `0` is valid.
- Mock Connection mode must show fake offline-data messaging and must not require credential acknowledgement.
- Polar Connection mode must include a local bridge URL. The app ensures the named Polar server exists through the bridge, discovers the Bitcoin backend when possible, and falls back to the `backend1` backend convention.

## PolarAutomationProfile

Represents the non-wallet-secret values needed to ask Polar to create the demo Lightning nodes. The setup form only requires the bridge URL; network/backend values are retained so successful discovery can be saved locally.

- `bridge_url`: Local Polar MCP bridge URL, default `http://localhost:37373`.
- `network_id`: Discovered Polar network name or id that already contains the Bitcoin backend node.
- `bitcoin_backend_name`: Discovered Polar node name for the Bitcoin Core backend, default fallback `backend1`.

### Validation Rules

- Bridge URL must be local `localhost` or `127.0.0.1`.
- Network id/name and Bitcoin backend node name may be blank before bridge discovery.
- The bridge must be reachable before the app creates or destroys demo nodes.

## ServerNodeProfile

Represents credential-bearing connection data for one LND node. This is retained as a future server-side adapter shape, but the current Polar automation flow does not require the learner to paste these values.

**Fields**:

- `node_id`: `Alice`, `Bob`, or `Carol`.
- `display_name`: Human-readable node name.
- `lnd_endpoint`: gRPC endpoint for the Polar LND node.
- `tls_cert_path_or_pem`: TLS certificate reference or content.
- `macaroon_path_or_hex`: Macaroon reference or content.
- `network`: Must be `regtest` for v1.

If a future direct-LND mode is added, these values should stay server-side by default instead of becoming the primary browser setup flow.

**Validation**:

- Node id must be one of the three supported demo nodes.
- Network must be regtest.
- Endpoint must be local or explicitly marked as lab-only.

## DemoNode

Represents a node/persona shown in the UI.

**Fields**:

- `node_id`: `Alice`, `Bob`, or `Carol`.
- `role`: Player, BeachMerchant, or MountainMerchant.
- `location`: Town, Beach, Mountain, or Desert.
- `alias`: LND node alias if available.
- `pubkey`: LND identity pubkey if available.
- `wallet_balance_sats`: Confirmed on-chain balance.
- `channel_balance_sats`: Current local Lightning balance.
- `status`: Offline, Online, Locked, Error.

## TradeRoute

Game-facing representation of a Lightning channel.

**Fields**:

- `route_id`: Stable app id for a node pair.
- `from_node`: Node id.
- `to_node`: Node id.
- `game_label`: Example `Alice to Bob trade route`.
- `lnd_channel_point`: Optional channel point when known.
- `capacity_sats`: Channel capacity.
- `local_balance_sats`: Local balance from the perspective of the left node.
- `remote_balance_sats`: Remote balance from the perspective of the left node.
- `status`: Missing, UnderConstruction, Active, Closing, Closed, Error.
- `requires_next_block`: Boolean.

**State transitions**:

- Missing -> UnderConstruction when opening starts.
- UnderConstruction -> Active after enough confirmations.
- Active -> Closing when cooperative or force close starts.
- Closing -> Closed after close transaction confirms.

## InvoiceRequest

Represents a request to receive payment.

**Fields**:

- `invoice_id`: App id.
- `creator_node`: Node creating the invoice.
- `expected_payer_node`: Optional intended payer.
- `amount_sats`: Usually setup `sats_per_transaction`.
- `memo`: Game action description.
- `payment_request`: BOLT11 invoice string.
- `status`: Created, Settled, Expired, Canceled, Error.
- `created_at`: Timestamp.
- `settled_at`: Optional timestamp.

**Validation**:

- Amount must match setup amount unless an explicit debug override exists.
- Receive action creates an invoice only; settlement requires a payment.

## PaymentAttempt

Represents a node trying to pay an invoice.

**Fields**:

- `payment_id`: App id.
- `payer_node`: Node paying.
- `payee_node`: Expected receiving node if known.
- `invoice_id`: Related invoice.
- `amount_sats`: Payment amount.
- `route_summary`: Optional route/path data.
- `status`: Pending, Succeeded, Failed.
- `failure_reason`: Optional user-readable reason.
- `requires_block`: Usually false for active Lightning channels.

## BlockWaitAction

Represents the game action that advances regtest.

**Fields**:

- `action_id`: App id.
- `reason`: ChannelOpenConfirmation, ChannelCloseConfirmation, WalletFundingConfirmation.
- `affected_route_id`: Optional route affected.
- `blocks_requested`: Default `1`.
- `status`: Pending, Mined, Failed.
- `resulting_height`: Optional block height after mining.

## OperationFaqRow

Represents one row in the teaching table.

**Fields**:

- `operation`: Display name.
- `needs_bitcoin_node`: Boolean.
- `needs_mined_block`: Boolean.
- `plain_explanation`: Short text.
- `game_example`: Optional game mapping.

**Initial rows**:

- Create invoice: needs Bitcoin node/backend in normal LND runtime, no new mined block.
- Pay invoice: needs active LND/channel state, no new mined block once channel is active.
- Fund wallet: needs Bitcoin backend, needs mined block for confirmed balance.
- Open channel: needs Bitcoin backend, needs mined block for channel activation.
- Close channel: needs Bitcoin backend, needs mined block for finality.
- Check payment status: needs LND, no new mined block.
- Wait for next block: needs Bitcoin backend/mining control, creates a new mined block in regtest.
