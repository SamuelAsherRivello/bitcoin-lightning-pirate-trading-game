# Tasks: Bitcoin Lightning Game POC

Generated during implementation because this feature folder did not include a pre-existing `tasks.md`.

## Implementation

- [X] T001 Update workspace metadata and ignore patterns for the new Lightning lab implementation.
- [X] T002 Add `packages/lightning-service` with setup validation, lab models, route/invoice/payment/block operations, server-side profile validation, and an optional `tonic_lnd` adapter boundary.
- [X] T003 Add operation wrappers matching the planned server-function contracts for setup testing, lab state, route opening, block waiting, invoice creation, payment, AutoSend, and FAQ rows.
- [X] T004 Persist non-sensitive setup preferences and demo lab state through `storage_service` for web localStorage and desktop `data/` JSON files.
- [X] T005 Replace placeholder routes with `Home`, `Set Up`, `Play Game`, and `Debug Network`.
- [X] T006 Implement the `Set Up` page with Polar setup guidance, sats validation, setup modes, connection testing, save/reset actions, and node status feedback.
- [X] T007 Implement the `Play Game` page with Alice/Bob/Carol locations, trade route opening, next-block confirmation, purchases, and action log feedback.
- [X] T008 Implement the `Debug Network` page with channel visuals, balances, AutoSend, and invoice/payment histories.
- [X] T008a Move why-this-demo-exists and FAQ/concepts content into `Home` with Bitcoin and Lightning summaries, a Bitcoin vs Lightning pros/cons table, and the LND operation vs block table.
- [X] T009 Update shared CSS for the lab UI across web and desktop assets.
- [X] T010 Update localization keys, tests, `AGENTS.md`, the constitution route constraint, and `Documentation/DioxusFeatureMatrix.md`.
- [X] T011 Run Rust formatting and compile checks for ui/web/desktop targets.
