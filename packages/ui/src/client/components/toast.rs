use dioxus::prelude::*;

pub const TOAST_TIMEOUT_MS: u32 = 5_000;
pub const PROMPT_MESSAGE_MINIMUM_MS: u32 = 250;

#[derive(Clone, Debug, PartialEq)]
pub struct Toast {
    pub id: u64,
    pub message: String,
    pub tone: ToastTone,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ToastTone {
    Info,
    Success,
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OperationPrompt {
    pub operation_id: u64,
    pub title: String,
    pub subtitle: Option<String>,
    pub message: String,
    pub tone: ToastTone,
    pub is_pending: bool,
    pub can_cancel: bool,
    pub cancel_requested: bool,
}

#[component]
pub fn ToastRegion(mut toast: Signal<Option<Toast>>) -> Element {
    use_effect(move || {
        if let Some(active_toast) = toast() {
            spawn(async move {
                wait_for_toast_timeout().await;

                if toast.peek().as_ref().map(|toast| toast.id) == Some(active_toast.id) {
                    toast.set(None);
                }
            });
        }
    });

    rsx! {
        div {
            class: "toast-region",
            aria_live: "polite",
            if let Some(toast) = toast() {
                div { class: toast_class(toast.tone),
                    span { class: "toast__message", "{toast.message}" }
                    button {
                        class: "toast__copy",
                        r#type: "button",
                        aria_label: "Copy toast message",
                        title: "Copy message",
                        onclick: {
                            let message = toast.message.clone();
                            move |_| {
                                copy_to_clipboard(message.clone());
                            }
                        },
                        svg {
                            "aria-hidden": "true",
                            width: "16",
                            height: "16",
                            view_box: "0 0 24 24",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2",
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            rect { x: "9", y: "9", width: "13", height: "13", rx: "2", ry: "2" }
                            path { d: "M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn copy_to_clipboard(message: String) {
    copy_to_clipboard_js(&message);
}

#[cfg(not(target_arch = "wasm32"))]
fn copy_to_clipboard(_message: String) {}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(
    inline_js = "export function copy_to_clipboard_js(message) { if (navigator.clipboard && navigator.clipboard.writeText) { navigator.clipboard.writeText(message); } }"
)]
extern "C" {
    fn copy_to_clipboard_js(message: &str);
}

#[component]
pub fn OperationPromptRegion(mut prompt: Signal<Option<OperationPrompt>>) -> Element {
    let active_prompt = prompt();

    rsx! {
        if let Some(active_prompt) = active_prompt {
            div {
                class: "operation-prompt-backdrop",
                role: "presentation",
                div {
                    class: prompt_class(active_prompt.tone),
                    role: "dialog",
                    aria_modal: "true",
                    aria_label: "{active_prompt.title}",
                    div { class: "operation-prompt__status" }
                    div { class: "operation-prompt__body",
                        span { class: "eyebrow", "{prompt_status_label(&active_prompt)}" }
                        h2 { "{active_prompt.title}" }
                        if let Some(subtitle) = active_prompt.subtitle.as_ref() {
                            p { class: "operation-prompt__subtitle", "{subtitle}" }
                        }
                        p { "{active_prompt.message}" }
                    }
                    div { class: "operation-prompt__actions",
                        if active_prompt.is_pending && active_prompt.can_cancel {
                            button {
                                class: "secondary-action danger-action",
                                r#type: "button",
                                disabled: active_prompt.cancel_requested,
                                onclick: move |_| {
                                    let active = { prompt.peek().as_ref().cloned() };
                                    if let Some(mut active) = active {
                                        active.cancel_requested = true;
                                        active.message = "Cancel requested. Waiting for Polar to finish so the app can undo the call.".to_string();
                                        active.tone = ToastTone::Info;
                                        prompt.set(Some(active));
                                    }
                                },
                                if active_prompt.cancel_requested {
                                    "Cancel requested"
                                } else {
                                    "Cancel"
                                }
                            }
                        } else if !active_prompt.is_pending {
                            button {
                                class: "primary-action",
                                r#type: "button",
                                onclick: move |_| prompt.set(None),
                                "Continue"
                            }
                        }
                    }
                }
            }
        }
    }
}

fn toast_class(tone: ToastTone) -> &'static str {
    match tone {
        ToastTone::Info => "toast toast--info",
        ToastTone::Success => "toast toast--success",
        ToastTone::Error => "toast toast--error",
    }
}

fn prompt_class(tone: ToastTone) -> &'static str {
    match tone {
        ToastTone::Info => "operation-prompt operation-prompt--info",
        ToastTone::Success => "operation-prompt operation-prompt--success",
        ToastTone::Error => "operation-prompt operation-prompt--error",
    }
}

#[cfg(target_arch = "wasm32")]
pub async fn wait_for_toast_timeout() {
    gloo_timers::future::TimeoutFuture::new(TOAST_TIMEOUT_MS).await;
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn wait_for_toast_timeout() {
    futures_timer::Delay::new(std::time::Duration::from_millis(TOAST_TIMEOUT_MS.into())).await;
}

fn prompt_status_label(prompt: &OperationPrompt) -> &'static str {
    if prompt.is_pending {
        "Pending operation"
    } else if prompt.tone == ToastTone::Error {
        "Action required"
    } else {
        "Complete"
    }
}

#[cfg(target_arch = "wasm32")]
pub async fn wait_for_prompt_message_minimum() {
    gloo_timers::future::TimeoutFuture::new(PROMPT_MESSAGE_MINIMUM_MS).await;
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn wait_for_prompt_message_minimum() {
    futures_timer::Delay::new(std::time::Duration::from_millis(
        PROMPT_MESSAGE_MINIMUM_MS.into(),
    ))
    .await;
}
