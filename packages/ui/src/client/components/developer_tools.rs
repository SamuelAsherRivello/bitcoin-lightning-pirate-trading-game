use dioxus::prelude::*;
use dioxus_i18n::{prelude::i18n, t};

use crate::client::components::toast::{Toast, ToastTone};
use crate::client::services::localization_service::AppLanguage;
use crate::client::services::server_contact_service::contact_server;
use crate::client::services::storage_service::{save_language, save_theme, Theme};

#[component]
pub fn DeveloperTools() -> Element {
    let mut theme = use_context::<Signal<Theme>>();
    let mut language = use_context::<Signal<AppLanguage>>();
    let selected_language = language();
    let mut language_menu_open = use_signal(|| false);
    let mut toast = use_context::<Signal<Option<Toast>>>();
    let mut server_toast_sequence = use_signal(|| 10_000_u64);
    let mut i18n = i18n();

    rsx! {
        div { class: "developer-tools",
            div { class: "developer-tools__group",
                span { class: "developer-tools__label", "Dev Tools" }
                div { class: "developer-tools__controls",
                    a {
                    class: "developer-tools__github",
                    href: "https://github.com/SamuelAsherRivello/dioxus-bitcoin-lightning-game",
                    target: "_blank",
                    rel: "noopener noreferrer",
                    aria_label: t!("open-github-repository"),
                    "data-tooltip": t!("open-github-repository"),
                    svg {
                        class: "developer-tools__github-icon",
                        width: "24",
                        height: "24",
                        view_box: "0 0 24 24",
                        path {
                            fill: "currentColor",
                            d: "M12 .5C5.65.5.5 5.65.5 12c0 5.1 3.29 9.42 7.86 10.95.58.11.79-.25.79-.56v-2.16c-3.2.7-3.88-1.36-3.88-1.36-.52-1.33-1.28-1.68-1.28-1.68-1.05-.72.08-.7.08-.7 1.16.08 1.77 1.19 1.77 1.19 1.03 1.76 2.7 1.25 3.36.96.1-.75.4-1.25.73-1.54-2.55-.29-5.24-1.28-5.24-5.69 0-1.26.45-2.29 1.19-3.09-.12-.29-.52-1.46.11-3.05 0 0 .97-.31 3.17 1.18A11.1 11.1 0 0 1 12 6.06c.98 0 1.96.13 2.88.39 2.2-1.49 3.17-1.18 3.17-1.18.63 1.59.23 2.76.11 3.05.74.8 1.19 1.83 1.19 3.09 0 4.42-2.69 5.39-5.25 5.68.41.35.78 1.05.78 2.12v3.18c0 .31.21.67.8.56A11.51 11.51 0 0 0 23.5 12C23.5 5.65 18.35.5 12 .5Z",
                        }
                    }
                    }
                    button {
                    class: "developer-tools__server",
                    r#type: "button",
                    aria_label: "Server",
                    "data-tooltip": "Server",
                    onclick: move |_| async move {
                        let message = match contact_server().await {
                            Ok(_) => "Server contacted with success",
                            Err(_) => "Server cannot be contacted",
                        };
                        let tone = if message == "Server contacted with success" {
                            ToastTone::Success
                        } else {
                            ToastTone::Error
                        };
                        let next_id = *server_toast_sequence.peek() + 1;
                        server_toast_sequence.set(next_id);
                        toast.set(Some(Toast {
                            id: next_id,
                            message: message.to_string(),
                            tone,
                        }));
                    },
                    svg {
                        class: "developer-tools__button-icon",
                        width: "18",
                        height: "18",
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        polygon { points: "13 2 3 14 12 14 11 22 21 10 12 10 13 2" }
                    }
                    span { "Server" }
                    }
                }
            }
            button {
                    class: "developer-tools__theme",
                    r#type: "button",
                    aria_label: t!("toggle-theme"),
                    "data-tooltip": t!("toggle-theme"),
                    onclick: move |_| {
                        let next_theme = theme.peek().toggled();
                        theme.set(next_theme);
                        save_theme(next_theme);
                    },
                    svg {
                        class: "developer-tools__button-icon",
                        width: "18",
                        height: "18",
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        path { d: "M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z" }
                    }
                    span { {t!("theme")} }
                }
            div { class: "developer-tools__language-menu",
                button {
                        class: "developer-tools__language",
                        r#type: "button",
                        aria_label: t!("language-selector"),
                        "data-tooltip": t!("language-selector"),
                        aria_expanded: "{language_menu_open()}",
                        onclick: move |_| {
                            let is_open = *language_menu_open.peek();
                            language_menu_open.set(!is_open);
                        },
                        img {
                            src: selected_language.flag_asset(),
                            width: "24",
                            height: "16",
                            alt: "",
                        }
                        span { class: "developer-tools__language-caret", "▾" }
                    }
                if language_menu_open() {
                    div { class: "developer-tools__language-options",
                        for option_language in AppLanguage::ALL {
                            button {
                                    class: if option_language == selected_language {
                                        "developer-tools__language-option developer-tools__language-option--active"
                                    } else {
                                        "developer-tools__language-option"
                                    },
                                    r#type: "button",
                                    aria_label: match option_language {
                                        AppLanguage::En => t!("language-en"),
                                        AppLanguage::Es => t!("language-es"),
                                        AppLanguage::Pt => t!("language-pt"),
                                        AppLanguage::Fr => t!("language-fr"),
                                    },
                                    "data-tooltip": match option_language {
                                        AppLanguage::En => t!("language-en"),
                                        AppLanguage::Es => t!("language-es"),
                                        AppLanguage::Pt => t!("language-pt"),
                                        AppLanguage::Fr => t!("language-fr"),
                                    },
                                    onclick: move |_| {
                                        language.set(option_language);
                                        i18n.set_language(option_language.language_id());
                                        save_language(option_language);
                                        language_menu_open.set(false);
                                    },
                                    img {
                                        src: option_language.flag_asset(),
                                        width: "24",
                                        height: "16",
                                        alt: "",
                                    }
                            }
                        }
                    }
                }
            }
        }
    }
}
