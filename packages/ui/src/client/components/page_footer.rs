use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn PageFooter() -> Element {
    rsx! {
        footer { class: "page-footer text-center text-[13px] leading-[1.45]",
            {t!("footer-rights")}
        }
    }
}
