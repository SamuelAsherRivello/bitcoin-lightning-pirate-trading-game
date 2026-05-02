# Learnings Of Polar Setup

## 2026-05-18 Autopilot Debug Mode

- Current Polar setup execution order is `Bridge URL`, `Server Name`, `Game Treasury (Sats)`, `Game Treasury (TRAs)`, `User Nodes`, `NPC Item Transfers`, `Block Height`, and `Unlock Routes`.
- A reliable autopilot should orchestrate the setup service functions directly instead of clicking DOM buttons. Button-click automation would depend on re-render timing and duplicated closure logic.
- Existing Dioxus-facing service calls already provide the right setup boundary: `verify_polar_bridge`, `ensure_polar_server`, `prepare_game_treasury`, `prepare_game_treasury_tras`, `create_polar_demo_nodes_with_progress`, `transfer_npc_starting_items`, `confirm_polar_block_height`, `complete_polar_setup`, then `get_lab_state` for a second verification read.
- Every autopilot run should use a generated server name such as `autopilot-<timestamp-ms>`, so timing checks do not accidentally reuse a previously working or partially broken server.
- Autopilot cannot require the user to type a name first.
- The 60 second goal is realistic for a bridge/server startup target, but full setup can still be gated by Polar readiness waits. Current service-level timeouts include user-node readiness and treasury readiness windows that can exceed 60 seconds on a cold or slow machine.
- Keep the autopilot flag local to the setup page for now. Persisting it in `SetupProfile` would make a debug mode sticky across normal app sessions.
- Autopilot must not require the user to press `Continue`, confirm dialogs, or click through prompts. It should use direct service calls, status-only operation prompts, and programmatic prompt closeout.

## Iteration History

- Initial implementation adds a debug panel before step 1 with `Autopilot: on/off`, a `Run` control, status text, and a sequential setup runner that updates the same setup profile, lab state, operation prompt, and toast surfaces as manual setup.
- The runner uses non-cancel status prompts only. Manual confirmation surfaces remain for manual reset flows, but autopilot bypasses those flows and drives the setup sequence directly.
- First real bridge-backed Playwright run stopped at Server Name because the field was empty. The runner now generates a fresh server name for empty input instead of requiring user feedback.
- Second real bridge-backed run showed that step 1 clears `polar_automation.network_id`, so autopilot must restore the requested server name into `network_id` immediately before `ensure_polar_server`.
- Third real bridge-backed run reused the default saved server and stopped on a Docker/Polar port conflict from an orphan `polar-n12-jack` container. The runner now always uses a fresh generated server name, not the visible default input.
- Fourth real bridge-backed run used a fresh server but still hit a Polar/Docker port allocation error. Autopilot now treats port allocation, socket reuse, orphan-container, and "could not start network" messages as retryable step 2 failures and tries another fresh server name up to five times.
- Fifth real bridge-backed run showed the same Polar/Docker start collision can surface during Game Treasury setup, after step 2 appears successful. Autopilot now retries step 3 by generating a fresh server, re-running server prep, and trying treasury setup again.
- Sixth real bridge-backed run showed that the replacement server prep inside the step 3 retry branch can also hit the same retryable port collision. That nested `ensure_polar_server` call must catch retryable errors and continue the retry loop.
- Seventh real bridge-backed run still exhausted fresh server attempts because failed Polar networks remained present. Autopilot cleanup is now scoped to generated `autopilot-...` servers between retries and on final retryable failure.
- Current local Polar/Docker environment is exhausted: read-only inspection showed many `polar-network-*` Docker networks and running/error `polar-n*` containers, and Polar returned `all predefined address pools have been fully subnetted`. Code can now retry and report this cleanly, but predictable new-server setup needs the local Polar/Docker network pool cleaned outside the app safety boundary.
- User reported a hang at `Finding or creating the Polar server...`. Root cause: autopilot awaited `ensure_polar_server` without its own timeout, so a bridge request that never resolved left the prompt pending forever. Autopilot server prep now uses a 20 second timeout and returns an explicit stop message instead of waiting indefinitely.
- Delete-all-networks is intentionally user initiated only: the setup page shows a danger button, then a separate `Are you sure?` confirmation prompt. The Polar bridge deletion service is not called by autopilot or by opening the prompt; it runs only from the confirmation button.
