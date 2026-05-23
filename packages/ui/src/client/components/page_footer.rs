use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn PageFooter() -> Element {
    rsx! {
        footer { class: "page-footer",
            {t!("footer-rights")}
        }
    }
}
