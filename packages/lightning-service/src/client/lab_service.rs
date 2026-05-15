use chrono::Utc;

use super::error::LightningError;
use super::models::{
    ActionLogEntry, BlockWaitAction, BlockWaitReason, BlockWaitStatus, ConnectionStatus, DemoNode,
    DemoNodeId, InvoiceRequest, InvoiceStatus, LabState, NodeStatus, OperationFaqRow,
    PaymentAttempt, PaymentStatus, RouteStatus, SetupMode, SetupProfile, TradeRoute,
    DEFAULT_ROUTE_CAPACITY_SATS, MAX_SATS_PER_TRANSACTION,
};
use crate::server::config::validate_polar_connection_profile;

pub fn validate_setup_profile(profile: &SetupProfile) -> Result<(), LightningError> {
    if profile.sats_per_transaction == 0 || profile.sats_per_transaction > MAX_SATS_PER_TRANSACTION
    {
        return Err(LightningError::InvalidDemoAmount);
    }

    if profile.setup_mode == SetupMode::ServerConfig {
        if profile.polar_automation.is_complete() {
            if !profile.polar_automation.is_local_bridge() {
                return Err(LightningError::NonLocalPolarBridge);
            }
        } else if profile.polar_connection.is_complete() {
            validate_polar_connection_profile(
                profile.network_name.clone(),
                &profile.polar_connection,
            )?;
        } else {
            return Err(LightningError::MissingPolarAutomationValues);
        }
    }

    Ok(())
}

pub fn test_setup(mut profile: SetupProfile) -> Result<LabState, LightningError> {
    validate_setup_profile(&profile)?;

    let mut warnings = Vec::new();
    match profile.setup_mode {
        SetupMode::BrowserRegtestOnly => {
            profile.connection_status = ConnectionStatus::Connected;
            profile.last_verified_at = Some(Utc::now());
            warnings.push(
                "Browser regtest-only mode uses local demo state and must never use real wallet credentials."
                    .to_string(),
            );
        }
        SetupMode::ServerConfig => {
            profile.connection_status = ConnectionStatus::Connected;
            profile.last_verified_at = Some(Utc::now());
            warnings.push(
                "Polar connection fields were validated as a local regtest lab profile."
                    .to_string(),
            );
        }
    }

    let mut state = default_lab_state(profile);
    state.warnings.extend(warnings);
    if state.profile.is_connected() {
        push_log(
            &mut state,
            "Setup verified",
            "Alice, Bob, and Carol are available for the local regtest learning lab.",
            &[],
        );
    }

    Ok(state)
}

pub fn default_lab_state(profile: SetupProfile) -> LabState {
    let connected = profile.is_connected();

    LabState {
        profile,
        nodes: default_nodes(connected),
        trade_routes: default_routes(),
        recent_invoices: Vec::new(),
        recent_payments: Vec::new(),
        block_actions: Vec::new(),
        tra_items: Vec::new(),
        operation_faq: get_operation_faq(),
        block_height: 0,
        polar_observed_block_height: None,
        warnings: Vec::new(),
        action_log: Vec::new(),
    }
}

pub fn open_trade_route(
    mut state: LabState,
    from_node: DemoNodeId,
    to_node: DemoNodeId,
    capacity_sats: u64,
) -> Result<LabState, LightningError> {
    ensure_connected(&state)?;

    let next_height = state.block_height + 1;
    let route = route_mut(&mut state, from_node, to_node)?;
    if matches!(
        route.status,
        RouteStatus::Active | RouteStatus::UnderConstruction | RouteStatus::Closing
    ) {
        return Err(LightningError::RouteAlreadyExists);
    }

    route.status = RouteStatus::UnderConstruction;
    route.capacity_sats = capacity_sats;
    route.local_balance_sats = capacity_sats;
    route.remote_balance_sats = 0;
    route.requires_next_block = true;
    route.lnd_channel_point = Some(format!(
        "regtest-{}-{}-{}",
        from_node.label().to_ascii_lowercase(),
        to_node.label().to_ascii_lowercase(),
        next_height
    ));

    push_log(
        &mut state,
        &format!("Opened {} trade", to_node.label()),
        "Channel open started. The trade is under construction until the next block.",
        &["Channel Open Request"],
    );

    Ok(state)
}

