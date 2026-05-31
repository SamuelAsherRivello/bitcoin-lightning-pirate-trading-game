# Contracts: Treasury Server Functions

These contracts describe UI-facing operations for setup and gameplay. They are technology-facing enough for planning, but keep credentials and direct node access behind the service boundary.

## create_or_verify_game_treasury

**Purpose**: Create or verify the dedicated GAME_TREASURY node.

**Request**:

- `bridge_url`
- `server_name`
- `existing_setup_snapshot` optional

**Response**:

- `treasury_status`
- `node_label`
- `message`
- `recoverable_error` optional

**Errors**:

- Bridge unreachable
- Server profile missing or invalid
- Treasury node creation failed
- Treasury node exists but is not usable

## fund_game_treasury

**Purpose**: Fund the GAME_TREASURY node with enough sats for configured game activity.

**Request**:

- `node_label`
- `game_scenario_id` optional
- `required_activity_profile`

**Response**:

- `treasury_status`
- `spendable_sats`
- `funding_entries`
- `message`

**Errors**:

- Treasury node not ready
- Funding source unavailable
- Funding amount insufficient
- Confirmation or refresh failed

## prepare_treasury_items

**Purpose**: Create or verify treasury-owned items that will later be distributed to NPCs.

**Request**:

- `node_label`
- `npc_starting_item_plan`

**Response**:

- `treasury_status`
- `owned_items`
- `created_or_verified_entries`
- `message`

**Errors**:

- Treasury node not ready
- Item creation failed
- Item ownership verification failed
- Partial item preparation requiring retry

## create_or_verify_user_nodes

**Purpose**: Create or verify Jack, Bob, and Carol after Game Treasury is ready.

**Request**:

- `required_users`: Jack, Bob, Carol
- `treasury_status`

**Response**:

- `user_nodes`
- `message`
- `recoverable_error` optional

**Errors**:

- Treasury not ready
- User node creation failed
- Existing node not usable

## transfer_npc_starting_items

**Purpose**: Transfer Bob and Carol's configured starting items from Game Treasury to those NPCs.

**Request**:

- `source`: Game Treasury
- `transfers`: list of item and destination pairs

**Response**:

- `transfer_results`
- `treasury_entries`
- `refreshed_ownership_summary`
- `message`

**Errors**:

- Treasury item missing
- NPC node missing
- Transfer failed
- Ownership verification failed
- Partial transfer requiring retry

## get_game_treasury_summary

**Purpose**: Return player-visible treasury summary, status, and recent history.

**Request**:

- `include_history_count`
- `force_refresh` optional

**Response**:

- `game_treasury`
- `recent_entries`
- `last_updated_at`
- `message`

**Errors**:

- Treasury not initialized
- Refresh failed
- Snapshot stale or inconsistent

## preview_treasury_impact

**Purpose**: Explain whether a proposed gameplay action can be supported by treasury resources.

**Request**:

- `action_type`
- `actor`
- `target` optional
- `item_id` optional
- `amount_sats` optional

**Response**:

- `impact_preview`
- `message`

**Errors**:

- Treasury state stale
- Required item missing
- Insufficient spendable sats
- Unsupported action type

## record_treasury_event

**Purpose**: Record a setup or gameplay treasury-impacting event in player-facing history.

**Request**:

- `related_action`
- `description`
- `direction`
- `amount_sats` optional
- `item` optional
- `source` optional
- `destination` optional

**Response**:

- `treasury_entry`
- `updated_recent_entries`

**Errors**:

- Invalid event with no amount, item, or no-change reason
- Sensitive detail rejected from description
- Snapshot write failed
