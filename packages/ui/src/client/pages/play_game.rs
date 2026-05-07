use dioxus::prelude::dioxus_router::Navigator;
use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::client::components::game::{
    GameAnimation, GameChannelAnimation, GameChannelVisual, GameInventorySlot, GameSide, GameView,
    GameViewConfig, HistoryItems, LabStatusWidget, RouteSummary,
};
use crate::client::components::toast::{
    wait_for_prompt_message_minimum, OperationPrompt, Toast, ToastTone,
};
use crate::client::models::{
    ConnectionStatus, DemoNode, DemoNodeId, LabState, PaymentStatus, RouteStatus, SetupMode,
    SetupProfile, TradeRoute, DEFAULT_ROUTE_CAPACITY_SATS,
};
use crate::client::services::lightning_server_functions::{
    close_polar_demo_channels_with_progress, close_trade_route, complete_polar_setup,
    create_invoice_and_maybe_autosend, create_polar_demo_nodes_with_progress,
    destroy_polar_demo_nodes, ensure_polar_server, get_lab_state_or_recover, open_trade_route,
    recover_if_polar_lab_unhealthy, verify_polar_bridge, wait_for_next_block, PolarLabRecovery,
};
use crate::client::Route;

const GAME_LEFT_BG: Asset = asset!("/assets/images/game/left-bg.svg");
const GAME_DESERT_BG: Asset = asset!("/assets/images/game/right-bg.svg");
const GAME_BLIZZARD_BG: Asset = asset!("/assets/images/game/blizzard-bg.svg");
const GAME_JUNGLE_BG: Asset = asset!("/assets/images/game/jungle-bg.svg");
const GAME_OCEAN_BG: Asset = asset!("/assets/images/game/ocean-bg.svg");
const GAME_PLAYER: Asset = asset!("/assets/images/game/player.svg");
const GAME_NPC: Asset = asset!("/assets/images/game/npc.svg");
const GAME_NPC_ALT: Asset = asset!("/assets/images/game/npc-alt.svg");
const GAME_PURSE: Asset = asset!("/assets/images/game/purse.svg");
const GAME_CHANNEL: Asset = asset!("/assets/images/game/channel.svg");
const GAME_CHANNEL_DOTTED: Asset = asset!("/assets/images/game/channel-dotted.svg");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GameLocation {
    Desert,
    Blizzard,
    Jungle,
    Ocean,
}

impl GameLocation {
    const ALL: [Self; 4] = [Self::Desert, Self::Blizzard, Self::Jungle, Self::Ocean];

    fn by_index(index: usize) -> Self {
        Self::ALL[index % Self::ALL.len()]
    }

    fn label(self) -> &'static str {
        match self {
            Self::Desert => "Desert",
            Self::Blizzard => "Blizzard",
            Self::Jungle => "Jungle",
            Self::Ocean => "Ocean",
        }
    }

    fn merchant(self) -> DemoNodeId {
        match self {
            Self::Desert | Self::Jungle => DemoNodeId::Bob,
            Self::Blizzard | Self::Ocean => DemoNodeId::Carol,
        }
    }

    fn background(self) -> Asset {
        match self {
            Self::Desert => GAME_DESERT_BG,
            Self::Blizzard => GAME_BLIZZARD_BG,
            Self::Jungle => GAME_JUNGLE_BG,
            Self::Ocean => GAME_OCEAN_BG,
        }
    }

    fn character(self) -> Asset {
        match self.merchant() {
            DemoNodeId::Bob => GAME_NPC,
            DemoNodeId::Carol => GAME_NPC_ALT,
            DemoNodeId::Alice => GAME_NPC,
        }
    }
}

