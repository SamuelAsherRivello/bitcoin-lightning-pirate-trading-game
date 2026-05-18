use dioxus::prelude::dioxus_router::Navigator;
use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::client::components::game::LabStatusWidget;
use crate::client::components::network::NetworkRouteVisual;
use crate::client::components::toast::{OperationPrompt, Toast, ToastTone};
use crate::client::models::{DemoNodeId, LabState, RouteStatus, SetupProfile};
use crate::client::services::lightning_server_functions::{
    create_invoice_and_maybe_autosend, get_lab_state_or_recover, open_trade_route,
    recover_if_polar_lab_unhealthy, wait_for_next_block, PolarLabRecovery,
};
use crate::client::Route;

#[component]
pub fn DebugNetwork() -> Element {
    let active_route = use_route::<Route>();
    let setup_profile = use_context::<Signal<SetupProfile>>();
    let mut lab_state = use_context::<Signal<Option<LabState>>>();
    let toast = use_context::<Signal<Option<Toast>>>();
    let operation_prompt = use_context::<Signal<Option<OperationPrompt>>>();
    let toast_sequence = use_signal(|| 40_000_u64);
    let prompt_sequence = use_signal(|| 60_000_u64);
    let mut is_busy = use_signal(|| false);
    let mut autosend_enabled = use_signal(|| true);
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
                    span { class: "eyebrow", "Polar nodes" }
                    h2 { "Lightning nodes" }
                }
                div { class: "record-list",
                    for node in state.nodes.clone() {
                        div { class: "record-row",
                            strong { "{node.alias}" }
                            span { "{node.role.label()} / {node.status.label()}" }
                            span { "Wallet: {node.wallet_balance_sats} sats" }
                            span { "Channel: {node.channel_balance_sats} sats" }
                        }
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

            section { class: "lab-panel",
                div { class: "section-heading section-heading--row",
                    div {
                        span { class: "eyebrow", "Channels" }
                        h2 { "Trade route rows" }
                    }
                    button {
                        class: if autosend_enabled() { "segment segment--active" } else { "segment" },
                        r#type: "button",
                        onclick: move |_| autosend_enabled.set(!autosend_enabled()),
                        if autosend_enabled() {
                            "AutoSend On"
                        } else {
                            "AutoSend Off"
                        }
                    }
                }
                p { class: "muted-copy",
                    "AutoSend is a lab/demo feature. It uses the configured sats-per-transaction amount as the maximum per automatic payment."
                }
                div { class: "network-route-list",
                    for route in state.trade_routes.clone() {
                        article { class: "network-route-card",
                            NetworkRouteVisual { route: route.clone() }
                            div { class: "route-metrics",
                                span { "Capacity: {route.capacity_sats} sats" }
                                span { "Local: {route.local_balance_sats} sats" }
                                span { "Remote: {route.remote_balance_sats} sats" }
                                span {
                                    if route.requires_next_block {
                                        "Block required: yes"
                                    } else {
                                        "Block required: no"
                                    }
                                }
                            }
                            div { class: "button-row",
                                if route.status == RouteStatus::Missing {
                                    button {
                                        class: "primary-action",
                                        r#type: "button",
                                        disabled: is_busy(),
                                        onclick: move |_| {
                                            let to_node = route.to_node;
                                            async move {
                                                is_busy.set(true);
                                                match open_trade_route(setup_profile(), to_node).await {
                                                    Ok(next_state) => {
                                                        lab_state.set(Some(next_state));
                                                        push_toast(toast, toast_sequence, "Trade route is under construction.", ToastTone::Success);
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
                                            }
                                        },
                                        "Open Trade Route"
                                    }
                                } else if route.requires_next_block {
                                    button {
                                        class: "primary-action",
                                        r#type: "button",
                                        disabled: is_busy(),
                                        onclick: move |_| {
                                            let route_id = route.route_id.clone();
                                            async move {
                                                is_busy.set(true);
                                                match wait_for_next_block(setup_profile(), Some(route_id)).await {
                                                    Ok(next_state) => {
                                                        lab_state.set(Some(next_state));
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
                                            }
                                        },
                                        "Wait for Block {state.block_height.saturating_add(1)}"
                                    }
                                } else if route.status == RouteStatus::Active {
                                    button {
                                        class: "primary-action",
                                        r#type: "button",
                                        disabled: is_busy(),
                                        onclick: move |_| {
                                            let merchant = route.to_node;
                                            let autosend = autosend_enabled();
                                            async move {
                                                is_busy.set(true);
                                                let memo = format!("{} creates an AutoSend lab invoice", merchant.label());
                                                match create_invoice_and_maybe_autosend(
                                                    setup_profile(),
                                                    merchant,
                                                    DemoNodeId::Alice,
                                                    autosend,
                                                    memo,
                                                )
                                                .await
                                                {
                                                    Ok(next_state) => {
                                                        lab_state.set(Some(next_state));
                                                        push_toast(toast, toast_sequence, autosend_result_message(autosend), ToastTone::Success);
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
                                            }
                                        },
                                        if autosend_enabled() {
                                            "Create Invoice + AutoSend"
                                        } else {
                                            "Create Invoice"
                                        }
                                    }
                                } else {
                                    button {
                                        class: "secondary-action",
                                        r#type: "button",
                                        disabled: true,
                                        "Waiting on route status"
                                    }
                                }
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

fn autosend_result_message(autosend: bool) -> &'static str {
    if autosend {
        "AutoSend flow complete."
    } else {
        "Invoice created."
    }
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
