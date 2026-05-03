use dioxus::prelude::*;

use crate::client::models::{OperationFaqRow, TradeRoute};

#[component]
pub fn NetworkRouteVisual(route: TradeRoute) -> Element {
    rsx! {
        div { class: "network-route-visual",
            div { class: "node-block",
                span { class: "node-block__wallet" }
                strong { "{route.from_node.label()}" }
            }
            div { class: "channel-line",
                span { class: "channel-line__status", "{route.status.label()}" }
            }
            div { class: "node-block",
                span { class: "node-block__wallet" }
                strong { "{route.to_node.label()}" }
            }
        }
    }
}

#[component]
pub fn OperationFaqTable(rows: Vec<OperationFaqRow>) -> Element {
    rsx! {
        div { class: "faq-table", role: "table", aria_label: "Lightning operation block requirements",
            div { class: "faq-table__row faq-table__row--head", role: "row",
                span { role: "columnheader", "Operation" }
                span { role: "columnheader", "Needs Bitcoin node" }
                span { role: "columnheader", "Needs mined block" }
                span { role: "columnheader", "Why" }
            }
            for row in rows {
                div { class: "faq-table__row", role: "row",
                    span { role: "cell", "{row.operation}" }
                    span { role: "cell", {status_mark(row.needs_bitcoin_node)} }
                    span { role: "cell", {status_mark(row.needs_mined_block)} }
                    span { role: "cell", "{row.plain_explanation}" }
                }
            }
        }
    }
}

fn status_mark(value: bool) -> Element {
    if value {
        rsx! {
            span {
                class: "faq-table__status faq-table__status--yes",
                aria_label: "Yes",
                title: "Yes",
                "✓"
            }
        }
    } else {
        rsx! {
            span {
                class: "faq-table__status faq-table__status--no",
                aria_label: "No",
                title: "No",
                "×"
            }
        }
    }
}