#[component]
pub fn PlayGame() -> Element {
    let active_route = use_route::<Route>();
    let setup_profile = use_context::<Signal<SetupProfile>>();
    let mut lab_state = use_context::<Signal<Option<LabState>>>();
    let toast = use_context::<Signal<Option<Toast>>>();
    let operation_prompt = use_context::<Signal<Option<OperationPrompt>>>();
    let toast_sequence = use_signal(|| 30_000_u64);
    let prompt_sequence = use_signal(|| 50_000_u64);
    let mut is_busy = use_signal(|| false);
    let mut game_animation = use_signal(GameAnimation::default);
    let mut channel_animation = use_signal(GameChannelAnimation::default);
    let mut location_index = use_signal(|| 0_usize);
    let navigator = navigator();

    use_effect(move || {
        let profile = setup_profile();
        if active_route == (Route::PlayGame {}) && profile.is_connected() {
            spawn(async move {
                match get_lab_state_or_recover(profile).await {
                    Ok(state) => {
                        if lab_state.peek().is_none() || lab_state.peek().as_ref() != Some(&state) {
                            lab_state.set(Some(state));
                        }
                    }
                    Err(recovery) => {
                        apply_lab_recovery(
                            setup_profile,
                            lab_state,
                            operation_prompt,
                            prompt_sequence,
                            navigator,
                            recovery,
                        );
                    }
                }
            });
        }
    });

    let profile = setup_profile();
    if !profile.is_connected() {
        return rsx! {
            LockedPage {
                title: t!("play-game-title"),
                detail: "Complete Set Up before gameplay starts.".to_string(),
            }
        };
    }

    let Some(state) = lab_state() else {
        return rsx! {
            main { class: "page-content lab-page",
                section { class: "lab-hero",
                    div {
                        span { class: "eyebrow", "Loading" }
                        h1 { {t!("play-game-title")} }
                        p { "Loading the local Lightning lab state..." }
                    }
                }
            }
        };
    };

    let next_block_height = state.block_height.saturating_add(1);
    let current_location = GameLocation::by_index(location_index());
    let merchant = current_location.merchant();
    let focused_route = state
        .trade_routes
        .iter()
        .find(|route| route.to_node == merchant)
        .cloned();
    let player_books = player_books_for(&state);
    let npc_books = npc_books_for(&state, merchant);
    let left_inventory = inventory_slots(player_books);
    let right_inventory = inventory_slots(npc_books);
    let player_name = node_display_name(&state, DemoNodeId::Alice);
    let npc_name = node_display_name(&state, merchant);
    let (player_sats, npc_sats) =
        game_sats_for_route(focused_route.as_ref(), state.profile.sats_per_transaction);
    let channel_visual = focused_route
        .as_ref()
        .map(|route| match route.status {
            RouteStatus::UnderConstruction | RouteStatus::Closing => GameChannelVisual::Pending,
            RouteStatus::Active => GameChannelVisual::Active,
            _ => GameChannelVisual::None,
        })
        .unwrap_or(GameChannelVisual::None);
    let render_channel = channel_visual != GameChannelVisual::None
        || channel_animation() != GameChannelAnimation::None;
    let game_view_config = {
        let config = GameViewConfig::default()
            .show_bg(GameSide::Left, GAME_LEFT_BG)
            .show_bg(GameSide::Right, current_location.background())
            .show_character(GameSide::Left, GAME_PLAYER)
            .show_character(GameSide::Right, current_location.character())
            .show_purse(GameSide::Left, GAME_PURSE)
            .show_purse(GameSide::Right, GAME_PURSE)
            .show_name(GameSide::Left, format!("Player: {player_name}"))
            .show_name(GameSide::Right, format!("NPC: {npc_name}"))
            .show_sats(GameSide::Left, player_sats)
            .show_sats(GameSide::Right, npc_sats)
            .show_inventory(GameSide::Left, left_inventory)
            .show_inventory(GameSide::Right, right_inventory)
            .show_animation(game_animation())
            .show_channel_visual(channel_visual)
            .show_channel_animation(channel_animation());

        if render_channel {
            config.show_channel(GAME_CHANNEL_DOTTED, GAME_CHANNEL)
        } else {
            config
        }
    };
    let can_open_trade = focused_route
        .as_ref()
        .map(|route| matches!(route.status, RouteStatus::Missing | RouteStatus::Closed))
        .unwrap_or(false);
    let can_close_trade = focused_route
        .as_ref()
        .map(|route| route.status == RouteStatus::Active)
        .unwrap_or(false);
    let can_wait_for_block = focused_route
        .as_ref()
        .map(|route| route.requires_next_block)
        .unwrap_or(false);
    let can_buy_item = focused_route
        .as_ref()
        .map(|route| {
            route.status == RouteStatus::Active
                && npc_books > 0
                && player_books < 3
                && route.local_balance_sats >= state.profile.sats_per_transaction
        })
        .unwrap_or(false);
    let can_sell_item = focused_route
        .as_ref()
        .map(|route| {
            route.status == RouteStatus::Active
                && player_books > 0
                && route.remote_balance_sats >= state.profile.sats_per_transaction
        })
        .unwrap_or(false);
    let focused_route_for_wait = focused_route.clone();
    let focused_route_for_panel = focused_route.clone();

    rsx! {
        main { class: "page-content lab-page play-page",
            section { class: "lab-hero",
                div {
                    span { class: "eyebrow", "Player and NPC" }
                    h1 { {t!("play-game-title")} }
                    p {
                        "Open a Lightning trade with the NPC, wait for the next block when the channel needs confirmation, then buy books over the active channel."
                    }
                }
                LabStatusWidget {
                    sats_per_transaction: state.profile.sats_per_transaction,
                    block_height: state.block_height,
                }
            }

            GameView {
                config: game_view_config,
                is_busy: is_busy(),
                can_open_trade,
                can_close_trade,
                can_wait_for_block,
                can_buy_item,
                can_sell_item,
                next_block_height,
                on_restart_game: move |_| {
                    spawn(async move {
                        restart_game_from_polar_setup(
                            is_busy,
                            setup_profile,
                            lab_state,
                            operation_prompt,
                            prompt_sequence,
                            toast,
                            toast_sequence,
                            game_animation,
                            channel_animation,
                            location_index,
                        )
                        .await;
                    });
                },
                on_open_trade: move |_| {
                    spawn(async move {
                        is_busy.set(true);
                        match open_trade_route(setup_profile(), merchant).await {
                            Ok(next_state) => {
                                lab_state.set(Some(next_state));
                                channel_animation.set(GameChannelAnimation::PendingFadeIn);
                                wait_for_channel_animation().await;
                                channel_animation.set(GameChannelAnimation::None);
                                push_toast(toast, toast_sequence, "Open Trade Route sent.", ToastTone::Success);
                            }
                            Err(message) => handle_lab_action_error(
                                setup_profile(),
                                setup_profile,
                                lab_state,
                                toast,
                                toast_sequence,
                                operation_prompt,
                                prompt_sequence,
                                navigator,
                                message,
                            )
                            .await,
                        }
                        is_busy.set(false);
                    });
                },
                on_close_trade: move |_| {
                    spawn(async move {
                        is_busy.set(true);
                        match close_trade_route(setup_profile(), merchant).await {
                            Ok(next_state) => {
                                lab_state.set(Some(next_state));
                                channel_animation.set(GameChannelAnimation::ActiveToPending);
                                wait_for_channel_animation().await;
                                channel_animation.set(GameChannelAnimation::None);
                                push_toast(toast, toast_sequence, "Close Trade Route sent.", ToastTone::Success);
                            }
                            Err(message) => handle_lab_action_error(
                                setup_profile(),
                                setup_profile,
                                lab_state,
                                toast,
                                toast_sequence,
                                operation_prompt,
                                prompt_sequence,
                                navigator,
                                message,
                            )
                            .await,
                        }
                        channel_animation.set(GameChannelAnimation::None);
                        is_busy.set(false);
                    });
                },
                on_wait_for_block: move |_| {
                    let route_id = focused_route_for_wait.as_ref().map(|route| route.route_id.clone());
                    let wait_animation = focused_route_for_wait
                        .as_ref()
                        .map(|route| match route.status {
                            RouteStatus::UnderConstruction => GameChannelAnimation::PendingToActive,
                            RouteStatus::Closing => GameChannelAnimation::PendingFadeOut,
                            _ => GameChannelAnimation::None,
                        })
                        .unwrap_or(GameChannelAnimation::None);
                    spawn(async move {
                        is_busy.set(true);
                        match wait_for_next_block(setup_profile(), route_id).await {
                            Ok(next_state) => {
                                lab_state.set(Some(next_state));
                                if wait_animation != GameChannelAnimation::None {
                                    channel_animation.set(wait_animation);
                                    wait_for_channel_animation().await;
                                    channel_animation.set(GameChannelAnimation::None);
                                }
                                push_toast(toast, toast_sequence, "Regtest mined the next block.", ToastTone::Success);
                            }
                            Err(message) => handle_lab_action_error(
                                setup_profile(),
                                setup_profile,
                                lab_state,
                                toast,
                                toast_sequence,
                                operation_prompt,
                                prompt_sequence,
                                navigator,
                                message,
                            )
                            .await,
                        }
                        is_busy.set(false);
                    });
                },
                on_change_location: move |_| {
                    let next_location = (location_index() + 1) % GameLocation::ALL.len();
                    game_animation.set(GameAnimation::None);
                    channel_animation.set(GameChannelAnimation::None);
                    location_index.set(next_location);
                },
                on_buy_item: move |_| {
                    spawn(async move {
                        is_busy.set(true);
                        game_animation.set(GameAnimation::PaymentLeftToRight);
                        wait_for_game_animation().await;
                        let memo = "Player buys a book from the NPC".to_string();
                        match create_invoice_and_maybe_autosend(
                            setup_profile(),
                            merchant,
                            DemoNodeId::Alice,
                            true,
                            memo,
                        )
                        .await
                        {
                            Ok(next_state) => {
                                wait_between_trade_animations().await;
                                game_animation.set(GameAnimation::ItemRightToLeft);
                                wait_for_game_animation().await;
                                lab_state.set(Some(next_state));
                                push_toast(toast, toast_sequence, "Invoice created and paid.", ToastTone::Success);
                            }
                            Err(message) => handle_lab_action_error(
                                setup_profile(),
                                setup_profile,
                                lab_state,
                                toast,
                                toast_sequence,
                                operation_prompt,
                                prompt_sequence,
                                navigator,
                                message,
                            )
                            .await,
                        }
                        game_animation.set(GameAnimation::None);
                        is_busy.set(false);
                    });
                },
                on_sell_item: move |_| {
                    spawn(async move {
                        is_busy.set(true);
                        game_animation.set(GameAnimation::ItemLeftToRight);
                        wait_for_game_animation().await;
                        let memo = "Player sells a book to the NPC".to_string();
                        match create_invoice_and_maybe_autosend(
                            setup_profile(),
                            DemoNodeId::Alice,
                            merchant,
                            true,
                            memo,
                        )
                        .await
                        {
                            Ok(next_state) => {
                                game_animation.set(GameAnimation::PaymentRightToLeft);
                                wait_for_game_animation().await;
                                lab_state.set(Some(next_state));
                                push_toast(toast, toast_sequence, "Item sold and invoice paid.", ToastTone::Success);
                            }
                            Err(message) => handle_lab_action_error(
                                setup_profile(),
                                setup_profile,
                                lab_state,
                                toast,
                                toast_sequence,
                                operation_prompt,
                                prompt_sequence,
                                navigator,
                                message,
                            )
                            .await,
                        }
                        game_animation.set(GameAnimation::None);
                        is_busy.set(false);
                    });
                },
            }

            if let Some(route) = focused_route_for_panel {
                section { class: "lab-grid",
                    article { class: "lab-panel route-card",
                        RouteSummary { route: route.clone() }
                        span { class: "eyebrow", "{current_location.label()} location" }
                        p { "{trade_status_copy(&route)}" }
                        div { class: "route-metrics",
                            span { "Capacity: {route.capacity_sats} sats" }
                            span { "Player side: {route.local_balance_sats} sats" }
                            span { "NPC side: {route.remote_balance_sats} sats" }
                        }
                    }
                }
            }

            HistoryItems { entries: state.action_log.clone() }
        }
    }
}

