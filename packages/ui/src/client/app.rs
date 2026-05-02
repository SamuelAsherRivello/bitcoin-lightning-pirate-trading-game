use dioxus::prelude::*;

use super::Route;

const PAGE_HEADER_CSS: Asset = asset!("/assets/styling/page_header.css");
const PAGE_FOOTER_CSS: Asset = asset!("/assets/styling/page_footer.css");

#[component]
pub fn App() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: PAGE_HEADER_CSS }
        document::Link { rel: "stylesheet", href: PAGE_FOOTER_CSS }
        Router::<Route> {}
    }
}
