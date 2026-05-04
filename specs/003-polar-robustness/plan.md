# Implementation Plan: Polar Lab Robustness

**Branch**: `003-polar-robustness` | **Date**: 2026-05-03 | **Spec**: [spec.md](spec.md)

## Summary

Add health classification and recovery for local Polar lab drift caused outside the app. The implementation validates the request/response Polar bridge after app refresh on any primary route, on gameplay/debug route entry, after failed lab actions, and through lightweight polling while connected. Healthy refreshes keep the current route active and keep the saved setup connected. External Polar block-height increases update the shared lab snapshot so pending trade routes become active without pressing the app's `Wait for Next Block` button. Broken lab state locks stale connected state, shows a blocking prompt, routes to `Home`, and resumes `Set Up` at the repair step. It also adds native/server observer event types for future LND streaming support.

## Technical Context

**Language/Version**: Rust workspace with Dioxus 0.7 UI.  
**Primary Dependencies**: Existing Dioxus/router setup, existing Polar HTTP bridge wrappers, existing `lightning-service` crate.  
**Storage**: Existing `storage_service` for non-sensitive setup profile and demo lab snapshot.  
**Target Platform**: Browser and desktop remain supported. Browser uses Polar bridge checks only; native/server can later map LND streams into observer events.  
**Testing**: Targeted Rust unit tests plus web/desktop compile checks and `Scripts/Other/RunTests.ps1`.

## Design

- `polar_bridge_service` owns pure health classification from Polar `list_networks` JSON and async health validation through `/health` and bridge tool calls.
- `lightning_server_functions` owns recovery mapping from classified health issues to persisted setup profile, cleared lab snapshot, and prompt text.
- `AppLayout` validates and refreshes a saved connected Polar setup after refresh on every primary route and then polls while connected; healthy setup does not navigate or show extra messaging.
- `Play Game` and `Network Dashboard` perform route-entry validation and post-error reclassification, then use router navigation to replace the current route with `Home`.
- Happy-path setup/game/debug actions continue to use subtle toasts. Broken-lab recovery and reset operations use a blocking queued prompt with a minimum 0.25 second display per prompt message; reset cancel requests attempt to restore the previous setup state after the Polar call completes.
- `lightning-service/src/server/lnd_client.rs` defines observer event/source types for future LND streams without starting long-running observers in this feature, and keeps those types free of Dioxus UI, route, toast, or navigation concerns so the service crate remains reusable in future Rust projects.

## Constraints

- Do not store LND credentials in browser storage.
- Do not add browser SQLite, OPFS, or background worker startup.
- Keep current routes and web/desktop targets intact.
- Keep Polar polling lightweight and app-level so every primary route observes external Polar changes without per-page duplicated timers.
