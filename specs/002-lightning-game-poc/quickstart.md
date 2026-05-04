# Quickstart: Bitcoin Lightning Game POC

This quickstart describes the target workflow for validating the feature after implementation.

## 1. Prepare Polar

1. Install [Docker](https://www.docker.com/products/docker-desktop/).
2. Run Docker.
3. Install [Polar](https://lightningpolar.com/).
4. Run Polar.

## 2. Prepare The App

1. Install project dependencies:

   ```powershell
   .\Scripts\Common\InstallDependencies.ps1
   ```

2. Use the default `Polar Connection (Networked)` setup wizard, or switch to `Mock Connection (Offline)` for local demo state.
3. Run the web app:

   ```powershell
   .\Scripts\Common\RunWeb.ps1
   ```
   `RunWeb.ps1` restarts this repo's Dioxus web/static server and generated app server before serving. If you intentionally need a second Dioxus web instance, use a port that is not already used by Polar or the LND nodes:
   ```powershell
   .\Scripts\Common\RunWeb.ps1 -Address 127.0.0.1 -Port 8090
   ```

4. Open the served web app URL shown by the script.

## 3. Verify Home And Set Up

1. Confirm the navigation shows `Home`, `Set Up`, `Play Game`, and `Network Dashboard` from left to right.
2. Confirm `Home` and `Set Up` are enabled before setup is verified, while `Play Game` and `Network Dashboard` are visible but disabled.
3. Open `Home`.
4. Confirm `Home` has two main content sections: `Overview` and `FAQ`.
5. Read `Overview` and confirm `Why this demo exists` plus the regtest-only and lab-control warnings are visible.
6. Confirm `FAQ` includes:
   - A `What is Bitcoin?` summary with an external learn-more link.
   - A `What is Bitcoin Lightning?` summary with an external learn-more link.
   - A `Bitcoin vs Lightning` table with pros and cons for each.
   - A block-dependency explanation that mentions mainnet blocks arrive about every 10 minutes on average.
7. Read the operations table and confirm it includes:
   - Create invoice: no mined block required.
   - Pay invoice: no mined block required after active channel.
   - Fund wallet: mined block required for confirmed balance.
   - Open channel: mined block required.
   - Close channel: mined block required.
7. Open `Set Up`.
8. In `Setup`, enter or confirm:
   - `Sats per transaction`: `1,000`.
   - `Polar Connection (Networked)`: selected by default, with `OS Setup` showing install/run steps for Docker and Polar and `Polar Setup` showing compact form rows; the app saves the bridge URL, reuses or creates the named Polar server, and then discovers the backend for demo-node creation.
   - `Mock Connection (Offline)`: the fake-data callout is visible after switching tabs.
9. Click the enabled `SUBMIT` buttons in order: save Polar MCP bridge URL, ensure Polar server name, create 3 demo nodes, and complete setup. `RESET` returns to the previous step.
10. Confirm Alice, Bob, and Carol are created, started, funded, and gameplay unlocks only after `Complete Setup`.
11. Refresh the page.
12. Confirm `Play Game` and `Network Dashboard` are enabled.

## 4. Verify Play Game

1. Open `Play Game`.
2. Confirm Alice starts in Town.
3. Open a trade route from Alice to Bob.
4. Confirm the route appears as under construction.
5. Mine the next block in Polar, or click `Wait for Next Block`.
6. Confirm the app detects the new regtest block.
7. Confirm the Alice-Bob trade route becomes active.
8. Buy an item from Bob.
9. Confirm the game log shows:
   - Bob created an invoice.
   - Alice paid the invoice.
   - The transaction amount was `1,000 sats`.
   - No new Bitcoin block was required for the Lightning payment.

## 5. Verify Network Dashboard

1. Open `Network Dashboard`.
2. Confirm the Alice-Bob row shows:
   - Alice node block.
   - Bob node block.
   - A visible wallet/purse-to-wallet/purse line.
   - Channel status.
   - Local and remote balances.
   - `Create Invoice`, `Pay Invoice`, and AutoSend controls.
3. Create an invoice from Bob.
4. Pay it from Alice.
5. Confirm the row balances update.
6. Confirm recent invoices and recent payments show the operation.
## 6. Verify Desktop

1. Stop the web server if needed.
2. Run:

   ```powershell
   .\Scripts\Common\RunDesktop.ps1
   ```

3. Repeat the setup detection and at least one read-only lab state check.

## Expected Result

The app teaches the learner that:

- Alice, Bob, and Carol are demo nodes controlled by the lab app.
- A trade route is a Lightning channel.
- Building a trade route requires the next Bitcoin block.
- Trading over an active route uses Lightning and does not wait for a new Bitcoin block.
- A real game should request player wallet approval instead of controlling player funds directly.
