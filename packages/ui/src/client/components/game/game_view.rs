use dioxus::prelude::*;

use crate::client::models::{TraOwnershipStatus, TraTransferStatus};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameSide {
    Left,
    Right,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GameAnimation {
    None,
    PaymentLeftToRight,
    PaymentRightToLeft,
    ItemLeftToRight,
    ItemRightToLeft,
}

impl Default for GameAnimation {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GameChannelAnimation {
    None,
    PendingFadeIn,
    PendingToActive,
    ActiveToPending,
    PendingFadeOut,
}

impl Default for GameChannelAnimation {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GameChannelVisual {
    None,
    Pending,
    Active,
}

impl Default for GameChannelVisual {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GameInventorySlot {
    Empty,
    Item {
        tra_id: String,
        unique_name: String,
        item_id: u32,
        display_name: String,
        visual_key: String,
        ownership_status: TraOwnershipStatus,
        transfer_status: TraTransferStatus,
    },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GameViewConfig {
    left_bg: Option<Asset>,
    right_bg: Option<Asset>,
    left_character: Option<Asset>,
    right_character: Option<Asset>,
    left_purse: Option<Asset>,
    right_purse: Option<Asset>,
    left_name: Option<String>,
    right_name: Option<String>,
    left_sats: Option<u64>,
    right_sats: Option<u64>,
    pending_channel: Option<Asset>,
    active_channel: Option<Asset>,
    channel_visual: GameChannelVisual,
    channel_animation: GameChannelAnimation,
    left_inventory: Vec<GameInventorySlot>,
    right_inventory: Vec<GameInventorySlot>,
    animation: GameAnimation,
}

impl GameViewConfig {
    pub fn show_bg(mut self, side: GameSide, src: Asset) -> Self {
        match side {
            GameSide::Left => self.left_bg = Some(src),
            GameSide::Right => self.right_bg = Some(src),
        }
        self
    }

    pub fn show_character(mut self, side: GameSide, src: Asset) -> Self {
        match side {
            GameSide::Left => self.left_character = Some(src),
            GameSide::Right => self.right_character = Some(src),
        }
        self
    }

    pub fn show_purse(mut self, side: GameSide, src: Asset) -> Self {
        match side {
            GameSide::Left => self.left_purse = Some(src),
            GameSide::Right => self.right_purse = Some(src),
        }
        self
    }

    pub fn show_name(mut self, side: GameSide, name: impl Into<String>) -> Self {
        match side {
            GameSide::Left => self.left_name = Some(name.into()),
            GameSide::Right => self.right_name = Some(name.into()),
        }
        self
    }

    pub fn show_sats(mut self, side: GameSide, sats: u64) -> Self {
        match side {
            GameSide::Left => self.left_sats = Some(sats),
            GameSide::Right => self.right_sats = Some(sats),
        }
        self
    }

    pub fn show_channel(mut self, pending_src: Asset, active_src: Asset) -> Self {
        self.pending_channel = Some(pending_src);
        self.active_channel = Some(active_src);
        self
    }

    pub fn show_channel_visual(mut self, visual: GameChannelVisual) -> Self {
        self.channel_visual = visual;
        self
    }

    pub fn show_channel_animation(mut self, animation: GameChannelAnimation) -> Self {
        self.channel_animation = animation;
        self
    }

    pub fn show_inventory(mut self, side: GameSide, slots: Vec<GameInventorySlot>) -> Self {
        match side {
            GameSide::Left => self.left_inventory = slots,
            GameSide::Right => self.right_inventory = slots,
        }
        self
    }

    pub fn show_animation(mut self, animation: GameAnimation) -> Self {
        self.animation = animation;
        self
    }
}

#[component]
pub fn GameView(
    config: GameViewConfig,
    is_busy: bool,
    can_open_trade: bool,
    can_close_trade: bool,
    can_wait_for_block: bool,
    can_buy_item: bool,
    can_sell_item: bool,
    next_block_height: u64,
    on_restart_game: EventHandler<()>,
    on_open_trade: EventHandler<()>,
    on_close_trade: EventHandler<()>,
    on_wait_for_block: EventHandler<()>,
    on_change_location: EventHandler<()>,
    on_buy_item: EventHandler<()>,
    on_sell_item: EventHandler<()>,
) -> Element {
    let animation_class = match config.animation {
        GameAnimation::None => "game-view__animation",
        GameAnimation::PaymentLeftToRight => "game-view__animation game-view__animation--payment",
        GameAnimation::PaymentRightToLeft => {
            "game-view__animation game-view__animation--payment-return"
        }
        GameAnimation::ItemLeftToRight => "game-view__animation game-view__animation--item-return",
        GameAnimation::ItemRightToLeft => "game-view__animation game-view__animation--item",
    };
    let channel_class = match config.channel_animation {
        GameChannelAnimation::None => match config.channel_visual {
            GameChannelVisual::None => "game-view__layer game-view__channel",
            GameChannelVisual::Pending => {
                "game-view__layer game-view__channel game-view__channel--pending"
            }
            GameChannelVisual::Active => {
                "game-view__layer game-view__channel game-view__channel--active"
            }
        },
        GameChannelAnimation::PendingFadeIn => {
            "game-view__layer game-view__channel game-view__channel--pending-fade-in"
        }
        GameChannelAnimation::PendingToActive => {
            "game-view__layer game-view__channel game-view__channel--pending-to-active"
        }
        GameChannelAnimation::ActiveToPending => {
            "game-view__layer game-view__channel game-view__channel--active-to-pending"
        }
        GameChannelAnimation::PendingFadeOut => {
            "game-view__layer game-view__channel game-view__channel--pending-fade-out"
        }
    };

    rsx! {
        section { class: "game-view", aria_label: "Lightning trade game view",
            div { class: "game-view__stage",
                CrossFadeImage {
                    src: config.left_bg.clone(),
                    class: "game-view__layer game-view__bg game-view__bg--left".to_string(),
                    image_class: "game-view__bg-image".to_string(),
                    alt: "".to_string(),
                }
                CrossFadeImage {
                    src: config.right_bg.clone(),
                    class: "game-view__layer game-view__bg game-view__bg--right".to_string(),
                    image_class: "game-view__bg-image".to_string(),
                    alt: "".to_string(),
                }
                if let (Some(pending_src), Some(active_src)) = (
                    config.pending_channel.clone(),
                    config.active_channel.clone(),
                ) {
                    div { class: channel_class, aria_label: "Lightning channel",
                        img {
                            class: "game-view__channel-layer game-view__channel-layer--pending",
                            src: pending_src,
                            alt: ""
                        }
                        img {
                            class: "game-view__channel-layer game-view__channel-layer--active",
                            src: active_src,
                            alt: ""
                        }
                    }
                }
                div { class: animation_class, aria_hidden: "true",
                    span { class: "game-view__animation-icon game-view__animation-icon--payment", "sats" }
                    span { class: "game-view__animation-icon game-view__animation-icon--item", "book" }
                }
                GameActor {
                    side: GameSide::Left,
                    name: config.left_name.clone().unwrap_or_else(|| "Player".to_string()),
                    sats: config.left_sats.unwrap_or(0),
                    character_src: config.left_character.clone(),
                    purse_src: config.left_purse.clone(),
                    inventory: config.left_inventory.clone(),
                }
                GameActor {
                    side: GameSide::Right,
                    name: config.right_name.clone().unwrap_or_else(|| "NPC".to_string()),
                    sats: config.right_sats.unwrap_or(0),
                    character_src: config.right_character.clone(),
                    purse_src: config.right_purse.clone(),
                    inventory: config.right_inventory.clone(),
                }
            }
            div { class: "game-view__actions",
                div { class: "game-view__actions-row game-view__actions-row--top",
                    div { class: "game-view__action-group game-view__action-group--game",
                        span { class: "game-view__action-label", "Game" }
                        div { class: "game-view__action-controls",
                            button {
                                class: "primary-action",
                                r#type: "button",
                                disabled: is_busy,
                                onclick: move |_| on_restart_game.call(()),
                                "Restart Game"
                            }
                            button {
                                class: "secondary-action",
                                r#type: "button",
                                disabled: is_busy,
                                onclick: move |_| on_change_location.call(()),
                                "Change Location"
                            }
                        }
                    }
                    div { class: "game-view__action-group game-view__action-group--requires-wait",
                        span { class: "game-view__action-label", "Requires" br {} "Wait" }
                        div { class: "game-view__action-controls",
                            button {
                                class: "secondary-action",
                                r#type: "button",
                                disabled: is_busy || !can_open_trade,
                                onclick: move |_| on_open_trade.call(()),
                                if is_busy && can_open_trade {
                                    "Opening..."
                                } else {
                                    "Open Trade Route"
                                }
                            }
                            button {
                                class: "secondary-action",
                                r#type: "button",
                                disabled: is_busy || !can_close_trade,
                                onclick: move |_| on_close_trade.call(()),
                                if is_busy && can_close_trade {
                                    "Closing..."
                                } else {
                                    "Close Trade Route"
                                }
                            }
                            button {
                                class: "secondary-action",
                                r#type: "button",
                                disabled: is_busy || !can_wait_for_block,
                                onclick: move |_| on_wait_for_block.call(()),
                                "Wait for Block {next_block_height}"
                            }
                        }
                    }
                }
                div { class: "game-view__actions-row game-view__actions-row--bottom",
                    div { class: "game-view__action-group game-view__action-group--requires-trade",
                        span { class: "game-view__action-label", "Requires" br {} "Trade Route" }
                        div { class: "game-view__action-controls",
                            button {
                                class: "secondary-action",
                                r#type: "button",
                                disabled: is_busy || !can_buy_item,
                                onclick: move |_| on_buy_item.call(()),
                                if is_busy && can_buy_item {
                                    "Buying..."
                                } else {
                                    "Buy Item"
                                }
                            }
                            button {
                                class: "secondary-action",
                                r#type: "button",
                                disabled: is_busy || !can_sell_item,
                                onclick: move |_| on_sell_item.call(()),
                                if is_busy && can_sell_item {
                                    "Selling..."
                                } else {
                                    "Sell Item"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn GameActor(
    side: GameSide,
    name: String,
    sats: u64,
    character_src: Option<Asset>,
    purse_src: Option<Asset>,
    inventory: Vec<GameInventorySlot>,
) -> Element {
    let actor_class = match side {
        GameSide::Left => "game-view__actor game-view__actor--left",
        GameSide::Right => "game-view__actor game-view__actor--right",
    };
    let body_class = match side {
        GameSide::Left => "game-view__body game-view__body--left",
        GameSide::Right => "game-view__body game-view__body--right",
    };
    let player_ui_class = match side {
        GameSide::Left => "game-view__player-ui game-view__player-ui--left",
        GameSide::Right => "game-view__player-ui game-view__player-ui--right",
    };
    let purse_class = match side {
        GameSide::Left => "game-view__purse game-view__purse--left",
        GameSide::Right => "game-view__purse game-view__purse--right",
    };

    rsx! {
        div { class: actor_class,
            div { class: body_class,
                if character_src.is_some() {
                    CrossFadeImage {
                        src: character_src,
                        class: "game-view__character".to_string(),
                        image_class: "game-view__character-image".to_string(),
                        alt: name.clone(),
                    }
                } else {
                    div { class: "game-view__character game-view__character--fallback", "{name}" }
                }
                if let Some(src) = purse_src {
                    img { class: purse_class, src, alt: "{name} wallet" }
                }
            }
            div { class: player_ui_class,
                div { class: "game-view__identity",
                    div { class: "game-view__name", "{name}" }
                    div { class: "game-view__sats", "{sats} sats" }
                }
                div { class: "game-view__inventory", aria_label: "{name} inventory",
                    for slot in normalized_inventory(inventory) {
                        InventorySlot { slot }
                    }
                }
            }
        }
    }
}

#[component]
fn InventorySlot(slot: GameInventorySlot) -> Element {
    let slot_class = inventory_slot_class(&slot).to_string();

    rsx! {
        button {
            class: slot_class,
            r#type: "button",
            aria_disabled: "true",
            tabindex: "-1",
            match slot {
                GameInventorySlot::Empty => rsx! { span { class: "game-view__empty-slot", "Empty" } },
                GameInventorySlot::Item {
                    tra_id: _,
                    unique_name: _,
                    item_id: _,
                    display_name,
                    visual_key,
                    ownership_status: _,
                    transfer_status: _,
                } => {
                    let image_src = inventory_item_asset(&visual_key);
                    rsx! {
                        img {
                            class: "game-view__book",
                            src: image_src,
                            alt: "{display_name}"
                        }
                        span { class: "game-view__item-name", "{display_name}" }
                    }
                },
            }
        }
    }
}

#[component]
fn CrossFadeImage(src: Option<Asset>, class: String, image_class: String, alt: String) -> Element {
    let mut current_src = use_signal(|| None::<Asset>);
    let mut previous_src = use_signal(|| None::<Asset>);
    let mut is_transitioning = use_signal(|| false);

    use_effect(use_reactive((&src,), move |(next_src,)| {
        let current = current_src.peek().clone();

        if next_src != current {
            previous_src.set(current);
            current_src.set(next_src);
            is_transitioning.set(true);

            spawn(async move {
                wait_for_crossfade().await;
                previous_src.set(None);
                is_transitioning.set(false);
            });
        }
    }));

    let previous = previous_src();
    let current = current_src();
    let current_class = if is_transitioning() {
        format!("{image_class} game-view__crossfade-layer game-view__crossfade-layer--in")
    } else {
        format!("{image_class} game-view__crossfade-layer")
    };
    let previous_class =
        format!("{image_class} game-view__crossfade-layer game-view__crossfade-layer--out");

    rsx! {
        div { class,
            if let Some(src) = previous {
                img {
                    class: previous_class.clone(),
                    src,
                    alt: "",
                    aria_hidden: "true",
                }
            }
            if let Some(src) = current {
                img {
                    class: current_class.clone(),
                    src,
                    alt,
                }
            }
        }
    }
}

fn normalized_inventory(mut inventory: Vec<GameInventorySlot>) -> Vec<GameInventorySlot> {
    inventory.truncate(3);
    while inventory.len() < 3 {
        inventory.push(GameInventorySlot::Empty);
    }
    inventory
}

fn inventory_slot_class(slot: &GameInventorySlot) -> &'static str {
    match slot {
        GameInventorySlot::Empty => "game-view__slot game-view__slot--empty",
        GameInventorySlot::Item { .. } => "game-view__slot game-view__slot--book",
    }
}

const GAME_BOOK: Asset = asset!("/assets/images/game/book.png");
const GAME_APPLE: Asset = asset!("/assets/images/game/apple.png");

fn inventory_item_asset(visual_key: &str) -> Asset {
    match visual_key {
        "book" => GAME_BOOK,
        "apple" => GAME_APPLE,
        _ => GAME_BOOK,
    }
}

#[cfg(target_arch = "wasm32")]
async fn wait_for_crossfade() {
    gloo_timers::future::TimeoutFuture::new(450).await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn wait_for_crossfade() {
    futures_timer::Delay::new(std::time::Duration::from_millis(450)).await;
}
