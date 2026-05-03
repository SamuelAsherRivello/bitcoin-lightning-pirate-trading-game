use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::client::components::game::{HistoryItems, RouteSummary};
use crate::client::components::toast::{Toast, ToastTone};
use crate::client::models::{DemoNodeId, LabState, RouteStatus, SetupProfile};
use crate::client::services::lightning_server_functions::{
    create_invoice_and_maybe_autosend, get_lab_state, open_trade_route, wait_for_next_block,
};
use crate::client::Route;

#[component]
pub fn PlayGame() -> Element {
    let setup_profile = use_context::<Signal<SetupProfile>>();
    let mut lab_state = use_context::<Signal<Option<LabState>>>();
    let toast = use_context::<Signal<Option<Toast>>>();
    let toast_sequence = use_signal(|| 30_000_u64);
    let mut is_busy = use_signal(|| false);

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

    rsx! {
        main { class: "page-content lab-page play-page",
            section { class: "lab-hero",
                div {
                    span { class: "eyebrow", "Alice starts in Town" }
                    h1 { {t!("play-game-title")} }
                    p {
                        "Open trade routes to Bob at the Beach and Carol at the Mountain. A route under construction needs Wait for Next Block; a purchase over an active route uses Lightning immediately."
                    }
                }
                div { class: "status-card",
                    span { class: "eyebrow", "Trade amount" }
                    strong { "{state.profile.sats_per_transaction} sats" }
                    p { "Every purchase in this POC uses the configured demo amount." }
                }
            }

            section { class: "game-map",
                div { class: "location-node location-node--town",
                    span { "Town" }
                    strong { "Alice" }
                }
                div { class: "location-node location-node--desert",
                    span { "Desert" }
                    strong { "Future route" }
                }
                div { class: "location-node location-node--beach",
                    span { "Beach" }
                    strong { "Bob" }
                }
                div { class: "location-node location-node--mountain",
                    span { "Mountain" }
                    strong { "Carol" }
                }
            }

            section { class: "lab-grid lab-grid--two",
                for route in state.trade_routes.clone() {
                    article { class: "lab-panel route-card",
                        RouteSummary { route: route.clone() }
                        p {
                            if route.status == RouteStatus::Missing {
                                "This trade route does not exist yet. Opening it starts an on-chain channel open."
                            } else if route.status == RouteStatus::UnderConstruction {
                                "The channel open is pending. Regtest can mine the next block instantly."
                            } else {
                                "This route is active. Alice can pay invoices over Lightning without waiting for a new block."
                            }
                        }
                        div { class: "route-metrics",
                            span { "Capacity: {route.capacity_sats} sats" }
                            span { "Alice side: {route.local_balance_sats} sats" }
                            span { "{route.to_node.label()} side: {route.remote_balance_sats} sats" }
                        }
                        div { class: "button-row",
                            button {
                                class: "primary-action",
                                r#type: "button",
                                disabled: is_busy() || route.status != RouteStatus::Missing,
                                onclick: move |_| {
                                    let to_node = route.to_node;
                                    async move {
                                        is_busy.set(true);
                                        match open_trade_route(setup_profile(), to_node).await {
                                            Ok(next_state) => {
                                                lab_state.set(Some(next_state));
                                                push_toast(toast, toast_sequence, "Trade route is under construction.", ToastTone::Success);
                                            }
                                            Err(message) => push_toast(toast, toast_sequence, message, ToastTone::Error),
                                        }
                                        is_busy.set(false);
                                    }
                                },
                                "Open Trade Route"
                            }
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
                                        let memo = format!("Alice buys a {} item", merchant.location().label());
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
                                                lab_state.set(Some(next_state));
                                                push_toast(toast, toast_sequence, "Invoice created and paid.", ToastTone::Success);
                                            }
                                            Err(message) => push_toast(toast, toast_sequence, message, ToastTone::Error),
                                        }
                                        is_busy.set(false);
                                    }
                                },
                                "Buy Item"
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
