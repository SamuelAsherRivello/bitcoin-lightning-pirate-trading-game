# Verification Notes

## 2026-05-19

Implementation slice completed:

- T001-T005: Polar MCP setup documentation, helper script, baseline instructions, and call inventory.
- T006-T013: Connector DTOs/errors, `polar_mcp_connector` boundary, bridge-service delegation, and focused tests.
- T026-T027, T040, T043: Required-tool validation plus local-only connector URL enforcement.
- Served web runtime check: existing `dx` dev server on `http://localhost:8080` / `http://127.0.0.1:8080`, Polar app running, and Polar MCP bridge healthy on `http://localhost:37373`.

Commands run:

```powershell
cargo fmt --check
cargo test -p lightning-service polar_connector
cargo test -p ui --test polar_mcp_connector_tests
cargo check -p ui --target wasm32-unknown-unknown
cargo test -p lightning-service
cargo test -p ui polar
cargo test -p ui setup
cargo check -p web --target wasm32-unknown-unknown
cargo check -p desktop
```

Results:

- `cargo fmt --check`: passed.
- `cargo test -p lightning-service polar_connector`: passed, 2 filtered connector model tests.
- `cargo test -p ui --test polar_mcp_connector_tests`: passed, 10 connector tests.
- `cargo check -p ui --target wasm32-unknown-unknown`: passed.
- `cargo test -p lightning-service`: passed, 33 unit tests plus 3 integration tests.
- `cargo test -p ui polar`: passed, 83 matching unit tests plus filtered integration crates.
- `cargo test -p ui setup`: passed, 35 matching unit tests plus matching integration tests.
- `cargo check -p web --target wasm32-unknown-unknown`: passed.
- `cargo check -p desktop`: passed.
- Polar MCP bridge health: `Invoke-RestMethod http://localhost:37373/health` returned `{"status":"ok","service":"polar-mcp-bridge"}`.
- Polar MCP tool execution: direct `list_networks` POST to `/api/mcp/execute` succeeded and returned local Polar networks.
- Served web setup check: `http://localhost:8080/setup` rendered the Polar setup tab, submitted the Bridge URL step successfully, and reached `Saved but offline` in about 5 seconds.
- Served web origin guard: `http://127.0.0.1:8080/setup` rendered the localhost instruction and disabled setup submit buttons because the current Polar bridge CORS behavior rejects browser fetches from the IPv4 origin.

Not yet run:

- a fresh `Scripts/Common/RunWeb.ps1` restart; the already-running `dx` server was reused to avoid disrupting the active app session.

Baseline timing has not been captured yet because it requires a running Polar app and existing local demo network.
