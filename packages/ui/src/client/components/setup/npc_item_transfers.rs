use dioxus::prelude::*;

use crate::client::models::NpcItemTransfer;

#[component]
pub fn NpcItemTransferStatus(transfers: Vec<NpcItemTransfer>) -> Element {
    rsx! {
        div { class: "tra-setup-status", role: "status",
            strong { "User Nodes (TRAs)" }
            if transfers.is_empty() {
                span { "TRA items will match the demo targets for Jack, Bob, and Carol." }
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
