use dioxus::prelude::*;
use qrcode::{Color, QrCode};

use crate::client::models::{QrAuthorizationModal, QrAuthorizationStatus};

#[component]
pub fn QrAuthorizationModalRegion(mut prompt: Signal<Option<QrAuthorizationModal>>) -> Element {
    let active_prompt = prompt();

    rsx! {
        if let Some(active_prompt) = active_prompt {
            div {
                class: "qr-auth-backdrop",
                role: "presentation",
                div {
                    class: "qr-auth-modal",
                    role: "dialog",
                    aria_modal: "true",
                    aria_label: "{active_prompt.title}",
                    div { class: "qr-auth-modal__body",
                        h2 { "{active_prompt.title}" }
                        p { "{active_prompt.description}" }
                        QrCodeView { payload: active_prompt.qr_payload.clone() }
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

#[component]
fn QrCodeView(payload: String) -> Element {
    let qr = qr_cells(&payload);

    rsx! {
        if let Some((width, cells)) = qr {
            div {
                class: "qr-code",
                aria_label: "Lightning authorization QR code",
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