fn player_books_for(state: &LabState) -> usize {
    let bought = state
        .recent_payments
        .iter()
        .filter(|payment| {
            payment.payer_node == DemoNodeId::Alice
                && payment.payee_node != DemoNodeId::Alice
                && payment.status == PaymentStatus::Succeeded
        })
        .count();
    let sold = state
        .recent_payments
        .iter()
        .filter(|payment| {
            payment.payer_node != DemoNodeId::Alice
                && payment.payee_node == DemoNodeId::Alice
                && payment.status == PaymentStatus::Succeeded
        })
        .count();

    bought.saturating_sub(sold).min(3)
}

fn npc_books_for(state: &LabState, merchant: DemoNodeId) -> usize {
    let sold_to_player = state
        .recent_payments
        .iter()
        .filter(|payment| {
            payment.payer_node == DemoNodeId::Alice
                && payment.payee_node == merchant
                && payment.status == PaymentStatus::Succeeded
        })
        .count();
    let bought_from_player = state
        .recent_payments
        .iter()
        .filter(|payment| {
            payment.payer_node == merchant
                && payment.payee_node == DemoNodeId::Alice
                && payment.status == PaymentStatus::Succeeded
        })
        .count();

    3_usize
        .saturating_sub(sold_to_player)
        .saturating_add(bought_from_player)
        .min(3)
}

