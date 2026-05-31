use dioxus::prelude::*;
use dioxus_i18n::t;
use futures::future::{select, Either, FutureExt};
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

use crate::client::components::help::FieldHelpIcon;
use crate::client::components::setup::{NpcItemTransferStatus, WarningCallout};
use crate::client::components::toast::{
    wait_for_prompt_message_minimum, OperationPrompt, Toast, ToastTone,
};
use crate::client::models::{
    ConnectionStatus, DemoNodeId, LabState, PolarAutomationProfile, SetupMode, SetupProfile,
    UserAuthMode, APPLE_ITEM_ID, BOOK_ITEM_ID, DEFAULT_BITCOIN_BACKEND_NAME,
};
use crate::client::services::lightning_server_functions::{
    complete_polar_setup, confirm_polar_block_height, count_polar_networks,
    create_required_polar_nodes_with_progress, default_lnauth_bridge_url,
    delete_all_polar_networks_with_progress, delete_created_polar_server, destroy_polar_demo_nodes,
    ensure_polar_server, get_lab_state, is_valid_lnauth_bridge_url, prepare_game_treasury,
    prepare_game_treasury_tras, prepare_user_node_sats, prepare_user_node_tras, reset_lab,
    test_lnauth_bridge_url, test_setup, verify_polar_bridge, PolarDeleteAllProgress,
    PolarServerEnsureResult, PolarServerEnsureStatus,
};
use crate::client::services::storage_service;

const DOCKER_DESKTOP_URL: &str = "https://www.docker.com/products/docker-desktop/";
const LOCAL_APP_URL: &str = "http://localhost:8080";
const POLAR_DOWNLOAD_URL: &str = "https://lightningpolar.com/";
const POLAR_DEMO_NODES_SUBMIT_ID: &str = "polar-user-nodes-submit";
const POLAR_AUTOPILOT_PROMPT_TITLE: &str = "Polar Autopilot";
const POLAR_AUTOPILOT_TARGET_SECONDS: i64 = 60;
const POLAR_AUTOPILOT_SERVER_ATTEMPTS: u8 = 5;
const POLAR_AUTOPILOT_SERVER_STEP_TIMEOUT_SECONDS: u32 = 90;
const POLAR_DELETE_ALL_COUNT_TIMEOUT_SECONDS: u32 = 60;
const POLAR_DELETE_ALL_TIMEOUT_SECONDS: u32 = 300;
static POLAR_AUTOPILOT_NAME_SEQUENCE: AtomicU64 = AtomicU64::new(0);
#[cfg(target_arch = "wasm32")]
const FOCUS_RETRY_ATTEMPTS: u8 = 12;
#[cfg(target_arch = "wasm32")]
const FOCUS_RETRY_DELAY_MS: u32 = 16;

#[derive(Clone, Copy, Eq, PartialEq)]
enum PolarWizardStep {
    LocalAppUrl,
    BridgeUrl,
    ServerName,
    CreateNodes,
    GameTreasury,
    GameTreasuryTras,
    UserNodesSats,
    UserNodesTras,
    BlockHeight,
    Complete,
    Done,
}

impl PolarWizardStep {
    fn order(self) -> u8 {
        match self {
            Self::LocalAppUrl => 1,
            Self::BridgeUrl => 2,
            Self::ServerName => 3,
            Self::CreateNodes => 4,
            Self::GameTreasury => 5,
            Self::GameTreasuryTras => 6,
            Self::UserNodesSats => 7,
            Self::UserNodesTras => 8,
            Self::BlockHeight => 9,
            Self::Complete | Self::Done => 10,
        }
    }
}

