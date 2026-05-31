# Research: Game Treasury

## Decision: Game Treasury is a first-class lab node

**Rationale**: The user described the treasury as the house or bank for game activity. Making it an explicit Lightning node named GAME_TREASURY gives setup and gameplay a concrete source for sats and item distribution.

**Alternatives considered**:

- Hidden accounting balance only: rejected because it would not teach value movement or explain where NPC items originate.
- Reuse Jack as treasury: rejected because it mixes player ownership with game-bank ownership.

## Decision: Setup sequence becomes seven ordered steps

**Rationale**: The treasury must exist after bridge/server connection but before Jack, Bob, and Carol are created. NPC item transfers depend on both treasury inventory and user nodes, so they occur after User Nodes and before Block Height.

**Alternatives considered**:

- Append treasury after Unlock Routes: rejected because gameplay would unlock before the economy is seeded.
- Merge treasury into user node creation: rejected because it hides the house/bank concept.

## Decision: Treasury owns only NPC-bound setup items initially

**Rationale**: The user specified that the treasury receives only the items that will soon be given to NPCs. This keeps v1 scope focused on startup economy seeding rather than a broad item vault.

**Alternatives considered**:

- Treasury owns every catalog item: rejected as broader than requested.
- NPC items continue to appear directly on NPCs: rejected because it bypasses the treasury transfer model.

## Decision: Sats funding is scenario-based

**Rationale**: The spec says the treasury should have enough sats to support the configured game activity. The implementation should calculate or reuse the game's configured activity requirements rather than hardcode an unrelated amount in the UI.

**Alternatives considered**:

- Fixed arbitrary sats amount: rejected because item prices and gameplay requirements can drift.
- User-entered treasury balance: rejected for v1 because it adds setup complexity and ambiguity.

## Decision: Browser persistence remains non-sensitive snapshots

**Rationale**: The constitution requires localStorage snapshots for browser demo state and forbids browser-stored sensitive Lightning credentials. Treasury summaries, item identities, status, and history are sufficient for UI continuity.

**Alternatives considered**:

- Store node credentials in browser localStorage: rejected for credential safety.
- Introduce browser SQLite/OPFS: rejected by project rules.

## Decision: Treasury entries use player-facing explanations

**Rationale**: The feature supports learning. Users need to see that sats/items moved because of setup, rewards, costs, or trades without reading wallet transport details.

**Alternatives considered**:

- Technical node logs: rejected because they are not user-centered and may expose unnecessary internals.
- Balance-only summary: rejected because it hides cause and effect.
