# Research: TRA Inventory Assets

**Date**: 2026-05-14

## Sources Reviewed

- Lightning Labs Builder's Guide, Taproot Assets: https://docs.lightning.engineering/lightning-network-tools/taproot-assets
- Lightning Labs Builder's Guide, Lightning Polar and Taproot Assets: https://docs.lightning.engineering/lightning-network-tools/taproot-assets/polar
- Lightning Labs Builder's Guide, First Steps: https://docs.lightning.engineering/lightning-network-tools/taproot-assets/first-steps
- Lightning Labs taproot-assets repository: https://github.com/lightninglabs/taproot-assets

## Decision: Use TRA As The App-Facing Term

**Rationale**: The feature needs learner-friendly language while still mapping to Taproot Assets. "TRA" is short enough for inventory UI and logs, while research/technical docs can continue to reference Taproot Assets.

**Alternatives considered**: Use "Taproot Assets" everywhere. Rejected because repeated full terminology is too heavy for inventory slots and game controls.

## Decision: Model Each Inventory Item As One Unique Collectible-Style TRA

**Rationale**: The game needs one distinct ownership record per mock item. Taproot Assets supports collectible-style assets, which matches a unique Book instance better than fungible balances.

**Alternatives considered**: Use fungible quantities per item type. Rejected because "3 inventory items max per node" and buy/sell ownership transfer are instance-based, not balance-based.

## Decision: Carry `item_id` In TRA Data And Keep Cost/Visuals In The Game Catalog

**Rationale**: `item_id` is stable type identity that can travel with the asset. Cost, visuals, and trade behavior are game rules and should remain hardcoded in the app so gameplay balance and rendering do not depend on mutable asset metadata. `item_id=1` resolves to Book.

**Alternatives considered**: Store price and visual metadata in each TRA. Rejected because it would duplicate game configuration, make price changes harder, and blur ownership data with gameplay rules.

## Decision: Place TRA Setup After Polar Lightning Node Connection And Funding

**Rationale**: Taproot Assets relies on a working local Bitcoin/LND environment. The app can only verify, mint, assign, and transfer assets after Polar has created and funded the local demo nodes.

**Alternatives considered**: Add TRA setup before Lightning setup. Rejected because the required backend readiness cannot be verified yet. Add it after Play Game opens. Rejected because trading would start with unverified inventory ownership.

## Decision: Keep `TraService` As The KISS Domain Boundary

**Rationale**: The repo already uses `packages/lightning-service` for local lab domain operations and `packages/ui/src/client/services/lightning_server_functions.rs` for UI-facing async wrappers. A `TraService` class/struct keeps mint/transfer/catalog/verification rules DRY and testable, while a future server adapter can handle real `tapd` or Litd calls.

**Alternatives considered**: Put TRA rules directly into Dioxus pages. Rejected because it would duplicate capacity, catalog, and transfer validation. Put everything in the Polar bridge service. Rejected because Polar automation and TRA domain ownership rules are separate concerns.

## Decision: Hide Real Taproot Assets Calls Behind A Server-Only Adapter

**Rationale**: The constitution requires direct wallet/node access to remain behind service boundaries and forbids sensitive credentials in browser storage. A server-only `tra_client` adapter can call Polar/Litd/`tapd` while `TraService` remains the stable domain API.

**Alternatives considered**: Browser-side Taproot Assets calls. Rejected because it would expose credentials/proofs and add browser infrastructure outside the project constraints.

## Decision: Treat Payment/TRA Divergence As Recoverable Partial Completion

**Rationale**: A Lightning payment can succeed while TRA proof delivery or verification fails. The learner should see a clear pending/failed ownership state and the app should prevent duplicate transfers for the same item until recovery.

**Alternatives considered**: Roll back payment automatically. Rejected because Lightning payments may be final and the local app cannot guarantee reversal.

## Decision: Play Game Inventory Must Use Concrete TRA Instances

**Rationale**: The learning goal is ownership transfer. Inventory images, buy eligibility, sell eligibility, and transfer calls must therefore be derived from verified `TraItem` records in `LabState.tra_items`. Payment history can explain previous activity, but it is not an ownership source and must not create item images or transfer targets.

**Alternatives considered**: Continue deriving book counts from successful payment history. Rejected because payment-derived counts can diverge from Taproot Assets ownership and would allow the UI to show items that are not backed by a real TRA owner.