fn game_sats_for_route(route: Option<&TradeRoute>, item_cost_sats: u64) -> (u64, u64) {
    route
        .filter(|route| {
            matches!(
                route.status,
                RouteStatus::UnderConstruction | RouteStatus::Active | RouteStatus::Closing
            )
        })
        .map(|route| (route.local_balance_sats, route.remote_balance_sats))
        .unwrap_or((item_cost_sats.saturating_mul(3), 0))
}

fn node_display_name(state: &LabState, node_id: DemoNodeId) -> String {
    state
        .nodes
        .iter()
        .find(|node| node.node_id == node_id)
        .map(display_node_name)
        .unwrap_or_else(|| node_id.label().to_string())
}

fn display_node_name(node: &DemoNode) -> String {
    let alias = node.alias.trim();
    if alias.is_empty() {
        node.node_id.label().to_string()
    } else {
        title_case_first(alias)
    }
}

fn title_case_first(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };

    first.to_uppercase().collect::<String>() + chars.as_str()
}

fn inventory_slots(book_count: usize) -> Vec<GameInventorySlot> {
    (0..3)
        .map(|index| {
            if index < book_count {
                GameInventorySlot::Book
            } else {
                GameInventorySlot::Empty
            }
        })
        .collect()
}

fn trade_status_copy(route: &TradeRoute) -> &'static str {
    match route.status {
        RouteStatus::Missing => {
            "No Lightning channel is open yet. Use Open Trade Route on the player side to start one."
        }
        RouteStatus::UnderConstruction => {
            "The channel open is pending. Mine the next block to make the trade active."
        }
        RouteStatus::Active => {
            "The Lightning channel is active. Buy Item and Sell Item create and pay invoices through it."
        }
        RouteStatus::Closing => {
            "The channel close is pending. Mine the next block to finish closing the trade."
        }
        RouteStatus::Closed => {
            "The Lightning channel is closed. Open Trade Route again to start a new trade route."
        }
        RouteStatus::Error => {
            "The Lightning channel needs attention before this trade can continue."
        }
    }
}

