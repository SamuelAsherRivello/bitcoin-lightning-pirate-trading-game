use dioxus::prelude::*;

#[component]
pub fn FieldHelpIcon(label: String) -> Element {
    rsx! {
        span {
            class: "field-help",
            aria_label: "{label}",
            tabindex: "0",
            "data-tooltip": "{label}",
            svg {
                "aria-hidden": "true",
                width: "14",
                height: "14",
                view_box: "0 0 24 24",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "2",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                circle { cx: "12", cy: "12", r: "10" }
                line { x1: "12", y1: "16", x2: "12", y2: "12" }
                line { x1: "12", y1: "8", x2: "12.01", y2: "8" }
            }
        }
    }
}
