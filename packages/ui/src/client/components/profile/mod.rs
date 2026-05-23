use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn ProfileNamePrompt(
    username: String,
    validation_error: Option<String>,
    is_submitting: bool,
    on_username_input: EventHandler<String>,
    on_submit: EventHandler<()>,
    on_cancel: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "qr-auth-backdrop", role: "presentation",
            div {
                class: "qr-auth-modal",
                role: "dialog",
                aria_modal: "true",
                aria_label: "Set Nostr profile name",
                div { class: "qr-auth-modal__body",
                    h2 { {t!("profile-group-label")} }
                    label {
                        class: "setup-field",
                        span { {t!("profile-username-label")} }
                        input {
                            value: "{username}",
                            disabled: is_submitting,
                            maxlength: "32",
                            oninput: move |event| on_username_input.call(event.value()),
                        }
                    }
                    if let Some(validation_error) = validation_error {
                        p { class: "muted-copy", "{validation_error}" }
                    }
                }
                div { class: "qr-auth-modal__actions",
                    button {
                        class: "primary-action",
                        r#type: "button",
                        disabled: is_submitting,
                        onclick: move |_| on_submit.call(()),
                        if is_submitting {
                            "Submitting..."
                        } else {
                            {t!("profile-submit")}
                        }
                    }
                    button {
                        class: "secondary-action",
                        r#type: "button",
                        disabled: is_submitting,
                        onclick: move |_| on_cancel.call(()),
                        {t!("profile-cancel")}
                    }
                }
            }
        }
    }
}