fn polar_wizard_step_label(step: PolarWizardStep) -> &'static str {
    match step {
        PolarWizardStep::LocalAppUrl => "App URL",
        PolarWizardStep::BridgeUrl => "Bridge URLs",
        PolarWizardStep::ServerName => "Server Name",
        PolarWizardStep::CreateNodes => "Create Nodes",
        PolarWizardStep::GameTreasury => "Game Treasury (Sats)",
        PolarWizardStep::GameTreasuryTras => "Game Treasury (TRAs)",
        PolarWizardStep::UserNodesSats => "User Nodes (Sats)",
        PolarWizardStep::UserNodesTras => "User Nodes (TRAs)",
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
    let mut user_auth_mode = use_signal(|| setup_profile().user_auth_mode);
    let mut polar_connection_tab = use_signal(PolarConnectionTab::load);
    let mut polar_bridge_url = use_signal(|| setup_profile().polar_automation.bridge_url.clone());
    let mut lnauth_bridge_url = use_signal(|| {
        let saved = setup_profile().lnauth_bridge_url;
        if saved.trim().is_empty() {
            default_lnauth_bridge_url()
        } else {
            saved
        }
    });
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
    let show_delete_all_networks_confirm = use_signal(|| false);
    let delete_all_networks_confirmation_count = use_signal(|| None::<usize>);
    let mut autopilot_enabled = use_signal(|| false);
    let mut autopilot_status = use_signal(|| "Ready".to_string());
    let current_profile = setup_profile();
    let current_lab_state = lab_state();
    let active_step = polar_wizard_step(&current_profile, current_lab_state.as_ref());
    let bridge_url_is_valid = is_valid_local_bridge_url(&polar_bridge_url());
    let lnauth_bridge_url_is_valid = user_auth_mode() != UserAuthMode::LnAuth
        || is_valid_lnauth_bridge_url(&lnauth_bridge_url());
    let browser_origin_is_valid = browser_origin_allows_polar_bridge();
    let bridge_url_can_submit = bridge_url_is_valid && lnauth_bridge_url_is_valid;
    let server_name_is_valid = !polar_server_name().trim().is_empty();

    rsx! {
            main { class: "page-content lab-page setup-page",
                section { class: "lab-hero",
                    div {
                        span { class: "eyebrow", "Polar regtest Lightning lab" }
                        h1 { {t!("setup-title")} }
                        p {
                            "Control Jack, Bob, and Carol in a local Lightning learning lab. The app separates game actions from the network mechanics behind channels, invoices, payments, and block confirmations."
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

                        div { class: "field-group",
                            span { "User Auth" }
                            div { class: "segmented-control", role: "tablist", aria_label: "User Auth",
                                button {
                                    class: if user_auth_mode() == UserAuthMode::App { "segment segment--active" } else { "segment" },
                                    r#type: "button",
                                    role: "tab",
                                    aria_selected: if user_auth_mode() == UserAuthMode::App { "true" } else { "false" },
                                    onclick: move |_| {
                                        apply_user_auth_mode(
                                            UserAuthMode::App,
                                            &mut user_auth_mode,
                                            setup_profile,
                                            lab_state,
                                        );
                                    },
                                    "App"
                                }
                                button {
                                    class: if user_auth_mode() == UserAuthMode::MockLnAuth { "segment segment--active" } else { "segment" },
                                    r#type: "button",
                                    role: "tab",
                                    aria_selected: if user_auth_mode() == UserAuthMode::MockLnAuth { "true" } else { "false" },
                                    onclick: move |_| {
                                        apply_user_auth_mode(
                                            UserAuthMode::MockLnAuth,
                                            &mut user_auth_mode,
                                            setup_profile,
                                            lab_state,
                                        );
                                    },
                                    "Mock LNAuth"
                                }
                                button {
                                    class: if user_auth_mode() == UserAuthMode::LnAuth { "segment segment--active" } else { "segment" },
                                    r#type: "button",
                                    role: "tab",
                                    aria_selected: if user_auth_mode() == UserAuthMode::LnAuth { "true" } else { "false" },
                                    onclick: move |_| {
                                        apply_user_auth_mode(
                                            UserAuthMode::LnAuth,
                                            &mut user_auth_mode,
                                            setup_profile,
                                            lab_state,
                                        );
                                    },
                                    "LNAuth"
                                    FieldHelpIcon {
                                        label: "Testing wallet: ZEUS. It works on Android and iOS and its docs list LNURL auth support. Use it to scan LNAuth login and key-event approval QR codes. Your wallet stays outside Polar; Polar only runs the local lab nodes.".to_string()
                                    }
                                }
                            }
                            p { class: "connection-tab-copy", "{user_auth_mode_description(user_auth_mode())}" }
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
                                                    user_auth_mode.set(default_profile.user_auth_mode);
                                                    polar_bridge_url.set(default_profile.polar_automation.bridge_url.clone());
                                                    lnauth_bridge_url.set(default_profile.lnauth_bridge_url.clone());
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
                                    "Create the Bitcoin backend in Polar, then use the app setup steps to connect the local bridge and create Jack, Bob, and Carol."
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
                                    div { class: "autopilot-panel",
                                        div { class: "autopilot-panel__body",
                                            span { class: "eyebrow", "Debug mode" }
                                            strong {
                                                if autopilot_enabled() {
                                                    "Autopilot: on"
                                                } else {
                                                    "Autopilot: off"
                                                }
                                            }
                                            p { "{autopilot_status}" }
                                        }
                                        div { class: "autopilot-panel__actions",
                                            button {
                                                id: "polar-autopilot-run",
                                                class: "primary-action",
                                                r#type: "button",
                                                disabled: is_busy() || !browser_origin_is_valid || !bridge_url_can_submit,
                                                onclick: move |_| async move {
                                                    run_polar_setup_autopilot(
                                                        is_busy,
                                                        setup_profile,
                                                        lab_state,
                                                        operation_prompt,
                                                        prompt_sequence,
                                                        toast,
                                                        toast_sequence,
                                                        bridge_connection_error,
                                                        amount_text(),
                                                        polar_server_name,
                                                        polar_bridge_url(),
                                                        lnauth_bridge_url(),
                                                        polar_block_height,
                                                        autopilot_enabled,
                                                        autopilot_status,
                                                    )
                                                    .await;
                                                },
                                                "Run"
                                            }
                                            button {
                                                id: "polar-autopilot-off",
                                                class: "secondary-action",
                                                r#type: "button",
                                                disabled: is_busy() || !autopilot_enabled(),
                                                onclick: move |_| {
                                                    autopilot_enabled.set(false);
                                                    autopilot_status.set("Ready".to_string());
                                                },
                                                "Off"
                                            }
                                            button {
                                                id: "polar-delete-all-networks",
                                                class: "secondary-action danger-action",
                                                r#type: "button",
                                                disabled: delete_all_networks_button_disabled(is_busy()),
                                                onclick: move |_| async move {
                                                    prepare_delete_all_networks_confirmation(
                                                        is_busy,
                                                        show_delete_all_networks_confirm,
                                                        delete_all_networks_confirmation_count,
                                                        setup_profile,
                                                        operation_prompt,
                                                        toast,
                                                        toast_sequence,
                                                        polar_bridge_url(),
                                                    ).await;
                                                },
                                                "Delete all networks"
                                            }
                                        }
                                    }
                                    InstructionList {
                                        Instruction {
                                            id: "polar-step-local-app-url".to_string(),
                                            class: wizard_step_class(active_step, PolarWizardStep::LocalAppUrl).to_string(),
                                            number: 1,
                                            info: "Open this app at localhost before connecting to Polar".to_string(),
                                            name: rsx! { "{polar_wizard_step_label(PolarWizardStep::LocalAppUrl)}" },
                                            value: Some(rsx! {
                                                label { class: "setup-field-row setup-field-row--stacked",
                                                    span { "Required app URL" }
                                                    input {
                                                        id: "polar-local-app-url-input",
                                                        r#type: "text",
                                                        value: LOCAL_APP_URL,
                                                        readonly: true,
                                                        disabled: active_step != PolarWizardStep::LocalAppUrl,
                                                    }
                                                }
                                            }),
                                            value_after: Some(rsx! {
                                                if !browser_origin_is_valid && active_step == PolarWizardStep::LocalAppUrl {
                                                    p { class: "field-error",
                                                        "Open this app at "
                                                        a {
                                                            class: "setup-resource-link",
                                                            href: LOCAL_APP_URL,
                                                            "{LOCAL_APP_URL}"
                                                        }
                                                        " before continuing."
                                                    }
                                                }
                                            }),
                                            actions: Some(rsx! {
                                                button {
                                                    id: "polar-local-app-url-submit",
                                                    class: "primary-action",
                                                    r#type: "button",
                                                    disabled: is_busy() || active_step != PolarWizardStep::LocalAppUrl,
                                                    onclick: move |_| async move {
                                                        is_busy.set(true);
                                                        if browser_origin_allows_polar_bridge() {
                                                            let mut profile = setup_profile();
                                                            profile.local_app_url_ready = true;
                                                            setup_profile.set(profile.clone());
                                                            storage_service::save_setup_profile(&profile);
                                                            push_toast(toast, toast_sequence, "App URL verified. Continue to step 2.", ToastTone::Success);
                                                            focus_step_control("polar-bridge-url-input").await;
                                                        } else {
                                                            push_toast(
                                                                toast,
                                                                toast_sequence,
                                                                format!("Open this app at {LOCAL_APP_URL} before continuing."),
                                                                ToastTone::Error,
                                                            );
                                                        }
                                                        is_busy.set(false);
                                                    },
                                                    "SUBMIT"
                                                }
                                            }),
                                        }

                                        Instruction {
                                            id: "polar-step-bridge-url".to_string(),
                                            class: wizard_step_class(active_step, PolarWizardStep::BridgeUrl).to_string(),
                                            number: 2,
                                            info: if user_auth_mode() == UserAuthMode::LnAuth {
                                                "Test the Polar bridge and the LNAuth callback bridge".to_string()
                                            } else {
                                                "Test the Polar bridge while Polar is open".to_string()
                                            },
                                            name: rsx! { "{polar_wizard_step_label(PolarWizardStep::BridgeUrl)}" },
                                            value: Some(rsx! {
                                                label { class: "setup-field-row setup-field-row--stacked",
                                                    span { "Polar Bridge URL" }
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
                                                if user_auth_mode() == UserAuthMode::LnAuth {
                                                    label { class: "setup-field-row setup-field-row--stacked",
                                                        span { "LNAuth Bridge URL" }
                                                        input {
                                                            id: "lnauth-bridge-url-input",
                                                            r#type: "text",
                                                            placeholder: "http://192.168.1.20:37374",
                                                            value: lnauth_bridge_url(),
                                                            disabled: active_step != PolarWizardStep::BridgeUrl,
                                                            oninput: move |event| {
                                                                bridge_connection_error.set(String::new());
                                                                lnauth_bridge_url.set(event.value());
                                                            },
                                                        }
                                                    }
                                                }
                                            }),
                                            value_after: Some(rsx! {
                                                if !bridge_url_is_valid && active_step == PolarWizardStep::BridgeUrl {
                                                    p { class: "field-error", "Use a local bridge URL such as http://localhost:37373." }
                                                }
                                                if user_auth_mode() == UserAuthMode::LnAuth && !lnauth_bridge_url_is_valid && active_step == PolarWizardStep::BridgeUrl {
                                                    p { class: "field-error", "Use an LNAuth bridge URL with host and port, such as http://192.168.1.20:37374." }
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
                                                            "Connect bridge URLs",
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
                                                                    profile.lnauth_bridge_url = lnauth_bridge_url().trim().to_string();
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
                                                                    profile.local_app_url_ready = true;
                                                                    profile.last_verified_at = None;
                                                                    profile.polar_automation.network_id.clear();
                                                                    match verify_polar_bridge(profile.clone()).await {
                                                                        Ok(saved_profile) => {
                                                                            let mut saved_profile = saved_profile;
                                                                            saved_profile.lnauth_bridge_url = profile.lnauth_bridge_url.clone();
                                                                            saved_profile.local_app_url_ready = true;
                                                                            if saved_profile.user_auth_mode == UserAuthMode::LnAuth {
                                                                                update_operation_prompt(
                                                                                    operation_prompt,
                                                                                    operation_id,
                                                                                    "Contacting the LNAuth bridge...",
                                                                                    ToastTone::Info,
                                                                                    true,
                                                                                    false,
                                                                                )
                                                                                .await;
                                                                                match test_lnauth_bridge_url(saved_profile.lnauth_bridge_url.clone()).await {
                                                                                    Ok(()) => {
                                                                                        bridge_connection_error.set(String::new());
                                                                                        setup_profile.set(saved_profile.clone());
                                                                                        storage_service::save_setup_profile(&saved_profile);
                                                                                        lab_state.set(None);
                                                                                        close_operation_prompt(operation_prompt, operation_id);
                                                                                        push_toast(toast, toast_sequence, "Connected to Polar and LNAuth bridges.", ToastTone::Success);
                                                                                        focus_step_control("polar-server-name-input").await;
                                                                                    }
                                                                                    Err(message) => {
                                                                                        let message = format!("LNAuth bridge check failed: {message}");
                                                                                        bridge_connection_error.set(message.clone());
                                                                                        close_operation_prompt(operation_prompt, operation_id);
                                                                                        push_toast(toast, toast_sequence, message, ToastTone::Error);
                                                                                    }
                                                                                }
                                                                            } else {
                                                                                bridge_connection_error.set(String::new());
                                                                                let saved_profile = saved_profile;
                                                                                storage_service::save_setup_profile(&saved_profile);
                                                                                setup_profile.set(saved_profile);
                                                                                lab_state.set(None);
                                                                                close_operation_prompt(operation_prompt, operation_id);
                                                                                push_toast(toast, toast_sequence, "Connected to Polar bridge.", ToastTone::Success);
                                                                                focus_step_control("polar-server-name-input").await;
                                                                            }
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
                                                button {
                                                    id: "polar-bridge-url-reset",
                                                    class: "secondary-action danger-action",
                                                    r#type: "button",
                                                    disabled: is_busy() || active_step != PolarWizardStep::BridgeUrl,
                                                    onclick: move |_| async move {
                                                        is_busy.set(true);
                                                        let saved_profile = reset_to_local_app_url_step(setup_profile());
                                                        setup_profile.set(saved_profile.clone());
                                                        lab_state.set(None);
                                                        polar_bridge_url.set(saved_profile.polar_automation.bridge_url.clone());
                                                        polar_server_name.set(saved_profile.network_name.clone());
                                                        push_toast(toast, toast_sequence, "Returned to step 1.", ToastTone::Success);
                                                        focus_step_control("polar-local-app-url-submit").await;
                                                        is_busy.set(false);
                                                    },
                                                    "RESET"
                                                }
                                            }),
                                        }

                                        Instruction {
                                            id: "polar-step-server-name".to_string(),
                                            class: wizard_step_class(active_step, PolarWizardStep::ServerName).to_string(),
                                            number: 3,
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
                                                        push_toast(toast, toast_sequence, "Returned to step 2.", ToastTone::Success);
                                                        focus_step_control("polar-bridge-url-input").await;

                                                        is_busy.set(false);
                                                    },
                                                    "RESET"
                                                }
                                            }),
                                        }

                                        Instruction {
                                            id: "polar-step-create-nodes".to_string(),
                                            class: wizard_step_class(active_step, PolarWizardStep::CreateNodes).to_string(),
                                            number: 4,
                                            info: "Finds or creates the required Polar Bitcoin, Lightning, Taproot, NPC, and player nodes".to_string(),
                                            name: rsx! { "{polar_wizard_step_label(PolarWizardStep::CreateNodes)}" },
                                            value: Some(rsx! {
                                                label { class: "setup-field-row",
                                                    input {
                                                        id: "polar-create-nodes-input",
                                                        r#type: "text",
                                                        value: "BITCOIN_TESTNET, GAME_LND, GAME_TAPROOT, Jack, Bob, Carol",
                                                        readonly: true,
                                                    }
                                                }
                                            }),
                                            actions: Some(rsx! {
                                                button {
                                                    id: POLAR_DEMO_NODES_SUBMIT_ID,
                                                    class: "primary-action",
                                                    r#type: "button",
                                                    disabled: is_busy() || active_step != PolarWizardStep::CreateNodes,
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
                                                    id: "polar-create-nodes-reset",
                                                    class: "secondary-action danger-action",
                                                    r#type: "button",
                                                    disabled: is_busy() || active_step != PolarWizardStep::CreateNodes,
                                                    onclick: move |_| async move {
                                                        is_busy.set(true);
                                                        let saved_profile = reset_to_server_name_step(setup_profile());
                                                        setup_profile.set(saved_profile.clone());
                                                        lab_state.set(None);
                                                        push_toast(toast, toast_sequence, "Returned to step 3.", ToastTone::Success);
                                                        focus_step_control("polar-server-name-input").await;
                                                        is_busy.set(false);
                                                    },
                                                    "RESET"
                                                }
                                            }),
                                        }

                                        Instruction {
                                            id: "polar-step-game-treasury".to_string(),
                                            class: wizard_step_class(active_step, PolarWizardStep::GameTreasury).to_string(),
                                            number: 5,
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
                                                        push_toast(toast, toast_sequence, "Returned to step 4.", ToastTone::Success);
                                                        focus_step_control(POLAR_DEMO_NODES_SUBMIT_ID).await;
                                                        is_busy.set(false);
                                                    },
                                                    "RESET"
                                                }
                                            }),
                                        }

                                        Instruction {
                                            id: "polar-step-game-treasury-tras".to_string(),
                                            class: wizard_step_class(active_step, PolarWizardStep::GameTreasuryTras).to_string(),
                                            number: 6,
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
                                                        push_toast(toast, toast_sequence, "Returned to step 5.", ToastTone::Success);
                                                        focus_step_control("polar-game-treasury-submit").await;
                                                        is_busy.set(false);
                                                    },
                                                    "RESET"
                                                }
                                            }),
                                        }

                                        Instruction {
                                            id: "polar-step-user-nodes-sats".to_string(),
                                            class: wizard_step_class(active_step, PolarWizardStep::UserNodesSats).to_string(),
                                            number: 7,
                                            info: "Rebalances sats between Game Treasury and the user nodes".to_string(),
                                            name: rsx! { "{polar_wizard_step_label(PolarWizardStep::UserNodesSats)}" },
                                            value: Some(rsx! {
                                                label { class: "setup-field-row",
                                                    input {
                                                        id: "polar-user-nodes-sats-input",
                                                        r#type: "text",
                                                        value: "Jack, Bob, Carol sats match the demo targets.",
                                                        readonly: true,
                                                    }
                                                }
                                            }),
                                            actions: Some(rsx! {
                                                button {
                                                    id: "polar-user-nodes-sats-submit",
                                                    class: "primary-action",
                                                    r#type: "button",
                                                    disabled: is_busy() || active_step != PolarWizardStep::UserNodesSats,
                                                    onclick: move |_| async move {
                                                        rebalance_user_nodes_sats_step(
                                                            is_busy,
                                                            setup_profile,
                                                            lab_state,
                                                            toast,
                                                            toast_sequence,
                                                        )
                                                        .await;
                                                    },
                                                    "SUBMIT"
                                                }
                                                button {
                                                    id: "polar-user-nodes-sats-reset",
                                                    class: "secondary-action danger-action",
                                                    r#type: "button",
                                                    disabled: is_busy() || active_step != PolarWizardStep::UserNodesSats,
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
                                                                push_toast(toast, toast_sequence, "Returned to step 6.", ToastTone::Success);
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
                                            id: "polar-step-user-nodes-tras".to_string(),
                                            class: wizard_step_class(active_step, PolarWizardStep::UserNodesTras).to_string(),
                                            number: 8,
                                            info: "Rebalances TRAs between Game Treasury and the user nodes".to_string(),
                                            name: rsx! { "{polar_wizard_step_label(PolarWizardStep::UserNodesTras)}" },
                                            value: Some(rsx! {
                                                if let Some(state) = current_lab_state.as_ref() {
                                                    NpcItemTransferStatus { transfers: state.npc_item_transfers.clone() }
                                                } else {
                                                    label { class: "setup-field-row",
                                                        input {
                                                            id: "polar-tra-assets-input",
                                                            r#type: "text",
                                                            value: "Jack, Bob, Carol TRAs match the demo targets.",
                                                            readonly: true,
                                                            disabled: active_step != PolarWizardStep::UserNodesTras,
                                                        }
                                                    }
                                                }
                                            }),
                                            actions: Some(rsx! {
                                                button {
                                                    id: "polar-tra-assets-submit",
                                                    class: "primary-action",
                                                    r#type: "button",
                                                    disabled: is_busy() || active_step != PolarWizardStep::UserNodesTras,
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
                                                    disabled: is_busy() || active_step != PolarWizardStep::UserNodesTras,
                                                    onclick: move |_| async move {
                                                        is_busy.set(true);
                                                        let saved_profile = reset_to_demo_nodes_step(setup_profile());
                                                        setup_profile.set(saved_profile.clone());
                                                        lab_state.set(lab_state_after_reset_to_demo_nodes_step(saved_profile.clone()));
                                                        push_toast(toast, toast_sequence, "Returned to step 7. User node sats will be rechecked on submit.", ToastTone::Success);
                                                        focus_step_control("polar-user-nodes-sats-submit").await;
                                                        is_busy.set(false);
                                                    },
                                                    "RESET"
                                                }
                                            }),
                                        }

                                        Instruction {
                                            id: "polar-step-block-height".to_string(),
                                            class: wizard_step_class(active_step, PolarWizardStep::BlockHeight).to_string(),
                                            number: 9,
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
                                                        let saved_profile = reset_to_user_nodes_tras_step(setup_profile());
                                                        setup_profile.set(saved_profile.clone());
                                                        if let Some(mut state) = lab_state().or_else(storage_service::load_lab_state_snapshot) {
                                                            state.profile = saved_profile;
                                                            state.npc_item_transfers.clear();
                                                            if let Ok(next_state) = lightning_service::TraService::prepare_game_treasury_items(state.clone()) {
                                                                state = next_state;
                                                            }
                                                            lab_state.set(Some(state));
                                                        }
                                                        push_toast(toast, toast_sequence, "Returned to step 8. User node TRAs will be rebalanced on submit.", ToastTone::Success);
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
                                            number: 10,
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
            if show_delete_all_networks_confirm() {
                DeleteAllNetworksConfirmationPrompt {
                    is_busy,
                    show_prompt: show_delete_all_networks_confirm,
                    setup_profile,
                    lab_state,
                    operation_prompt,
                    toast,
                    toast_sequence,
                    delete_all_networks_confirmation_count,
                    amount_text,
                    polar_server_name,
                    polar_bridge_url,
                    polar_block_height,
                    bridge_connection_error,
                    autopilot_enabled,
                    autopilot_status,
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
fn DeleteAllNetworksConfirmationPrompt(
    mut is_busy: Signal<bool>,
    mut show_prompt: Signal<bool>,
    setup_profile: Signal<SetupProfile>,
    lab_state: Signal<Option<LabState>>,
    mut operation_prompt: Signal<Option<OperationPrompt>>,
    toast: Signal<Option<Toast>>,
    toast_sequence: Signal<u64>,
    delete_all_networks_confirmation_count: Signal<Option<usize>>,
    amount_text: Signal<String>,
    polar_server_name: Signal<String>,
    polar_bridge_url: Signal<String>,
    polar_block_height: Signal<String>,
    bridge_connection_error: Signal<String>,
    autopilot_enabled: Signal<bool>,
    autopilot_status: Signal<String>,
) -> Element {
    let active_delete_prompt = operation_prompt();

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
                    if let Some(prompt) = active_delete_prompt.as_ref() {
                        if prompt.is_pending {
                            span { class: "eyebrow", "Pending operation" }
                        } else {
                            span { class: "eyebrow", "Action required" }
                        }
                        h2 { "{prompt.title}" }
                        p { "{prompt.message}" }
                    } else {
                        span { class: "eyebrow", "Action required" }
                        h2 { "Are you sure?" }
                        p { "{delete_all_networks_confirmation_message(delete_all_networks_confirmation_count())}" }
                    }
                }
                div { class: "operation-prompt__actions",
                    if active_delete_prompt.as_ref().map(|prompt| !prompt.is_pending).unwrap_or(false) {
                        button {
                            class: "primary-action",
                            r#type: "button",
                            onclick: move |_| {
                                operation_prompt.set(None);
                                show_prompt.set(false);
                            },
                            "Continue"
                        }
                    } else if !is_busy() {
                        button {
                            class: "secondary-action",
                            r#type: "button",
                            onclick: move |_| show_prompt.set(false),
                            "Cancel"
                        }
                        button {
                            id: "polar-delete-all-networks-confirm",
                            class: "secondary-action danger-action",
                            r#type: "button",
                            disabled: delete_all_networks_button_disabled(is_busy()),
                            onclick: move |_| async move {
                                let delete_succeeded = delete_all_networks_from_setup(
                                    is_busy,
                                    setup_profile,
                                    lab_state,
                                    operation_prompt,
                                    toast,
                                    toast_sequence,
                                    amount_text,
                                    polar_server_name,
                                    polar_bridge_url,
                                    polar_block_height,
                                    bridge_connection_error,
                                    autopilot_enabled,
                                    autopilot_status,
                                    delete_all_networks_confirmation_count,
                                ).await;
                                if delete_succeeded {
                                    show_prompt.set(false);
                                }
                            },
                            "Delete all"
                        }
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
        PolarWizardStep::LocalAppUrl => Some("polar-bridge-url-input"),
        PolarWizardStep::BridgeUrl => Some("polar-server-name-input"),
        PolarWizardStep::ServerName => Some(POLAR_DEMO_NODES_SUBMIT_ID),
        PolarWizardStep::CreateNodes => Some("polar-game-treasury-submit"),
        PolarWizardStep::GameTreasury => Some("polar-game-treasury-tras-submit"),
        PolarWizardStep::GameTreasuryTras => Some("polar-user-nodes-sats-submit"),
        PolarWizardStep::UserNodesSats => Some("polar-tra-assets-submit"),
        PolarWizardStep::UserNodesTras => Some("polar-block-height-input"),
        PolarWizardStep::BlockHeight => Some("polar-complete-submit"),
        PolarWizardStep::Complete | PolarWizardStep::Done => None,
    }
}

fn reset_focus_target(step: PolarWizardStep) -> &'static str {
    match step {
        PolarWizardStep::LocalAppUrl => "polar-local-app-url-submit",
        PolarWizardStep::BridgeUrl => "polar-local-app-url-submit",
        PolarWizardStep::ServerName => "polar-bridge-url-input",
        PolarWizardStep::CreateNodes => "polar-server-name-input",
        PolarWizardStep::GameTreasury => POLAR_DEMO_NODES_SUBMIT_ID,
        PolarWizardStep::GameTreasuryTras => "polar-game-treasury-submit",
        PolarWizardStep::UserNodesSats => "polar-game-treasury-tras-submit",
        PolarWizardStep::UserNodesTras => "polar-user-nodes-sats-submit",
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

fn user_auth_mode_description(mode: UserAuthMode) -> &'static str {
    match mode {
        UserAuthMode::App => {
            "Development mode: the app acts on behalf of the player without QR approvals."
        }
        UserAuthMode::MockLnAuth => {
            "Development QR mode: the app shows LNAuth-style prompts and auto-completes them for testing."
        }
        UserAuthMode::LnAuth => {
            "Wallet authorization mode: Play Game asks the player wallet to approve login and value-moving actions."
        }
    }
}

fn apply_user_auth_mode(
    mode: UserAuthMode,
    user_auth_mode: &mut Signal<UserAuthMode>,
    mut setup_profile: Signal<SetupProfile>,
    mut lab_state: Signal<Option<LabState>>,
) {
    user_auth_mode.set(mode);

    let mut profile = setup_profile();
    if profile.user_auth_mode == mode {
        return;
    }

    profile.user_auth_mode = mode;
    if mode == UserAuthMode::LnAuth && profile.lnauth_bridge_url.trim().is_empty() {
        profile.lnauth_bridge_url = default_lnauth_bridge_url();
    }
    profile.player_identity = None;
    profile.last_auth_status = None;
    if profile.is_connected() {
        profile.connection_status = ConnectionStatus::PartiallyConnected;
        profile.last_verified_at = None;
    }

    setup_profile.set(profile.clone());
    storage_service::save_setup_profile(&profile);

    if let Some(mut state) = lab_state() {
        state.profile = profile;
        state.player_auth_session = None;
        state.recent_transaction_approvals.clear();
        state.auth_warnings.clear();
        storage_service::save_lab_state_snapshot(&state);
        lab_state.set(Some(state));
    }
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

    if !local_app_url_step_ready(profile, browser_origin_allows_polar_bridge()) {
        return PolarWizardStep::LocalAppUrl;
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

    if !user_nodes_ready(lab_state) {
        return PolarWizardStep::CreateNodes;
    }

    if !profile.game_treasury_ready {
        return PolarWizardStep::GameTreasury;
    }

    if !treasury_tras_ready(lab_state) {
        return PolarWizardStep::GameTreasuryTras;
    }

    if !user_nodes_sats_ready(lab_state) {
        return PolarWizardStep::UserNodesSats;
    }

    if !npc_item_transfers_ready(lab_state) {
        return PolarWizardStep::UserNodesTras;
    }

    if !profile.polar_block_height_confirmed {
        return PolarWizardStep::BlockHeight;
    }

    PolarWizardStep::Complete
}

fn local_app_url_step_ready(profile: &SetupProfile, browser_origin_ready: bool) -> bool {
    profile.local_app_url_ready && browser_origin_ready
}

fn lab_state_has_status(lab_state: Option<&LabState>, status: ConnectionStatus) -> bool {
    match lab_state {
        Some(state) => state.profile.connection_status == status,
        None => false,
    }
}

fn user_nodes_sats_ready(lab_state: Option<&LabState>) -> bool {
    user_nodes_ready(lab_state)
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
            let treasury_books = verified_tra_count(state, DemoNodeId::GameTreasury, BOOK_ITEM_ID);
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
fn current_browser_hostname() -> Option<String> {
    web_sys::window()
        .and_then(|window| window.location().hostname().ok())
        .map(|hostname| hostname.to_ascii_lowercase())
}

#[cfg(target_arch = "wasm32")]
fn browser_origin_allows_polar_bridge() -> bool {
    current_browser_hostname()
        .map(|hostname| browser_hostname_allows_polar_bridge(&hostname))
        .unwrap_or(false)
}

#[cfg(any(target_arch = "wasm32", test))]
fn browser_hostname_allows_polar_bridge(hostname: &str) -> bool {
    hostname.eq_ignore_ascii_case("localhost")
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

fn is_retryable_polar_start_error(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    message.contains("port is already allocated")
        || message.contains("ports are not available")
        || message.contains("only one usage of each socket address")
        || message.contains("orphan containers")
        || message.contains("could not start network")
        || message.contains("is not listed by the current polar bridge")
        || message.contains("timed out after")
        || message.contains("timeout")
}

fn bridge_step_error_message(_message: impl AsRef<str>) -> String {
    "Error: Cannot connect to Polar, revisit 2. Environment step 04 for more info".to_string()
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

fn reset_to_local_app_url_step(mut profile: SetupProfile) -> SetupProfile {
    profile.connection_status = ConnectionStatus::NotConfigured;
    profile.local_app_url_ready = false;
    profile.polar_block_height_confirmed = false;
    profile.last_verified_at = None;
    profile.polar_automation.network_id.clear();
    profile.polar_automation.bitcoin_backend_name = DEFAULT_BITCOIN_BACKEND_NAME.to_string();
    storage_service::save_setup_profile(&profile);
    profile
}

fn reset_to_bridge_url_step(mut profile: SetupProfile) -> SetupProfile {
    profile.connection_status = ConnectionStatus::NotConfigured;
    profile.local_app_url_ready = true;
    profile.polar_block_height_confirmed = false;
    profile.last_verified_at = None;
    profile.polar_automation.network_id.clear();
    profile.polar_automation.bitcoin_backend_name = DEFAULT_BITCOIN_BACKEND_NAME.to_string();
    storage_service::save_setup_profile(&profile);
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
        state
            .nodes
            .retain(|node| node.node_id == DemoNodeId::GameTreasury);
        state.tra_items.clear();
        state.game_treasury.owned_items.clear();
        state.game_treasury.inventory_value_sats = 0;
        state.npc_item_transfers.clear();
        state.profile.connection_status = ConnectionStatus::PartiallyConnected;
        state.profile.polar_block_height_confirmed = false;
        state
    });

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

    profile
}

fn lab_state_after_reset_to_demo_nodes_step(profile: SetupProfile) -> Option<LabState> {
    let current = storage_service::load_lab_state_snapshot();
    let mut state =
        current.unwrap_or_else(|| lightning_service::default_lab_state(profile.clone()));
    state.profile = profile;
    state
        .nodes
        .retain(|node| node.node_id == DemoNodeId::GameTreasury);
    state.npc_item_transfers.clear();
    if state.game_treasury.status != crate::client::models::TreasuryStatus::Ready {
        state = lightning_service::TraService::prepare_game_treasury(state).ok()?;
    }
    let state = lightning_service::TraService::prepare_game_treasury_items(state).ok()?;

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

fn reset_to_user_nodes_tras_step(mut profile: SetupProfile) -> SetupProfile {
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
        profile.local_app_url_ready = true;
        profile.connection_status = status;
        profile.network_name = DEFAULT_NETWORK_NAME.to_string();
        profile.polar_automation.network_id = network_id.to_string();
        profile
    }

    #[test]
    fn polar_wizard_starts_at_local_app_url_until_url_is_verified() {
        let mut profile = profile_with_status(ConnectionStatus::NotConfigured, "");
        profile.local_app_url_ready = false;

        assert_eq!(
            polar_wizard_step(&profile, None).order(),
            PolarWizardStep::LocalAppUrl.order()
        );
    }

    #[test]
    fn local_app_url_step_requires_saved_flag_and_runtime_origin() {
        let mut profile = profile_with_status(ConnectionStatus::SavedOffline, "");
        profile.local_app_url_ready = true;

        assert!(local_app_url_step_ready(&profile, true));
        assert!(!local_app_url_step_ready(&profile, false));

        profile.local_app_url_ready = false;
        assert!(!local_app_url_step_ready(&profile, true));
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
            PolarWizardStep::CreateNodes.order()
        );
        let create_nodes_state = user_nodes_ready_state(server_ready.clone());
        assert_eq!(
            polar_wizard_step(&server_ready, Some(&create_nodes_state)).order(),
            PolarWizardStep::GameTreasury.order()
        );
        assert_eq!(
            polar_wizard_step(&treasury_ready, Some(&create_nodes_state)).order(),
            PolarWizardStep::GameTreasuryTras.order()
        );
        let mut treasury_tras_state = lightning_service::default_lab_state(treasury_ready.clone());
        for node in treasury_tras_state.nodes.iter_mut() {
            if DemoNodeId::ALL.contains(&node.node_id) {
                node.status = crate::client::models::NodeStatus::Online;
                node.pubkey = Some(format!("{}-pubkey", node.alias));
            }
        }
        treasury_tras_state.tra_items = treasury_tra_items();
        assert_eq!(
            polar_wizard_step(&treasury_ready, Some(&treasury_tras_state)).order(),
            PolarWizardStep::UserNodesTras.order()
        );
        let mut demo_nodes_state = user_nodes_ready_state(demo_nodes_ready.clone());
        demo_nodes_state.tra_items = treasury_tra_items();
        assert_eq!(
            polar_wizard_step(&demo_nodes_ready, Some(&demo_nodes_state)).order(),
            PolarWizardStep::UserNodesTras.order()
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
    fn user_nodes_tras_reset_without_snapshot_revisits_create_nodes() {
        let mut profile = profile_with_status(ConnectionStatus::PartiallyConnected, "network-1");
        profile.game_treasury_ready = true;
        let reset_profile = reset_to_demo_nodes_step(profile.clone());
        let state = lab_state_after_reset_to_demo_nodes_step(reset_profile.clone());

        assert!(state
            .as_ref()
            .is_some_and(|state| treasury_tras_ready(Some(state))));
        assert_eq!(
            polar_wizard_step(&reset_profile, state.as_ref()).order(),
            PolarWizardStep::CreateNodes.order()
        );
    }

    #[test]
    fn block_height_reset_returns_to_user_nodes_tras() {
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

        let reset_profile = reset_to_user_nodes_tras_step(profile);
        state.tra_items = treasury_tra_items();
        state.npc_item_transfers.clear();
        state.profile = reset_profile.clone();

        assert_eq!(
            polar_wizard_step(&reset_profile, Some(&state)).order(),
            PolarWizardStep::UserNodesTras.order()
        );
        assert!(!reset_profile.polar_block_height_confirmed);
    }

    #[test]
    fn submit_focus_targets_advance_to_next_step() {
        assert_eq!(
            submit_focus_target(PolarWizardStep::LocalAppUrl),
            Some("polar-bridge-url-input")
        );
        assert_eq!(
            submit_focus_target(PolarWizardStep::BridgeUrl),
            Some("polar-server-name-input")
        );
        assert_eq!(
            submit_focus_target(PolarWizardStep::ServerName),
            Some(POLAR_DEMO_NODES_SUBMIT_ID)
        );
        assert_eq!(
            submit_focus_target(PolarWizardStep::CreateNodes),
            Some("polar-game-treasury-submit")
        );
        assert_eq!(
            submit_focus_target(PolarWizardStep::GameTreasury),
            Some("polar-game-treasury-tras-submit")
        );
        assert_eq!(
            submit_focus_target(PolarWizardStep::GameTreasuryTras),
            Some("polar-user-nodes-sats-submit")
        );
        assert_eq!(
            submit_focus_target(PolarWizardStep::UserNodesSats),
            Some("polar-tra-assets-submit")
        );
        assert_eq!(
            submit_focus_target(PolarWizardStep::UserNodesTras),
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
            polar_wizard_step_label(PolarWizardStep::LocalAppUrl),
            polar_wizard_step_label(PolarWizardStep::BridgeUrl),
            polar_wizard_step_label(PolarWizardStep::ServerName),
            polar_wizard_step_label(PolarWizardStep::CreateNodes),
            polar_wizard_step_label(PolarWizardStep::GameTreasury),
            polar_wizard_step_label(PolarWizardStep::GameTreasuryTras),
            polar_wizard_step_label(PolarWizardStep::UserNodesSats),
            polar_wizard_step_label(PolarWizardStep::UserNodesTras),
            polar_wizard_step_label(PolarWizardStep::BlockHeight),
            polar_wizard_step_label(PolarWizardStep::Complete),
        ];

        assert_eq!(
            labels,
            [
                "App URL",
                "Bridge URLs",
                "Server Name",
                "Create Nodes",
                "Game Treasury (Sats)",
                "Game Treasury (TRAs)",
                "User Nodes (Sats)",
                "User Nodes (TRAs)",
                "Block Height",
                "Unlock Routes"
            ]
        );
    }

    #[test]
    fn reset_focus_targets_return_to_previous_step() {
        assert_eq!(
            reset_focus_target(PolarWizardStep::BridgeUrl),
            "polar-local-app-url-submit"
        );
        assert_eq!(
            reset_focus_target(PolarWizardStep::ServerName),
            "polar-bridge-url-input"
        );
        assert_eq!(
            reset_focus_target(PolarWizardStep::GameTreasury),
            POLAR_DEMO_NODES_SUBMIT_ID
        );
        assert_eq!(
            reset_focus_target(PolarWizardStep::CreateNodes),
            "polar-server-name-input"
        );
        assert_eq!(
            reset_focus_target(PolarWizardStep::GameTreasuryTras),
            "polar-game-treasury-submit"
        );
        assert_eq!(
            reset_focus_target(PolarWizardStep::UserNodesSats),
            "polar-game-treasury-tras-submit"
        );
        assert_eq!(
            reset_focus_target(PolarWizardStep::UserNodesTras),
            "polar-user-nodes-sats-submit"
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
        assert!(!browser_hostname_allows_polar_bridge("127.0.0.1"));
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
    fn autopilot_retries_polar_start_port_collisions() {
        assert!(is_retryable_polar_start_error(
            "Polar bridge could not start network 13. ports are not available"
        ));
        assert!(is_retryable_polar_start_error(
            "Bind for 0.0.0.0:64613 failed: port is already allocated"
        ));
        assert!(is_retryable_polar_start_error(
            "Polar bridge POST http://localhost:37373/api/mcp/execute timed out after 30 seconds."
        ));
        assert!(is_retryable_polar_start_error(
            "Polar network autopilot-1779143204525-1 is not listed by the current Polar bridge."
        ));
        assert!(!is_retryable_polar_start_error(
            "Enter a Polar server name before creating it."
        ));
    }

    #[test]
    fn autopilot_server_step_timeout_has_actionable_message() {
        assert_eq!(
            autopilot_step_timeout_message("finding or creating the Polar server", 20),
            "Timed out after 20s while finding or creating the Polar server. Autopilot stopped instead of waiting forever."
        );
    }

    #[test]
    fn delete_all_networks_progress_message_includes_deleted_and_total() {
        assert_eq!(
            delete_all_networks_progress_message_for_counts(8, 8),
            "Deleting 8/8 polar networks visible to the local bridge..."
        );
        assert_eq!(
            delete_all_networks_progress_message_for_counts(0, 21),
            "Deleting 0/21 polar networks visible to the local bridge..."
        );
    }

    #[test]
    fn delete_all_networks_uses_counting_message_before_total_is_known() {
        assert_eq!(
            delete_all_networks_started_message(" http://localhost:37373 "),
            "Delete request started. Checking Polar bridge at http://localhost:37373..."
        );
        assert_eq!(
            delete_all_networks_count_started_message(" http://localhost:37373 "),
            "Checking how many Polar networks are visible at http://localhost:37373..."
        );
        assert_eq!(
            delete_all_networks_counting_message(),
            "Finding polar networks visible to the local bridge... This will fail after 300s if the bridge does not answer."
        );
        assert_eq!(
            delete_all_networks_progress_message_for_counts(0, 0),
            "No polar networks visible to the local bridge."
        );
    }

    #[test]
    fn delete_all_networks_confirmation_message_includes_network_count() {
        assert_eq!(
            delete_all_networks_confirmation_message(Some(3)),
            "The local Polar bridge reports 3 Polar network(s). Delete all 3 network(s) and lock the app setup?"
        );
        assert_eq!(
            delete_all_networks_confirmation_message(Some(0)),
            "The local Polar bridge reports 0 networks. Confirming will verify setup is already clear and keep the app locked."
        );
    }

    #[test]
    fn delete_all_networks_timeout_message_is_actionable() {
        assert_eq!(
            delete_all_networks_timeout_message(300),
            "Timed out after 300s while deleting Polar networks. The app stopped waiting so setup is not left busy forever."
        );
    }

    #[test]
    fn delete_all_networks_button_depends_only_on_busy_state() {
        assert!(!delete_all_networks_button_disabled(false));
        assert!(delete_all_networks_button_disabled(true));
    }

    #[test]
    fn delete_all_networks_uses_current_bridge_input_before_step_one_is_saved() {
        let mut profile = SetupProfile::default();
        profile.connection_status = ConnectionStatus::NotConfigured;
        profile.polar_automation.bridge_url = "http://old-bridge:37373".to_string();

        let delete_profile =
            delete_all_networks_profile_from_input(profile, " http://localhost:37373 ".to_string());

        assert_eq!(
            delete_profile.connection_status,
            ConnectionStatus::NotConfigured
        );
        assert_eq!(
            delete_profile.polar_automation.bridge_url,
            "http://localhost:37373"
        );
    }

    #[test]
    fn bridge_step_error_points_back_to_localhost_step_two() {
        let message = bridge_step_error_message("TypeError: Failed to fetch");

        assert_eq!(
            message,
            "Error: Cannot connect to Polar, revisit 2. Environment step 04 for more info"
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

    #[test]
    fn autopilot_server_names_are_fresh_and_predictable() {
        assert_eq!(
            autopilot_server_name_from_timestamp(1_779_120_000),
            "autopilot-1779120000"
        );
    }

    #[test]
    fn autopilot_initial_server_name_honors_step_2_field() {
        assert_eq!(
            requested_autopilot_server_name(" User Selected Polar "),
            "User Selected Polar"
        );

        let generated = requested_autopilot_server_name(" ");
        assert!(generated.starts_with("autopilot-"));
    }

    #[test]
    fn autopilot_fresh_retry_profile_rewinds_to_server_ready_state() {
        let mut profile = profile_with_status(ConnectionStatus::Connected, "stale-network");
        profile.network_name = "stale-network".to_string();
        profile.polar_block_height_confirmed = true;
        profile.game_treasury_ready = true;
        profile.game_treasury_funded_sats = 10_000;
        profile.last_verified_at = Some(chrono::Utc::now());

        let retry_profile =
            autopilot_profile_for_fresh_server_retry(profile, "autopilot-1779210731555");

        assert_eq!(retry_profile.network_name, "autopilot-1779210731555");
        assert_eq!(
            retry_profile.polar_automation.network_id,
            "autopilot-1779210731555"
        );
        assert_eq!(
            retry_profile.connection_status,
            ConnectionStatus::SavedOffline
        );
        assert!(!retry_profile.polar_block_height_confirmed);
        assert!(!retry_profile.game_treasury_ready);
        assert_eq!(retry_profile.game_treasury_funded_sats, 0);
        assert_eq!(retry_profile.last_verified_at, None);
    }
}

async fn run_polar_setup_autopilot(
    mut is_busy: Signal<bool>,
    mut setup_profile: Signal<SetupProfile>,
    mut lab_state: Signal<Option<LabState>>,
    operation_prompt: Signal<Option<OperationPrompt>>,
    prompt_sequence: Signal<u64>,
    toast: Signal<Option<Toast>>,
    toast_sequence: Signal<u64>,
    mut bridge_connection_error: Signal<String>,
    amount_text: String,
    mut polar_server_name: Signal<String>,
    polar_bridge_url: String,
    lnauth_bridge_url: String,
    mut polar_block_height: Signal<String>,
    mut autopilot_enabled: Signal<bool>,
    mut autopilot_status: Signal<String>,
) {
    is_busy.set(true);
    autopilot_enabled.set(true);
    autopilot_status.set("Starting setup run...".to_string());
    let started_at = chrono::Utc::now();
    let operation_id = begin_operation_prompt(
        operation_prompt,
        prompt_sequence,
        POLAR_AUTOPILOT_PROMPT_TITLE,
        "Starting setup run...",
        false,
    )
    .await;

    let result = run_polar_setup_autopilot_inner(
        setup_profile,
        lab_state,
        operation_prompt,
        operation_id,
        &mut bridge_connection_error,
        amount_text,
        &mut polar_server_name,
        polar_bridge_url,
        lnauth_bridge_url,
        &mut polar_block_height,
        &mut autopilot_status,
    )
    .await;

    let elapsed_seconds = (chrono::Utc::now() - started_at).num_seconds().max(0);
    close_operation_prompt(operation_prompt, operation_id);

    match result {
        Ok(state) => {
            let message = if elapsed_seconds <= POLAR_AUTOPILOT_TARGET_SECONDS {
                format!(
                    "Autopilot completed in {elapsed_seconds}s. Final setup verification passed."
                )
            } else {
                format!(
                    "Autopilot completed in {elapsed_seconds}s and verified setup. Target is {POLAR_AUTOPILOT_TARGET_SECONDS}s."
                )
            };
            setup_profile.set(state.profile.clone());
            lab_state.set(Some(state));
            autopilot_status.set(message.clone());
            push_toast(toast, toast_sequence, message, ToastTone::Success);
        }
        Err(message) => {
            autopilot_status.set(format!("Stopped: {message}"));
            push_toast(toast, toast_sequence, message, ToastTone::Error);
        }
    }

    is_busy.set(false);
}

async fn delete_all_networks_from_setup(
    mut is_busy: Signal<bool>,
    mut setup_profile: Signal<SetupProfile>,
    mut lab_state: Signal<Option<LabState>>,
    mut operation_prompt: Signal<Option<OperationPrompt>>,
    toast: Signal<Option<Toast>>,
    toast_sequence: Signal<u64>,
    mut amount_text: Signal<String>,
    mut polar_server_name: Signal<String>,
    mut polar_bridge_url: Signal<String>,
    mut polar_block_height: Signal<String>,
    mut bridge_connection_error: Signal<String>,
    mut autopilot_enabled: Signal<bool>,
    mut autopilot_status: Signal<String>,
    delete_all_networks_confirmation_count: Signal<Option<usize>>,
) -> bool {
    is_busy.set(true);
    let delete_profile =
        delete_all_networks_profile_from_input(setup_profile(), polar_bridge_url());
    let initial_total = delete_all_networks_confirmation_count();
    operation_prompt.set(Some(OperationPrompt {
        operation_id: 90_000,
        title: "Delete all Polar networks".to_string(),
        subtitle: None,
        message: initial_total
            .map(|total| delete_all_networks_progress_message_for_counts(0, total))
            .unwrap_or_else(delete_all_networks_counting_message),
        tone: ToastTone::Error,
        is_pending: true,
        can_cancel: false,
        cancel_requested: false,
    }));

    let mut progress_prompt = operation_prompt;
    match with_delete_all_networks_timeout(
        delete_all_polar_networks_with_progress(delete_profile, move |progress| {
            progress_prompt.set(Some(OperationPrompt {
                operation_id: 90_000,
                title: "Delete all Polar networks".to_string(),
                subtitle: None,
                message: delete_all_networks_progress_message(&progress),
                tone: ToastTone::Error,
                is_pending: true,
                can_cancel: false,
                cancel_requested: false,
            }));
        }),
        POLAR_DELETE_ALL_TIMEOUT_SECONDS,
    )
    .await
    {
        Ok((default_profile, result)) => {
            setup_profile.set(default_profile.clone());
            lab_state.set(None);
            amount_text.set(default_profile.sats_per_transaction.to_string());
            polar_server_name.set(default_profile.network_name.clone());
            polar_bridge_url.set(default_profile.polar_automation.bridge_url.clone());
            polar_block_height.set("0".to_string());
            bridge_connection_error.set(String::new());
            autopilot_enabled.set(false);
            autopilot_status.set("Ready".to_string());
            operation_prompt.set(None);
            push_toast(
                toast,
                toast_sequence,
                format!("Deleted {} Polar network(s).", result.deleted_count),
                ToastTone::Success,
            );
            is_busy.set(false);
            true
        }
        Err(message) => {
            operation_prompt.set(Some(OperationPrompt {
                operation_id: 90_001,
                title: "Delete all Polar networks failed".to_string(),
                subtitle: None,
                message: message.clone(),
                tone: ToastTone::Error,
                is_pending: false,
                can_cancel: false,
                cancel_requested: false,
            }));
            push_toast(toast, toast_sequence, message, ToastTone::Error);
            is_busy.set(false);
            false
        }
    }
}

async fn prepare_delete_all_networks_confirmation(
    mut is_busy: Signal<bool>,
    mut show_prompt: Signal<bool>,
    mut confirmation_count: Signal<Option<usize>>,
    setup_profile: Signal<SetupProfile>,
    mut operation_prompt: Signal<Option<OperationPrompt>>,
    toast: Signal<Option<Toast>>,
    toast_sequence: Signal<u64>,
    polar_bridge_url: String,
) {
    is_busy.set(true);
    confirmation_count.set(None);
    show_prompt.set(false);

    let delete_profile = delete_all_networks_profile_from_input(setup_profile(), polar_bridge_url);
    let bridge_url = delete_profile.polar_automation.bridge_url.clone();
    operation_prompt.set(Some(OperationPrompt {
        operation_id: 89_999,
        title: "Count Polar networks".to_string(),
        subtitle: None,
        message: delete_all_networks_count_started_message(&bridge_url),
        tone: ToastTone::Error,
        is_pending: true,
        can_cancel: false,
        cancel_requested: false,
    }));

    let result = with_delete_all_networks_timeout(
        count_polar_networks(delete_profile),
        POLAR_DELETE_ALL_COUNT_TIMEOUT_SECONDS,
    )
    .await;

    match result {
        Ok(count) => {
            operation_prompt.set(None);
            confirmation_count.set(Some(count));
            show_prompt.set(true);
        }
        Err(message) => {
            operation_prompt.set(Some(OperationPrompt {
                operation_id: 89_998,
                title: "Could not count Polar networks".to_string(),
                subtitle: None,
                message: message.clone(),
                tone: ToastTone::Error,
                is_pending: false,
                can_cancel: false,
                cancel_requested: false,
            }));
            push_toast(toast, toast_sequence, message, ToastTone::Error);
        }
    }

    is_busy.set(false);
}

fn delete_all_networks_count_started_message(bridge_url: &str) -> String {
    format!(
        "Checking how many Polar networks are visible at {}...",
        bridge_url.trim()
    )
}

fn delete_all_networks_confirmation_message(count: Option<usize>) -> String {
    match count {
        Some(0) => {
            "The local Polar bridge reports 0 networks. Confirming will verify setup is already clear and keep the app locked.".to_string()
        }
        Some(count) => format!(
            "The local Polar bridge reports {count} Polar network(s). Delete all {count} network(s) and lock the app setup?"
        ),
        None => {
            "The app is still counting Polar networks. Wait for the count before confirming.".to_string()
        }
    }
}

#[cfg(test)]
fn delete_all_networks_started_message(bridge_url: &str) -> String {
    format!(
        "Delete request started. Checking Polar bridge at {}...",
        bridge_url.trim()
    )
}

fn delete_all_networks_counting_message() -> String {
    format!(
        "Finding polar networks visible to the local bridge... This will fail after {POLAR_DELETE_ALL_TIMEOUT_SECONDS}s if the bridge does not answer."
    )
}

fn delete_all_networks_progress_message(progress: &PolarDeleteAllProgress) -> String {
    delete_all_networks_progress_message_for_counts(progress.deleted, progress.total)
}

fn delete_all_networks_progress_message_for_counts(deleted: usize, total: usize) -> String {
    if total == 0 {
        return "No polar networks visible to the local bridge.".to_string();
    }

    format!("Deleting {deleted}/{total} polar networks visible to the local bridge...")
}

async fn with_delete_all_networks_timeout<F, T>(future: F, seconds: u32) -> Result<T, String>
where
    F: std::future::Future<Output = Result<T, String>>,
{
    match select(
        future.boxed_local(),
        delete_all_networks_timeout_delay(seconds).boxed_local(),
    )
    .await
    {
        Either::Left((result, _)) => result,
        Either::Right((_, _)) => Err(delete_all_networks_timeout_message(seconds)),
    }
}

#[cfg(target_arch = "wasm32")]
async fn delete_all_networks_timeout_delay(seconds: u32) {
    gloo_timers::future::TimeoutFuture::new(seconds.saturating_mul(1_000)).await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn delete_all_networks_timeout_delay(seconds: u32) {
    futures_timer::Delay::new(std::time::Duration::from_secs(seconds as u64)).await;
}

fn delete_all_networks_timeout_message(seconds: u32) -> String {
    format!(
        "Timed out after {seconds}s while deleting Polar networks. The app stopped waiting so setup is not left busy forever."
    )
}

fn delete_all_networks_button_disabled(is_busy: bool) -> bool {
    is_busy
}

fn delete_all_networks_profile_from_input(
    mut profile: SetupProfile,
    polar_bridge_url: String,
) -> SetupProfile {
    profile.polar_automation =
        polar_automation_from_input(polar_bridge_url, profile.polar_automation);
    profile
}

async fn run_polar_setup_autopilot_inner(
    mut setup_profile: Signal<SetupProfile>,
    mut lab_state: Signal<Option<LabState>>,
    operation_prompt: Signal<Option<OperationPrompt>>,
    operation_id: u64,
    bridge_connection_error: &mut Signal<String>,
    amount_text: String,
    polar_server_name: &mut Signal<String>,
    polar_bridge_url: String,
    lnauth_bridge_url: String,
    polar_block_height: &mut Signal<String>,
    autopilot_status: &mut Signal<String>,
) -> Result<LabState, String> {
    let current_profile = setup_profile();
    let current_step = polar_wizard_step(&current_profile, lab_state().as_ref());
    let should_start_fresh = current_profile.is_connected()
        || current_step == PolarWizardStep::LocalAppUrl
        || current_step == PolarWizardStep::BridgeUrl
        || (current_step == PolarWizardStep::ServerName
            && current_profile
                .polar_automation
                .network_id
                .trim()
                .is_empty());
    let requested_server_name = if should_start_fresh {
        requested_autopilot_server_name(&polar_server_name())
    } else {
        current_profile.network_name.clone()
    };
    polar_server_name.set(requested_server_name.clone());
    if should_start_fresh {
        autopilot_status.set(format!(
            "Using Polar server name {requested_server_name}..."
        ));
    } else {
        autopilot_status.set(format!(
            "Continuing Polar server {requested_server_name}..."
        ));
    }

    let mut profile = profile_from_inputs(
        amount_text,
        requested_server_name.clone(),
        SetupMode::ServerConfig,
        polar_automation_for_requested_server(
            polar_bridge_url.clone(),
            requested_server_name,
            setup_profile().polar_automation,
        ),
        setup_profile(),
    )?;
    profile.lnauth_bridge_url = lnauth_bridge_url.trim().to_string();

    if should_start_fresh && profile.is_connected() {
        profile.connection_status = ConnectionStatus::NotConfigured;
        profile.polar_automation.network_id.clear();
        profile.polar_block_height_confirmed = false;
        profile.game_treasury_ready = false;
        profile.game_treasury_funded_sats = 0;
        profile.last_verified_at = None;
        setup_profile.set(profile.clone());
        lab_state.set(None);
        storage_service::clear_lab_state_snapshot();
    }

    if polar_wizard_step(&profile, lab_state().as_ref()) == PolarWizardStep::LocalAppUrl {
        update_autopilot_operation_prompt(
            operation_prompt,
            operation_id,
            PolarWizardStep::LocalAppUrl,
            "Checking the local app URL...",
            ToastTone::Info,
            true,
            false,
        )
        .await;
        autopilot_status.set("Step 1 of 10: App URL".to_string());
        if !browser_origin_allows_polar_bridge() {
            return Err(format!(
                "Open this app at {LOCAL_APP_URL} before running autopilot."
            ));
        }
        profile.local_app_url_ready = true;
        setup_profile.set(profile.clone());
        storage_service::save_setup_profile(&profile);
    }

    if polar_wizard_step(&profile, lab_state().as_ref()) == PolarWizardStep::BridgeUrl {
        update_autopilot_operation_prompt(
            operation_prompt,
            operation_id,
            PolarWizardStep::BridgeUrl,
            if profile.user_auth_mode == UserAuthMode::LnAuth {
                "Connecting to the Polar and LNAuth bridges..."
            } else {
                "Connecting to the Polar bridge..."
            },
            ToastTone::Info,
            true,
            false,
        )
        .await;
        autopilot_status.set("Step 2 of 10: Bridge URLs".to_string());
        profile.connection_status = ConnectionStatus::SavedOffline;
        profile.last_verified_at = None;
        profile.polar_automation.network_id.clear();
        profile = verify_polar_bridge(profile).await.map_err(|message| {
            *bridge_connection_error.write() = bridge_step_error_message(message.clone());
            bridge_step_error_message(message)
        })?;
        if profile.user_auth_mode == UserAuthMode::LnAuth {
            test_lnauth_bridge_url(profile.lnauth_bridge_url.clone())
                .await
                .map_err(|message| {
                    let message = format!("LNAuth bridge check failed: {message}");
                    *bridge_connection_error.write() = message.clone();
                    message
                })?;
        }
        bridge_connection_error.set(String::new());
        setup_profile.set(profile.clone());
        lab_state.set(None);
    }

    if polar_wizard_step(&profile, lab_state().as_ref()) == PolarWizardStep::ServerName {
        update_autopilot_operation_prompt(
            operation_prompt,
            operation_id,
            PolarWizardStep::ServerName,
            "Finding or creating the named Polar server...",
            ToastTone::Info,
            true,
            false,
        )
        .await;
        autopilot_status.set("Step 3 of 10: Server Name".to_string());
        let mut result = None;
        let mut last_error = None;
        for attempt in 1..=POLAR_AUTOPILOT_SERVER_ATTEMPTS {
            if profile.network_name.trim().is_empty() {
                profile.network_name = polar_server_name().trim().to_string();
            }
            if profile.polar_automation.network_id.trim().is_empty() {
                profile.polar_automation.network_id = profile.network_name.trim().to_string();
            }

            match ensure_polar_server_for_autopilot(profile.clone()).await {
                Ok(server_result) => {
                    result = Some(server_result);
                    break;
                }
                Err(message)
                    if attempt < POLAR_AUTOPILOT_SERVER_ATTEMPTS
                        && is_retryable_polar_start_error(&message) =>
                {
                    last_error = Some(message);
                    cleanup_failed_autopilot_server(&profile).await;
                    let retry_server_name = fresh_autopilot_server_name();
                    polar_server_name.set(retry_server_name.clone());
                    profile.network_name = retry_server_name.clone();
                    profile.polar_automation.network_id = retry_server_name;
                    profile.polar_block_height_confirmed = false;
                    profile.game_treasury_ready = false;
                    profile.game_treasury_funded_sats = 0;
                    setup_profile.set(profile.clone());
                    lab_state.set(None);
                    update_autopilot_operation_prompt(
                        operation_prompt,
                        operation_id,
                        PolarWizardStep::ServerName,
                        format!(
                            "Retry {}/{}: previous Polar port/start allocation failed. Trying fresh server {}...",
                            attempt + 1,
                            POLAR_AUTOPILOT_SERVER_ATTEMPTS,
                            profile.network_name
                        ),
                        ToastTone::Info,
                        true,
                        false,
                    )
                    .await;
                    autopilot_status.set(format!(
                        "Step 3 retry {}/{}: Server Name",
                        attempt + 1,
                        POLAR_AUTOPILOT_SERVER_ATTEMPTS
                    ));
                }
                Err(message) if is_retryable_polar_start_error(&message) => {
                    cleanup_failed_autopilot_server(&profile).await;
                    return Err(format!(
                        "Polar server start failed after {POLAR_AUTOPILOT_SERVER_ATTEMPTS} fresh server attempts: {message}"
                    ));
                }
                Err(message) => return Err(message),
            }
        }
        let result = result.ok_or_else(|| {
            last_error.unwrap_or_else(|| "Polar server could not be prepared.".to_string())
        })?;
        profile.polar_automation = result.profile;
        profile.connection_status = ConnectionStatus::SavedOffline;
        profile.polar_block_height_confirmed = false;
        profile.game_treasury_ready = false;
        profile.game_treasury_funded_sats = 0;
        profile.last_verified_at = None;
        setup_profile.set(profile.clone());
        lab_state.set(None);
    }

    if polar_wizard_step(&profile, lab_state().as_ref()) == PolarWizardStep::CreateNodes {
        update_autopilot_operation_prompt(
            operation_prompt,
            operation_id,
            PolarWizardStep::CreateNodes,
            "Creating required Polar nodes...",
            ToastTone::Info,
            true,
            false,
        )
        .await;
        autopilot_status.set("Step 4 of 10: Create Nodes".to_string());
        let mut state = None;
        let mut last_error = None;
        for attempt in 1..=POLAR_AUTOPILOT_SERVER_ATTEMPTS {
            let progress_prompt = operation_prompt;
            let progress_operation_id = operation_id;
            match create_required_polar_nodes_with_progress(profile.clone(), move |message| {
                update_autopilot_operation_prompt_now(
                    progress_prompt,
                    progress_operation_id,
                    PolarWizardStep::CreateNodes,
                    message,
                    ToastTone::Info,
                    true,
                    false,
                );
            })
            .await
            {
                Ok(created_state) => {
                    state = Some(created_state);
                    break;
                }
                Err(message)
                    if attempt < POLAR_AUTOPILOT_SERVER_ATTEMPTS
                        && is_retryable_polar_start_error(&message) =>
                {
                    last_error = Some(message);
                    cleanup_failed_autopilot_server(&profile).await;
                    let retry_server_name = fresh_autopilot_server_name();
                    polar_server_name.set(retry_server_name.clone());
                    profile = autopilot_profile_for_fresh_server_retry(profile, &retry_server_name);
                    setup_profile.set(profile.clone());
                    lab_state.set(None);
                    update_autopilot_operation_prompt(
                        operation_prompt,
                        operation_id,
                        PolarWizardStep::CreateNodes,
                        format!(
                            "Retry {}/{}: Polar server disappeared or failed during node creation. Trying fresh server {}...",
                            attempt + 1,
                            POLAR_AUTOPILOT_SERVER_ATTEMPTS,
                            profile.network_name
                        ),
                        ToastTone::Info,
                        true,
                        false,
                    )
                    .await;
                    autopilot_status.set(format!(
                        "Step 4 retry {}/{}: Create Nodes",
                        attempt + 1,
                        POLAR_AUTOPILOT_SERVER_ATTEMPTS
                    ));

                    match ensure_polar_server_for_autopilot(profile.clone()).await {
                        Ok(result) => {
                            profile.polar_automation = result.profile;
                            profile.connection_status = ConnectionStatus::SavedOffline;
                            profile.polar_block_height_confirmed = false;
                            profile.game_treasury_ready = false;
                            profile.game_treasury_funded_sats = 0;
                            profile.last_verified_at = None;
                            setup_profile.set(profile.clone());
                            lab_state.set(None);
                        }
                        Err(message)
                            if attempt < POLAR_AUTOPILOT_SERVER_ATTEMPTS
                                && is_retryable_polar_start_error(&message) =>
                        {
                            last_error = Some(message);
                            cleanup_failed_autopilot_server(&profile).await;
                            continue;
                        }
                        Err(message) if is_retryable_polar_start_error(&message) => {
                            cleanup_failed_autopilot_server(&profile).await;
                            return Err(format!(
                                "Polar server start failed after {POLAR_AUTOPILOT_SERVER_ATTEMPTS} fresh server attempts: {message}"
                            ));
                        }
                        Err(message) => return Err(message),
                    }
                }
                Err(message) if is_retryable_polar_start_error(&message) => {
                    cleanup_failed_autopilot_server(&profile).await;
                    return Err(format!(
                        "Create Nodes failed after {POLAR_AUTOPILOT_SERVER_ATTEMPTS} fresh server attempts: {message}"
                    ));
                }
                Err(message) => return Err(message),
            }
        }
        let state = state.ok_or_else(|| {
            last_error.unwrap_or_else(|| "Required Polar nodes could not be created.".to_string())
        })?;
        polar_block_height.set(state.block_height.to_string());
        profile = state.profile.clone();
        setup_profile.set(profile.clone());
        lab_state.set(Some(state));
    }

    if polar_wizard_step(&profile, lab_state().as_ref()) == PolarWizardStep::GameTreasury {
        update_autopilot_operation_prompt(
            operation_prompt,
            operation_id,
            PolarWizardStep::GameTreasury,
            "Checking Game Treasury sats...",
            ToastTone::Info,
            true,
            false,
        )
        .await;
        autopilot_status.set("Step 5 of 10: Game Treasury (Sats)".to_string());
        let mut state = None;
        let mut last_error = None;
        for attempt in 1..=POLAR_AUTOPILOT_SERVER_ATTEMPTS {
            match prepare_game_treasury(profile.clone()).await {
                Ok(treasury_state) => {
                    state = Some(treasury_state);
                    break;
                }
                Err(message)
                    if attempt < POLAR_AUTOPILOT_SERVER_ATTEMPTS
                        && is_retryable_polar_start_error(&message) =>
                {
                    last_error = Some(message);
                    cleanup_failed_autopilot_server(&profile).await;
                    let retry_server_name = fresh_autopilot_server_name();
                    polar_server_name.set(retry_server_name.clone());
                    profile.network_name = retry_server_name.clone();
                    profile.polar_automation.network_id = retry_server_name;
                    profile.connection_status = ConnectionStatus::SavedOffline;
                    profile.polar_block_height_confirmed = false;
                    profile.game_treasury_ready = false;
                    profile.game_treasury_funded_sats = 0;
                    setup_profile.set(profile.clone());
                    lab_state.set(None);
                    update_autopilot_operation_prompt(
                        operation_prompt,
                        operation_id,
                        PolarWizardStep::GameTreasury,
                        format!(
                            "Retry {}/{}: Polar start allocation failed. Trying fresh server {}...",
                            attempt + 1,
                            POLAR_AUTOPILOT_SERVER_ATTEMPTS,
                            profile.network_name
                        ),
                        ToastTone::Info,
                        true,
                        false,
                    )
                    .await;
                    autopilot_status.set(format!(
                        "Step 5 retry {}/{}: Game Treasury (Sats)",
                        attempt + 1,
                        POLAR_AUTOPILOT_SERVER_ATTEMPTS
                    ));

                    match ensure_polar_server_for_autopilot(profile.clone()).await {
                        Ok(result) => {
                            profile.polar_automation = result.profile;
                            profile.connection_status = ConnectionStatus::SavedOffline;
                            profile.polar_block_height_confirmed = false;
                            profile.game_treasury_ready = false;
                            profile.game_treasury_funded_sats = 0;
                            profile.last_verified_at = None;
                            setup_profile.set(profile.clone());
                            lab_state.set(None);
                        }
                        Err(message)
                            if attempt < POLAR_AUTOPILOT_SERVER_ATTEMPTS
                                && is_retryable_polar_start_error(&message) =>
                        {
                            last_error = Some(message);
                            cleanup_failed_autopilot_server(&profile).await;
                            continue;
                        }
                        Err(message) if is_retryable_polar_start_error(&message) => {
                            cleanup_failed_autopilot_server(&profile).await;
                            return Err(format!(
                                "Polar server start failed after {POLAR_AUTOPILOT_SERVER_ATTEMPTS} fresh server attempts: {message}"
                            ));
                        }
                        Err(message) => return Err(message),
                    }
                }
                Err(message) if is_retryable_polar_start_error(&message) => {
                    cleanup_failed_autopilot_server(&profile).await;
                    return Err(format!(
                        "Game Treasury setup failed after {POLAR_AUTOPILOT_SERVER_ATTEMPTS} fresh server attempts: {message}"
                    ));
                }
                Err(message) => return Err(message),
            }
        }
        let state = state.ok_or_else(|| {
            last_error.unwrap_or_else(|| "Game Treasury could not be prepared.".to_string())
        })?;
        profile = state.profile.clone();
        setup_profile.set(profile.clone());
        lab_state.set(Some(state));
    }

    if polar_wizard_step(&profile, lab_state().as_ref()) == PolarWizardStep::GameTreasuryTras {
        update_autopilot_operation_prompt(
            operation_prompt,
            operation_id,
            PolarWizardStep::GameTreasuryTras,
            "Checking Game Treasury TRA inventory...",
            ToastTone::Info,
            true,
            false,
        )
        .await;
        autopilot_status.set("Step 6 of 10: Game Treasury (TRAs)".to_string());
        let state = prepare_game_treasury_tras(profile.clone()).await?;
        profile = state.profile.clone();
        setup_profile.set(profile.clone());
        lab_state.set(Some(state));
    }

    if polar_wizard_step(&profile, lab_state().as_ref()) == PolarWizardStep::UserNodesSats {
        update_autopilot_operation_prompt(
            operation_prompt,
            operation_id,
            PolarWizardStep::UserNodesSats,
            "Checking user node sats...",
            ToastTone::Info,
            true,
            false,
        )
        .await;
        autopilot_status.set("Step 7 of 10: User Nodes (Sats)".to_string());
        let state = prepare_user_node_sats(profile.clone()).await?;
        profile = state.profile.clone();
        setup_profile.set(profile.clone());
        lab_state.set(Some(state));
    }

    if polar_wizard_step(&profile, lab_state().as_ref()) == PolarWizardStep::UserNodesTras {
        update_autopilot_operation_prompt(
            operation_prompt,
            operation_id,
            PolarWizardStep::UserNodesTras,
            "Rebalancing user node TRAs...",
            ToastTone::Info,
            true,
            false,
        )
        .await;
        autopilot_status.set("Step 8 of 10: User Nodes (TRAs)".to_string());
        let state = prepare_user_node_tras(profile.clone()).await?;
        profile = state.profile.clone();
        setup_profile.set(profile.clone());
        lab_state.set(Some(state));
    }

    if polar_wizard_step(&profile, lab_state().as_ref()) == PolarWizardStep::BlockHeight {
        update_autopilot_operation_prompt(
            operation_prompt,
            operation_id,
            PolarWizardStep::BlockHeight,
            "Saving the app block-height baseline...",
            ToastTone::Info,
            true,
            false,
        )
        .await;
        autopilot_status.set("Step 9 of 10: Block Height".to_string());
        let fallback_height = lab_state()
            .map(|state| state.block_height.to_string())
            .unwrap_or_else(|| "0".to_string());
        let block_height = block_height_from_input(polar_block_height()).or_else(|_| {
            polar_block_height.set(fallback_height.clone());
            block_height_from_input(fallback_height)
        })?;
        let state = confirm_polar_block_height(profile.clone(), block_height).await?;
        polar_block_height.set(state.block_height.to_string());
        profile = state.profile.clone();
        setup_profile.set(profile.clone());
        lab_state.set(Some(state));
    }

    if polar_wizard_step(&profile, lab_state().as_ref()) == PolarWizardStep::Complete {
        update_autopilot_operation_prompt(
            operation_prompt,
            operation_id,
            PolarWizardStep::Complete,
            "Unlocking routes...",
            ToastTone::Info,
            true,
            false,
        )
        .await;
        autopilot_status.set("Step 10 of 10: Unlock Routes".to_string());
        let state = complete_polar_setup(profile.clone()).await?;
        profile = state.profile.clone();
        setup_profile.set(profile.clone());
        lab_state.set(Some(state));
    }

    update_autopilot_operation_prompt(
        operation_prompt,
        operation_id,
        PolarWizardStep::Complete,
        "Testing completed setup again...",
        ToastTone::Info,
        true,
        false,
    )
    .await;
    autopilot_status.set("Testing completed setup again...".to_string());
    let verified_state = get_lab_state(profile).await?;
    if !verified_state.profile.is_connected() {
        return Err("Autopilot verification did not finish with a connected setup.".to_string());
    }

    Ok(verified_state)
}

fn fresh_autopilot_server_name() -> String {
    let sequence = POLAR_AUTOPILOT_NAME_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let timestamp = chrono::Utc::now().timestamp_millis();
    if sequence == 0 {
        autopilot_server_name_from_timestamp(timestamp)
    } else {
        format!(
            "{}-{sequence}",
            autopilot_server_name_from_timestamp(timestamp)
        )
    }
}

fn autopilot_server_name_from_timestamp(timestamp: i64) -> String {
    format!("autopilot-{timestamp}")
}

fn requested_autopilot_server_name(server_name: &str) -> String {
    let server_name = server_name.trim();
    if server_name.is_empty() {
        fresh_autopilot_server_name()
    } else {
        server_name.to_string()
    }
}

fn autopilot_profile_for_fresh_server_retry(
    mut profile: SetupProfile,
    retry_server_name: &str,
) -> SetupProfile {
    let retry_server_name = retry_server_name.trim().to_string();
    profile.network_name = retry_server_name.clone();
    profile.polar_automation.network_id = retry_server_name;
    profile.connection_status = ConnectionStatus::SavedOffline;
    profile.polar_block_height_confirmed = false;
    profile.game_treasury_ready = false;
    profile.game_treasury_funded_sats = 0;
    profile.last_verified_at = None;
    profile
}

async fn ensure_polar_server_for_autopilot(
    profile: SetupProfile,
) -> Result<PolarServerEnsureResult, String> {
    with_autopilot_timeout(
        ensure_polar_server(profile),
        "finding or creating the Polar server",
        POLAR_AUTOPILOT_SERVER_STEP_TIMEOUT_SECONDS,
    )
    .await
}

async fn with_autopilot_timeout<F, T>(
    future: F,
    label: &'static str,
    seconds: u32,
) -> Result<T, String>
where
    F: std::future::Future<Output = Result<T, String>>,
{
    match select(
        future.boxed_local(),
        autopilot_timeout_delay(seconds).boxed_local(),
    )
    .await
    {
        Either::Left((result, _)) => result,
        Either::Right((_, _)) => Err(autopilot_step_timeout_message(label, seconds)),
    }
}

#[cfg(target_arch = "wasm32")]
async fn autopilot_timeout_delay(seconds: u32) {
    gloo_timers::future::TimeoutFuture::new(seconds.saturating_mul(1_000)).await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn autopilot_timeout_delay(seconds: u32) {
    futures_timer::Delay::new(std::time::Duration::from_secs(seconds as u64)).await;
}

fn autopilot_step_timeout_message(label: &str, seconds: u32) -> String {
    format!(
        "Timed out after {seconds}s while {label}. Autopilot stopped instead of waiting forever."
    )
}

async fn cleanup_failed_autopilot_server(profile: &SetupProfile) {
    let network_name = profile.network_name.trim();
    let network_id = profile.polar_automation.network_id.trim();
    if network_name.starts_with("autopilot-") || network_id.starts_with("autopilot-") {
        let _ = delete_created_polar_server(profile.clone()).await;
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
        "Create Nodes",
        "Finding or creating required Polar nodes...",
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
            let create_result =
                create_required_polar_nodes_with_progress(profile, move |message| {
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
                                    "Node creation canceled. Jack, Bob, and Carol were removed.",
                                    ToastTone::Success,
                                    false,
                                    false,
                                )
                                .await;
                                close_operation_prompt(operation_prompt, operation_id);
                                push_toast(
                                    toast,
                                    toast_sequence,
                                    "Node creation canceled.",
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
                        push_toast(
                            toast,
                            toast_sequence,
                            "Create Nodes sent",
                            ToastTone::Success,
                        );
                        focus_step_control("polar-game-treasury-submit").await;
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
                            "Return to step 2 and connect to the Polar bridge before creating nodes.",
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

async fn rebalance_user_nodes_sats_step(
    mut is_busy: Signal<bool>,
    setup_profile: Signal<SetupProfile>,
    mut lab_state: Signal<Option<LabState>>,
    toast: Signal<Option<Toast>>,
    toast_sequence: Signal<u64>,
) {
    is_busy.set(true);
    match prepare_user_node_sats(setup_profile()).await {
        Ok(state) => {
            lab_state.set(Some(state));
            push_toast(
                toast,
                toast_sequence,
                "User node sats ready.",
                ToastTone::Success,
            );
            focus_step_control("polar-tra-assets-submit").await;
        }
        Err(message) => {
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
        "Returned to step 9. Block Height must be submitted before routes unlock again.",
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
        "Creating GAME_TAPROOT and treasury-owned TRA inventory items...",
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
            focus_step_control("polar-user-nodes-sats-submit").await;
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
        "User Nodes (TRAs)",
        "Rebalancing user node TRAs from Game Treasury...",
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

    let result = prepare_user_node_tras(setup_profile()).await;

    match result {
        Ok(state) => {
            setup_profile.set(state.profile.clone());
            lab_state.set(Some(state));
            close_operation_prompt(operation_prompt, operation_id);
            push_toast(
                toast,
                toast_sequence,
                "User node TRAs ready.",
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
        subtitle: None,
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

async fn update_operation_prompt_with_subtitle(
    mut prompt: Signal<Option<OperationPrompt>>,
    operation_id: u64,
    subtitle: Option<String>,
    message: impl Into<String>,
    tone: ToastTone,
    is_pending: bool,
    can_cancel: bool,
) {
    let active_prompt = { prompt.peek().as_ref().cloned() };
    if let Some(mut active_prompt) = active_prompt {
        if active_prompt.operation_id == operation_id {
            active_prompt.subtitle = subtitle;
            active_prompt.message = message.into();
            active_prompt.tone = tone;
            active_prompt.is_pending = is_pending;
            active_prompt.can_cancel = can_cancel;
            prompt.set(Some(active_prompt));
            wait_for_prompt_message_minimum().await;
        }
    }
}

async fn update_autopilot_operation_prompt(
    prompt: Signal<Option<OperationPrompt>>,
    operation_id: u64,
    step: PolarWizardStep,
    message: impl Into<String>,
    tone: ToastTone,
    is_pending: bool,
    can_cancel: bool,
) {
    update_operation_prompt_with_subtitle(
        prompt,
        operation_id,
        Some(autopilot_step_subtitle(step)),
        message,
        tone,
        is_pending,
        can_cancel,
    )
    .await;
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

fn update_operation_prompt_now_with_subtitle(
    mut prompt: Signal<Option<OperationPrompt>>,
    operation_id: u64,
    subtitle: Option<String>,
    message: impl Into<String>,
    tone: ToastTone,
    is_pending: bool,
    can_cancel: bool,
) {
    let active_prompt = { prompt.peek().as_ref().cloned() };
    if let Some(mut active_prompt) = active_prompt {
        if active_prompt.operation_id == operation_id {
            active_prompt.subtitle = subtitle;
            active_prompt.message = message.into();
            active_prompt.tone = tone;
            active_prompt.is_pending = is_pending;
            active_prompt.can_cancel = can_cancel;
            prompt.set(Some(active_prompt));
        }
    }
}

fn update_autopilot_operation_prompt_now(
    prompt: Signal<Option<OperationPrompt>>,
    operation_id: u64,
    step: PolarWizardStep,
    message: impl Into<String>,
    tone: ToastTone,
    is_pending: bool,
    can_cancel: bool,
) {
    update_operation_prompt_now_with_subtitle(
        prompt,
        operation_id,
        Some(autopilot_step_subtitle(step)),
        message,
        tone,
        is_pending,
        can_cancel,
    );
}

fn autopilot_step_subtitle(step: PolarWizardStep) -> String {
    format!("{}. {}", step.order(), polar_wizard_step_label(step))
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