pub fn close_trade_route(
    mut state: LabState,
    from_node: DemoNodeId,
    to_node: DemoNodeId,
) -> Result<LabState, LightningError> {
    ensure_connected(&state)?;

    let route = route_mut(&mut state, from_node, to_node)?;
    if route.status == RouteStatus::Closing || route.status == RouteStatus::Closed {
        return Err(LightningError::RouteAlreadyClosing);
    }
    if route.status != RouteStatus::Active {
        return Err(LightningError::RouteNotActive);
    }

    route.status = RouteStatus::Closing;
    route.requires_next_block = true;

    push_log(
        &mut state,
        &format!("Closed {} trade", to_node.label()),
        "Channel close started. Mine the next block to finish closing the Lightning channel.",
        &["Channel Close Request"],
    );

    Ok(state)
}

pub fn wait_for_next_block(
    mut state: LabState,
    reason: BlockWaitReason,
    affected_route_id: Option<String>,
) -> Result<LabState, LightningError> {
    ensure_connected(&state)?;

    state.block_height += 1;
    let mut completed_trades = Vec::new();
    for route in &mut state.trade_routes {
        let affects_route = affected_route_id
            .as_ref()
            .map(|id| id == &route.route_id)
            .unwrap_or(true);

        if affects_route {
            if route.status == RouteStatus::UnderConstruction {
                route.status = RouteStatus::Active;
                route.requires_next_block = false;
                completed_trades.push(route.game_label.clone());
            } else if route.status == RouteStatus::Closing {
                route.status = RouteStatus::Closed;
                route.requires_next_block = false;
                route.lnd_channel_point = None;
                route.local_balance_sats = 0;
                route.remote_balance_sats = 0;
                completed_trades.push(route.game_label.clone());
            }
        }
    }

    let action = BlockWaitAction {
        action_id: format!("block-{}", state.block_actions.len() + 1),
        reason,
        affected_route_id,
        blocks_requested: 1,
        status: BlockWaitStatus::Mined,
        resulting_height: Some(state.block_height),
    };
    state.block_actions.push(action);

    let detail = if completed_trades.is_empty() {
        "Regtest mined one block instantly. No pending channel changed state.".to_string()
    } else {
        format!(
            "Regtest mined one block instantly. Completed channel updates: {}.",
            completed_trades.join(", ")
        )
    };
    let details = if completed_trades.is_empty() {
        vec!["Block Mined"]
    } else if reason == BlockWaitReason::ChannelCloseConfirmation {
        vec![
            "Channel Close Request",
            "Block Mined",
            "Channel Close Complete",
        ]
    } else {
        vec![
            "Channel Open Request",
            "Block Mined",
            "Channel Open Complete",
        ]
    };
    push_log(&mut state, "Waited for next block", &detail, &details);

    Ok(state)
}

pub fn apply_external_block_height(
    mut state: LabState,
    observed_height: u64,
) -> Result<LabState, LightningError> {
    ensure_connected(&state)?;

    if observed_height <= state.block_height {
        state.block_height = observed_height;
        return Ok(state);
    }

    let previous_height = state.block_height;
    state.block_height = observed_height;

    let mut activated_trades = Vec::new();
    for route in &mut state.trade_routes {
        if route.status == RouteStatus::UnderConstruction && route.requires_next_block {
            route.status = RouteStatus::Active;
            route.requires_next_block = false;
            activated_trades.push(route.game_label.clone());
        }
    }

    if activated_trades.is_empty() {
        return Ok(state);
    }

    let action = BlockWaitAction {
        action_id: format!("block-{}", state.block_actions.len() + 1),
        reason: BlockWaitReason::ChannelOpenConfirmation,
        affected_route_id: None,
        blocks_requested: observed_height.saturating_sub(previous_height),
        status: BlockWaitStatus::Mined,
        resulting_height: Some(observed_height),
    };
    state.block_actions.push(action);

    let detail = format!(
        "Polar reached block {observed_height}. Active trades: {}.",
        activated_trades.join(", ")
    );
    push_log(
        &mut state,
        "Detected Polar block",
        &detail,
        &[
            "Channel Open Request",
            "External Block Detected",
            "Channel Open Complete",
        ],
    );

    Ok(state)
}

