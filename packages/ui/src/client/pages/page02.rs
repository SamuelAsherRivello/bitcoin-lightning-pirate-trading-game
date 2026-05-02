use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::client::pages::template_page::TemplatePage;

#[component]
pub fn Page02() -> Element {
    rsx! {
        TemplatePage {
            title: t!("page-02-title"),
            body_01: t!("page-02-body-01"),
            body_02: t!("page-02-body-02"),
            body_03: t!("page-02-body-03"),
            data_text: None,
        }
    }
}
