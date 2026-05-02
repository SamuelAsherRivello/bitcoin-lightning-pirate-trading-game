use dioxus::prelude::*;

#[component]
pub fn TemplatePage(
    title: String,
    body_01: String,
    body_02: String,
    body_03: String,
    data_text: Option<String>,
    #[props(default = None)] readme_text: Option<String>,
    #[props(default = None)] readme_href: Option<String>,
) -> Element {
    rsx! {
        main { class: "page-content template-page",
            h1 { "{title}" }
            if let Some(data_text) = data_text {
                p { class: "template-page__data", "{data_text}" }
            }
            div { class: "template-page__body",
                p { "{body_01}" }
                p { "{body_02}" }
                p { "{body_03}" }
            }
            if let (Some(readme_text), Some(readme_href)) = (readme_text, readme_href) {
                a {
                    class: "template-page__readme-link",
                    href: readme_href,
                    target: "_blank",
                    rel: "noopener noreferrer",
                    "{readme_text}"
                }
            }
        }
    }
}
