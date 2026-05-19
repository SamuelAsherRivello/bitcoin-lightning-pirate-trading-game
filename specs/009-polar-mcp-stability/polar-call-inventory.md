# Polar Call Inventory

Generated during T005 for the Polar MCP stability refactor.

## Current Boundary

- Primary orchestration file: `packages/ui/src/client/services/polar_bridge_service.rs`
- Dioxus-facing wrapper file: `packages/ui/src/client/services/lightning_server_functions.rs`
- Default local bridge: `http://localhost:37373`
- Current raw tool endpoint: `/api/mcp/execute`
- Current health endpoint: `/health`

## Current Raw Transport Functions

- `get_json(...)`: local bridge GET transport for health and reads.
- `post_json(...)`: local bridge POST transport for tool execution.
- `execute_tool(...)`: wraps `post_json(..., "/api/mcp/execute", ...)`.
- `execute_tool_with_log_level(...)`: same as `execute_tool`, with lower-noise polling support.
- `list_networks(...)`: wraps the `list_networks` Polar tool.
- `list_networks_with_log_level(...)`: lower-noise `list_networks` polling wrapper.

## High-Value Call Sites To Preserve

- `test_bridge(...)`: bridge health.
- `ensure_server(...)`: find/create/start selected Polar network.
- `resolve_automation_profile(...)`: refresh selected network and backend names.
- `read_network_node_names(...)`: dashboard and saved-profile node-name refresh.
- `create_required_nodes_with_progress(...)`: create and start demo topology.
- `create_game_treasury_node(...)`: treasury node readiness and funding shell.
- `fund_demo_user_nodes(...)`: user-node sats balancing.
- `ensure_taproot_assets_node(...)`: Taproot Assets node setup.
- `validate_lab_health(...)`: setup recovery and dashboard readiness checks.
- `mine_blocks(...)`: block-height setup and wait-for-block actions.
- `delete_polar_network(...)` and reset helpers: existing app-owned cleanup paths only.

## Current Polar Tool Names Seen In Code

- `list_networks`
- `create_network`
- `start_network`
- `start_server`
- `add_node`
- `remove_node`
- `start_node`
- `stop_network`
- `delete_network`
- `set_lightning_backend`
- `deposit_funds`
- `get_wallet_balance`
- `get_blockchain_info`
- `mine_blocks`
- `open_channel`
- `close_channel`
- `list_channels`
- `create_invoice`
- `pay_invoice`
- Taproot Assets operations used behind setup and TRA flows.

## Refactor Notes

- Keep setup labels and route locking unchanged.
- Centralize raw HTTP transport, MCP error extraction, timeout text, transient classification, and sensitive-detail redaction in `polar_mcp_connector.rs`.
- Keep higher-level topology, funding, channel, invoice, and asset orchestration in `polar_bridge_service.rs` until each flow can be safely split.
