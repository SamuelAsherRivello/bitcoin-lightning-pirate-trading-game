use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::client::components::developer_tools::DeveloperTools;
use crate::client::components::toast::{Toast, ToastRegion};
use crate::client::Route;

#[component]
pub fn PageHeader() -> Element {
    let active_route = use_route::<Route>();
    let is_page01 = active_route == (Route::Page01 {});
    let is_page02 = active_route == (Route::Page02 {});
    let is_page03 = active_route == (Route::Page03 {});
    let toast = use_context::<Signal<Option<Toast>>>();

    rsx! {
        header { id: "page-header",
            nav {
                div { class: "page-header__pages",
                    Link {
                        class: if is_page01 {
                            "page-header__page-link page-header__page-link--active"
                        } else {
                            "page-header__page-link"
                        },
                        to: Route::Page01 {},
                        aria_current: if is_page01 { "page" } else { "false" },
                        aria_label: t!("view-page-01"),
                        "data-tooltip": t!("view-page-01"),
                        span { class: "page-header__label-full", {t!("nav-page-01")} }
                        span { class: "page-header__label-short", {t!("nav-page-01-short")} }
                    }
                    Link {
                        class: if is_page02 {
                            "page-header__page-link page-header__page-link--active"
                        } else {
                            "page-header__page-link"
                        },
                        to: Route::Page02 {},
                        aria_current: if is_page02 { "page" } else { "false" },
                        aria_label: t!("view-page-02"),
                        "data-tooltip": t!("view-page-02"),
                        span { class: "page-header__label-full", {t!("nav-page-02")} }
                        span { class: "page-header__label-short", {t!("nav-page-02-short")} }
                    }
                    Link {
                        class: if is_page03 {
                            "page-header__page-link page-header__page-link--active"
                        } else {
                            "page-header__page-link"
                        },
                        to: Route::Page03 {},
                        aria_current: if is_page03 { "page" } else { "false" },
                        aria_label: t!("view-page-03"),
                        "data-tooltip": t!("view-page-03"),
                        span { class: "page-header__label-full", {t!("nav-page-03")} }
                        span { class: "page-header__label-short", {t!("nav-page-03-short")} }
                    }
                }
                DeveloperTools {}
            }
            ToastRegion { toast }
        }
    }
}