pub fn create_invoice(
    mut state: LabState,
    creator_node: DemoNodeId,
    expected_payer_node: Option<DemoNodeId>,
    amount_sats: u64,
    memo: String,
) -> Result<LabState, LightningError> {
    ensure_connected(&state)?;
    validate_amount(amount_sats)?;

    let invoice_id = format!("invoice-{}", state.recent_invoices.len() + 1);
    let invoice = InvoiceRequest {
        invoice_id: invoice_id.clone(),
        creator_node,
        expected_payer_node,
        amount_sats,
        memo: memo.clone(),
        payment_request: format!(
            "lnbcrt{}n1{}{}",
            amount_sats,
            creator_node.label().to_ascii_lowercase(),
            state.recent_invoices.len() + 1
        ),
        status: InvoiceStatus::Created,
        created_at: Utc::now(),
        settled_at: None,
    };
    state.recent_invoices.insert(0, invoice);

    push_log(
        &mut state,
        &format!("{} created an invoice", creator_node.label()),
        &format!("{memo}: {amount_sats} sats requested. Creating an invoice does not need a new Bitcoin block."),
        &["Invoice Sent"],
    );

    Ok(state)
}

pub fn pay_invoice(
    mut state: LabState,
    payer_node: DemoNodeId,
    invoice_id: String,
) -> Result<LabState, LightningError> {
    ensure_connected(&state)?;

    let invoice_index = state
        .recent_invoices
        .iter()
        .position(|invoice| {
            invoice.invoice_id == invoice_id && invoice.status == InvoiceStatus::Created
        })
        .ok_or(LightningError::InvoiceUnavailable)?;

    let payee_node = state.recent_invoices[invoice_index].creator_node;
    let amount_sats = state.recent_invoices[invoice_index].amount_sats;

    let settled_over_route =
        apply_payment_to_route(&mut state, payer_node, payee_node, amount_sats)?;

    state.recent_invoices[invoice_index].status = InvoiceStatus::Settled;
    state.recent_invoices[invoice_index].settled_at = Some(Utc::now());

    let payment = PaymentAttempt {
        payment_id: format!("payment-{}", state.recent_payments.len() + 1),
        payer_node,
        payee_node,
        invoice_id: invoice_id.clone(),
        amount_sats,
        route_summary: Some(format!("{} -> {}", payer_node.label(), payee_node.label())),
        status: PaymentStatus::Succeeded,
        failure_reason: None,
        requires_block: false,
    };
    state.recent_payments.insert(0, payment);

    push_log(
        &mut state,
        &format!("{} paid {}", payer_node.label(), payee_node.label()),
        &payment_detail(amount_sats, settled_over_route),
        &["Invoice Sent", "Invoice Paid"],
    );

    Ok(state)
}

pub fn create_invoice_and_maybe_autosend(
    state: LabState,
    creator_node: DemoNodeId,
    candidate_payer_node: DemoNodeId,
    amount_sats: u64,
    memo: String,
    autosend_enabled: bool,
) -> Result<LabState, LightningError> {
    let state = create_invoice(
        state,
        creator_node,
        Some(candidate_payer_node),
        amount_sats,
        memo,
    )?;

    if !autosend_enabled {
        return Ok(state);
    }

    let invoice_id = state
        .recent_invoices
        .first()
        .map(|invoice| invoice.invoice_id.clone())
        .ok_or(LightningError::InvoiceUnavailable)?;

    pay_invoice(state, candidate_payer_node, invoice_id)
}

