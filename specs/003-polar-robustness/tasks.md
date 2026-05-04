# Tasks: Polar Lab Robustness

## Implementation

- [X] T001 Create the `003-polar-robustness` Spec Kit feature branch and feature artifacts.
- [X] T002 Add Polar lab health classification for bridge, network, backend, and demo-node failures.
- [X] T003 Add setup recovery state mapping and persistence for each health issue.
- [X] T004 Add route-entry and action-triggered recovery handling for `Play Game` and `Network Dashboard`.
- [X] T005 Add native/server observer event types for future LND stream integration.
- [X] T006 Update `Documentation/DioxusFeatureMatrix.md` for health recovery and observer behavior.
- [X] T007 Add unit tests for health classification and recovery mapping.
- [X] T008 Run formatting and targeted web/desktop/test checks.
- [X] T009 Add refresh-time Polar validation from the app layout while preserving the current route when health passes.
- [X] T010 Split messaging so happy-path actions use toasts and broken/reset flows use the blocking prompt with cancel-aware reset restoration.
- [X] T011 Add an "Are you sure?" confirmation prompt before Polar setup step 4 reset locks a running app.
- [X] T012 Use the blocking prompt with cancel rollback for long-running Polar setup step 3 submit.
- [X] T013 Poll Polar after step 3 submit and only confirm success after server, demo-node, and wallet-balance readiness checks pass.
- [X] T014 Restrict Polar setup step 2 to name-only server matching, with create-or-start behavior for the requested name.
- [X] T015 Poll connected Polar state from every primary route and activate pending trade routes when Polar mines blocks outside the app.
