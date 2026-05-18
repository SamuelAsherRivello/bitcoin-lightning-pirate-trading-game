use dioxus::prelude::dioxus_router::Navigator;
use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::client::components::game::{
    GameAnimation, GameChannelAnimation, GameChannelVisual, GameInventorySlot, GameSide,
    GameTreasuryPanel, GameView, GameViewConfig, HistoryItems, LabStatusWidget, RouteSummary,
};
use crate::client::components::toast::{
    wait_for_prompt_message_minimum, OperationPrompt, Toast, ToastTone,
};
use crate::client::models::{
    ConnectionStatus, DemoNode, DemoNodeId, GameItemDefinition, LabState, RouteStatus, SetupMode,
    SetupProfile, TraItem, TraOwnershipStatus, TraTransferStatus, TradeRoute, TransferTraRequest,
    DEFAULT_ROUTE_CAPACITY_SATS, DEFAULT_SATS_PER_TRANSACTION, MAX_TRA_ITEMS_PER_NODE,
};
use crate::client::services::lightning_server_functions::{
    close_polar_demo_channels_with_progress, close_trade_route, complete_polar_setup,
    create_invoice_and_maybe_autosend_for_amount, create_polar_demo_nodes_with_progress,
    destroy_polar_demo_nodes, ensure_polar_server, initial_tra_setup_items, mint_tra,
    open_trade_route, recover_if_polar_lab_unhealthy, reset_tra_inventory, transfer_tra,
    verify_polar_bridge, verify_tra_setup, wait_for_next_block, PolarLabRecovery,
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum PlayGameRefreshStatus {
    #[default]
    Idle,
    Refreshing,
    Refreshed,
    Failed,
}

impl PlayGameRefreshStatus {
    fn label(self) -> &'static str {
        match self {
            Self::Idle => "Waiting to refresh sats and inventory",
            Self::Refreshing => "Refreshing sats and TRA inventory...",
            Self::Refreshed => "Sats and TRA inventory refreshed",
            Self::Failed => "Refresh needs attention",
        }
    }
}

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
            DemoNodeId::Alice | DemoNodeId::GameTreasury => GAME_NPC,
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
    let mut play_game_refresh_status = use_signal(PlayGameRefreshStatus::default);
    let navigator = navigator();

    use_effect(move || {
        let profile = setup_profile();
        if active_route == (Route::PlayGame {}) && profile.is_connected() {
            play_game_refresh_status.set(PlayGameRefreshStatus::Refreshing);
            spawn(async move {
                match verify_tra_setup(profile.clone()).await {
                    Ok(state) => {
                        lab_state.set(Some(state));
                        play_game_refresh_status.set(PlayGameRefreshStatus::Refreshed);
                        push_toast(
                            toast,
                            toast_sequence,
                            "Refreshed sat balances and TRA inventory.",
                            ToastTone::Success,
                        );
                    }
                    Err(message) => {
                        play_game_refresh_status.set(PlayGameRefreshStatus::Failed);
                        handle_lab_action_error(
                            profile,
                            setup_profile,
                            lab_state,
                            toast,
                            toast_sequence,
                            operation_prompt,
                            prompt_sequence,
                            navigator,
                            message,
                        )
                        .await;
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
                        p { "Refreshing sat balances and TRA inventory from the local Lightning lab..." }
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
    let catalog = item_catalog();
    let player_items = tradable_items_for(&state, DemoNodeId::Alice, &catalog);
    let npc_items = tradable_items_for(&state, merchant, &catalog);
    let selected_npc_item = rightmost_transferable_item(&npc_items);
    let selected_player_item = rightmost_transferable_item(&player_items);
    let left_inventory = inventory_slots_for(&state, DemoNodeId::Alice, &catalog);
    let right_inventory = inventory_slots_for(&state, merchant, &catalog);
    let player_name = node_display_name(&state, DemoNodeId::Alice);
    let npc_name = node_display_name(&state, merchant);
    let (player_sats, npc_sats) = game_sats_for_nodes(&state, merchant);
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
    let can_buy_item = can_buy_item_from_current_npc(
        &state,
        focused_route.as_ref(),
        merchant,
        selected_npc_item.as_ref(),
    );
    let can_sell_item = can_sell_item_to_current_npc(
        &state,
        focused_route.as_ref(),
        merchant,
        selected_player_item.as_ref(),
    );
    let focused_route_for_wait = focused_route.clone();
    let focused_route_for_panel = focused_route.clone();
    let selected_npc_item_for_buy = selected_npc_item.clone();
    let selected_player_item_for_sell = selected_player_item.clone();
    let refresh_status = play_game_refresh_status();
    let is_route_refreshing = refresh_status == PlayGameRefreshStatus::Refreshing;

    rsx! {
        main { class: "page-content lab-page play-page",
            section { class: "lab-hero",
                div {
                    span { class: "eyebrow", "Player and NPC" }
                    h1 { {t!("play-game-title")} }
                    p {
                        "Open a Lightning trade with the NPC, wait for the next block when the channel needs confirmation, then buy books over the active channel."
                    }
                    span { class: "status-pill", "{refresh_status.label()}" }
                }
                LabStatusWidget {
                    sats_per_transaction: state.profile.sats_per_transaction,
                    block_height: state.block_height,
                }
            }

            GameView {
                config: game_view_config,
                is_busy: is_busy() || is_route_refreshing,
                can_open_trade: can_open_trade && !is_route_refreshing,
                can_close_trade: can_close_trade && !is_route_refreshing,
                can_wait_for_block: can_wait_for_block && !is_route_refreshing,
                can_buy_item: can_buy_item && !is_route_refreshing,
                can_sell_item: can_sell_item && !is_route_refreshing,
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
                    if !can_buy_item {
                        push_toast(
                            toast,
                            toast_sequence,
                            "Open an active trade route before buying an item.",
                            ToastTone::Error,
                        );
                        return;
                    }
                    let selected_item = selected_npc_item_for_buy.clone();
                    spawn(async move {
                        let Some(selected_item) = selected_item.clone() else {
                            push_toast(
                                toast,
                                toast_sequence,
                                "No verified TRA item is available to buy.",
                                ToastTone::Error,
                            );
                            return;
                        };
                        is_busy.set(true);
                        game_animation.set(GameAnimation::PaymentLeftToRight);
                        wait_for_game_animation().await;
                        let memo = format!(
                            "Player buys TRA item {} ({}) from {}",
                            selected_item.unique_name,
                            selected_item.tra_id,
                            merchant.label()
                        );
                        match create_invoice_and_maybe_autosend_for_amount(
                            setup_profile(),
                            merchant,
                            DemoNodeId::Alice,
                            selected_item.cost_sats,
                            true,
                            memo,
                        )
                        .await
                        {
                            Ok(_) => {
                                wait_between_trade_animations().await;
                                game_animation.set(GameAnimation::ItemRightToLeft);
                                wait_for_game_animation().await;
                                match transfer_tra(
                                    setup_profile(),
                                    TransferTraRequest {
                                        tra_id: selected_item.tra_id.clone(),
                                        from_node: merchant,
                                        to_node: DemoNodeId::Alice,
                                    },
                                )
                                .await
                                {
                                    Ok(next_state) => {
                                        lab_state.set(Some(next_state));
                                        push_toast(
                                            toast,
                                            toast_sequence,
                                            format!(
                                                "Bought {} and verified TRA ownership.",
                                                selected_item.unique_name
                                            ),
                                            ToastTone::Success,
                                        );
                                    }
                                    Err(message) => {
                                        push_toast(
                                            toast,
                                            toast_sequence,
                                            format!(
                                                "Payment succeeded, but TRA transfer needs recovery: {message}"
                                            ),
                                            ToastTone::Error,
                                        );
                                    }
                                }
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
                    if !can_sell_item {
                        push_toast(
                            toast,
                            toast_sequence,
                            "Open an active trade route before selling an item.",
                            ToastTone::Error,
                        );
                        return;
                    }
                    let selected_item = selected_player_item_for_sell.clone();
                    spawn(async move {
                        let Some(selected_item) = selected_item.clone() else {
                            push_toast(
                                toast,
                                toast_sequence,
                                "No verified player-owned TRA item is available to sell.",
                                ToastTone::Error,
                            );
                            return;
                        };
                        is_busy.set(true);
                        game_animation.set(GameAnimation::ItemLeftToRight);
                        wait_for_game_animation().await;
                        let memo = format!(
                            "Player sells TRA item {} ({}) to {}",
                            selected_item.unique_name,
                            selected_item.tra_id,
                            merchant.label()
                        );
                        match create_invoice_and_maybe_autosend_for_amount(
                            setup_profile(),
                            DemoNodeId::Alice,
                            merchant,
                            selected_item.cost_sats,
                            true,
                            memo,
                        )
                        .await
                        {
                            Ok(_) => {
                                game_animation.set(GameAnimation::PaymentRightToLeft);
                                wait_for_game_animation().await;
                                match transfer_tra(
                                    setup_profile(),
                                    TransferTraRequest {
                                        tra_id: selected_item.tra_id.clone(),
                                        from_node: DemoNodeId::Alice,
                                        to_node: merchant,
                                    },
                                )
                                .await
                                {
                                    Ok(next_state) => {
                                        lab_state.set(Some(next_state));
                                        push_toast(
                                            toast,
                                            toast_sequence,
                                            format!(
                                                "Sold {} and verified TRA ownership.",
                                                selected_item.unique_name
                                            ),
                                            ToastTone::Success,
                                        );
                                    }
                                    Err(message) => {
                                        push_toast(
                                            toast,
                                            toast_sequence,
                                            format!(
                                                "Payment succeeded, but TRA transfer needs recovery: {message}"
                                            ),
                                            ToastTone::Error,
                                        );
                                    }
                                }
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

            GameTreasuryPanel { treasury: state.game_treasury.clone() }

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

fn game_sats_for_nodes(state: &LabState, merchant: DemoNodeId) -> (u64, u64) {
    (
        available_node_sats(state, DemoNodeId::Alice),
        available_node_sats(state, merchant),
    )
}

fn available_node_sats(state: &LabState, node_id: DemoNodeId) -> u64 {
    state
        .nodes
        .iter()
        .find(|node| node.node_id == node_id)
        .map(|node| {
            node.wallet_balance_sats
                .saturating_add(node.channel_balance_sats)
        })
        .unwrap_or(0)
}

fn can_sell_item_to_current_npc(
    state: &LabState,
    focused_route: Option<&TradeRoute>,
    merchant: DemoNodeId,
    selected_player_item: Option<&TransferableItem>,
) -> bool {
    if !focused_route
        .map(|route| route.to_node == merchant && route.status == RouteStatus::Active)
        .unwrap_or(false)
    {
        return false;
    }

    selected_player_item
        .map(|item| {
            owner_item_count(state, merchant) < MAX_TRA_ITEMS_PER_NODE
                && available_node_sats(state, merchant) >= item.cost_sats
        })
        .unwrap_or(false)
}

fn can_buy_item_from_current_npc(
    state: &LabState,
    focused_route: Option<&TradeRoute>,
    merchant: DemoNodeId,
    selected_npc_item: Option<&TransferableItem>,
) -> bool {
    focused_route
        .map(|route| {
            route.to_node == merchant
                && route.status == RouteStatus::Active
                && selected_npc_item.is_some()
                && owner_item_count(state, DemoNodeId::Alice) < MAX_TRA_ITEMS_PER_NODE
                && route.local_balance_sats
                    >= selected_npc_item
                        .map(|item| item.cost_sats)
                        .unwrap_or(DEFAULT_SATS_PER_TRANSACTION)
        })
        .unwrap_or(false)
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

#[derive(Clone, Debug, PartialEq)]
struct TransferableItem {
    tra_id: String,
    unique_name: String,
    item_id: u32,
    owner_node: DemoNodeId,
    cost_sats: u64,
}

fn item_catalog() -> Vec<GameItemDefinition> {
    lightning_service::TraService::item_catalog()
}

fn catalog_item<'a>(
    catalog: &'a [GameItemDefinition],
    item_id: u32,
) -> Option<&'a GameItemDefinition> {
    catalog.iter().find(|item| item.item_id == item_id)
}

fn owner_item_count(state: &LabState, owner_node: DemoNodeId) -> usize {
    state
        .tra_items
        .iter()
        .filter(|item| item.owner_node == owner_node)
        .count()
}

fn tradable_items_for(
    state: &LabState,
    owner_node: DemoNodeId,
    catalog: &[GameItemDefinition],
) -> Vec<TransferableItem> {
    state
        .tra_items
        .iter()
        .filter(|item| item.owner_node == owner_node)
        .filter(|item| {
            item.ownership_status == TraOwnershipStatus::Verified
                && matches!(
                    item.transfer_status,
                    TraTransferStatus::None | TraTransferStatus::Succeeded
                )
        })
        .filter_map(|item| {
            catalog_item(catalog, item.item_id).map(|definition| TransferableItem {
                tra_id: item.tra_id.clone(),
                unique_name: item.unique_name.clone(),
                item_id: item.item_id,
                owner_node: item.owner_node,
                cost_sats: definition.cost_sats,
            })
        })
        .collect()
}

fn rightmost_transferable_item(items: &[TransferableItem]) -> Option<TransferableItem> {
    items.last().cloned()
}

fn inventory_slots_for(
    state: &LabState,
    owner_node: DemoNodeId,
    catalog: &[GameItemDefinition],
) -> Vec<GameInventorySlot> {
    let mut slots: Vec<GameInventorySlot> = state
        .tra_items
        .iter()
        .filter(|item| item.owner_node == owner_node)
        .take(MAX_TRA_ITEMS_PER_NODE)
        .map(|item| inventory_slot_for(item, catalog))
        .collect();

    while slots.len() < MAX_TRA_ITEMS_PER_NODE {
        slots.push(GameInventorySlot::Empty);
    }

    slots
}

fn inventory_slot_for(item: &TraItem, catalog: &[GameItemDefinition]) -> GameInventorySlot {
    let (display_name, visual_key) = catalog_item(catalog, item.item_id)
        .map(|definition| {
            (
                definition.display_name.clone(),
                definition.visual_key.clone(),
            )
        })
        .unwrap_or_else(|| ("Unsupported".to_string(), "unsupported".to_string()));

    GameInventorySlot::Item {
        tra_id: item.tra_id.clone(),
        unique_name: item.unique_name.clone(),
        item_id: item.item_id,
        display_name,
        visual_key,
        ownership_status: item.ownership_status,
        transfer_status: item.transfer_status,
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
        "Step 5 of 6: Adding Tap Root Assets...",
        ToastTone::Info,
        true,
        false,
    )
    .await;
    let mut state = reset_tra_inventory(state.profile.clone()).await?;
    state = verify_tra_setup(state.profile.clone()).await?;
    for request in initial_tra_setup_items() {
        state = mint_tra(state.profile.clone(), request).await?;
    }
    state = verify_tra_setup(state.profile.clone()).await?;

    update_operation_prompt(
        operation_prompt,
        operation_id,
        "Step 6 of 6: Unlocking Play Game and Network Dashboard...",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::models::{ConnectionStatus, SetupProfile, APPLE_ITEM_ID, BOOK_ITEM_ID};

    fn state_with_items(items: Vec<TraItem>) -> LabState {
        let mut profile = SetupProfile::default();
        profile.connection_status = ConnectionStatus::Connected;
        let mut state = lightning_service::default_lab_state(profile);
        state.tra_items = items;
        state
    }

    fn activate_route_to(state: &mut LabState, merchant: DemoNodeId) {
        let route = state
            .trade_routes
            .iter_mut()
            .find(|route| route.to_node == merchant)
            .expect("route to merchant");
        route.status = RouteStatus::Active;
        route.local_balance_sats = DEFAULT_ROUTE_CAPACITY_SATS;
        route.remote_balance_sats = DEFAULT_ROUTE_CAPACITY_SATS;
    }

    fn item(owner_node: DemoNodeId, unique_name: &str, status: TraOwnershipStatus) -> TraItem {
        TraItem {
            tra_id: format!("tra-{unique_name}"),
            asset_id: format!("asset-{unique_name}"),
            unique_name: unique_name.to_string(),
            item_id: BOOK_ITEM_ID,
            owner_node,
            ownership_status: status,
            transfer_status: TraTransferStatus::None,
        }
    }

    #[test]
    fn inventory_slots_derive_from_lab_state_tra_items() {
        let catalog = item_catalog();
        let state = state_with_items(vec![item(
            DemoNodeId::Bob,
            "Book",
            TraOwnershipStatus::Verified,
        )]);

        let slots = inventory_slots_for(&state, DemoNodeId::Bob, &catalog);

        assert!(matches!(
            &slots[0],
            GameInventorySlot::Item { tra_id, unique_name, .. }
                if tra_id == "tra-Book" && unique_name == "Book"
        ));
        assert!(matches!(slots[1], GameInventorySlot::Empty));
    }

    #[test]
    fn apple_inventory_slot_uses_catalog_display_name_and_visual_key() {
        let catalog = item_catalog();
        let mut apple = item(DemoNodeId::Carol, "Apple", TraOwnershipStatus::Verified);
        apple.item_id = APPLE_ITEM_ID;
        let state = state_with_items(vec![apple]);

        let slots = inventory_slots_for(&state, DemoNodeId::Carol, &catalog);

        assert!(matches!(
            &slots[0],
            GameInventorySlot::Item {
                display_name,
                visual_key,
                ..
            } if display_name == "Apple" && visual_key == "apple"
        ));
    }

    #[test]
    fn unsupported_items_are_not_transferable() {
        let catalog = item_catalog();
        let state = state_with_items(vec![item(
            DemoNodeId::Bob,
            "Broken Book",
            TraOwnershipStatus::Unsupported,
        )]);

        assert!(rightmost_transferable_item(&tradable_items_for(
            &state,
            DemoNodeId::Bob,
            &catalog
        ))
        .is_none());
    }

    #[test]
    fn selected_transfer_target_uses_rightmost_concrete_tra_id_and_catalog_price() {
        let catalog = item_catalog();
        let state = state_with_items(vec![
            item(DemoNodeId::Bob, "Book", TraOwnershipStatus::Verified),
            item(DemoNodeId::Bob, "Book 2", TraOwnershipStatus::Verified),
        ]);

        let selected =
            rightmost_transferable_item(&tradable_items_for(&state, DemoNodeId::Bob, &catalog))
                .expect("selected TRA item");

        assert_eq!(selected.tra_id, "tra-Book 2");
        assert_eq!(selected.item_id, BOOK_ITEM_ID);
        assert_eq!(selected.cost_sats, DEFAULT_SATS_PER_TRANSACTION);
        assert_eq!(selected.owner_node, DemoNodeId::Bob);
    }

    #[test]
    fn sell_is_available_to_current_npc_with_sats_and_empty_inventory_slot() {
        let catalog = item_catalog();
        let mut apple = item(DemoNodeId::Alice, "Apple", TraOwnershipStatus::Verified);
        apple.item_id = APPLE_ITEM_ID;
        let mut state = state_with_items(vec![
            apple,
            item(DemoNodeId::Bob, "Book", TraOwnershipStatus::Verified),
            item(DemoNodeId::Bob, "Book 2", TraOwnershipStatus::Verified),
        ]);
        activate_route_to(&mut state, DemoNodeId::Bob);
        let selected =
            rightmost_transferable_item(&tradable_items_for(&state, DemoNodeId::Alice, &catalog))
                .expect("player item");
        let focused_route = state
            .trade_routes
            .iter()
            .find(|route| route.to_node == DemoNodeId::Bob);

        assert!(can_sell_item_to_current_npc(
            &state,
            focused_route,
            DemoNodeId::Bob,
            Some(&selected)
        ));
    }

    #[test]
    fn sell_is_unavailable_when_current_npc_inventory_is_full() {
        let catalog = item_catalog();
        let mut apple = item(DemoNodeId::Alice, "Apple", TraOwnershipStatus::Verified);
        apple.item_id = APPLE_ITEM_ID;
        let mut state = state_with_items(vec![
            apple,
            item(DemoNodeId::Bob, "Book", TraOwnershipStatus::Verified),
            item(DemoNodeId::Bob, "Book 2", TraOwnershipStatus::Verified),
            item(DemoNodeId::Bob, "Book 3", TraOwnershipStatus::Verified),
        ]);
        activate_route_to(&mut state, DemoNodeId::Bob);
        let selected =
            rightmost_transferable_item(&tradable_items_for(&state, DemoNodeId::Alice, &catalog))
                .expect("player item");
        let focused_route = state
            .trade_routes
            .iter()
            .find(|route| route.to_node == DemoNodeId::Bob);

        assert!(!can_sell_item_to_current_npc(
            &state,
            focused_route,
            DemoNodeId::Bob,
            Some(&selected)
        ));
    }

    #[test]
    fn buy_and_sell_are_unavailable_without_active_trade_route() {
        let catalog = item_catalog();
        let mut apple = item(DemoNodeId::Alice, "Apple", TraOwnershipStatus::Verified);
        apple.item_id = APPLE_ITEM_ID;
        let state = state_with_items(vec![
            apple,
            item(DemoNodeId::Bob, "Book", TraOwnershipStatus::Verified),
        ]);
        let selected_npc_item =
            rightmost_transferable_item(&tradable_items_for(&state, DemoNodeId::Bob, &catalog))
                .expect("npc item");
        let selected_player_item =
            rightmost_transferable_item(&tradable_items_for(&state, DemoNodeId::Alice, &catalog))
                .expect("player item");
        let focused_route = state
            .trade_routes
            .iter()
            .find(|route| route.to_node == DemoNodeId::Bob);

        assert!(!can_buy_item_from_current_npc(
            &state,
            focused_route,
            DemoNodeId::Bob,
            Some(&selected_npc_item)
        ));
        assert!(!can_sell_item_to_current_npc(
            &state,
            focused_route,
            DemoNodeId::Bob,
            Some(&selected_player_item)
        ));
    }
}