async fn restart_game_from_polar_setup(
    mut is_busy: Signal<bool>,
    mut setup_profile: Signal<SetupProfile>,
    mut lab_state: Signal<Option<LabState>>,
    operation_prompt: Signal<Option<OperationPrompt>>,
    prompt_sequence: Signal<u64>,
    toast: Signal<Option<Toast>>,
    toast_sequence: Signal<u64>,
    mut game_animation: Signal<GameAnimation>,
    mut channel_animation: Signal<GameChannelAnimation>,
    mut location_index: Signal<usize>,
) {
    if is_busy() {
        return;
    }

    is_busy.set(true);
    game_animation.set(GameAnimation::None);
    channel_animation.set(GameChannelAnimation::None);
    location_index.set(0);

    let player_count = DemoNodeId::ALL.len();
    let required_balance_sats = setup_profile()
        .sats_per_transaction
        .max(DEFAULT_ROUTE_CAPACITY_SATS);
    let operation_id = begin_operation_prompt(
        operation_prompt,
        prompt_sequence,
        "Restart game",
        format!(
            "Preparing Polar setup for {player_count} players with {required_balance_sats} sats each..."
        ),
        false,
    )
    .await;

    let result = restart_game_from_polar_setup_inner(
        setup_profile(),
        operation_prompt,
        operation_id,
        player_count,
        required_balance_sats,
    )
    .await;

    match result {
        Ok(state) => {
            setup_profile.set(state.profile.clone());
            lab_state.set(Some(state));
            update_operation_prompt(
                operation_prompt,
                operation_id,
                format!(
                    "Game restarted. Alice, Bob, and Carol are ready with {required_balance_sats} sats each."
                ),
                ToastTone::Success,
                false,
                false,
            )
            .await;
            push_toast(toast, toast_sequence, "Game restarted.", ToastTone::Success);
        }
        Err(message) => {
            update_operation_prompt(
                operation_prompt,
                operation_id,
                message.clone(),
                ToastTone::Error,
                false,
                false,
            )
            .await;
            push_toast(toast, toast_sequence, message, ToastTone::Error);
        }
    }

    is_busy.set(false);
}

