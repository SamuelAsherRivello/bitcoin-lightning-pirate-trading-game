# Quickstart: TRA Inventory Assets

## Prerequisites

- Windows 11 development environment.
- Project dependencies installed with `.\Scripts\Common\InstallDependencies.ps1`.
- Polar available locally.
- A Polar regtest lab with Alice, Bob, and Carol created through the app setup flow.
- For real TRA integration tasks, use a Polar/Litd or `tapd` setup that exposes Taproot Assets capability on the same regtest network as the demo LND nodes.
- Current implementation includes a server-only fake Taproot Assets adapter boundary for local tests. Real Polar/Litd or `tapd` wiring must provide the adapter endpoint/profile values server-side only.

## Verify Current Service Boundary

```powershell
cargo test -p lightning-service
cargo check -p ui --target wasm32-unknown-unknown
cargo check -p desktop
```

Expected:

- `TraService` tests pass.
- `ui` compiles for wasm.
- `desktop` compiles.

## Manual Setup Flow

1. Run the web app:

   ```powershell
   .\Scripts\Common\RunWeb.ps1
   ```

2. Open the served app URL.
3. Go to `Set Up`.
4. Complete the existing Polar server and demo node setup steps.
5. Confirm the app observes Polar block height and marks Lightning setup ready.
6. Run the `Add Tap Root Assets` setup step.

Expected:

- The TRA step shows loading/status feedback while verifying capability and preparing items.
- Setup does not continue if TRA capability cannot be verified.
- One NPC receives 2 unique Book TRA items backed by `item_id=1`.
- The other NPC receives 2 unique Apple TRA items backed by `item_id=2`.
- No node receives more than 3 inventory items.
- If you reset back to `Add Tap Root Assets` and submit again, previous setup inventory is cleared, prior setup item identities are no longer valid game inventory, and the initial TRA items are recreated from scratch.

## Manual Play Game Flow

1. Go to `Play Game`.
2. Open a trade with an NPC and wait for block confirmation if needed.
3. Confirm NPC inventory shows unique item names and item type status.
4. Confirm each occupied inventory image corresponds to a concrete TRA item with a unique name and verified owner.
5. Buy a Book from the NPC.
6. Confirm Lightning payment completes.
7. Confirm TRA ownership moves from NPC to player for the selected item, not for a payment-derived book count.
8. Sell the same Book or another selected player-owned TRA item back to an NPC with capacity.

Expected:

- Book uses catalog cost/visuals from `item_id=1`; Apple uses catalog cost/visuals from `item_id=2`.
- Inventory slots show only the item icon and catalog display name.
- Buy Item and Sell Item operate on selected concrete TRA instances and their `tra_id`.
- Inventory images are derived from `LabState.tra_items`; payment history alone does not create item images.
- Ownership is not finalized until TRA verification succeeds.
- If verification fails, the item is shown as pending or failed and cannot be duplicated.
- During served-web verification, record that TRA status feedback appears within 1 second of starting/failing actions and that the initial 3-item TRA setup completes within 2 minutes after Lightning nodes are running and funded.

## Recovery Flow

1. Complete TRA setup.
2. Restart the app.
3. Return to `Set Up` or `Play Game`.

Expected:

- Saved non-sensitive TRA item identity is loaded.
- Known items are verified against the local lab before buy/sell controls are enabled.
- Missing or unsupported items are flagged for recovery.

## Stop The Web App

```powershell
.\Scripts\Other\StopWeb.ps1
```
