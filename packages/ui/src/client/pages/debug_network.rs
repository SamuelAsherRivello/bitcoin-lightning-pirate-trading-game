use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::client::components::network::NetworkRouteVisual;
use crate::client::components::toast::{Toast, ToastTone};
use crate::client::models::{DemoNodeId, LabState, RouteStatus, SetupProfile};
use crate::client::services::lightning_server_functions::{
    create_invoice, create_invoice_and_maybe_autosend, get_lab_state, pay_latest_invoice,
    wait_for_next_block,
};
use crate::client::Route;

#[component]
pub fn DebugNetwork() -> Element {
    let setup_profile = use_context::<Signal<SetupProfile>>();
    let mut lab_state = use_context::<Signal<Option<LabState>>>();
    let toast = use_context::<Signal<Option<Toast>>>();
    let toast_sequence = use_signal(|| 40_000_u64);
    let mut is_busy = use_signal(|| false);
    let mut autosend_enabled = use_signal(|| true);

    use_effect(move || {
        let profile = setup_profile();
        if profile.is_connected() && lab_state.peek().is_none() {
            spawn(async move {
                if let Ok(state) = get_lab_state(profile).await {
                    lab_state.set(Some(state));
                }
            });
        }
    });

    let profile = setup_profile();
    if !profile.is_connected() {
        return rsx! {
            LockedPage {
                title: t!("debug-network-title"),
                detail: "Complete Set Up before network debugging starts.".to_string(),
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
                        "Inspect nodes, trade routes, invoices, payments, balances, and which operations need Bitcoin blocks."
                    }
                }
                div { class: "status-card",
                    span { class: "eyebrow", "Block height" }
                    strong { "{state.block_height}" }
                    p { "Regtest can mine the next block instantly when a pending channel needs confirmation." }
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
                                button {
                                    class: "secondary-action",
                                    r#type: "button",
                                    disabled: is_busy() || !route.requires_next_block,
                                    onclick: move |_| {
                                        let route_id = route.route_id.clone();
                                        async move {
                                            is_busy.set(true);
                                            match wait_for_next_block(setup_profile(), Some(route_id)).await {
                                                Ok(next_state) => {
                                                    lab_state.set(Some(next_state));
                                                    push_toast(toast, toast_sequence, "Regtest mined the next block.", ToastTone::Success);
                                                }
                                                Err(message) => push_toast(toast, toast_sequence, message, ToastTone::Error),
                                            }
                                            is_busy.set(false);
                                        }
                                    },
                                    "Wait for Next Block"
                                }
                                button {
                                    class: "secondary-action",
                                    r#type: "button",
                                    disabled: is_busy() || route.status != RouteStatus::Active,
                                    onclick: move |_| {
                                        let merchant = route.to_node;
                                        async move {
                                            is_busy.set(true);
                                            let memo = format!("{} creates a debug invoice", merchant.label());
                                            match create_invoice(
                                                setup_profile(),
                                                merchant,
                                                Some(DemoNodeId::Alice),
                                                memo,
                                            )
                                            .await
                                            {
                                                Ok(next_state) => {
                                                    lab_state.set(Some(next_state));
                                                    push_toast(toast, toast_sequence, "Invoice created.", ToastTone::Success);
                                                }
                                                Err(message) => push_toast(toast, toast_sequence, message, ToastTone::Error),
                                            }
                                            is_busy.set(false);
                                        }
                                    },
                                    "Create Invoice"
                                }
                                button {
                                    class: "secondary-action",
                                    r#type: "button",
                                    disabled: is_busy() || route.status != RouteStatus::Active,
                                    onclick: move |_| async move {
                                        is_busy.set(true);
                                        match pay_latest_invoice(setup_profile(), DemoNodeId::Alice).await {
                                            Ok(next_state) => {
                                                lab_state.set(Some(next_state));
                                                push_toast(toast, toast_sequence, "Latest invoice paid.", ToastTone::Success);
                                            }
                                            Err(message) => push_toast(toast, toast_sequence, message, ToastTone::Error),
                                        }
                                        is_busy.set(false);
                                    },
                                    "Pay Invoice"
                                }
                                button {
                                    class: "primary-action",
                                    r#type: "button",
                                    disabled: is_busy() || route.status != RouteStatus::Active,
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
                                                    push_toast(toast, toast_sequence, "AutoSend flow complete.", ToastTone::Success);
                                                }
                                                Err(message) => push_toast(toast, toast_sequence, message, ToastTone::Error),
                                            }
                                            is_busy.set(false);
                                        }
                                    },
                                    "Create Invoice + AutoSend"
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