pub fn get_operation_faq() -> Vec<OperationFaqRow> {
    vec![
        OperationFaqRow {
            operation: "Create invoice".to_string(),
            needs_bitcoin_node: true,
            needs_mined_block: false,
            plain_explanation: "The receiving LND node creates a Lightning payment request.".to_string(),
            game_example: Some("The NPC asks the player to pay for a book.".to_string()),
        },
        OperationFaqRow {
            operation: "Pay invoice".to_string(),
            needs_bitcoin_node: true,
            needs_mined_block: false,
            plain_explanation: "Payment uses an active Lightning channel and settles without waiting for a new block.".to_string(),
            game_example: Some("The player pays the NPC after the trade is active.".to_string()),
        },
        OperationFaqRow {
            operation: "Fund wallet".to_string(),
            needs_bitcoin_node: true,
            needs_mined_block: true,
            plain_explanation: "A wallet funding transaction needs a mined Bitcoin block before LND treats it as confirmed.".to_string(),
            game_example: Some("Polar funds Alice before the lab starts.".to_string()),
        },
        OperationFaqRow {
            operation: "Open channel".to_string(),
            needs_bitcoin_node: true,
            needs_mined_block: true,
            plain_explanation: "The channel opening transaction must confirm before Lightning payments can use it.".to_string(),
            game_example: Some("Open Trade starts channel construction.".to_string()),
        },
        OperationFaqRow {
            operation: "Close channel".to_string(),
            needs_bitcoin_node: true,
            needs_mined_block: true,
            plain_explanation: "Closing returns funds on chain and needs a Bitcoin confirmation for finality.".to_string(),
            game_example: Some("Close Trade exits the channel back to the chain.".to_string()),
        },
        OperationFaqRow {
            operation: "Check payment status".to_string(),
            needs_bitcoin_node: true,
            needs_mined_block: false,
            plain_explanation: "LND can report payment state without mining a new block.".to_string(),
            game_example: Some("Network Dashboard reads the latest payment.".to_string()),
        },
        OperationFaqRow {
            operation: "Wait for next block".to_string(),
            needs_bitcoin_node: true,
            needs_mined_block: true,
            plain_explanation: "Mainnet blocks arrive about every 10 minutes on average; regtest can mine one instantly.".to_string(),
            game_example: Some("A trade under construction becomes active.".to_string()),
        },
    ]
}

fn ensure_connected(state: &LabState) -> Result<(), LightningError> {
    if state.profile.is_connected() {
        Ok(())
    } else {
        Err(LightningError::SetupIncomplete)
    }
}

fn validate_amount(amount_sats: u64) -> Result<(), LightningError> {
    if amount_sats == 0 || amount_sats > MAX_SATS_PER_TRANSACTION {
        return Err(LightningError::InvalidDemoAmount);
    }

    Ok(())
}

fn default_nodes(connected: bool) -> Vec<DemoNode> {
    DemoNodeId::ALL
        .into_iter()
        .map(|node_id| DemoNode {
            node_id,
            role: node_id.role(),
            location: node_id.location(),
            alias: node_id.label().to_ascii_lowercase(),
            pubkey: connected.then(|| {
                format!(
                    "{}-regtest-demo-pubkey",
                    node_id.label().to_ascii_lowercase()
                )
            }),
            wallet_balance_sats: if connected { 1_000_000 } else { 0 },
            channel_balance_sats: if connected && node_id == DemoNodeId::Alice {
                DEFAULT_ROUTE_CAPACITY_SATS
            } else {
                0
            },
            status: if connected {
                NodeStatus::Online
            } else {
                NodeStatus::Offline
            },
        })
        .collect()
}

fn default_routes() -> Vec<TradeRoute> {
    vec![
        missing_route(DemoNodeId::Alice, DemoNodeId::Bob),
        missing_route(DemoNodeId::Alice, DemoNodeId::Carol),
    ]
}

