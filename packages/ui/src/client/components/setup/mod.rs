use dioxus::prelude::*;

#[component]
pub fn SetupChecklist(items: Vec<String>) -> Element {
    rsx! {
        ol { class: "setup-checklist",
            for item in items {
                li { "{item}" }
            }
        }
    }
}

#[component]
pub fn WarningCallout(title: String, body: String) -> Element {
    rsx! {
        div { class: "warning-callout", role: "alert",
            span { class: "warning-callout__icon", "aria-hidden": "true",
                svg {
                    width: "20",
                    height: "20",
                    view_box: "0 0 24 24",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "2",
                    stroke_linecap: "round",
                    stroke_linejoin: "round",
                    path { d: "M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0Z" }
                    line { x1: "12", y1: "9", x2: "12", y2: "13" }
                    line { x1: "12", y1: "17", x2: "12.01", y2: "17" }
                }
            }
            div { class: "warning-callout__content",
                strong { "{title}" }
                p { "{body}" }
            }
        }
    }
}

#[component]
pub fn TraSetupStatus(summary: String, detail: String) -> Element {
    rsx! {
        div { class: "tra-setup-status", role: "status",
            strong { "{summary}" }
            span { "{detail}" }
        }
    }
}