async fn restart_game_from_polar_setup_inner(
    mut profile: SetupProfile,
    operation_prompt: Signal<Option<OperationPrompt>>,
    operation_id: u64,
    player_count: usize,
    required_balance_sats: u64,
) -> Result<LabState, String> {
    if profile.setup_mode != SetupMode::ServerConfig {
        return Err("Restart Game requires the Polar setup mode.".to_string());
    }

    update_operation_prompt(
        operation_prompt,
        operation_id,
        "Step 1 of 5: Checking the Polar bridge...",
        ToastTone::Info,
        true,
        false,
    )
    .await;
    profile.connection_status = ConnectionStatus::SavedOffline;
    profile.last_verified_at = None;
    profile = verify_polar_bridge(profile).await?;

    update_operation_prompt(
        operation_prompt,
        operation_id,
        "Step 2 of 5: Finding or creating the Polar server...",
        ToastTone::Info,
        true,
        false,
    )
    .await;
    let server_result = ensure_polar_server(profile.clone()).await?;
    profile.polar_automation = server_result.profile;
    profile.connection_status = ConnectionStatus::SavedOffline;
    profile.last_verified_at = None;

    let close_prompt = operation_prompt;
    let close_operation_id = operation_id;
    profile = close_polar_demo_channels_with_progress(profile, move |message| {
        update_operation_prompt_now(
            close_prompt,
            close_operation_id,
            format!("Step 3 of 5: {message}"),
            ToastTone::Info,
            true,
            false,
        );
    })
    .await?;

    update_operation_prompt(
        operation_prompt,
        operation_id,
        format!("Step 3 of 5: Clearing demo players before recreating {player_count} players with {required_balance_sats} sats each..."),
        ToastTone::Info,
        true,
        false,
    )
    .await;
    profile = destroy_polar_demo_nodes(profile).await?;

    update_operation_prompt(
        operation_prompt,
        operation_id,
        "Step 3 of 5: Reconfirming the Polar server before recreating demo players...",
        ToastTone::Info,
        true,
        false,
    )
    .await;
    let server_result = ensure_polar_server(profile.clone()).await?;
    profile.polar_automation = server_result.profile;
    profile.connection_status = ConnectionStatus::SavedOffline;
    profile.last_verified_at = None;

    let progress_prompt = operation_prompt;
    let progress_operation_id = operation_id;
    let state = create_polar_demo_nodes_with_progress(profile, move |message| {
        update_operation_prompt_now(
            progress_prompt,
            progress_operation_id,
            format!("Step 3 of 5: {message}"),
            ToastTone::Info,
            true,
            false,
        );
    })
    .await?;

    update_operation_prompt(
        operation_prompt,
        operation_id,
        format!(
            "Step 4 of 5: Keeping Block Height {} for the restarted game...",
            state.block_height
        ),
        ToastTone::Info,
        true,
        false,
    )
    .await;

    update_operation_prompt(
        operation_prompt,
        operation_id,
        "Step 5 of 5: Unlocking Play Game and Network Dashboard...",
        ToastTone::Info,
        true,
        false,
    )
    .await;
    complete_polar_setup(state.profile).await
}

async fn begin_operation_prompt(
    mut prompt: Signal<Option<OperationPrompt>>,
    mut sequence: Signal<u64>,
    title: impl Into<String>,
    message: impl Into<String>,
    can_cancel: bool,
) -> u64 {
    let operation_id = *sequence.peek() + 1;
    sequence.set(operation_id);
    prompt.set(Some(OperationPrompt {
        operation_id,
        title: title.into(),
        message: message.into(),
        tone: ToastTone::Info,
        is_pending: true,
        can_cancel,
        cancel_requested: false,
    }));
    wait_for_prompt_message_minimum().await;
    operation_id
}

