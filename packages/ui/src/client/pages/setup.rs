use dioxus::prelude::*;
use dioxus_i18n::t;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

use crate::client::components::help::FieldHelpIcon;
use crate::client::components::setup::{NpcItemTransferStatus, WarningCallout};
use crate::client::components::toast::{
    wait_for_prompt_message_minimum, OperationPrompt, Toast, ToastTone,
};
use crate::client::models::{
    ConnectionStatus, DemoNodeId, LabState, PolarAutomationProfile, SetupMode, SetupProfile,
    APPLE_ITEM_ID, BOOK_ITEM_ID, DEFAULT_BITCOIN_BACKEND_NAME,
};
use crate::client::services::lightning_server_functions::{
    complete_polar_setup, confirm_polar_block_height, create_polar_demo_nodes_with_progress,
    destroy_polar_demo_nodes, ensure_polar_server, prepare_game_treasury,
    prepare_game_treasury_tras, reset_lab, test_setup, transfer_npc_starting_items,
    verify_polar_bridge, PolarServerEnsureStatus,
};
use crate::client::services::storage_service;

const DOCKER_DESKTOP_URL: &str = "https://www.docker.com/products/docker-desktop/";
const LOCAL_APP_URL: &str = "http://localhost:8080";
const POLAR_DOWNLOAD_URL: &str = "https://lightningpolar.com/";
const POLAR_DEMO_NODES_SUBMIT_ID: &str = "polar-user-nodes-submit";
#[cfg(target_arch = "wasm32")]
const FOCUS_RETRY_ATTEMPTS: u8 = 12;
#[cfg(target_arch = "wasm32")]
const FOCUS_RETRY_DELAY_MS: u32 = 16;

#[derive(Clone, Copy, Eq, PartialEq)]
enum PolarWizardStep {
    BridgeUrl,
    ServerName,
    GameTreasury,
    GameTreasuryTras,
    UserNodes,
    NpcItemTransfers,
    BlockHeight,
    Complete,
    Done,
}

impl PolarWizardStep {
    fn order(self) -> u8 {
        match self {
            Self::BridgeUrl => 1,
            Self::ServerName => 2,
            Self::GameTreasury => 3,
            Self::GameTreasuryTras => 4,
            Self::UserNodes => 5,
            Self::NpcItemTransfers => 6,
            Self::BlockHeight => 7,
            Self::Complete | Self::Done => 8,
        }
    }
}

fn polar_wizard_step_label(step: PolarWizardStep) -> &'static str {
    match step {
        PolarWizardStep::BridgeUrl => "Bridge URL",
        PolarWizardStep::ServerName => "Server Name",
        PolarWizardStep::GameTreasury => "Game Treasury (Sats)",
        PolarWizardStep::GameTreasuryTras => "Game Treasury (TRAs)",
        PolarWizardStep::UserNodes => "User Nodes",
        PolarWizardStep::NpcItemTransfers => "NPC Item Transfers",
        PolarWizardStep::BlockHeight => "Block Height",
        PolarWizardStep::Complete | PolarWizardStep::Done => "Unlock Routes",
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PolarConnectionTab {
    Environment,
    Polar,
}

impl PolarConnectionTab {
    fn from_storage_value(value: impl AsRef<str>) -> Option<Self> {
        match value.as_ref() {
            "environment" => Some(Self::Environment),
            "polar" => Some(Self::Polar),
            _ => None,
        }
    }

    fn load() -> Self {
        storage_service::load_setup_polar_tab()
            .and_then(Self::from_storage_value)
            .unwrap_or(Self::Environment)
    }

    fn save(self) {
        storage_service::save_setup_polar_tab(self.storage_value());
    }

    fn storage_value(self) -> &'static str {
        match self {
            Self::Environment => "environment",
            Self::Polar => "polar",
        }
    }
}

