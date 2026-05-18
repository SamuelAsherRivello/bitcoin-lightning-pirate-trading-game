# Quickstart: Game Treasury

## Prerequisites

- Local Polar setup is available for the existing Lightning lab.
- Project dependencies are installed with `.\Scripts\Common\InstallDependencies.ps1` if needed.
- The feature is implemented according to `specs/006-game-treasury/tasks.md`.

## Manual Setup Flow Smoke

1. Start the web app with `.\Scripts\Common\RunWeb.ps1`.
2. Open the served app in a browser.
3. Navigate to `Set Up`.
4. Confirm setup shows these ordered steps: Bridge URL, Server Name, Game Treasury, User Nodes, NPC Item Transfers, Block Height, Unlock Routes.
5. Complete Bridge URL and Server Name.
6. Run Game Treasury setup.
7. Confirm the UI shows visible progress for node creation/verification, funding, and item preparation.
8. Confirm the GAME_TREASURY node is shown as ready before User Nodes can complete.
9. Complete User Nodes and confirm Alice, Bob, and Carol are created or verified.
10. Run NPC Item Transfers.
11. Confirm Bob and Carol receive their configured starting items from Game Treasury.
12. Continue through Block Height and Unlock Routes.

## Gameplay Smoke

1. Navigate to `Play Game` after setup completes.
2. Confirm Game Treasury summary is visible or reachable from the gameplay experience.
3. Confirm the treasury summary distinguishes spendable sats, inventory-backed value, status, and recent history.
4. View a treasury-dependent action.
5. Confirm an impact preview explains whether the action will increase, decrease, or not affect treasury resources.
6. Attempt an action when resources are insufficient or stale.
7. Confirm the action is blocked or prompts refresh with a visible reason.
8. Complete a treasury-impacting action.
9. Confirm the treasury updates or shows a pending-refresh state within 2 seconds.

## Suggested Checks

- Run `cargo test -p lightning-service` for treasury domain behavior.
- Run `cargo check -p ui --target wasm32-unknown-unknown` for browser UI compatibility.
- Run `cargo check -p desktop` for desktop compatibility.
- Run `.\Scripts\Other\RunTests.ps1` before final delivery when practical.
- Use served-web smoke for browser-visible setup and gameplay behavior.

## Expected User-Visible Results

- Game Treasury is a distinct setup step between Server Name and User Nodes.
- Game Treasury is described as the house or bank for game activity.
- Bob and Carol's starting items originate from Game Treasury transfers.
- Loading/failure states are never displayed as ready zero balances.
- Recent treasury history uses plain-language entries, not technical wallet logs.

