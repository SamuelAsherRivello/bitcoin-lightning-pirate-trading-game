use dioxus::prelude::*;

use crate::client::models::GameTreasury;

#[component]
pub fn GameTreasurySetupStatus(treasury: GameTreasury) -> Element {
    rsx! {
        div { class: "tra-setup-status", role: "status",
            strong { "Game Treasury: {treasury.status.label()}" }
            span { "{treasury.spendable_sats} sats available for the game bank" }
        }
    }
}
