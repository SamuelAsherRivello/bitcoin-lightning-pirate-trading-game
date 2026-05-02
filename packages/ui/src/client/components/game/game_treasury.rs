use dioxus::prelude::*;

use crate::client::models::GameTreasury;

#[component]
pub fn GameTreasuryPanel(treasury: GameTreasury) -> Element {
    rsx! {
        section { class: "lab-panel treasury-panel",
            div { class: "section-heading",
                div {
                    span { class: "eyebrow", "Game Treasury" }
                    h2 { "House bank" }
                }
                span { class: "status-pill", "{treasury.status.label()}" }
            }
            div { class: "route-metrics",
                span { "Spendable: {treasury.spendable_sats} sats" }
                span { "Inventory value: {treasury.inventory_value_sats} sats" }
                span { "Recent entries: {treasury.recent_entries.len()}" }
            }
            if treasury.recent_entries.is_empty() {
                p { class: "muted-copy", "Treasury funding, item distribution, rewards, costs, and trades will appear here." }
            } else {
                div { class: "history-items",
                    for entry in treasury.recent_entries {
                        article { class: "history-item",
                            div { class: "history-item__copy",
                                strong { "{entry.related_action}" }
                                p { "{entry.description}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
