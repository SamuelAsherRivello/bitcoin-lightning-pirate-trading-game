use dioxus::prelude::*;

use crate::client::Route;

#[component]
pub fn Page(
    route: Route,
    #[props(default = false)] will_preload: bool,
    children: Element,
) -> Element {
    let active_route = use_route::<Route>();
    let is_active = active_route == route;

    if !will_preload && !is_active {
        return rsx! {};
    }

    let class = if is_active {
        "page page--active"
    } else {
        "page page--preloaded"
    };
    let style = if is_active {
        "opacity: 1; pointer-events: auto; position: relative; z-index: 1; transition: opacity 0.25s ease;"
    } else {
        "opacity: 0; pointer-events: none; position: absolute; inset: 0; width: 100%; z-index: 0; transition: opacity 0.25s ease;"
    };
    let aria_hidden = if is_active { "false" } else { "true" };

    rsx! {
        div {
            class,
            style,
            "aria-hidden": aria_hidden,
            {children}
        }
    }
}