fn missing_route(from_node: DemoNodeId, to_node: DemoNodeId) -> TradeRoute {
    TradeRoute {
        route_id: route_id(from_node, to_node),
        from_node,
        to_node,
        game_label: format!("{} to {} trade", from_node.label(), to_node.label()),
        lnd_channel_point: None,
        capacity_sats: DEFAULT_ROUTE_CAPACITY_SATS,
        local_balance_sats: 0,
        remote_balance_sats: 0,
        status: RouteStatus::Missing,
        requires_next_block: false,
    }
}

fn route_mut(
    state: &mut LabState,
    from_node: DemoNodeId,
    to_node: DemoNodeId,
) -> Result<&mut TradeRoute, LightningError> {
    let id = route_id(from_node, to_node);
    state
        .trade_routes
        .iter_mut()
        .find(|route| route.route_id == id)
        .ok_or(LightningError::RouteNotActive)
}

fn route_id(from_node: DemoNodeId, to_node: DemoNodeId) -> String {
    format!(
        "{}-{}",
        from_node.label().to_ascii_lowercase(),
        to_node.label().to_ascii_lowercase()
    )
}

fn apply_payment_to_route(
    state: &mut LabState,
    payer_node: DemoNodeId,
    payee_node: DemoNodeId,
    amount_sats: u64,
) -> Result<bool, LightningError> {
    let route_payment_result = state
        .trade_routes
        .iter_mut()
        .find(|route| route.connects(payer_node, payee_node))
        .ok_or(LightningError::RouteNotActive)
        .and_then(|route| {
            if route.status != RouteStatus::Active {
                return Err(LightningError::RouteNotActive);
            }

            if payer_node == route.from_node {
                if route.local_balance_sats < amount_sats {
                    return Err(LightningError::InsufficientLiquidity);
                }

                route.local_balance_sats -= amount_sats;
                route.remote_balance_sats += amount_sats;
            } else {
                if route.remote_balance_sats < amount_sats {
                    return Err(LightningError::InsufficientLiquidity);
                }

                route.remote_balance_sats -= amount_sats;
                route.local_balance_sats += amount_sats;
            }

            Ok(())
        });

    if route_payment_result.is_ok() {
        for node in &mut state.nodes {
            if node.node_id == payer_node {
                node.channel_balance_sats = node.channel_balance_sats.saturating_sub(amount_sats);
            } else if node.node_id == payee_node {
                node.channel_balance_sats = node.channel_balance_sats.saturating_add(amount_sats);
            }
        }

        return Ok(true);
    }

    if payer_node == DemoNodeId::Alice {
        return route_payment_result.map(|_| true);
    }

    apply_wallet_payment(state, payer_node, payee_node, amount_sats)?;
    Ok(false)
}

fn apply_wallet_payment(
    state: &mut LabState,
    payer_node: DemoNodeId,
    payee_node: DemoNodeId,
    amount_sats: u64,
) -> Result<(), LightningError> {
    let payer_index = state
        .nodes
        .iter()
        .position(|node| node.node_id == payer_node)
        .ok_or(LightningError::RouteNotActive)?;
    let payee_index = state
        .nodes
        .iter()
        .position(|node| node.node_id == payee_node)
        .ok_or(LightningError::RouteNotActive)?;

    if state.nodes[payer_index].wallet_balance_sats < amount_sats {
        return Err(LightningError::InsufficientLiquidity);
    }

    state.nodes[payer_index].wallet_balance_sats -= amount_sats;
    state.nodes[payee_index].wallet_balance_sats = state.nodes[payee_index]
        .wallet_balance_sats
        .saturating_add(amount_sats);

    Ok(())
}

fn payment_detail(amount_sats: u64, settled_over_route: bool) -> String {
    if settled_over_route {
        format!(
            "Paid {amount_sats} sats over an active Lightning channel. No new Bitcoin block was required."
        )
    } else {
        format!(
            "Paid {amount_sats} sats from the NPC wallet balance because the current sell action did not need channel-side liquidity."
        )
    }
}

