use dioxus::prelude::*;

use crate::client::components::help::FieldHelpIcon;
use crate::client::models::{ActionLogEntry, TradeRoute};

mod game_treasury;
mod game_view;

pub use game_treasury::GameTreasuryPanel;
pub use game_view::{
    GameAnimation, GameChannelAnimation, GameChannelVisual, GameInventorySlot, GameSide, GameView,
    GameViewConfig,
};

#[component]
pub fn LabStatusWidget(sats_per_transaction: u64, block_height: u64) -> Element {
    rsx! {
        div { class: "status-card",
            div { class: "status-card__metric",
                span { class: "eyebrow", "Trade Route amount" }
                div { class: "status-card__value",
                    strong { "{sats_per_transaction} sats" }
                    FieldHelpIcon { label: "Every purchase in this POC uses the configured demo amount.".to_string() }
                }
            }
            div { class: "status-card__metric",
                span { class: "eyebrow", "Block Height" }
                div { class: "status-card__value",
                    strong { "{block_height}" }
                    FieldHelpIcon { label: "Current Bitcoin regtest block height reported by the connected Polar lab.".to_string() }
                }
            }
        }
    }
}

#[component]
pub fn HistoryItems(entries: Vec<ActionLogEntry>) -> Element {
    rsx! {
        section { class: "lab-panel",
            div { class: "section-heading section-heading--history",
                div {
                    span { class: "eyebrow", "Game log" }
                    h2 { "Recent actions" }
                }
                span { class: "history-details-heading", "Details" }
            }
            if entries.is_empty() {
                p { class: "muted-copy", "Game actions will appear here after setup, trade opening, block waits, invoice creation, payments, and trade closing." }
            } else {
                div { class: "history-items",
                    for entry in entries {
                        HistoryItem { entry: entry }
                    }
                }
            }
        }
    }
}

#[component]
fn HistoryItem(entry: ActionLogEntry) -> Element {
    let summary = entry.summary;
    let network_detail = entry.network_detail;
    let details = entry.details;
    let has_details = !details.is_empty();

    rsx! {
        article { class: "history-item",
            div { class: "history-item__copy",
                strong { "{summary}" }
                p { "{network_detail}" }
            }
            if has_details {
                div { class: "history-item__details", aria_label: "Details",
                    for (index, detail) in details.into_iter().enumerate() {
                        if index > 0 {
                            RightArrowIcon {}
                        }
                        HistoryItemDetail { label: detail }
                    }
                }
            }
        }
    }
}

#[component]
fn HistoryItemDetail(label: String) -> Element {
    rsx! {
        span { class: "history-item-detail", "{label}" }
    }
}

#[component]
fn RightArrowIcon() -> Element {
    rsx! {
        span { class: "history-item-detail-arrow", "aria-hidden": "true",
            svg {
                width: "14",
                height: "14",
                view_box: "0 0 24 24",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "2",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                path { d: "M5 12h14" }
                path { d: "m12 5 7 7-7 7" }
            }
        }
    }
}

#[component]
pub fn RouteSummary(route: TradeRoute) -> Element {
    rsx! {
        div { class: "route-summary",
            div {
                span { class: "eyebrow", "Lightning trade" }
                strong { "Player to {route.to_node.label()}" }
            }
            span { class: "status-pill", "{route.status.label()}" }
        }
    }
}