#[component]
pub fn SetUp() -> Element {
    let mut setup_profile = use_context::<Signal<SetupProfile>>();
    let mut lab_state = use_context::<Signal<Option<LabState>>>();
    let toast = use_context::<Signal<Option<Toast>>>();
    let operation_prompt = use_context::<Signal<Option<OperationPrompt>>>();
    let toast_sequence = use_signal(|| 10_000_u64);
    let prompt_sequence = use_signal(|| 20_000_u64);
    let mut amount_text = use_signal(|| setup_profile().sats_per_transaction.to_string());
    let mut setup_mode = use_signal(|| setup_profile().setup_mode);
    let mut polar_connection_tab = use_signal(PolarConnectionTab::load);
    let mut polar_bridge_url = use_signal(|| setup_profile().polar_automation.bridge_url.clone());
    let mut polar_server_name = use_signal(|| setup_profile().network_name.clone());
    let mut polar_block_height = use_signal(|| {
        lab_state()
            .map(|state| state.block_height)
            .unwrap_or_default()
            .to_string()
    });
    let mut is_busy = use_signal(|| false);
    let mut bridge_connection_error = use_signal(String::new);
    let mut show_complete_reset_confirm = use_signal(|| false);
    let current_profile = setup_profile();
    let current_lab_state = lab_state();
    let active_step = polar_wizard_step(&current_profile, current_lab_state.as_ref());
    let bridge_url_is_valid = is_valid_local_bridge_url(&polar_bridge_url());
    let browser_origin_is_valid = browser_origin_allows_polar_bridge();
    let bridge_url_can_submit = bridge_url_is_valid && browser_origin_is_valid;
    let server_name_is_valid = !polar_server_name().trim().is_empty();

    rsx! {
            main { class: "page-content lab-page setup-page",
                section { class: "lab-hero",
                    div {
                        span { class: "eyebrow", "Polar regtest Lightning lab" }
                        h1 { {t!("setup-title")} }
                        p {
                            "Control Alice, Bob, and Carol in a local Lightning learning lab. The app separates game actions from the network mechanics behind channels, invoices, payments, and block confirmations."
                        }
                    }
                    div { class: "status-card",
                        span { class: "eyebrow", "Setup status" }
                        strong { "{current_profile.connection_status.label()}" }
                        p {
                            if current_profile.is_connected() {
                                "Play Game and Network Dashboard are unlocked."
                            } else {
                                "Complete the Polar app setup before gameplay unlocks."
                            }
                        }
                    }
            }

            section { class: "lab-panel setup-panel",
                section { class: "setup-subsection",
                    div { class: "section-heading section-heading--subsection",
                        h2 { "General" }
                    }

                        label { class: "field-group",
                            span { "Sats per transaction" }
                            input {
                                r#type: "number",
                                min: "1",
                                max: "100000",
                                step: "1",
                                value: amount_text(),
                                oninput: move |event| amount_text.set(event.value()),
                            }
                        }
                    }

                section { class: "setup-subsection setup-subsection--connection",
                    div { class: "section-heading section-heading--subsection",
                        h2 { "Connection" }
                    }

                        div { class: "connection-tabs",
                            div { class: "segmented-control", role: "tablist", aria_label: "Connection",
                                button {
                                    class: if setup_mode() == SetupMode::ServerConfig { "segment segment--active" } else { "segment" },
                                    r#type: "button",
                                    role: "tab",
                                    aria_selected: if setup_mode() == SetupMode::ServerConfig { "true" } else { "false" },
                                    onclick: move |_| setup_mode.set(SetupMode::ServerConfig),
                                    "Polar Connection (Networked)"
                                }
                                button {
                                    class: if setup_mode() == SetupMode::BrowserRegtestOnly { "segment segment--active" } else { "segment" },
                                    r#type: "button",
                                    role: "tab",
                                    aria_selected: if setup_mode() == SetupMode::BrowserRegtestOnly { "true" } else { "false" },
                                    onclick: move |_| setup_mode.set(SetupMode::BrowserRegtestOnly),
                                    "Mock Connection (Offline)"
                                }
                            }

                            div {
                                class: if setup_mode() == SetupMode::ServerConfig { "connection-tab-panel connection-tab-panel--polar" } else { "connection-tab-panel connection-tab-panel--mock" },
                                role: "tabpanel",
                                aria_label: if setup_mode() == SetupMode::ServerConfig { "Polar Connection" } else { "Mock Connection" },

                            if setup_mode() == SetupMode::BrowserRegtestOnly {
                                p { class: "connection-tab-copy", "Use safe local demo data without Polar." }
                                WarningCallout {
                                    title: "Mock data only".to_string(),
                                    body: "This is fake data without any connection to the Lightning network.".to_string(),
                                }
                                div { class: "button-row",
                                    button {
                                        class: "primary-action",
                                        r#type: "button",
                                        disabled: is_busy(),
                                        onclick: move |_| async move {
                                            is_busy.set(true);
                                            push_toast(toast, toast_sequence, "Testing mock setup...", ToastTone::Info);

                                            match profile_from_inputs(
                                                amount_text(),
                                                polar_server_name(),
                                                setup_mode(),
                                                polar_automation_from_input(
                                                    polar_bridge_url(),
                                                    setup_profile().polar_automation,
                                                ),
                                                setup_profile(),
                                            ) {
                                                Ok(profile) => match test_setup(profile).await {
                                                    Ok(state) => {
                                                        setup_profile.set(state.profile.clone());
                                                        lab_state.set(Some(state));
                                                        push_toast(toast, toast_sequence, "Mock setup saved.", ToastTone::Success);
                                                    }
                                                    Err(message) => push_toast(toast, toast_sequence, message, ToastTone::Error),
                                                },
                                                Err(message) => push_toast(toast, toast_sequence, message, ToastTone::Error),
                                            }

                                            is_busy.set(false);
                                        },
                                        "SUBMIT"
                                    }
                                    button {
                                        class: "secondary-action danger-action",
                                        r#type: "button",
                                        disabled: is_busy(),
                                        onclick: move |_| async move {
                                            match reset_lab().await {
                                                Ok(default_profile) => {
                                                    setup_profile.set(default_profile.clone());
                                                    lab_state.set(None);
                                                    amount_text.set(default_profile.sats_per_transaction.to_string());
                                                    setup_mode.set(default_profile.setup_mode);
                                                    polar_bridge_url.set(default_profile.polar_automation.bridge_url.clone());
                                                    polar_server_name.set(default_profile.network_name.clone());
                                                    push_toast(toast, toast_sequence, "Local setup reset.", ToastTone::Success);
                                                }
                                                Err(message) => push_toast(toast, toast_sequence, message, ToastTone::Error),
                                            }
                                        },
                                        "RESET"
                                    }
                                }
                            } else {
                                p { class: "connection-tab-copy",
                                    "Create the Bitcoin backend in Polar, then use the app setup steps to connect the local bridge and create Alice, Bob, and Carol."
                                }
                                WarningCallout {
                                    title: "Testnet Only".to_string(),
                                    body: "Use only a local Polar regtest network. This is a demo not meant for mainnet.".to_string(),
                                }

                                div { class: "polar-connection-tabs",
                                    div { class: "segmented-control segmented-control--nested", role: "tablist", aria_label: "Polar setup",
                                        button {
                                            class: if polar_connection_tab() == PolarConnectionTab::Environment { "segment segment--active" } else { "segment" },
                                            r#type: "button",
                                            role: "tab",
                                            aria_selected: if polar_connection_tab() == PolarConnectionTab::Environment { "true" } else { "false" },
                                            onclick: move |_| {
                                                let tab = PolarConnectionTab::Environment;
                                                polar_connection_tab.set(tab);
                                                tab.save();
                                            },
                                            "1. Environment"
                                        }
                                        button {
                                            class: if polar_connection_tab() == PolarConnectionTab::Polar { "segment segment--active" } else { "segment" },
                                            r#type: "button",
                                            role: "tab",
                                            aria_selected: if polar_connection_tab() == PolarConnectionTab::Polar { "true" } else { "false" },
                                            onclick: move |_| {
                                                let tab = PolarConnectionTab::Polar;
                                                polar_connection_tab.set(tab);
                                                tab.save();
                                            },
                                            "2. Polar"
                                        }
                                    }
                                }

                            if polar_connection_tab() == PolarConnectionTab::Environment {
                            section {
                                class: "polar-setup-section polar-connection-tab-panel",
                                role: "tabpanel",
                                aria_label: "1. Environment",
                                    p { class: "connection-tab-copy",
                                        "Prepare the local apps before continuing with App Setup."
                                    }
                                    InstructionList { class: "manual-step-list".to_string(),
                                        Instruction {
                                            class: "wizard-step manual-step".to_string(),
                                            number: 1,
                                            info: "Download Docker Desktop".to_string(),
                                            name: rsx! {
                                                span {
                                                    "Install "
                                                    a {
                                                        class: "setup-resource-link",
                                                        href: DOCKER_DESKTOP_URL,
                                                        target: "_blank",
                                                        rel: "noopener noreferrer",
                                                        "Docker"
                                                    }
                                                }
                                            }
                                        }
                                        Instruction {
                                            class: "wizard-step manual-step".to_string(),
                                            number: 2,
                                            info: "Start Docker Desktop".to_string(),
                                            name: rsx! { "Run Docker" },
                                        }
                                        Instruction {
                                            class: "wizard-step manual-step".to_string(),
                                            number: 3,
                                            info: "Download Polar".to_string(),
                                            name: rsx! {
                                                span {
                                                    "Install "
                                                    a {
                                                        class: "setup-resource-link",
                                                        href: POLAR_DOWNLOAD_URL,
                                                        target: "_blank",
                                                        rel: "noopener noreferrer",
                                                        "Polar"
                                                    }
                                                }
                                            }
                                        }
                                        Instruction {
                                            class: "wizard-step manual-step".to_string(),
                                            number: 4,
                                            info: format!(
                                                "Start Polar, keep the app open at {LOCAL_APP_URL}, then submit step 1 again."
                                            ),
                                            name: rsx! { "Run Polar" },
                                        }
                                    }
                                }
                            }

                            if polar_connection_tab() == PolarConnectionTab::Polar {
                            section {
                                class: "app-setup-section polar-connection-tab-panel",
                                role: "tabpanel",
                                aria_label: "2. Polar",
                                    InstructionList {
                                        Instruction {
                                            id: "polar-step-bridge-url".to_string(),
                                            class: wizard_step_class(active_step, PolarWizardStep::BridgeUrl).to_string(),
                                            number: 1,
                                            info: "Default localhost bridge while Polar is open".to_string(),
                                            name: rsx! { "{polar_wizard_step_label(PolarWizardStep::BridgeUrl)}" },
                                            value: Some(rsx! {
                                                label { class: "setup-field-row",
                                                    input {
                                                        id: "polar-bridge-url-input",
                                                        r#type: "text",
                                                        placeholder: "http://localhost:37373",
                                                        value: polar_bridge_url(),
                                                        disabled: active_step != PolarWizardStep::BridgeUrl,
                                                        oninput: move |event| {
                                                            bridge_connection_error.set(String::new());
                                                            polar_bridge_url.set(event.value());
                                                        },
                                                    }
                                                }
                                            }),
                                            value_after: Some(rsx! {
                                                if !bridge_url_is_valid && active_step == PolarWizardStep::BridgeUrl {
                                                    p { class: "field-error", "Use a local bridge URL such as http://localhost:37373." }
                                                }
                                                if bridge_url_is_valid && !browser_origin_is_valid && active_step == PolarWizardStep::BridgeUrl {
                                                    p { class: "field-error",
                                                        "Open this app at "
                                                        a {
                                                            class: "setup-resource-link",
                                                            href: LOCAL_APP_URL,
                                                            "{LOCAL_APP_URL}"
                                                        }
                                                        " before connecting to Polar."
                                                    }
                                                }
                                                if !bridge_connection_error().is_empty() && active_step == PolarWizardStep::BridgeUrl {
                                                    p { class: "field-error", "{bridge_connection_error}" }
                                                }
                                            }),
                                            actions: Some(rsx! {
                                                button {
                                                    id: "polar-bridge-url-submit",
                                                    class: "primary-action",
                                                    r#type: "button",
                                                    disabled: is_busy() || active_step != PolarWizardStep::BridgeUrl || !bridge_url_can_submit,
                                                    onclick: move |_| async move {
                                                        is_busy.set(true);
                                                        let operation_id = begin_operation_prompt(
                                                            operation_prompt,
                                                            prompt_sequence,
                                                            "Connect Polar bridge",
                                                            "Checking bridge URL...",
                                                            false,
                                                        )
                                                        .await;

                                                        match profile_from_inputs(
                                                            amount_text(),
                                                            polar_server_name(),
                                                            SetupMode::ServerConfig,
                                                            polar_automation_from_input(
                                                                polar_bridge_url(),
                                                                setup_profile().polar_automation,
                                                            ),
                                                                setup_profile(),
                                                            ) {
                                                                Ok(mut profile) => {
                                                                    update_operation_prompt(
                                                                        operation_prompt,
                                                                        operation_id,
                                                                        "Contacting the Polar bridge...",
                                                                        ToastTone::Info,
                                                                        true,
                                                                        false,
                                                                    )
                                                                    .await;
                                                                    profile.connection_status = ConnectionStatus::SavedOffline;
                                                                    profile.last_verified_at = None;
                                                                    profile.polar_automation.network_id.clear();
                                                                    match verify_polar_bridge(profile.clone()).await {
                                                                        Ok(saved_profile) => {
                                                                            bridge_connection_error.set(String::new());
                                                                            setup_profile.set(saved_profile);
                                                                            lab_state.set(None);
                                                                            close_operation_prompt(operation_prompt, operation_id);
                                                                            push_toast(toast, toast_sequence, "Connected to Polar bridge.", ToastTone::Success);
                                                                            focus_step_control("polar-server-name-input").await;
                                                                        }
                                                                        Err(message) => {
                                                                            let message = bridge_step_error_message(message);
                                                                            bridge_connection_error.set(message.clone());
                                                                            close_operation_prompt(operation_prompt, operation_id);
                                                                            push_toast(toast, toast_sequence, message, ToastTone::Error);
                                                                        }
                                                                    }
                                                                }
                                                            Err(message) => {
                                                                close_operation_prompt(operation_prompt, operation_id);
                                                                push_toast(toast, toast_sequence, message, ToastTone::Error);
                                                            }
                                                        }

                                                        is_busy.set(false);
                                                    },
                                                    "SUBMIT"
                                                }
                                            }),
                                        }

                                        Instruction {
                                            id: "polar-step-server-name".to_string(),
                                            class: wizard_step_class(active_step, PolarWizardStep::ServerName).to_string(),
                                            number: 2,
                                            info: "App creates this Polar network".to_string(),
                                            name: rsx! { "{polar_wizard_step_label(PolarWizardStep::ServerName)}" },
                                            value: Some(rsx! {
                                                label { class: "setup-field-row",
                                                    input {
                                                        id: "polar-server-name-input",
                                                        r#type: "text",
                                                        value: polar_server_name(),
                                                        disabled: active_step != PolarWizardStep::ServerName,
                                                        oninput: move |event| polar_server_name.set(event.value()),
                                                    }
                                                }
                                            }),
                                            value_after: Some(rsx! {
                                                if !server_name_is_valid && active_step == PolarWizardStep::ServerName {
                                                    p { class: "field-error", "Enter a Polar server name." }
                                                }
                                            }),
                                            actions: Some(rsx! {
                                                button {
                                                    id: "polar-server-name-submit",
                                                    class: "primary-action",
                                                    r#type: "button",
                                                    disabled: is_busy() || active_step != PolarWizardStep::ServerName || !server_name_is_valid,
                                                    onclick: move |_| async move {
                                                        is_busy.set(true);
                                                        let operation_id = begin_operation_prompt(
                                                            operation_prompt,
                                                            prompt_sequence,
                                                            "Prepare Polar server",
                                                            "Checking server name...",
                                                            false,
                                                        )
                                                        .await;

                                                        match profile_from_inputs(
                                                            amount_text(),
                                                            polar_server_name(),
                                                            SetupMode::ServerConfig,
                                                            polar_automation_for_requested_server(
                                                                polar_bridge_url(),
                                                                polar_server_name(),
                                                                setup_profile().polar_automation,
                                                            ),
                                                                setup_profile(),
                                                            ) {
                                                                Ok(profile) => {
                                                                    update_operation_prompt(
                                                                        operation_prompt,
                                                                        operation_id,
                                                                        "Finding or creating the Polar server...",
                                                                        ToastTone::Info,
                                                                        true,
                                                                        false,
                                                                    )
                                                                    .await;
                                                                    match ensure_polar_server(profile.clone()).await {
                                                                    Ok(result) => {
                                                                        update_operation_prompt(
                                                                            operation_prompt,
                                                                            operation_id,
                                                                            "Saving Polar server connection...",
                                                                            ToastTone::Info,
                                                                            true,
                                                                            false,
                                                                        )
                                                                        .await;
                                                                        bridge_connection_error.set(String::new());
                                                                        let mut saved_profile = profile;
                                                                        saved_profile.polar_automation = result.profile;
                                                                        saved_profile.connection_status = ConnectionStatus::SavedOffline;
                                                                        saved_profile.polar_block_height_confirmed = false;
                                                                        saved_profile.game_treasury_ready = false;
                                                                        saved_profile.game_treasury_funded_sats = 0;
                                                                        saved_profile.last_verified_at = None;
                                                                        setup_profile.set(saved_profile);
                                                                        lab_state.set(None);

                                                                        let message = match result.status {
                                                                            PolarServerEnsureStatus::Existed => "Polar server already exists.",
                                                                            PolarServerEnsureStatus::Created => "Polar server created.",
                                                                        };
                                                                        close_operation_prompt(operation_prompt, operation_id);
                                                                        push_toast(toast, toast_sequence, message, ToastTone::Success);
                                                                        focus_step_control("polar-game-treasury-submit").await;
                                                                    }
                                                                    Err(message) => {
                                                                        if is_bridge_connection_error(&message) {
                                                                            let saved_profile = reset_to_bridge_url_step(setup_profile());
                                                                        setup_profile.set(saved_profile.clone());
                                                                        lab_state.set(None);
                                                                            polar_bridge_url.set(saved_profile.polar_automation.bridge_url.clone());
                                                                            polar_server_name.set(saved_profile.network_name.clone());
                                                                            let message = bridge_step_error_message(message);
                                                                            bridge_connection_error.set(message.clone());
                                                                            close_operation_prompt(operation_prompt, operation_id);
                                                                            push_toast(toast, toast_sequence, message, ToastTone::Error);
                                                                            focus_step_control("polar-bridge-url-input").await;
                                                                        } else {
                                                                            close_operation_prompt(operation_prompt, operation_id);
                                                                            push_toast(toast, toast_sequence, message, ToastTone::Error);
                                                                        }
                                                                    },
                                                                    }
                                                                },
                                                            Err(message) => {
                                                                close_operation_prompt(operation_prompt, operation_id);
                                                                push_toast(toast, toast_sequence, message, ToastTone::Error);
                                                            }
                                                        }

                                                        is_busy.set(false);
                                                    },
                                                    "SUBMIT"
                                                }
                                                button {
                                                    id: "polar-server-name-reset",
                                                    class: "secondary-action danger-action",
                                                    r#type: "button",
                                                    disabled: is_busy() || active_step != PolarWizardStep::ServerName,
                                                    onclick: move |_| async move {
                                                        is_busy.set(true);
                                                        let saved_profile = reset_to_bridge_url_step(setup_profile());
                                                        setup_profile.set(saved_profile.clone());
                                                        lab_state.set(None);
                                                        polar_bridge_url.set(saved_profile.polar_automation.bridge_url.clone());
                                                        polar_server_name.set(saved_profile.network_name.clone());
                                                        push_toast(toast, toast_sequence, "Returned to step 1.", ToastTone::Success);
                                                        focus_step_control("polar-bridge-url-input").await;

                                                        is_busy.set(false);
                                                    },
                                                    "RESET"
                                                }
                                            }),
                                        }

                                        Instruction {
                                            id: "polar-step-game-treasury".to_string(),
                                            class: wizard_step_class(active_step, PolarWizardStep::GameTreasury).to_string(),
                                            number: 3,
                                            info: "Funds the Game Treasury with sats for gameplay spending".to_string(),
                                            name: rsx! { "{polar_wizard_step_label(PolarWizardStep::GameTreasury)}" },
                                            value: Some(rsx! {
                                                if setup_profile().game_treasury_ready {
                                                    div { class: "tra-setup-status", role: "status",
                                                        span { "Game Treasury was funded with all sats for use in gameplay." }
                                                    }
                                                } else {
                                                    label { class: "setup-field-row",
                                                        input {
                                                            id: "polar-game-treasury-input",
                                                            r#type: "text",
                                                            value: "Game Treasury is funded with all sats for use in gameplay.",
                                                            readonly: true,
                                                            disabled: active_step != PolarWizardStep::GameTreasury,
                                                        }
                                                    }
                                                }
                                            }),
                                            actions: Some(rsx! {
                                                button {
                                                    id: "polar-game-treasury-submit",
                                                    class: "primary-action",
                                                    r#type: "button",
                                                    disabled: is_busy() || active_step != PolarWizardStep::GameTreasury,
                                                    onclick: move |_| async move {
                                                        is_busy.set(true);
                                                        let operation_id = begin_operation_prompt(
                                                            operation_prompt,
                                                            prompt_sequence,
                                                            "Game Treasury (Sats)",
                                                            "Creating and funding the Game Treasury house node...",
                                                            false,
                                                        )
                                                        .await;

                                                        match profile_from_inputs(
                                                            amount_text(),
                                                            polar_server_name(),
                                                            SetupMode::ServerConfig,
                                                            polar_automation_from_input(
                                                                polar_bridge_url(),
                                                                setup_profile().polar_automation,
                                                            ),
                                                            setup_profile(),
                                                        ) {
                                                            Ok(profile) => match prepare_game_treasury(profile).await {
                                                                Ok(state) => {
                                                                    setup_profile.set(state.profile.clone());
                                                                    lab_state.set(Some(state));
                                                                    close_operation_prompt(operation_prompt, operation_id);
                                                                    push_toast(toast, toast_sequence, "Game Treasury sats ready.", ToastTone::Success);
                                                                    focus_step_control("polar-game-treasury-tras-submit").await;
                                                                }
                                                                Err(message) => {
                                                                    close_operation_prompt(operation_prompt, operation_id);
                                                                    push_toast(toast, toast_sequence, message, ToastTone::Error);
                                                                }
                                                            },
                                                            Err(message) => {
                                                                close_operation_prompt(operation_prompt, operation_id);
                                                                push_toast(toast, toast_sequence, message, ToastTone::Error);
                                                            }
                                                        }

                                                        is_busy.set(false);
                                                    },
                                                    "SUBMIT"
                                                }
                                                button {
                                                    id: "polar-game-treasury-reset",
                                                    class: "secondary-action danger-action",
                                                    r#type: "button",
                                                    disabled: is_busy() || active_step != PolarWizardStep::GameTreasury,
                                                    onclick: move |_| async move {
                                                        is_busy.set(true);
                                                        let saved_profile = reset_to_server_name_step(setup_profile());
                                                        setup_profile.set(saved_profile.clone());
                                                        lab_state.set(None);
                                                        push_toast(toast, toast_sequence, "Returned to step 2.", ToastTone::Success);
                                                        focus_step_control("polar-server-name-input").await;
                                                        is_busy.set(false);
                                                    },
                                                    "RESET"
                                                }
                                            }),
                                        }

                                        Instruction {
                                            id: "polar-step-game-treasury-tras".to_string(),
                                            class: wizard_step_class(active_step, PolarWizardStep::GameTreasuryTras).to_string(),
                                            number: 4,
                                            info: "Gifts the Game Treasury with TRA items for gameplay inventory".to_string(),
                                            name: rsx! { "{polar_wizard_step_label(PolarWizardStep::GameTreasuryTras)}" },
                                            value: Some(rsx! {
                                                if treasury_tras_ready(current_lab_state.as_ref()) {
                                                    div { class: "tra-setup-status", role: "status",
                                                        span { "Game Treasury was gifted with all TRA items for use in gameplay." }
                                                    }
                                                } else {
                                                    label { class: "setup-field-row",
                                                        input {
                                                            id: "polar-game-treasury-tras-input",
                                                            r#type: "text",
                                                            value: "Game Treasury is gifted with all TRA items for use in gameplay.",
                                                            readonly: true,
                                                            disabled: active_step != PolarWizardStep::GameTreasuryTras,
                                                        }
                                                    }
                                                }
                                            }),
                                            actions: Some(rsx! {
                                                button {
                                                    id: "polar-game-treasury-tras-submit",
                                                    class: "primary-action",
                                                    r#type: "button",
                                                    disabled: is_busy() || active_step != PolarWizardStep::GameTreasuryTras,
                                                    onclick: move |_| async move {
                                                        prepare_treasury_tras_step(
                                                            is_busy,
                                                            setup_profile,
                                                            lab_state,
                                                            operation_prompt,
                                                            prompt_sequence,
                                                            toast,
                                                            toast_sequence,
                                                        )
                                                        .await;
                                                    },
                                                    "SUBMIT"
                                                }
                                                button {
                                                    id: "polar-game-treasury-tras-reset",
                                                    class: "secondary-action danger-action",
                                                    r#type: "button",
                                                    disabled: is_busy() || active_step != PolarWizardStep::GameTreasuryTras,
                                                    onclick: move |_| async move {
                                                        is_busy.set(true);
                                                        let saved_profile = reset_to_game_treasury_step(setup_profile());
                                                        setup_profile.set(saved_profile.clone());
                                                        lab_state.set(None);
                                                        push_toast(toast, toast_sequence, "Returned to step 3.", ToastTone::Success);
                                                        focus_step_control("polar-game-treasury-submit").await;
                                                        is_busy.set(false);
                                                    },
                                                    "RESET"
                                                }
                                            }),
                                        }

                                        Instruction {
                                            id: "polar-step-user-nodes".to_string(),
                                            class: wizard_step_class(active_step, PolarWizardStep::UserNodes).to_string(),
                                            number: 5,
                                            info: "App creates the user LND nodes".to_string(),
                                            name: rsx! { "{polar_wizard_step_label(PolarWizardStep::UserNodes)}" },
                                            value: Some(rsx! {
                                                label { class: "setup-field-row",
                                                    input {
                                                        id: "polar-user-nodes-input",
                                                        r#type: "text",
                                                        value: "Alice, Bob, Carol",
                                                        readonly: true,
                                                    }
                                                }
                                            }),
                                            actions: Some(rsx! {
                                                button {
                                                    id: POLAR_DEMO_NODES_SUBMIT_ID,
                                                    class: "primary-action",
                                                    r#type: "button",
                                                        disabled: is_busy() || active_step != PolarWizardStep::UserNodes,
                                                    onclick: move |_| async move {
                                                        create_demo_nodes_step(
                                                            is_busy,
                                                            setup_profile,
                                                            lab_state,
                                                            operation_prompt,
                                                            prompt_sequence,
                                                            toast,
                                                            toast_sequence,
                                                            bridge_connection_error,
                                                            amount_text(),
                                                            polar_server_name(),
                                                            polar_bridge_url(),
                                                            polar_block_height,
                                                        ).await;
                                                    },
                                                    "SUBMIT"
                                                }
                                                button {
                                                    id: "polar-user-nodes-reset",
                                                    class: "secondary-action danger-action",
                                                    r#type: "button",
                                                    disabled: is_busy() || active_step != PolarWizardStep::UserNodes,
                                                    onclick: move |_| async move {
                                                        is_busy.set(true);
                                                        match profile_from_inputs(
                                                            amount_text(),
                                                            polar_server_name(),
                                                            SetupMode::ServerConfig,
                                                            polar_automation_from_input(
                                                                polar_bridge_url(),
                                                                setup_profile().polar_automation,
                                                            ),
                                                            setup_profile(),
                                                        ) {
                                                            Ok(profile) => {
                                                                let saved_profile = reset_to_game_treasury_tras_step(profile);
                                                                setup_profile.set(saved_profile.clone());
                                                                lab_state.set(lab_state_after_reset_to_game_treasury_tras_step());
                                                                polar_bridge_url.set(saved_profile.polar_automation.bridge_url.clone());
                                                                polar_server_name.set(saved_profile.network_name.clone());
                                                                push_toast(toast, toast_sequence, "Returned to step 4.", ToastTone::Success);
                                                                focus_step_control("polar-game-treasury-tras-submit").await;
                                                            }
                                                            Err(message) => push_toast(toast, toast_sequence, message, ToastTone::Error),
                                                        }

                                                        is_busy.set(false);
                                                    },
                                                    "RESET"
                                                }
                                            }),
                                        }

                                        Instruction {
                                            id: "polar-step-tra-assets".to_string(),
                                            class: wizard_step_class(active_step, PolarWizardStep::NpcItemTransfers).to_string(),
                                            number: 6,
                                            info: "Transfers starting items from Game Treasury to Bob and Carol".to_string(),
                                            name: rsx! { "{polar_wizard_step_label(PolarWizardStep::NpcItemTransfers)}" },
                                            value: Some(rsx! {
                                                if let Some(state) = current_lab_state.as_ref() {
                                                    NpcItemTransferStatus { transfers: state.npc_item_transfers.clone() }
                                                } else {
                                                    label { class: "setup-field-row",
                                                        input {
                                                            id: "polar-tra-assets-input",
                                                            r#type: "text",
                                                            value: "Bob and Carol starting items come from Game Treasury.",
                                                            readonly: true,
                                                            disabled: active_step != PolarWizardStep::NpcItemTransfers,
                                                        }
                                                    }
                                                }
                                            }),
                                            actions: Some(rsx! {
                                                button {
                                                    id: "polar-tra-assets-submit",
                                                    class: "primary-action",
                                                    r#type: "button",
                                                    disabled: is_busy() || active_step != PolarWizardStep::NpcItemTransfers,
                                                    onclick: move |_| async move {
                                                        prepare_tra_inventory_step(
                                                            is_busy,
                                                            setup_profile,
                                                            lab_state,
                                                            operation_prompt,
                                                            prompt_sequence,
                                                            toast,
                                                            toast_sequence,
                                                        )
                                                        .await;
                                                    },
                                                    "SUBMIT"
                                                }
                                                button {
                                                    id: "polar-tra-assets-reset",
                                                    class: "secondary-action danger-action",
                                                    r#type: "button",
                                                    disabled: is_busy() || active_step != PolarWizardStep::NpcItemTransfers,
                                                    onclick: move |_| async move {
                                                        is_busy.set(true);
                                                        let saved_profile = reset_to_demo_nodes_step(setup_profile());
                                                        setup_profile.set(saved_profile.clone());
                                                        lab_state.set(lab_state_after_reset_to_demo_nodes_step(saved_profile.clone()));
                                                        push_toast(toast, toast_sequence, "Returned to step 4. NPC item transfers will be recreated on submit.", ToastTone::Success);
                                                        focus_step_control(POLAR_DEMO_NODES_SUBMIT_ID).await;
                                                        is_busy.set(false);
                                                    },
                                                    "RESET"
                                                }
                                            }),
                                        }

                                        Instruction {
                                            id: "polar-step-block-height".to_string(),
                                            class: wizard_step_class(active_step, PolarWizardStep::BlockHeight).to_string(),
                                            number: 7,
                                            info: "Sets the game block-height baseline".to_string(),
                                            name: rsx! { "{polar_wizard_step_label(PolarWizardStep::BlockHeight)}" },
                                            value: Some(rsx! {
                                                label { class: "setup-field-row",
                                                    input {
                                                        id: "polar-block-height-input",
                                                        r#type: "number",
                                                        min: "0",
                                                        step: "1",
                                                        value: polar_block_height(),
                                                        disabled: active_step != PolarWizardStep::BlockHeight,
                                                        oninput: move |event| polar_block_height.set(event.value()),
                                                    }
                                                }
                                            }),
                                            actions: Some(rsx! {
                                                button {
                                                    id: "polar-block-height-submit",
                                                    class: "primary-action",
                                                    r#type: "button",
                                                    disabled: is_busy() || active_step != PolarWizardStep::BlockHeight,
                                                    onclick: move |_| async move {
                                                        is_busy.set(true);
                                                        let operation_id = begin_operation_prompt(
                                                            operation_prompt,
                                                            prompt_sequence,
                                                            "Set block height",
                                                            "Checking requested block height...",
                                                            false,
                                                        )
                                                        .await;

                                                        match block_height_from_input(polar_block_height()) {
                                                            Ok(block_height) => match profile_from_inputs(
                                                                amount_text(),
                                                                polar_server_name(),
                                                                SetupMode::ServerConfig,
                                                                polar_automation_from_input(
                                                                    polar_bridge_url(),
                                                                    setup_profile().polar_automation,
                                                                ),
                                                                setup_profile(),
                                                            ) {
                                                                Ok(profile) => {
                                                                    update_operation_prompt(
                                                                        operation_prompt,
                                                                        operation_id,
                                                                        format!("Saving app baseline block height {block_height}..."),
                                                                        ToastTone::Info,
                                                                        true,
                                                                        false,
                                                                    )
                                                                    .await;
                                                                    match confirm_polar_block_height(profile, block_height).await {
                                                                        Ok(state) => {
                                                                            update_operation_prompt(
                                                                                operation_prompt,
                                                                                operation_id,
                                                                                format!("Block Height saved as {block_height}."),
                                                                                ToastTone::Info,
                                                                                true,
                                                                                false,
                                                                            )
                                                                            .await;
                                                                            polar_block_height.set(state.block_height.to_string());
                                                                            setup_profile.set(state.profile.clone());
                                                                            lab_state.set(Some(state));
                                                                            close_operation_prompt(operation_prompt, operation_id);
                                                                            push_toast(toast, toast_sequence, "Block Height sent", ToastTone::Success);
                                                                            focus_step_control("polar-complete-submit").await;
                                                                        }
                                                                        Err(message) => {
                                                                            close_operation_prompt(operation_prompt, operation_id);
                                                                            push_toast(toast, toast_sequence, message, ToastTone::Error);
                                                                        },
                                                                    }
                                                                },
                                                                Err(message) => {
                                                                    close_operation_prompt(operation_prompt, operation_id);
                                                                    push_toast(toast, toast_sequence, message, ToastTone::Error);
                                                                },
                                                            },
                                                            Err(message) => {
                                                                close_operation_prompt(operation_prompt, operation_id);
                                                                push_toast(toast, toast_sequence, message, ToastTone::Error);
                                                            },
                                                        }

                                                        is_busy.set(false);
                                                    },
                                                    "SUBMIT"
                                                }
                                                button {
                                                    id: "polar-block-height-reset",
                                                    class: "secondary-action danger-action",
                                                    r#type: "button",
                                                    disabled: is_busy() || active_step != PolarWizardStep::BlockHeight,
                                                    onclick: move |_| async move {
                                                        is_busy.set(true);
                                                        let saved_profile = reset_to_npc_item_transfers_step(setup_profile());
                                                        setup_profile.set(saved_profile.clone());
                                                        if let Some(mut state) = lab_state().or_else(storage_service::load_lab_state_snapshot) {
                                                            state.profile = saved_profile;
                                                            state.npc_item_transfers.clear();
                                                            if let Ok(next_state) = lightning_service::TraService::prepare_game_treasury_items(state.clone()) {
                                                                state = next_state;
                                                            }
                                                            storage_service::save_lab_state_snapshot(&state);
                                                            lab_state.set(Some(state));
                                                        }
                                                        push_toast(toast, toast_sequence, "Returned to step 6. NPC Item Transfers will be recreated on submit.", ToastTone::Success);
                                                        focus_step_control("polar-tra-assets-submit").await;
                                                        is_busy.set(false);
                                                    },
                                                    "RESET"
                                                }
                                            }),
                                        }

                                        Instruction {
                                            id: "polar-step-complete".to_string(),
                                            class: wizard_step_class(active_step, PolarWizardStep::Complete).to_string(),
                                            number: 8,
                                            info: "Saves setup as connected".to_string(),
                                            name: rsx! { "{polar_wizard_step_label(PolarWizardStep::Complete)}" },
                                            value: Some(rsx! {
                                                label { class: "setup-field-row",
                                                    input {
                                                        id: "polar-complete-input",
                                                        r#type: "text",
                                                        value: "Play Game, Network Dashboard",
                                                        readonly: true,
                                                    }
                                                }
                                            }),
                                            actions: Some(rsx! {
                                                button {
                                                    id: "polar-complete-submit",
                                                    class: "primary-action",
                                                    r#type: "button",
                                                    disabled: is_busy() || active_step != PolarWizardStep::Complete,
                                                    onclick: move |_| async move {
                                                        is_busy.set(true);
                                                        let operation_id = begin_operation_prompt(
                                                            operation_prompt,
                                                            prompt_sequence,
                                                            "Unlock routes",
                                                            "Checking final setup state...",
                                                            false,
                                                        )
                                                        .await;

                                                        match profile_from_inputs(
                                                            amount_text(),
                                                            polar_server_name(),
                                                            SetupMode::ServerConfig,
                                                            polar_automation_from_input(
                                                                polar_bridge_url(),
                                                                setup_profile().polar_automation,
                                                            ),
                                                            setup_profile(),
                                                        ) {
                                                            Ok(profile) => {
                                                                update_operation_prompt(
                                                                    operation_prompt,
                                                                    operation_id,
                                                                    "Saving connected setup and unlocking routes...",
                                                                    ToastTone::Info,
                                                                    true,
                                                                    false,
                                                                )
                                                                .await;
                                                                match complete_polar_setup(profile).await {
                                                                Ok(state) => {
                                                                    bridge_connection_error.set(String::new());
                                                                    setup_profile.set(state.profile.clone());
                                                                    lab_state.set(Some(state));
                                                                    close_operation_prompt(operation_prompt, operation_id);
                                                                    push_toast(toast, toast_sequence, "Unlock routes sent", ToastTone::Success);
                                                                }
                                                                Err(message) => {
                                                                    if is_bridge_connection_error(&message) {
                                                                        let saved_profile = reset_to_bridge_url_step(setup_profile());
                                                                        setup_profile.set(saved_profile.clone());
                                                                        lab_state.set(None);
                                                                        polar_bridge_url.set(saved_profile.polar_automation.bridge_url.clone());
                                                                        polar_server_name.set(saved_profile.network_name.clone());
                                                                        let message = bridge_step_error_message(message);
                                                                        bridge_connection_error.set(message.clone());
                                                                        close_operation_prompt(operation_prompt, operation_id);
                                                                        push_toast(toast, toast_sequence, message, ToastTone::Error);
                                                                        focus_step_control("polar-bridge-url-input").await;
                                                                    } else {
                                                                        close_operation_prompt(operation_prompt, operation_id);
                                                                        push_toast(toast, toast_sequence, message, ToastTone::Error);
                                                                    }
                                                                },
                                                                }
                                                            },
                                                            Err(message) => {
                                                                close_operation_prompt(operation_prompt, operation_id);
                                                                push_toast(toast, toast_sequence, message, ToastTone::Error);
                                                            }
                                                        }

                                                        is_busy.set(false);
                                                    },
                                                    "SUBMIT"
                                                }
                                                button {
                                                    id: "polar-complete-reset",
                                                    class: "secondary-action danger-action",
                                                    r#type: "button",
                                                    disabled: is_busy() || (active_step != PolarWizardStep::Complete && active_step != PolarWizardStep::Done),
                                                    onclick: move |_| show_complete_reset_confirm.set(true),
                                                    "RESET"
                                                }
                                            }),
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            }
            if show_complete_reset_confirm() {
                CompleteResetConfirmationPrompt {
                    is_busy,
                    show_prompt: show_complete_reset_confirm,
                    setup_profile,
                    lab_state,
                    operation_prompt,
                    toast,
                    toast_sequence,
                }
            }
        }
    }
}

#[component]
fn CompleteResetConfirmationPrompt(
    mut is_busy: Signal<bool>,
    mut show_prompt: Signal<bool>,
    setup_profile: Signal<SetupProfile>,
    lab_state: Signal<Option<LabState>>,
    operation_prompt: Signal<Option<OperationPrompt>>,
    toast: Signal<Option<Toast>>,
    toast_sequence: Signal<u64>,
) -> Element {
    rsx! {
        div {
            class: "operation-prompt-backdrop",
            role: "presentation",
            div {
                class: "operation-prompt operation-prompt--error",
                role: "dialog",
                aria_modal: "true",
                aria_label: "Are you sure?",
                div { class: "operation-prompt__status" }
                div { class: "operation-prompt__body",
                    span { class: "eyebrow", "Action required" }
                    h2 { "Are you sure?" }
                    p { "The app is properly running. Resetting this step will lock Play Game and Network Dashboard until setup is unlocked again." }
                }
                div { class: "operation-prompt__actions",
                    button {
                        class: "secondary-action",
                        r#type: "button",
                        disabled: is_busy(),
                        onclick: move |_| show_prompt.set(false),
                        "Cancel"
                    }
                    button {
                        class: "secondary-action danger-action",
                        r#type: "button",
                        disabled: is_busy(),
                        onclick: move |_| async move {
                            show_prompt.set(false);
                            reset_complete_step(
                                is_busy,
                                setup_profile,
                                lab_state,
                                operation_prompt,
                                toast,
                                toast_sequence,
                            ).await;
                        },
                        "Reset"
                    }
                }
            }
        }
    }
}

#[component]
fn InstructionList(#[props(default = String::new())] class: String, children: Element) -> Element {
    let class = if class.trim().is_empty() {
        "wizard-step-list".to_string()
    } else {
        format!("wizard-step-list {}", class.trim())
    };

    rsx! {
        div { class,
            {children}
        }
    }
}

#[component]
fn Instruction(
    number: u8,
    info: String,
    name: Element,
    #[props(default = String::new())] id: String,
    #[props(default = "wizard-step".to_string())] class: String,
    #[props(default = None)] value: Option<Element>,
    #[props(default = None)] value_after: Option<Element>,
    #[props(default = None)] actions: Option<Element>,
) -> Element {
    let has_value = value.is_some();
    let class = if actions.is_some() {
        format!("{} instruction instruction--with-actions", class)
    } else {
        format!("{} instruction", class)
    };
    let body_class = if has_value {
        "instruction__body instruction__body--with-value"
    } else {
        "instruction__body"
    };
    let value = value.map(|value| {
        rsx! {
            div { class: "instruction__value",
                {value}
            }
        }
    });
    let value_after = value_after.map(|value_after| {
        rsx! {
            div { class: "instruction__value-after",
                {value_after}
            }
        }
    });
    let actions = actions.map(|actions| {
        rsx! {
            div { class: "wizard-step__actions",
                {actions}
            }
        }
    });

    rsx! {
        div { id, class,
            div { class: "wizard-step__number", "{number}" }
            div { class: body_class,
                div { class: "instruction__name",
                    span { class: "instruction__name-text",
                        {name}
                    }
                    FieldHelpIcon { label: info }
                }
                {value}
                {value_after}
            }
            {actions}
        }
    }
}

#[cfg(target_arch = "wasm32")]
async fn focus_step_control(id: &'static str) {
    for _ in 0..FOCUS_RETRY_ATTEMPTS {
        gloo_timers::future::TimeoutFuture::new(FOCUS_RETRY_DELAY_MS).await;

        let Some(window) = web_sys::window() else {
            return;
        };
        let Some(document) = window.document() else {
            return;
        };
        let Some(element) = document.get_element_by_id(id) else {
            continue;
        };

        if element.has_attribute("disabled") {
            continue;
        }

        let Some(element) = element.dyn_ref::<web_sys::HtmlElement>() else {
            return;
        };

        let _ = element.focus();
        return;
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn focus_step_control(_id: &'static str) {}

#[cfg(test)]
fn submit_focus_target(step: PolarWizardStep) -> Option<&'static str> {
    match step {
        PolarWizardStep::BridgeUrl => Some("polar-server-name-input"),
        PolarWizardStep::ServerName => Some("polar-game-treasury-submit"),
        PolarWizardStep::GameTreasury => Some("polar-game-treasury-tras-submit"),
        PolarWizardStep::GameTreasuryTras => Some(POLAR_DEMO_NODES_SUBMIT_ID),
        PolarWizardStep::UserNodes => Some("polar-tra-assets-submit"),
        PolarWizardStep::NpcItemTransfers => Some("polar-block-height-input"),
        PolarWizardStep::BlockHeight => Some("polar-complete-submit"),
        PolarWizardStep::Complete | PolarWizardStep::Done => None,
    }
}

fn reset_focus_target(step: PolarWizardStep) -> &'static str {
    match step {
        PolarWizardStep::BridgeUrl | PolarWizardStep::ServerName => "polar-bridge-url-input",
        PolarWizardStep::GameTreasury => "polar-server-name-input",
        PolarWizardStep::GameTreasuryTras => "polar-game-treasury-submit",
        PolarWizardStep::UserNodes => "polar-game-treasury-tras-submit",
        PolarWizardStep::NpcItemTransfers => POLAR_DEMO_NODES_SUBMIT_ID,
        PolarWizardStep::BlockHeight => "polar-tra-assets-submit",
        PolarWizardStep::Complete | PolarWizardStep::Done => "polar-block-height-input",
    }
}

fn complete_reset_focus_target() -> &'static str {
    reset_focus_target(PolarWizardStep::Complete)
}

#[cfg(target_arch = "wasm32")]
fn schedule_step_control_focus(id: &'static str) {
    wasm_bindgen_futures::spawn_local(async move {
        focus_step_control(id).await;
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn schedule_step_control_focus(_id: &'static str) {}

fn profile_from_inputs(
    amount_text: String,
    network_name: String,
    setup_mode: SetupMode,
    polar_automation: PolarAutomationProfile,
    current_profile: SetupProfile,
) -> Result<SetupProfile, String> {
    let amount = amount_text
        .trim()
        .replace(',', "")
        .parse::<u64>()
        .map_err(|_| "Sats per transaction must be a whole number.".to_string())?;
    let network_name = network_name.trim().to_string();

    if setup_mode == SetupMode::ServerConfig && network_name.is_empty() {
        return Err("Enter a Polar server name.".to_string());
    }

    let mut profile = current_profile;
    profile.sats_per_transaction = amount;
    profile.network_name = network_name;
    profile.setup_mode = setup_mode;
    profile.polar_automation = polar_automation;

    Ok(profile)
}

fn block_height_from_input(block_height: String) -> Result<u64, String> {
    block_height
        .trim()
        .replace(',', "")
        .parse::<u64>()
        .map_err(|_| "Block Height must be a whole number.".to_string())
}

fn polar_automation_from_input(
    bridge_url: String,
    current: PolarAutomationProfile,
) -> PolarAutomationProfile {
    PolarAutomationProfile {
        bridge_url: bridge_url.trim().to_string(),
        network_id: current.network_id,
        bitcoin_backend_name: current.bitcoin_backend_name,
    }
}

fn polar_automation_for_requested_server(
    bridge_url: String,
    server_name: String,
    current: PolarAutomationProfile,
) -> PolarAutomationProfile {
    PolarAutomationProfile {
        bridge_url: bridge_url.trim().to_string(),
        network_id: server_name.trim().to_string(),
        bitcoin_backend_name: current.bitcoin_backend_name,
    }
}

fn polar_wizard_step(profile: &SetupProfile, lab_state: Option<&LabState>) -> PolarWizardStep {
    if profile.is_connected() {
        return PolarWizardStep::Done;
    }

    if profile.connection_status != ConnectionStatus::SavedOffline
        && profile.connection_status != ConnectionStatus::PartiallyConnected
        && !lab_state_has_status(lab_state, ConnectionStatus::PartiallyConnected)
    {
        return PolarWizardStep::BridgeUrl;
    }

    if profile.polar_automation.network_id.trim().is_empty() {
        return PolarWizardStep::ServerName;
    }

    if !profile.game_treasury_ready {
        return PolarWizardStep::GameTreasury;
    }

    if !treasury_tras_ready(lab_state) {
        return PolarWizardStep::GameTreasuryTras;
    }

    if !user_nodes_ready(lab_state) {
        return PolarWizardStep::UserNodes;
    }

    if !npc_item_transfers_ready(lab_state) {
        return PolarWizardStep::NpcItemTransfers;
    }

    if !profile.polar_block_height_confirmed {
        return PolarWizardStep::BlockHeight;
    }

    PolarWizardStep::Complete
}

fn lab_state_has_status(lab_state: Option<&LabState>, status: ConnectionStatus) -> bool {
    match lab_state {
        Some(state) => state.profile.connection_status == status,
        None => false,
    }
}

fn npc_item_transfers_ready(lab_state: Option<&LabState>) -> bool {
    lab_state
        .map(|state| {
            let bob_books = verified_tra_count(state, DemoNodeId::Bob, BOOK_ITEM_ID);
            let carol_apples = verified_tra_count(state, DemoNodeId::Carol, APPLE_ITEM_ID);
            let bob_book_transfers =
                successful_npc_transfer_count(state, DemoNodeId::Bob, BOOK_ITEM_ID);
            let carol_apple_transfers =
                successful_npc_transfer_count(state, DemoNodeId::Carol, APPLE_ITEM_ID);

            bob_books >= 2
                && carol_apples >= 2
                && bob_book_transfers >= 2
                && carol_apple_transfers >= 2
        })
        .unwrap_or(false)
}

fn treasury_tras_ready(lab_state: Option<&LabState>) -> bool {
    if npc_item_transfers_ready(lab_state) {
        return true;
    }

    lab_state
        .map(|state| {
            let treasury_books =
                verified_tra_count(state, DemoNodeId::GameTreasury, BOOK_ITEM_ID);
            let treasury_apples =
                verified_tra_count(state, DemoNodeId::GameTreasury, APPLE_ITEM_ID);

            treasury_books >= 2 && treasury_apples >= 2
        })
        .unwrap_or(false)
}

fn user_nodes_ready(lab_state: Option<&LabState>) -> bool {
    lab_state
        .map(|state| {
            DemoNodeId::ALL.into_iter().all(|node_id| {
                state.nodes.iter().any(|node| {
                    node.node_id == node_id
                        && node.status == crate::client::models::NodeStatus::Online
                        && node.pubkey.is_some()
                })
            })
        })
        .unwrap_or(false)
}

fn verified_tra_count(state: &LabState, owner_node: DemoNodeId, item_id: u32) -> usize {
    state
        .tra_items
        .iter()
        .filter(|item| {
            item.owner_node == owner_node
                && item.item_id == item_id
                && item.ownership_status == crate::client::models::TraOwnershipStatus::Verified
        })
        .count()
}

fn successful_npc_transfer_count(state: &LabState, destination: DemoNodeId, item_id: u32) -> usize {
    state
        .npc_item_transfers
        .iter()
        .filter(|transfer| {
            transfer.destination == destination
                && transfer.item_id == item_id
                && transfer.status == crate::client::models::TraTransferStatus::Succeeded
        })
        .count()
}

fn wizard_step_class(active_step: PolarWizardStep, step: PolarWizardStep) -> &'static str {
    if active_step == PolarWizardStep::Done || active_step.order() > step.order() {
        "wizard-step wizard-step--complete"
    } else if active_step == step {
        "wizard-step wizard-step--current"
    } else {
        "wizard-step wizard-step--locked"
    }
}

fn is_valid_local_bridge_url(bridge_url: &str) -> bool {
    PolarAutomationProfile::is_valid_local_bridge_url(bridge_url)
}

#[cfg(target_arch = "wasm32")]
fn browser_origin_allows_polar_bridge() -> bool {
    web_sys::window()
        .and_then(|window| window.location().hostname().ok())
        .map(|hostname| browser_hostname_allows_polar_bridge(&hostname))
        .unwrap_or(false)
}

#[cfg(any(target_arch = "wasm32", test))]
fn browser_hostname_allows_polar_bridge(hostname: &str) -> bool {
    hostname.eq_ignore_ascii_case("localhost") || hostname == "127.0.0.1"
}

#[cfg(not(target_arch = "wasm32"))]
fn browser_origin_allows_polar_bridge() -> bool {
    true
}

fn is_bridge_connection_error(message: &str) -> bool {
    message.contains("Cannot reach Polar bridge")
        || message.contains("/health")
        || message.contains("Failed to fetch")
}

fn bridge_step_error_message(_message: impl AsRef<str>) -> String {
    "Error: Cannot connect to Polar, revisit 1. Environment step 04 for more info".to_string()
}

fn push_toast(
    mut toast: Signal<Option<Toast>>,
    mut sequence: Signal<u64>,
    message: impl Into<String>,
    tone: ToastTone,
) {
    let next_id = *sequence.peek() + 1;
    sequence.set(next_id);
    toast.set(Some(Toast {
        id: next_id,
        message: message.into(),
        tone,
    }));
}

fn restore_saved_setup(profile: &SetupProfile, lab_state: Option<&LabState>) {
    storage_service::save_setup_profile(profile);
    match lab_state {
        Some(state) => storage_service::save_lab_state_snapshot(state),
        None => storage_service::clear_lab_state_snapshot(),
    }
}

fn reset_to_bridge_url_step(mut profile: SetupProfile) -> SetupProfile {
    profile.connection_status = ConnectionStatus::NotConfigured;
    profile.polar_block_height_confirmed = false;
    profile.last_verified_at = None;
    profile.polar_automation.network_id.clear();
    profile.polar_automation.bitcoin_backend_name = DEFAULT_BITCOIN_BACKEND_NAME.to_string();
    storage_service::save_setup_profile(&profile);
    storage_service::clear_lab_state_snapshot();
    profile
}

fn reset_to_server_name_step(mut profile: SetupProfile) -> SetupProfile {
    profile.connection_status = ConnectionStatus::SavedOffline;
    profile.polar_block_height_confirmed = false;
    profile.game_treasury_ready = false;
    profile.game_treasury_funded_sats = 0;
    profile.last_verified_at = None;
    profile.polar_automation.network_id.clear();
    storage_service::save_setup_profile(&profile);
    storage_service::clear_lab_state_snapshot();
    profile
}

fn reset_to_game_treasury_step(mut profile: SetupProfile) -> SetupProfile {
    profile.connection_status = ConnectionStatus::SavedOffline;
    profile.polar_block_height_confirmed = false;
    profile.game_treasury_ready = false;
    profile.game_treasury_funded_sats = 0;
    profile.last_verified_at = None;
    if profile.polar_automation.network_id.trim().is_empty() {
        profile.polar_automation.network_id = profile.network_name.trim().to_string();
    }

    storage_service::save_setup_profile(&profile);
    storage_service::clear_lab_state_snapshot();

    profile
}

fn reset_to_game_treasury_tras_step(mut profile: SetupProfile) -> SetupProfile {
    profile.connection_status = ConnectionStatus::PartiallyConnected;
    profile.polar_block_height_confirmed = false;
    profile.last_verified_at = None;
    if profile.polar_automation.network_id.trim().is_empty() {
        profile.polar_automation.network_id = profile.network_name.trim().to_string();
    }

    storage_service::save_setup_profile(&profile);

    profile
}

fn lab_state_after_reset_to_game_treasury_tras_step() -> Option<LabState> {
    let current = storage_service::load_lab_state_snapshot();
    let next = current.map(|mut state| {
        state.nodes.retain(|node| node.node_id == DemoNodeId::GameTreasury);
        state.tra_items.clear();
        state.game_treasury.owned_items.clear();
        state.game_treasury.inventory_value_sats = 0;
        state.npc_item_transfers.clear();
        state.profile.connection_status = ConnectionStatus::PartiallyConnected;
        state.profile.polar_block_height_confirmed = false;
        state
    });

    match next.as_ref() {
        Some(state) => storage_service::save_lab_state_snapshot(state),
        None => storage_service::clear_lab_state_snapshot(),
    }

    next
}

fn reset_to_demo_nodes_step(mut profile: SetupProfile) -> SetupProfile {
    profile.connection_status = ConnectionStatus::PartiallyConnected;
    profile.polar_block_height_confirmed = false;
    profile.last_verified_at = None;
    if profile.polar_automation.network_id.trim().is_empty() {
        profile.polar_automation.network_id = profile.network_name.trim().to_string();
    }

    storage_service::save_setup_profile(&profile);
    storage_service::clear_lab_state_snapshot();

    profile
}

fn lab_state_after_reset_to_demo_nodes_step(profile: SetupProfile) -> Option<LabState> {
    let current = storage_service::load_lab_state_snapshot();
    let mut state = current.unwrap_or_else(|| lightning_service::default_lab_state(profile.clone()));
    state.profile = profile;
    state.nodes.retain(|node| node.node_id == DemoNodeId::GameTreasury);
    state.npc_item_transfers.clear();
    if state.game_treasury.status != crate::client::models::TreasuryStatus::Ready {
        state = lightning_service::TraService::prepare_game_treasury(state).ok()?;
    }
    let state = lightning_service::TraService::prepare_game_treasury_items(state).ok()?;

    storage_service::save_lab_state_snapshot(&state);
    Some(state)
}

fn reset_to_block_height_step(mut profile: SetupProfile) -> SetupProfile {
    profile.connection_status = ConnectionStatus::PartiallyConnected;
    profile.polar_block_height_confirmed = false;
    profile.last_verified_at = None;
    if profile.polar_automation.network_id.trim().is_empty() {
        profile.polar_automation.network_id = profile.network_name.trim().to_string();
    }
    storage_service::save_setup_profile(&profile);
    profile
}

fn reset_to_npc_item_transfers_step(mut profile: SetupProfile) -> SetupProfile {
    profile.connection_status = ConnectionStatus::PartiallyConnected;
    profile.polar_block_height_confirmed = false;
    profile.last_verified_at = None;
    storage_service::save_setup_profile(&profile);
    profile
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::models::DEFAULT_NETWORK_NAME;

    fn profile_with_status(status: ConnectionStatus, network_id: &str) -> SetupProfile {
        let mut profile = SetupProfile::default();
        profile.connection_status = status;
        profile.network_name = DEFAULT_NETWORK_NAME.to_string();
        profile.polar_automation.network_id = network_id.to_string();
        profile
    }

    #[test]
    fn polar_wizard_starts_at_bridge_url_until_bridge_connects() {
        let profile = profile_with_status(ConnectionStatus::NotConfigured, "");

        assert_eq!(
            polar_wizard_step(&profile, None).order(),
            PolarWizardStep::BridgeUrl.order()
        );
    }

    #[test]
    fn polar_wizard_advances_only_after_saved_bridge_state() {
        let bridge_connected = profile_with_status(ConnectionStatus::SavedOffline, "");
        let server_ready = profile_with_status(ConnectionStatus::SavedOffline, "network-1");
        let mut treasury_ready =
            profile_with_status(ConnectionStatus::PartiallyConnected, "network-1");
        treasury_ready.game_treasury_ready = true;
        let mut demo_nodes_ready =
            profile_with_status(ConnectionStatus::PartiallyConnected, "network-1");
        demo_nodes_ready.game_treasury_ready = true;
        let connected = profile_with_status(ConnectionStatus::Connected, "network-1");

        assert_eq!(
            polar_wizard_step(&bridge_connected, None).order(),
            PolarWizardStep::ServerName.order()
        );
        assert_eq!(
            polar_wizard_step(&server_ready, None).order(),
            PolarWizardStep::GameTreasury.order()
        );
        assert_eq!(
            polar_wizard_step(&treasury_ready, None).order(),
            PolarWizardStep::GameTreasuryTras.order()
        );
        let mut treasury_tras_state = lightning_service::default_lab_state(treasury_ready.clone());
        treasury_tras_state.tra_items = treasury_tra_items();
        assert_eq!(
            polar_wizard_step(&treasury_ready, Some(&treasury_tras_state)).order(),
            PolarWizardStep::UserNodes.order()
        );
        let mut demo_nodes_state = user_nodes_ready_state(demo_nodes_ready.clone());
        demo_nodes_state.tra_items = treasury_tra_items();
        assert_eq!(
            polar_wizard_step(&demo_nodes_ready, Some(&demo_nodes_state)).order(),
            PolarWizardStep::NpcItemTransfers.order()
        );
        demo_nodes_state.tra_items = vec![
            verified_item(DemoNodeId::Bob, "Book", BOOK_ITEM_ID),
            verified_item(DemoNodeId::Bob, "Book 2", BOOK_ITEM_ID),
            verified_item(DemoNodeId::Carol, "Apple", APPLE_ITEM_ID),
            verified_item(DemoNodeId::Carol, "Apple 2", APPLE_ITEM_ID),
        ];
        demo_nodes_state.npc_item_transfers = successful_npc_transfers();
        assert_eq!(
            polar_wizard_step(&demo_nodes_ready, Some(&demo_nodes_state)).order(),
            PolarWizardStep::BlockHeight.order()
        );
        let mut block_height_ready = demo_nodes_ready;
        block_height_ready.polar_block_height_confirmed = true;
        demo_nodes_state.profile = block_height_ready.clone();
        assert_eq!(
            polar_wizard_step(&block_height_ready, Some(&demo_nodes_state)).order(),
            PolarWizardStep::Complete.order()
        );
        assert_eq!(
            polar_wizard_step(&connected, None).order(),
            PolarWizardStep::Done.order()
        );
    }

    fn verified_item(
        owner_node: DemoNodeId,
        unique_name: &str,
        item_id: u32,
    ) -> crate::client::models::TraItem {
        crate::client::models::TraItem {
            tra_id: unique_name.to_ascii_lowercase().replace(' ', "-"),
            asset_id: format!("asset-{unique_name}"),
            unique_name: unique_name.to_string(),
            item_id,
            owner_node,
            ownership_status: crate::client::models::TraOwnershipStatus::Verified,
            transfer_status: crate::client::models::TraTransferStatus::None,
        }
    }

    fn successful_npc_transfer(
        destination: DemoNodeId,
        unique_name: &str,
        item_id: u32,
        index: usize,
    ) -> crate::client::models::NpcItemTransfer {
        crate::client::models::NpcItemTransfer {
            transfer_id: format!("npc-transfer-{index}"),
            item_id,
            item_name: unique_name.to_string(),
            source: lightning_service::GAME_TREASURY_NODE_LABEL.to_string(),
            destination,
            status: crate::client::models::TraTransferStatus::Succeeded,
            entry_id: Some(format!("treasury-entry-{index}")),
        }
    }

    fn successful_npc_transfers() -> Vec<crate::client::models::NpcItemTransfer> {
        vec![
            successful_npc_transfer(DemoNodeId::Bob, "Book", BOOK_ITEM_ID, 1),
            successful_npc_transfer(DemoNodeId::Bob, "Book 2", BOOK_ITEM_ID, 2),
            successful_npc_transfer(DemoNodeId::Carol, "Apple", APPLE_ITEM_ID, 3),
            successful_npc_transfer(DemoNodeId::Carol, "Apple 2", APPLE_ITEM_ID, 4),
        ]
    }

    fn treasury_tra_items() -> Vec<crate::client::models::TraItem> {
        vec![
            verified_item(DemoNodeId::GameTreasury, "Book", BOOK_ITEM_ID),
            verified_item(DemoNodeId::GameTreasury, "Book 2", BOOK_ITEM_ID),
            verified_item(DemoNodeId::GameTreasury, "Apple", APPLE_ITEM_ID),
            verified_item(DemoNodeId::GameTreasury, "Apple 2", APPLE_ITEM_ID),
        ]
    }

    fn user_nodes_ready_state(profile: SetupProfile) -> LabState {
        let mut state = lightning_service::default_lab_state(profile);
        for node in state.nodes.iter_mut() {
            if DemoNodeId::ALL.contains(&node.node_id) {
                node.status = crate::client::models::NodeStatus::Online;
                node.pubkey = Some(format!("{}-pubkey", node.alias));
            }
        }
        state
    }

    #[test]
    fn complete_reset_returns_to_block_height() {
        let mut profile = profile_with_status(ConnectionStatus::Connected, "network-1");
        profile.game_treasury_ready = true;
        profile.network_name = DEFAULT_NETWORK_NAME.to_string();

        let reset_profile = reset_to_block_height_step(profile);
        let mut state = user_nodes_ready_state(reset_profile.clone());
        state.tra_items = vec![
            verified_item(DemoNodeId::Bob, "Book", BOOK_ITEM_ID),
            verified_item(DemoNodeId::Bob, "Book 2", BOOK_ITEM_ID),
            verified_item(DemoNodeId::Carol, "Apple", APPLE_ITEM_ID),
            verified_item(DemoNodeId::Carol, "Apple 2", APPLE_ITEM_ID),
        ];
        state.npc_item_transfers = successful_npc_transfers();

        assert_eq!(
            polar_wizard_step(&reset_profile, Some(&state)).order(),
            PolarWizardStep::BlockHeight.order()
        );
        assert!(!reset_profile.polar_block_height_confirmed);
    }

    #[test]
    fn npc_item_transfers_ready_requires_two_books_and_two_apples() {
        let mut profile = profile_with_status(ConnectionStatus::PartiallyConnected, "network-1");
        profile.polar_block_height_confirmed = true;
        let mut state = lightning_service::default_lab_state(profile);
        state.tra_items = vec![
            verified_item(DemoNodeId::Bob, "Book", BOOK_ITEM_ID),
            verified_item(DemoNodeId::Bob, "Book 2", BOOK_ITEM_ID),
            verified_item(DemoNodeId::Carol, "Apple", APPLE_ITEM_ID),
        ];

        assert!(!npc_item_transfers_ready(Some(&state)));

        state
            .tra_items
            .push(verified_item(DemoNodeId::Carol, "Apple 2", APPLE_ITEM_ID));
        state.npc_item_transfers = successful_npc_transfers();

        assert!(npc_item_transfers_ready(Some(&state)));
    }

    #[test]
    fn npc_item_transfers_ready_requires_transfer_records() {
        let mut profile = profile_with_status(ConnectionStatus::PartiallyConnected, "network-1");
        profile.game_treasury_ready = true;
        let mut state = user_nodes_ready_state(profile);
        state.tra_items = vec![
            verified_item(DemoNodeId::Bob, "Book", BOOK_ITEM_ID),
            verified_item(DemoNodeId::Bob, "Book 2", BOOK_ITEM_ID),
            verified_item(DemoNodeId::Carol, "Apple", APPLE_ITEM_ID),
            verified_item(DemoNodeId::Carol, "Apple 2", APPLE_ITEM_ID),
        ];

        assert!(!npc_item_transfers_ready(Some(&state)));
    }

    #[test]
    fn npc_item_transfers_reset_clears_snapshot_to_revisit_user_nodes() {
        let mut profile = profile_with_status(ConnectionStatus::PartiallyConnected, "network-1");
        profile.game_treasury_ready = true;
        let reset_profile = reset_to_demo_nodes_step(profile.clone());
        let state = lab_state_after_reset_to_demo_nodes_step(reset_profile.clone());

        assert!(state.as_ref().is_some_and(|state| treasury_tras_ready(Some(state))));
        assert_eq!(
            polar_wizard_step(&reset_profile, state.as_ref()).order(),
            PolarWizardStep::UserNodes.order()
        );
    }

    #[test]
    fn block_height_reset_returns_to_npc_item_transfers() {
        let mut profile = profile_with_status(ConnectionStatus::Connected, "network-1");
        profile.game_treasury_ready = true;
        profile.polar_block_height_confirmed = true;
        profile.network_name = DEFAULT_NETWORK_NAME.to_string();
        let mut state = user_nodes_ready_state(profile.clone());
        state.tra_items = vec![
            verified_item(DemoNodeId::Bob, "Book", BOOK_ITEM_ID),
            verified_item(DemoNodeId::Bob, "Book 2", BOOK_ITEM_ID),
            verified_item(DemoNodeId::Carol, "Apple", APPLE_ITEM_ID),
            verified_item(DemoNodeId::Carol, "Apple 2", APPLE_ITEM_ID),
        ];
        state.npc_item_transfers = successful_npc_transfers();

        let reset_profile = reset_to_npc_item_transfers_step(profile);
        state.tra_items = treasury_tra_items();
        state.npc_item_transfers.clear();
        state.profile = reset_profile.clone();

        assert_eq!(
            polar_wizard_step(&reset_profile, Some(&state)).order(),
            PolarWizardStep::NpcItemTransfers.order()
        );
        assert!(!reset_profile.polar_block_height_confirmed);
    }

    #[test]
    fn submit_focus_targets_advance_to_next_step() {
        assert_eq!(
            submit_focus_target(PolarWizardStep::BridgeUrl),
            Some("polar-server-name-input")
        );
        assert_eq!(
            submit_focus_target(PolarWizardStep::ServerName),
            Some("polar-game-treasury-submit")
        );
        assert_eq!(
            submit_focus_target(PolarWizardStep::GameTreasury),
            Some("polar-game-treasury-tras-submit")
        );
        assert_eq!(
            submit_focus_target(PolarWizardStep::GameTreasuryTras),
            Some(POLAR_DEMO_NODES_SUBMIT_ID)
        );
        assert_eq!(
            submit_focus_target(PolarWizardStep::UserNodes),
            Some("polar-tra-assets-submit")
        );
        assert_eq!(
            submit_focus_target(PolarWizardStep::NpcItemTransfers),
            Some("polar-block-height-input")
        );
        assert_eq!(
            submit_focus_target(PolarWizardStep::BlockHeight),
            Some("polar-complete-submit")
        );
        assert_eq!(submit_focus_target(PolarWizardStep::Complete), None);
    }

    #[test]
    fn polar_wizard_step_labels_match_required_visual_order() {
        let labels = [
            polar_wizard_step_label(PolarWizardStep::BridgeUrl),
            polar_wizard_step_label(PolarWizardStep::ServerName),
            polar_wizard_step_label(PolarWizardStep::GameTreasury),
            polar_wizard_step_label(PolarWizardStep::GameTreasuryTras),
            polar_wizard_step_label(PolarWizardStep::UserNodes),
            polar_wizard_step_label(PolarWizardStep::NpcItemTransfers),
            polar_wizard_step_label(PolarWizardStep::BlockHeight),
            polar_wizard_step_label(PolarWizardStep::Complete),
        ];

        assert_eq!(
            labels,
            [
                "Bridge URL",
                "Server Name",
                "Game Treasury (Sats)",
                "Game Treasury (TRAs)",
                "User Nodes",
                "NPC Item Transfers",
                "Block Height",
                "Unlock Routes"
            ]
        );
    }

    #[test]
    fn reset_focus_targets_return_to_previous_step() {
        assert_eq!(
            reset_focus_target(PolarWizardStep::ServerName),
            "polar-bridge-url-input"
        );
        assert_eq!(
            reset_focus_target(PolarWizardStep::GameTreasury),
            "polar-server-name-input"
        );
        assert_eq!(
            reset_focus_target(PolarWizardStep::UserNodes),
            "polar-game-treasury-tras-submit"
        );
        assert_eq!(
            reset_focus_target(PolarWizardStep::GameTreasuryTras),
            "polar-game-treasury-submit"
        );
        assert_eq!(
            reset_focus_target(PolarWizardStep::NpcItemTransfers),
            POLAR_DEMO_NODES_SUBMIT_ID
        );
        assert_eq!(
            reset_focus_target(PolarWizardStep::BlockHeight),
            "polar-tra-assets-submit"
        );
        assert_eq!(
            reset_focus_target(PolarWizardStep::Complete),
            "polar-block-height-input"
        );
    }

    #[test]
    fn complete_reset_focuses_step_six_primary_control() {
        assert_eq!(complete_reset_focus_target(), "polar-block-height-input");
    }

    #[test]
    fn block_height_accepts_zero() {
        assert_eq!(block_height_from_input("0".to_string()), Ok(0));
    }

    #[test]
    fn local_browser_hostname_allows_loopback_hosts() {
        assert!(browser_hostname_allows_polar_bridge("localhost"));
        assert!(browser_hostname_allows_polar_bridge("127.0.0.1"));
        assert!(!browser_hostname_allows_polar_bridge("192.168.0.10"));
    }

    #[test]
    fn bridge_connection_errors_match_fetch_failures_from_later_steps() {
        assert!(is_bridge_connection_error(
            "error GET http://localhost:37373/health error=Cannot reach Polar bridge: TypeError: Failed to fetch"
        ));
        assert!(is_bridge_connection_error(
            "Cannot reach Polar bridge: TypeError: Failed to fetch"
        ));
        assert!(!is_bridge_connection_error(
            "Polar server bitcoin-network is not listed by that name."
        ));
    }

    #[test]
    fn bridge_step_error_points_back_to_localhost_step_one() {
        let message = bridge_step_error_message("TypeError: Failed to fetch");

        assert_eq!(
            message,
            "Error: Cannot connect to Polar, revisit 1. Environment step 04 for more info"
        );
    }

    #[test]
    fn polar_connection_tab_storage_values_round_trip() {
        assert_eq!(
            PolarConnectionTab::from_storage_value("environment"),
            Some(PolarConnectionTab::Environment)
        );
        assert_eq!(
            PolarConnectionTab::from_storage_value("polar"),
            Some(PolarConnectionTab::Polar)
        );
        assert_eq!(PolarConnectionTab::from_storage_value("unknown"), None);
        assert_eq!(
            PolarConnectionTab::Environment.storage_value(),
            "environment"
        );
        assert_eq!(PolarConnectionTab::Polar.storage_value(), "polar");
    }
}

async fn create_demo_nodes_step(
    mut is_busy: Signal<bool>,
    mut setup_profile: Signal<SetupProfile>,
    mut lab_state: Signal<Option<LabState>>,
    operation_prompt: Signal<Option<OperationPrompt>>,
    prompt_sequence: Signal<u64>,
    toast: Signal<Option<Toast>>,
    toast_sequence: Signal<u64>,
    mut bridge_connection_error: Signal<String>,
    amount_text: String,
    polar_server_name: String,
    polar_bridge_url: String,
    mut polar_block_height: Signal<String>,
) {
    is_busy.set(true);
    let previous_profile = setup_profile();
    let previous_lab_state = lab_state();
    let operation_id = begin_operation_prompt(
        operation_prompt,
        prompt_sequence,
        "Create user nodes",
        "Starting user-node creation...",
        true,
    )
    .await;

    match profile_from_inputs(
        amount_text,
        polar_server_name,
        SetupMode::ServerConfig,
        polar_automation_from_input(polar_bridge_url, setup_profile().polar_automation),
        setup_profile(),
    ) {
        Ok(profile) => {
            let progress_prompt = operation_prompt;
            let progress_operation_id = operation_id;
            let create_result = create_polar_demo_nodes_with_progress(profile, move |message| {
                update_operation_prompt_now(
                    progress_prompt,
                    progress_operation_id,
                    message,
                    ToastTone::Info,
                    true,
                    true,
                );
            })
            .await;

            match create_result {
                Ok(state) => {
                    if prompt_cancel_requested(operation_prompt, operation_id) {
                        match destroy_polar_demo_nodes(state.profile.clone()).await {
                            Ok(_) => {
                                restore_saved_setup(&previous_profile, previous_lab_state.as_ref());
                                setup_profile.set(previous_profile);
                                lab_state.set(previous_lab_state);
                                update_operation_prompt(
                                    operation_prompt,
                                    operation_id,
                                    "User node creation canceled. Alice, Bob, and Carol were removed.",
                                    ToastTone::Success,
                                    false,
                                    false,
                                )
                                .await;
                                close_operation_prompt(operation_prompt, operation_id);
                                push_toast(
                                    toast,
                                    toast_sequence,
                                    "User node creation canceled.",
                                    ToastTone::Success,
                                );
                            }
                            Err(message) => {
                                setup_profile.set(state.profile.clone());
                                lab_state.set(Some(state));
                                update_operation_prompt(
                                    operation_prompt,
                                    operation_id,
                                    format!(
                                        "Cancel could not remove the created user nodes: {message}"
                                    ),
                                    ToastTone::Error,
                                    false,
                                    false,
                                )
                                .await;
                                close_operation_prompt(operation_prompt, operation_id);
                                push_toast(
                                    toast,
                                    toast_sequence,
                                    "Cancel could not remove the created user nodes.",
                                    ToastTone::Error,
                                );
                            }
                        }
                    } else {
                        setup_profile.set(state.profile.clone());
                        polar_block_height.set(state.block_height.to_string());
                        lab_state.set(Some(state));
                        close_operation_prompt(operation_prompt, operation_id);
                        push_toast(toast, toast_sequence, "User Nodes sent", ToastTone::Success);
                        focus_step_control("polar-tra-assets-submit").await;
                    }
                }
                Err(message) => {
                    if is_bridge_connection_error(&message) {
                        let saved_profile = reset_to_bridge_url_step(setup_profile());
                        setup_profile.set(saved_profile);
                        lab_state.set(None);
                        let message = bridge_step_error_message(message);
                        bridge_connection_error.set(message.clone());
                        update_operation_prompt(
                            operation_prompt,
                            operation_id,
                            "Return to step 1 and connect to the Polar bridge before creating user nodes.",
                            ToastTone::Error,
                            false,
                            false,
                        )
                        .await;
                        close_operation_prompt(operation_prompt, operation_id);
                        push_toast(toast, toast_sequence, message, ToastTone::Error);
                        focus_step_control("polar-bridge-url-input").await;
                    } else {
                        update_operation_prompt(
                            operation_prompt,
                            operation_id,
                            message.clone(),
                            ToastTone::Error,
                            false,
                            false,
                        )
                        .await;
                        close_operation_prompt(operation_prompt, operation_id);
                        push_toast(toast, toast_sequence, message, ToastTone::Error);
                    }
                }
            }
        }
        Err(message) => {
            update_operation_prompt(
                operation_prompt,
                operation_id,
                message.clone(),
                ToastTone::Error,
                false,
                false,
            )
            .await;
            close_operation_prompt(operation_prompt, operation_id);
            push_toast(toast, toast_sequence, message, ToastTone::Error);
        }
    }

    is_busy.set(false);
}

async fn reset_complete_step(
    mut is_busy: Signal<bool>,
    mut setup_profile: Signal<SetupProfile>,
    mut lab_state: Signal<Option<LabState>>,
    mut operation_prompt: Signal<Option<OperationPrompt>>,
    toast: Signal<Option<Toast>>,
    toast_sequence: Signal<u64>,
) {
    is_busy.set(true);
    operation_prompt.set(None);
    let saved_profile = reset_to_block_height_step(setup_profile());
    setup_profile.set(saved_profile.clone());
    let current_lab_state = lab_state().or_else(storage_service::load_lab_state_snapshot);
    if let Some(mut state) = current_lab_state {
        state.profile = saved_profile;
        storage_service::save_lab_state_snapshot(&state);
        lab_state.set(Some(state));
    }
    push_toast(
        toast,
        toast_sequence,
        "Returned to step 7. Block Height must be submitted before routes unlock again.",
        ToastTone::Success,
    );
    is_busy.set(false);
    schedule_step_control_focus(complete_reset_focus_target());
}

async fn prepare_treasury_tras_step(
    mut is_busy: Signal<bool>,
    mut setup_profile: Signal<SetupProfile>,
    mut lab_state: Signal<Option<LabState>>,
    operation_prompt: Signal<Option<OperationPrompt>>,
    prompt_sequence: Signal<u64>,
    toast: Signal<Option<Toast>>,
    toast_sequence: Signal<u64>,
) {
    is_busy.set(true);
    let operation_id = begin_operation_prompt(
        operation_prompt,
        prompt_sequence,
        "Game Treasury (TRAs)",
        "Creating treasury-owned TRA inventory items...",
        false,
    )
    .await;

    let result = prepare_game_treasury_tras(setup_profile()).await;

    match result {
        Ok(state) => {
            setup_profile.set(state.profile.clone());
            lab_state.set(Some(state));
            close_operation_prompt(operation_prompt, operation_id);
            push_toast(
                toast,
                toast_sequence,
                "Game Treasury TRAs ready.",
                ToastTone::Success,
            );
            focus_step_control(POLAR_DEMO_NODES_SUBMIT_ID).await;
        }
        Err(message) => {
            update_operation_prompt(
                operation_prompt,
                operation_id,
                message.clone(),
                ToastTone::Error,
                false,
                false,
            )
            .await;
            close_operation_prompt(operation_prompt, operation_id);
            push_toast(toast, toast_sequence, message, ToastTone::Error);
        }
    }

    is_busy.set(false);
}

async fn prepare_tra_inventory_step(
    mut is_busy: Signal<bool>,
    mut setup_profile: Signal<SetupProfile>,
    mut lab_state: Signal<Option<LabState>>,
    operation_prompt: Signal<Option<OperationPrompt>>,
    prompt_sequence: Signal<u64>,
    toast: Signal<Option<Toast>>,
    toast_sequence: Signal<u64>,
) {
    is_busy.set(true);
    let operation_id = begin_operation_prompt(
        operation_prompt,
        prompt_sequence,
        "Add NPC Item Transfers",
        "Recreating NPC Item Transfers from scratch...",
        false,
    )
    .await;

    update_operation_prompt(
        operation_prompt,
        operation_id,
        "Transferring Bob and Carol starting items from Game Treasury...",
        ToastTone::Info,
        true,
        false,
    )
    .await;

    let result = transfer_npc_starting_items(setup_profile()).await;

    match result {
        Ok(state) => {
            setup_profile.set(state.profile.clone());
            lab_state.set(Some(state));
            close_operation_prompt(operation_prompt, operation_id);
            push_toast(
                toast,
                toast_sequence,
                "NPC Item Transfers added.",
                ToastTone::Success,
            );
            focus_step_control("polar-block-height-input").await;
        }
        Err(message) => {
            update_operation_prompt(
                operation_prompt,
                operation_id,
                message.clone(),
                ToastTone::Error,
                false,
                false,
            )
            .await;
            close_operation_prompt(operation_prompt, operation_id);
            push_toast(toast, toast_sequence, message, ToastTone::Error);
        }
    }

    is_busy.set(false);
}

async fn begin_operation_prompt(
    mut prompt: Signal<Option<OperationPrompt>>,
    mut sequence: Signal<u64>,
    title: impl Into<String>,
    message: impl Into<String>,
    can_cancel: bool,
) -> u64 {
    let operation_id = *sequence.peek() + 1;
    sequence.set(operation_id);
    prompt.set(Some(OperationPrompt {
        operation_id,
        title: title.into(),
        message: message.into(),
        tone: ToastTone::Info,
        is_pending: true,
        can_cancel,
        cancel_requested: false,
    }));
    wait_for_prompt_message_minimum().await;
    operation_id
}

async fn update_operation_prompt(
    mut prompt: Signal<Option<OperationPrompt>>,
    operation_id: u64,
    message: impl Into<String>,
    tone: ToastTone,
    is_pending: bool,
    can_cancel: bool,
) {
    let active_prompt = { prompt.peek().as_ref().cloned() };
    if let Some(mut active_prompt) = active_prompt {
        if active_prompt.operation_id == operation_id {
            active_prompt.message = message.into();
            active_prompt.tone = tone;
            active_prompt.is_pending = is_pending;
            active_prompt.can_cancel = can_cancel;
            prompt.set(Some(active_prompt));
            wait_for_prompt_message_minimum().await;
        }
    }
}

fn update_operation_prompt_now(
    mut prompt: Signal<Option<OperationPrompt>>,
    operation_id: u64,
    message: impl Into<String>,
    tone: ToastTone,
    is_pending: bool,
    can_cancel: bool,
) {
    let active_prompt = { prompt.peek().as_ref().cloned() };
    if let Some(mut active_prompt) = active_prompt {
        if active_prompt.operation_id == operation_id {
            active_prompt.message = message.into();
            active_prompt.tone = tone;
            active_prompt.is_pending = is_pending;
            active_prompt.can_cancel = can_cancel;
            prompt.set(Some(active_prompt));
        }
    }
}

fn close_operation_prompt(mut prompt: Signal<Option<OperationPrompt>>, operation_id: u64) {
    if prompt
        .peek()
        .as_ref()
        .map(|active_prompt| active_prompt.operation_id == operation_id)
        .unwrap_or(false)
    {
        prompt.set(None);
    }
}

fn prompt_cancel_requested(prompt: Signal<Option<OperationPrompt>>, operation_id: u64) -> bool {
    prompt
        .peek()
        .as_ref()
        .map(|active_prompt| {
            active_prompt.operation_id == operation_id && active_prompt.cancel_requested
        })
        .unwrap_or(false)
}
