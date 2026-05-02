use dioxus::prelude::*;

use ui::App as UiApp;

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::LaunchBuilder::desktop()
        .with_cfg(dioxus::desktop::Config::new().with_background_color((8, 11, 16, 255)))
        .launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        UiApp {}
    }
}
