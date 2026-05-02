use dioxus::prelude::*;
use qrcode::{Color, QrCode};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

use crate::client::models::{QrAuthorizationKind, QrAuthorizationModal, QrAuthorizationStatus};

const QR_AUTH_MODAL_FOCUS_ID: &str = "qr-auth-modal";

#[component]
pub fn QrAuthorizationModalRegion(mut prompt: Signal<Option<QrAuthorizationModal>>) -> Element {
    let active_prompt = prompt();
    install_spacebar_approval_listener(prompt);

    use_effect(move || {
        if prompt.peek().is_some() {
            schedule_qr_modal_focus();
        }
    });

    rsx! {
        if let Some(active_prompt) = active_prompt {
            div {
                class: "qr-auth-backdrop",
                role: "presentation",
                div {
                    id: QR_AUTH_MODAL_FOCUS_ID,
                    class: "qr-auth-modal",
                    role: "dialog",
                    tabindex: "0",
                    aria_modal: "true",
                    aria_label: "{active_prompt.title}",
                    onkeydown: move |event| {
                        if event.data.key().to_string() == " " {
                            let _ = approve_mock_prompt(&mut prompt);
                        }
                    },
                    div { class: "qr-auth-modal__body",
                        h2 { "{active_prompt.title}" }
                        p { "{active_prompt.description}" }
                        QrCodeView {
                            payload: active_prompt.qr_payload.clone(),
                            qr_kind: active_prompt.qr_kind,
                        }
                        if active_prompt.status == QrAuthorizationStatus::Open {
                            span { class: "qr-auth-modal__status", "Waiting for wallet scan..." }
                        }
                        if active_prompt.status == QrAuthorizationStatus::MockCompleting {
                            span { class: "qr-auth-modal__status", "Mock wallet approval pending..." }
                        }
                        if active_prompt.status == QrAuthorizationStatus::Canceled {
                            span { class: "qr-auth-modal__status qr-auth-modal__status--error", "Authorization canceled." }
                        }
                    }
                    div { class: "qr-auth-modal__actions",
                        if active_prompt.can_cancel {
                            button {
                                class: "secondary-action",
                                r#type: "button",
                                onclick: move |_| {
                                    let active = prompt.peek().as_ref().cloned();
                                    if let Some(mut active) = active {
                                        active.status = QrAuthorizationStatus::Canceled;
                                        prompt.set(Some(active));
                                    }
                                },
                                "Cancel"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn install_spacebar_approval_listener(mut prompt: Signal<Option<QrAuthorizationModal>>) {
    use_hook(move || {
        let closure = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::KeyboardEvent)>::wrap(
            Box::new(move |event: web_sys::KeyboardEvent| {
                if event.key() == " " && approve_mock_prompt(&mut prompt) {
                    event.prevent_default();
                }
            }),
        );

        if let Some(window) = web_sys::window() {
            let _ = window
                .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref());
        }

        closure.forget();
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn install_spacebar_approval_listener(_prompt: Signal<Option<QrAuthorizationModal>>) {}

fn approve_mock_prompt(prompt: &mut Signal<Option<QrAuthorizationModal>>) -> bool {
    let active = prompt.peek().as_ref().cloned();
    if let Some(mut active) = active {
        if active.status == QrAuthorizationStatus::MockCompleting
            && active.auto_complete_after_ms.is_some()
        {
            active.status = QrAuthorizationStatus::Approved;
            prompt.set(Some(active));
            return true;
        }
    }
    false
}

#[cfg(target_arch = "wasm32")]
fn schedule_qr_modal_focus() {
    wasm_bindgen_futures::spawn_local(async {
        gloo_timers::future::TimeoutFuture::new(0).await;
        let Some(window) = web_sys::window() else {
            return;
        };
        let Some(document) = window.document() else {
            return;
        };
        let Some(element) = document.get_element_by_id(QR_AUTH_MODAL_FOCUS_ID) else {
            return;
        };
        let Some(element) = element.dyn_ref::<web_sys::HtmlElement>() else {
            return;
        };
        let _ = element.focus();
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn schedule_qr_modal_focus() {}

#[component]
fn QrCodeView(payload: String, qr_kind: QrAuthorizationKind) -> Element {
    let aria_label = match qr_kind {
        QrAuthorizationKind::NostrProfile => "Nostr profile authorization QR code",
        QrAuthorizationKind::Login | QrAuthorizationKind::SendSats => {
            "Lightning authorization QR code"
        }
    };
    let qr = qr_cells(&payload);

    rsx! {
        if let Some((width, cells)) = qr {
            div {
                class: "qr-code",
                aria_label: "{aria_label}",
                style: "grid-template-columns: repeat({width}, 1fr);",
                for filled in cells {
                    div { class: if filled { "qr-code__cell qr-code__cell--dark" } else { "qr-code__cell" } }
                }
            }
        } else {
            pre { class: "qr-auth-modal__payload", "{payload}" }
        }
    }
}

fn qr_cells(payload: &str) -> Option<(usize, Vec<bool>)> {
    let code = QrCode::new(payload.as_bytes()).ok()?;
    let width = code.width();
    let mut cells = Vec::with_capacity(width * width);

    for y in 0..width {
        for x in 0..width {
            cells.push(code[(x, y)] == Color::Dark);
        }
    }

    Some((width, cells))
}
