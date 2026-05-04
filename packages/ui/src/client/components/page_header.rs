use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::client::components::developer_tools::DeveloperTools;
use crate::client::components::toast::{
    OperationPrompt, OperationPromptRegion, Toast, ToastRegion,
};
use crate::client::models::SetupProfile;
use crate::client::Route;

#[component]
pub fn PageHeader() -> Element {
    let active_route = use_route::<Route>();
    let is_home = active_route == (Route::Home {});
    let is_setup = active_route == (Route::SetUp {});
    let is_play_game = active_route == (Route::PlayGame {});
    let is_debug_network = active_route == (Route::DebugNetwork {});
    let setup_profile = use_context::<Signal<SetupProfile>>();
    let setup_is_connected = setup_profile().is_connected();
    let toast = use_context::<Signal<Option<Toast>>>();
    let operation_prompt = use_context::<Signal<Option<OperationPrompt>>>();

    rsx! {
        header { id: "page-header",
            nav {
                div { class: "page-header__pages",
                    Link {
                        class: if is_home {
                            "page-header__page-link page-header__page-link--active"
                        } else {
                            "page-header__page-link"
                        },
                        to: Route::Home {},
                        aria_current: if is_home { "page" } else { "false" },
                        aria_label: t!("view-home"),
                        "data-tooltip": t!("view-home"),
                        span { class: "page-header__label-full", {t!("nav-home")} }
                        span { class: "page-header__label-short", {t!("nav-home-short")} }
                    }
                    Link {
                        class: if is_setup {
                            "page-header__page-link page-header__page-link--active"
                        } else {
                            "page-header__page-link"
                        },
                        to: Route::SetUp {},
                        aria_current: if is_setup { "page" } else { "false" },
                        aria_label: t!("view-setup"),
                        "data-tooltip": t!("view-setup"),
                        span { class: "page-header__label-full", {t!("nav-setup")} }
                        span { class: "page-header__label-short", {t!("nav-setup-short")} }
                    }
                    if setup_is_connected {
                        Link {
                            class: if is_play_game {
                                "page-header__page-link page-header__page-link--active"
                            } else {
                                "page-header__page-link"
                            },
                            to: Route::PlayGame {},
                            aria_current: if is_play_game { "page" } else { "false" },
                            aria_label: t!("view-play-game"),
                            "data-tooltip": t!("view-play-game"),
                            span { class: "page-header__label-full", {t!("nav-play-game")} }
                            span { class: "page-header__label-short", {t!("nav-play-game-short")} }
                        }
                    } else {
                        button {
                            class: "page-header__page-link page-header__page-link--disabled",
                            r#type: "button",
                            disabled: true,
                            aria_label: t!("locked-play-game"),
                            "data-tooltip": t!("locked-play-game"),
                            span { class: "page-header__label-full", {t!("nav-play-game")} }
                            span { class: "page-header__label-short", {t!("nav-play-game-short")} }
                        }
                    }
                    if setup_is_connected {
                        Link {
                            class: if is_debug_network {
                                "page-header__page-link page-header__page-link--active"
                            } else {
                                "page-header__page-link"
                            },
                            to: Route::DebugNetwork {},
                            aria_current: if is_debug_network { "page" } else { "false" },
                            aria_label: t!("view-debug-network"),
                            "data-tooltip": t!("view-debug-network"),
                            span { class: "page-header__label-full", {t!("nav-debug-network")} }
                            span { class: "page-header__label-short", {t!("nav-debug-network-short")} }
                        }
                    } else {
                        button {
                            class: "page-header__page-link page-header__page-link--disabled",
                            r#type: "button",
                            disabled: true,
                            aria_label: t!("locked-debug-network"),
                            "data-tooltip": t!("locked-debug-network"),
                            span { class: "page-header__label-full", {t!("nav-debug-network")} }
                            span { class: "page-header__label-short", {t!("nav-debug-network-short")} }
                        }
                    }
                }
                DeveloperTools {}
            }
            ToastRegion { toast }
            OperationPromptRegion { prompt: operation_prompt }
        }
    }
}
