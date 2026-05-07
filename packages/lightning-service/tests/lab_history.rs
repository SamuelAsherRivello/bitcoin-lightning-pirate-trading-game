use lightning_service as service;
use lightning_service::{
    BlockWaitReason, ConnectionStatus, DemoNodeId, RouteStatus, SetupProfile,
    DEFAULT_ROUTE_CAPACITY_SATS, DEFAULT_SATS_PER_TRANSACTION,
};

fn connected_profile() -> SetupProfile {
    let mut profile = SetupProfile::default();
    profile.connection_status = ConnectionStatus::Connected;
    profile
}

#[test]
fn history_details_describe_channel_open_completion() {
    let state = service::default_lab_state(connected_profile());
    let state = service::open_trade_route(
        state,
        DemoNodeId::Alice,
        DemoNodeId::Bob,
        DEFAULT_ROUTE_CAPACITY_SATS,
    )
    .expect("route should open");

    assert_eq!(
        state.action_log[0].details,
        vec!["Channel Open Request".to_string()]
    );

    let route_id = state
        .trade_routes
        .iter()
        .find(|route| route.to_node == DemoNodeId::Bob)
        .expect("Bob route should exist")
        .route_id
        .clone();
    let state = service::wait_for_next_block(
        state,
        BlockWaitReason::ChannelOpenConfirmation,
        Some(route_id),
    )
    .expect("next block should confirm route");

    assert_eq!(state.trade_routes[0].status, RouteStatus::Active);
    assert_eq!(
        state.action_log[0].details,
        vec![
            "Channel Open Request".to_string(),
            "Block Mined".to_string(),
            "Channel Open Complete".to_string(),
        ]
    );
}

#[test]
fn history_details_describe_invoice_payment() {
    let state = service::default_lab_state(connected_profile());
    let state = service::open_trade_route(
        state,
        DemoNodeId::Alice,
        DemoNodeId::Bob,
        DEFAULT_ROUTE_CAPACITY_SATS,
    )
    .expect("route should open");
    let route_id = state
        .trade_routes
        .iter()
        .find(|route| route.to_node == DemoNodeId::Bob)
        .expect("Bob route should exist")
        .route_id
        .clone();
    let state = service::wait_for_next_block(
        state,
        BlockWaitReason::ChannelOpenConfirmation,
        Some(route_id),
    )
    .expect("next block should confirm route");
    let state = service::create_invoice_and_maybe_autosend(
        state,
        DemoNodeId::Bob,
        DemoNodeId::Alice,
        DEFAULT_SATS_PER_TRANSACTION,
        "Alice buys a Beach item".to_string(),
        true,
    )
    .expect("invoice should be paid");

    assert_eq!(
        state.action_log[0].details,
        vec!["Invoice Sent".to_string(), "Invoice Paid".to_string()]
    );
    assert_eq!(
        state.action_log[1].details,
        vec!["Invoice Sent".to_string()]
    );
}

#[test]
fn reverse_invoice_payment_restores_player_liquidity() {
    let state = service::default_lab_state(connected_profile());
    let state = service::open_trade_route(
        state,
        DemoNodeId::Alice,
        DemoNodeId::Bob,
        DEFAULT_ROUTE_CAPACITY_SATS,
    )
    .expect("route should open");
    let route_id = state
        .trade_routes
        .iter()
        .find(|route| route.to_node == DemoNodeId::Bob)
        .expect("Bob route should exist")
        .route_id
        .clone();
    let state = service::wait_for_next_block(
        state,
        BlockWaitReason::ChannelOpenConfirmation,
        Some(route_id),
    )
    .expect("next block should confirm route");
    let state = service::create_invoice_and_maybe_autosend(
        state,
        DemoNodeId::Bob,
        DemoNodeId::Alice,
        DEFAULT_SATS_PER_TRANSACTION,
        "Player buys a book from the NPC".to_string(),
        true,
    )
    .expect("buy invoice should be paid");
    let state = service::create_invoice_and_maybe_autosend(
        state,
        DemoNodeId::Alice,
        DemoNodeId::Bob,
        DEFAULT_SATS_PER_TRANSACTION,
        "Player sells a book to the NPC".to_string(),
        true,
    )
    .expect("sell invoice should be paid");

    assert_eq!(
        state.trade_routes[0].local_balance_sats,
        DEFAULT_ROUTE_CAPACITY_SATS
    );
    assert_eq!(state.trade_routes[0].remote_balance_sats, 0);
    assert_eq!(state.recent_payments[0].payer_node, DemoNodeId::Bob);
    assert_eq!(state.recent_payments[0].payee_node, DemoNodeId::Alice);
}