fn push_log(state: &mut LabState, summary: &str, network_detail: &str, details: &[&str]) {
    state.action_log.insert(
        0,
        ActionLogEntry {
            id: format!("log-{}", state.action_log.len() + 1),
            summary: summary.to_string(),
            network_detail: network_detail.to_string(),
            details: details.iter().map(|detail| (*detail).to_string()).collect(),
            created_at: Utc::now(),
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connected_profile() -> SetupProfile {
        SetupProfile {
            connection_status: ConnectionStatus::Connected,
            ..SetupProfile::default()
        }
    }

    #[test]
    fn external_block_height_activates_pending_trade_route() {
        let state = default_lab_state(connected_profile());
        let state = open_trade_route(
            state,
            DemoNodeId::Alice,
            DemoNodeId::Bob,
            DEFAULT_ROUTE_CAPACITY_SATS,
        )
        .expect("open route");

        assert_eq!(state.trade_routes[0].status, RouteStatus::UnderConstruction);
        assert!(state.trade_routes[0].requires_next_block);

        let state = apply_external_block_height(state, 234).expect("apply external block");

        assert_eq!(state.block_height, 234);
        assert_eq!(state.trade_routes[0].status, RouteStatus::Active);
        assert!(!state.trade_routes[0].requires_next_block);
        assert_eq!(state.block_actions[0].resulting_height, Some(234));
        assert_eq!(state.action_log[0].summary, "Detected Polar block");
    }

    #[test]
    fn next_block_closes_closing_trade_route() {
        let state = default_lab_state(connected_profile());
        let state = open_trade_route(
            state,
            DemoNodeId::Alice,
            DemoNodeId::Bob,
            DEFAULT_ROUTE_CAPACITY_SATS,
        )
        .expect("open route");
        let state = wait_for_next_block(
            state,
            BlockWaitReason::ChannelOpenConfirmation,
            Some("alice-bob".to_string()),
        )
        .expect("activate route");
        let state =
            close_trade_route(state, DemoNodeId::Alice, DemoNodeId::Bob).expect("close route");

        assert_eq!(state.trade_routes[0].status, RouteStatus::Closing);
        assert!(state.trade_routes[0].requires_next_block);

        let state = wait_for_next_block(
            state,
            BlockWaitReason::ChannelCloseConfirmation,
            Some("alice-bob".to_string()),
        )
        .expect("finish close");

        assert_eq!(state.trade_routes[0].status, RouteStatus::Closed);
        assert!(!state.trade_routes[0].requires_next_block);
        assert_eq!(state.trade_routes[0].local_balance_sats, 0);
        assert_eq!(state.trade_routes[0].remote_balance_sats, 0);
    }

    #[test]
    fn npc_can_pay_player_from_wallet_when_route_has_no_remote_liquidity() {
        let state = default_lab_state(connected_profile());
        let bob_wallet_before = state
            .nodes
            .iter()
            .find(|node| node.node_id == DemoNodeId::Bob)
            .expect("Bob node")
            .wallet_balance_sats;
        let alice_wallet_before = state
            .nodes
            .iter()
            .find(|node| node.node_id == DemoNodeId::Alice)
            .expect("Alice node")
            .wallet_balance_sats;

        let state = create_invoice_and_maybe_autosend(
            state,
            DemoNodeId::Alice,
            DemoNodeId::Bob,
            1_000,
            "Player sells an item to Bob".to_string(),
            true,
        )
        .expect("Bob wallet payment should settle");

        let bob_wallet_after = state
            .nodes
            .iter()
            .find(|node| node.node_id == DemoNodeId::Bob)
            .expect("Bob node")
            .wallet_balance_sats;
        let alice_wallet_after = state
            .nodes
            .iter()
            .find(|node| node.node_id == DemoNodeId::Alice)
            .expect("Alice node")
            .wallet_balance_sats;

        assert_eq!(bob_wallet_after, bob_wallet_before - 1_000);
        assert_eq!(alice_wallet_after, alice_wallet_before + 1_000);
        assert_eq!(state.recent_payments[0].payer_node, DemoNodeId::Bob);
        assert_eq!(state.recent_payments[0].payee_node, DemoNodeId::Alice);
    }
}