async fn update_operation_prompt(
    mut prompt: Signal<Option<OperationPrompt>>,
    operation_id: u64,
    message: impl Into<String>,
    tone: ToastTone,
    is_pending: bool,
    can_cancel: bool,
) {
    let active_prompt = { prompt.peek().as_ref().cloned() };
    if let Some(mut active_prompt) = active_prompt {
        if active_prompt.operation_id == operation_id {
            active_prompt.message = message.into();
            active_prompt.tone = tone;
            active_prompt.is_pending = is_pending;
            active_prompt.can_cancel = can_cancel;
            prompt.set(Some(active_prompt));
            wait_for_prompt_message_minimum().await;
        }
    }
}

fn update_operation_prompt_now(
    mut prompt: Signal<Option<OperationPrompt>>,
    operation_id: u64,
    message: impl Into<String>,
    tone: ToastTone,
    is_pending: bool,
    can_cancel: bool,
) {
    let active_prompt = { prompt.peek().as_ref().cloned() };
    if let Some(mut active_prompt) = active_prompt {
        if active_prompt.operation_id == operation_id {
            active_prompt.message = message.into();
            active_prompt.tone = tone;
            active_prompt.is_pending = is_pending;
            active_prompt.can_cancel = can_cancel;
            prompt.set(Some(active_prompt));
        }
    }
}

#[cfg(target_arch = "wasm32")]
async fn wait_for_game_animation() {
    gloo_timers::future::TimeoutFuture::new(850).await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn wait_for_game_animation() {
    futures_timer::Delay::new(std::time::Duration::from_millis(850)).await;
}

#[cfg(target_arch = "wasm32")]
async fn wait_between_trade_animations() {
    gloo_timers::future::TimeoutFuture::new(500).await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn wait_between_trade_animations() {
    futures_timer::Delay::new(std::time::Duration::from_millis(500)).await;
}

#[cfg(target_arch = "wasm32")]
async fn wait_for_channel_animation() {
    gloo_timers::future::TimeoutFuture::new(450).await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn wait_for_channel_animation() {
    futures_timer::Delay::new(std::time::Duration::from_millis(450)).await;
}

#[component]
fn LockedPage(title: String, detail: String) -> Element {
    rsx! {
        main { class: "page-content lab-page locked-page",
            section { class: "lab-hero",
                div {
                    span { class: "eyebrow", "Locked" }
                    h1 { "{title}" }
                    p { "{detail}" }
                    Link {
                        class: "primary-action inline-link-action",
                        to: Route::SetUp {},
                        "Go to Set Up"
                    }
                }
            }
        }
    }
}

fn push_toast(
    mut toast: Signal<Option<Toast>>,
    mut sequence: Signal<u64>,
    message: impl Into<String>,
    tone: ToastTone,
) {
    let next_id = *sequence.peek() + 1;
    sequence.set(next_id);
    toast.set(Some(Toast {
        id: next_id,
        message: message.into(),
        tone,
    }));
}

async fn handle_lab_action_error(
    profile: SetupProfile,
    setup_profile: Signal<SetupProfile>,
    lab_state: Signal<Option<LabState>>,
    toast: Signal<Option<Toast>>,
    toast_sequence: Signal<u64>,
    operation_prompt: Signal<Option<OperationPrompt>>,
    prompt_sequence: Signal<u64>,
    navigator: Navigator,
    message: String,
) {
    if let Some(recovery) = recover_if_polar_lab_unhealthy(profile).await {
        apply_lab_recovery(
            setup_profile,
            lab_state,
            operation_prompt,
            prompt_sequence,
            navigator,
            recovery,
        );
    } else {
        push_toast(toast, toast_sequence, message, ToastTone::Error);
    }
}

fn apply_lab_recovery(
    mut setup_profile: Signal<SetupProfile>,
    mut lab_state: Signal<Option<LabState>>,
    mut operation_prompt: Signal<Option<OperationPrompt>>,
    mut prompt_sequence: Signal<u64>,
    navigator: Navigator,
    recovery: PolarLabRecovery,
) {
    let next_id = *prompt_sequence.peek() + 1;
    prompt_sequence.set(next_id);
    setup_profile.set(recovery.profile);
    lab_state.set(recovery.lab_state);
    operation_prompt.set(Some(OperationPrompt {
        operation_id: next_id,
        title: "Polar setup needs attention".to_string(),
        message: recovery.message,
        tone: ToastTone::Error,
        is_pending: false,
        can_cancel: false,
        cancel_requested: false,
    }));
    let _ = navigator.replace(Route::Home {});
}
