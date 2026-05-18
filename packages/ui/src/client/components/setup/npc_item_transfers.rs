use dioxus::prelude::*;

use crate::client::models::NpcItemTransfer;

#[component]
pub fn NpcItemTransferStatus(transfers: Vec<NpcItemTransfer>) -> Element {
    rsx! {
        div { class: "tra-setup-status", role: "status",
            strong { "NPC Item Transfers" }
            if transfers.is_empty() {
                span { "Starting items will move from Game Treasury to Bob and Carol." }
            } else {
                for transfer in transfers {
                    span {
                        "{transfer.item_name} -> {transfer.destination.label()} ({transfer.status.label()})"
                    }
                }
            }
        }
    }
}
