use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::client::components::network::OperationFaqTable;
use crate::client::components::setup::WarningCallout;

const BITCOIN_BASICS_URL: &str = "https://bitcoin.org/en/how-it-works";
const LIGHTNING_OVERVIEW_URL: &str =
    "https://docs.lightning.engineering/the-lightning-network/overview";

#[component]
pub fn Home() -> Element {
    let operation_rows = lightning_service::get_operation_faq();

    rsx! {
        main { class: "page-content lab-page home-page faq-page",
            section { class: "lab-hero",
                div {
                    span { class: "eyebrow", "Polar regtest Lightning lab" }
                    h1 { {t!("home-title")} }
                    p {
                        "Control Alice, Bob, and Carol in a local Lightning learning lab. The app separates game actions from the network mechanics behind channels, invoices, payments, and block confirmations."
                    }
                }
                div { class: "status-card",
                    span { class: "eyebrow", "Block timing" }
                    strong { "10 min" }
                    p {
                        "Bitcoin mainnet blocks arrive about every 10 minutes on average. This regtest lab mines on demand."
                    }
                }
            }

            section { class: "home-section",
                div { class: "home-section__heading",
                    h2 { "Overview" }
                }
                article { class: "lab-panel lab-panel--full",
                    div { class: "section-heading",
                        h3 { "Why this demo exists" }
                    }
                    p {
                        "A Lightning channel is easier to understand when it feels like a trade route. Alice starts in Town, opens routes to Bob at the Beach and Carol at the Mountain, then buys items with real Lightning-shaped operations."
                    }
                    p {
                        "This is a lab tool. It controls all demo nodes so you can learn quickly. A production game would normally request payment from the player's own Lightning wallet instead of spending from it directly."
                    }
                    WarningCallout {
                        title: "Regtest safety check".to_string(),
                        body: "The app accepts local Polar regtest profiles only. Setup validation rejects hosted, production, mainnet, and other non-regtest profiles before lab actions unlock.".to_string(),
                    }
                }
            }

            section { class: "home-section",
                div { class: "home-section__heading",
                    h2 { "FAQ" }
                }
                div { class: "lab-grid lab-grid--two faq-concept-grid",
                    article { class: "lab-panel faq-concept",
                        div { class: "section-heading",
                            span { class: "eyebrow", "Bitcoin" }
                            h3 { "What is Bitcoin?" }
                        }
                        p {
                            "Bitcoin is open money: a peer-to-peer network, currency, and public ledger that lets people send value without a bank. Transactions are broadcast, verified by nodes, and confirmed into blocks by miners, creating shared ownership history. In this lab, Bitcoin is the base layer that funds and secures Lightning channels safely."
                        }
                        a {
                            class: "learn-more-link",
                            href: BITCOIN_BASICS_URL,
                            target: "_blank",
                            rel: "noopener noreferrer",
                            "Learn Bitcoin basics"
                        }
                    }
                    article { class: "lab-panel faq-concept",
                        div { class: "section-heading",
                            span { class: "eyebrow", "Lightning" }
                            h3 { "What is Bitcoin Lightning?" }
                        }
                        p {
                            "Lightning is a second-layer payment network for Bitcoin. It uses channels funded by on-chain Bitcoin transactions, then lets participants send small payments quickly through connected nodes. Active channels usually avoid waiting for new blocks, but opening or closing channels returns to Bitcoin for settlement, security, and dispute resolution when needed."
                        }
                        a {
                            class: "learn-more-link",
                            href: LIGHTNING_OVERVIEW_URL,
                            target: "_blank",
                            rel: "noopener noreferrer",
                            "Learn Lightning basics"
                        }
                    }
                }

                article { class: "lab-panel",
                    div { class: "section-heading",
                        span { class: "eyebrow", "Comparison" }
                        h3 { "Bitcoin vs Lightning" }
                    }
                    table {
                        class: "comparison-table",
                        aria_label: "Bitcoin and Lightning transaction comparison",
                        thead {
                            tr {
                                th { scope: "col", "System" }
                                th { scope: "col", "Transaction Cost (USD)" }
                                th { scope: "col", "Transactions Per Block" }
                                th { scope: "col", "Block Time" }
                                th { scope: "col", "Use Case" }
                            }
                        }
                        tbody {
                            tr {
                                th { scope: "row", "Bitcoin" }
                                td { "Variable network fee; can rise during congestion. (Approx. $1-$20)" }
                                td { "Limited by block space; usually thousands of transactions. (Approx. 2,000-3,000)" }
                                td { "About 10 minutes on average. (Approx. 10 min)" }
                                td { "Slow secure operations." }
                            }
                            tr {
                                th { scope: "row", "Lightning" }
                                td { "Usually tiny routing fees for active-channel payments. (Approx. less than $0.01)" }
                                td { "Not block-bound after channels are active. (Approx. not block-limited)" }
                                td { "Seconds or less for active-channel payments. (Approx. seconds)" }
                                td { "Faster operations." }
                            }
                        }
                    }
                }

                article { class: "lab-panel",
                    div { class: "section-heading",
                        span { class: "eyebrow", "Operations" }
                        h3 { "Which Lightning Operations need a Bitcoin Block?" }
                    }
                    p {
                        "On Bitcoin mainnet, a confirmation means waiting for a block, which arrives about every 10 minutes on average. Operations that enter or exit Lightning channels wait for that base-layer confirmation; payments over an already-active channel do not."
                    }
                    OperationFaqTable { rows: operation_rows }
                }
            }
        }
    }
}
