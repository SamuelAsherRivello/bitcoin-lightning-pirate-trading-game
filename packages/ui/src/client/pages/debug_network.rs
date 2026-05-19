use dioxus::prelude::dioxus_router::Navigator;
use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::client::components::game::{HistoryItems, LabStatusWidget};
use crate::client::components::toast::{OperationPrompt, ToastTone};
use crate::client::models::{DemoNodeId, LabState, SetupProfile};
use crate::client::services::lightning_server_functions::{
    get_lab_state_or_recover, PolarLabRecovery,
};
use crate::client::Route;

#[component]
pub fn DebugNetwork() -> Element {
    let active_route = use_route::<Route>();
    let setup_profile = use_context::<Signal<SetupProfile>>();
    let mut lab_state = use_context::<Signal<Option<LabState>>>();
    let operation_prompt = use_context::<Signal<Option<OperationPrompt>>>();
    let prompt_sequence = use_signal(|| 60_000_u64);
    let navigator = navigator();

    use_effect(move || {
        let profile = setup_profile();
        if active_route == (Route::DebugNetwork {}) && profile.is_connected() {
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
                title: t!("debug-network-title"),
                detail: "Complete Set Up before the network dashboard opens.".to_string(),
            }
        };
    }

    let Some(state) = lab_state() else {
        return rsx! {
            main { class: "page-content lab-page",
                section { class: "lab-hero",
                    div {
                        span { class: "eyebrow", "Loading" }
                        h1 { {t!("debug-network-title")} }
                        p { "Loading the local network view..." }
                    }
                }
            }
        };
    };

    rsx! {
        main { class: "page-content lab-page debug-page",
            section { class: "lab-hero",
                div {
                    span { class: "eyebrow", "Network mechanics" }
                    h1 { {t!("debug-network-title")} }
                    p {
                        "Inspect nodes, trade routes, TRA ownership, invoices, payments, balances, and which operations need Bitcoin blocks."
                    }
                }
                LabStatusWidget {
                    sats_per_transaction: state.profile.sats_per_transaction,
                    block_height: state.block_height,
                }
            }

            section { class: "lab-panel",
                div { class: "section-heading",
                    span { class: "eyebrow", "Polar network" }
                    h2 { "Nodes" }
                }
                div { class: "tra-table node-table", role: "table", aria_label: "Network node rows",
                    div { class: "tra-table__row node-table__row tra-table__row--head", role: "row",
                        span { role: "columnheader", "Node" }
                        span { role: "columnheader", "Layer" }
                        span { role: "columnheader", "Purpose" }
                        span { role: "columnheader", "Status" }
                        span { role: "columnheader", "Balances" }
                        span { role: "columnheader", "Details" }
                    }
                    div { class: "tra-table__row node-table__row", role: "row",
                        span {
                            role: "cell",
                            title: "{state.profile.polar_automation.bitcoin_backend_name}",
                            "{state.profile.polar_automation.bitcoin_backend_name}"
                        }
                        span { role: "cell", "Bitcoin" }
                        span { role: "cell", "Regtest backend" }
                        span { role: "cell", "Started" }
                        span { role: "cell", "Block {state.block_height}" }
                        span { role: "cell", "Mines funding and channel-confirmation blocks" }
                    }
                    div { class: "tra-table__row node-table__row", role: "row",
                        span { role: "cell", title: "{state.game_treasury.node_label}", "{state.game_treasury.node_label}" }
                        span { role: "cell", "Lightning" }
                        span { role: "cell", "Game treasury" }
                        span { role: "cell", "{state.game_treasury.status.label()}" }
                        span { role: "cell", "{state.game_treasury.spendable_sats} sats" }
                        span { role: "cell", "Inventory value: {state.game_treasury.inventory_value_sats} sats" }
                    }
                    for node in state.nodes.clone() {
                        div { class: "tra-table__row node-table__row", role: "row",
                            span { role: "cell", title: "{node.alias}", "{node.alias}" }
                            span { role: "cell", "Lightning" }
                            span { role: "cell", "{node.role.label()}" }
                            span { role: "cell", "{node.status.label()}" }
                            span { role: "cell", "{node.wallet_balance_sats} sats" }
                            span { role: "cell", "Channel balance: {node.channel_balance_sats} sats" }
                        }
                    }
                    div { class: "tra-table__row node-table__row", role: "row",
                        span { role: "cell", "Taproot Assets" }
                        span { role: "cell", "Taproot" }
                        span { role: "cell", "TRA inventory" }
                        span { role: "cell", "{taproot_node_status(state.tra_items.len())}" }
                        span { role: "cell", "{state.tra_items.len()} instances" }
                        span { role: "cell", "{taproot_owner_summary(&state)}" }
                    }
                }
            }

            section { class: "lab-panel",
                div { class: "section-heading",
                    span { class: "eyebrow", "Taproots assets v0.7.0-alpha" }
                    h2 { "TRA instances" }
                }
                if state.tra_items.is_empty() {
                    p { class: "muted-copy", "TRA instances created by Game Treasury (TRAs) will appear here with GAME_TREASURY as the owner." }
                } else {
                    p { class: "muted-copy",
                        "After setup step 3, treasury inventory should show as concrete TRA instances owned by GAME_TREASURY. NPC transfers move these rows from GAME_TREASURY to Bob and Carol."
                    }
                    div { class: "tra-table", role: "table", aria_label: "Tap Root Assets inventory rows",
                        div { class: "tra-table__row tra-table__row--head", role: "row",
                            span { role: "columnheader", "TRA ID" }
                            span { role: "columnheader", "Asset ID" }
                            span { role: "columnheader", "Name" }
                            span { role: "columnheader", "Item ID" }
                            span { role: "columnheader", "Catalog" }
                            span { role: "columnheader", "Owner" }
                            span { role: "columnheader", "Ownership" }
                            span { role: "columnheader", "Transfer" }
                        }
                        for item in state.tra_items.clone() {
                            div { class: "tra-table__row", role: "row",
                                span { role: "cell", title: "{item.tra_id}", "{item.tra_id}" }
                                span { role: "cell", title: "{item.asset_id}", "{item.asset_id}" }
                                span { role: "cell", title: "{item.unique_name}", "{item.unique_name}" }
                                span { role: "cell", "{item.item_id}" }
                                span { role: "cell", "{tra_catalog_detail(item.item_id)}" }
                                span { role: "cell", "{item.owner_node.label()}" }
                                span { role: "cell", "{item.ownership_status.label()}" }
                                span { role: "cell", "{item.transfer_status.label()}" }
                            }
                        }
                    }
                }
            }

            section { class: "lab-grid lab-grid--two",
                article { class: "lab-panel",
                    div { class: "section-heading",
                        span { class: "eyebrow", "Invoices" }
                        h2 { "Recent invoices" }
                    }
                    if state.recent_invoices.is_empty() {
                        p { class: "muted-copy", "Invoices created by the app will appear here." }
                    } else {
                        div { class: "record-list",
                            for invoice in state.recent_invoices.clone() {
                                div { class: "record-row",
                                    strong { "{invoice.invoice_id}" }
                                    span { "{invoice.creator_node.label()} requested {invoice.amount_sats} sats" }
                                    span { "{invoice.status.label()}" }
                                }
                            }
                        }
                    }
                }
                article { class: "lab-panel",
                    div { class: "section-heading",
                        span { class: "eyebrow", "Payments" }
                        h2 { "Recent payments" }
                    }
                    if state.recent_payments.is_empty() {
                        p { class: "muted-copy", "Payments generated by the app will appear here." }
                    } else {
                        div { class: "record-list",
                            for payment in state.recent_payments.clone() {
                                div { class: "record-row",
                                    strong { "{payment.payment_id}" }
                                    span { "{payment.payer_node.label()} paid {payment.payee_node.label()} {payment.amount_sats} sats" }
                                    span { "{payment.status.label()} - block required: {yes_no(payment.requires_block)}" }
                                }
                            }
                        }
                    }
                }
            }

            HistoryItems { entries: state.action_log.clone() }

        }
    }
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

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn taproot_node_status(tra_count: usize) -> &'static str {
    if tra_count == 0 {
        "Waiting for inventory"
    } else {
        "Inventory indexed"
    }
}

fn taproot_owner_summary(state: &LabState) -> String {
    let treasury_items = state
        .tra_items
        .iter()
        .filter(|item| item.owner_node == DemoNodeId::GameTreasury)
        .count();
    let user_items = state.tra_items.len().saturating_sub(treasury_items);

    format!("Treasury: {treasury_items} / Users: {user_items}")
}

fn tra_catalog_detail(item_id: u32) -> String {
    lightning_service::TraService::catalog_item(item_id)
        .map(|item| {
            format!(
                "{} / {} / {} sats",
                item.display_name, item.item_type, item.cost_sats
            )
        })
        .unwrap_or_else(|| "Unsupported item_id".to_string())
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
