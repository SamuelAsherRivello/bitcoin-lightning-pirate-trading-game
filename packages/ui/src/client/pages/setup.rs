use dioxus::prelude::*;
use dioxus_i18n::t;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

use crate::client::components::setup::WarningCallout;
use crate::client::components::toast::{
    wait_for_prompt_message_minimum, OperationPrompt, Toast, ToastTone,
};
use crate::client::models::{
    ConnectionStatus, LabState, PolarAutomationProfile, SetupMode, SetupProfile,
    DEFAULT_BITCOIN_BACKEND_NAME,
};
use crate::client::services::lightning_server_functions::{
    complete_polar_setup, create_polar_demo_nodes_with_progress, destroy_polar_demo_nodes,
    ensure_polar_server, reset_lab, test_setup, verify_polar_bridge, PolarServerEnsureStatus,
};
use crate::client::services::storage_service;

const DOCKER_DESKTOP_URL: &str = "https://www.docker.com/products/docker-desktop/";
const POLAR_DOWNLOAD_URL: &str = "https://lightningpolar.com/";

#[derive(Clone, Copy, Eq, PartialEq)]
enum PolarWizardStep {
    BridgeUrl,
    ServerName,
    DemoNodes,
    Complete,
    Done,
}

impl PolarWizardStep {
    fn order(self) -> u8 {
        match self {
            Self::BridgeUrl => 1,
            Self::ServerName => 2,
            Self::DemoNodes => 3,
            Self::Complete | Self::Done => 4,
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
    let mut polar_bridge_url = use_signal(|| setup_profile().polar_automation.bridge_url.clone());
    let mut polar_server_name = use_signal(|| setup_profile().network_name.clone());
    let mut is_busy = use_signal(|| false);
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
                                    title: "Polar regtest values only".to_string(),
                                    body: "Use only a local Polar regtest network. Browser calls to Polar's bridge must be opened from localhost so the bridge CORS check accepts them.".to_string(),
                                }

                            section { class: "polar-setup-section",
                                div { class: "section-heading",
                                    h3 { "OS" }
                                }
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
                                            info: "Start Polar".to_string(),
                                            name: rsx! { "Run Polar" },
                                        }
                                    }
                                }

                            section { class: "app-setup-section",
                                div { class: "section-heading",
                                    h3 { "Polar" }
                                }
                                    InstructionList {
                                        Instruction {
                                            id: "polar-step-bridge-url".to_string(),
                                            class: wizard_step_class(active_step, PolarWizardStep::BridgeUrl).to_string(),
                                            number: 1,
                                            info: "Default localhost bridge while Polar is open".to_string(),
                                            name: rsx! { "Bridge URL" },
                                            value: Some(rsx! {
                                                label { class: "setup-field-row",
                                                    input {
                                                        id: "polar-bridge-url-input",
                                                        r#type: "text",
                                                        placeholder: "http://localhost:37373",
                                                        value: polar_bridge_url(),
                                                        disabled: active_step != PolarWizardStep::BridgeUrl,
                                                        oninput: move |event| polar_bridge_url.set(event.value()),
                                                    }
                                                }
                                            }),
                                            value_after: Some(rsx! {
                                                if !bridge_url_is_valid && active_step == PolarWizardStep::BridgeUrl {
                                                    p { class: "field-error", "Use a local bridge URL such as http://localhost:37373." }
                                                }
                                                if bridge_url_is_valid && !browser_origin_is_valid && active_step == PolarWizardStep::BridgeUrl {
                                                    p { class: "field-error", "Open this app at http://localhost:8080 before connecting to Polar." }
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
                                                        push_toast(toast, toast_sequence, "Checking Polar bridge...", ToastTone::Info);

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
                                                                profile.connection_status = ConnectionStatus::SavedOffline;
                                                                profile.last_verified_at = None;
                                                                profile.polar_automation.network_id.clear();
                                                                match verify_polar_bridge(profile.clone()).await {
                                                                    Ok(saved_profile) => {
                                                                        setup_profile.set(saved_profile);
                                                                        lab_state.set(None);
                                                                        push_toast(toast, toast_sequence, "Connected to Polar bridge.", ToastTone::Success);
                                                                    }
                                                                    Err(message) => push_toast(toast, toast_sequence, message, ToastTone::Error),
                                                                }
                                                            }
                                                            Err(message) => push_toast(toast, toast_sequence, message, ToastTone::Error),
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
                                            name: rsx! { "Server name" },
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
                                                        push_toast(toast, toast_sequence, "Checking Polar server...", ToastTone::Info);

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
                                                            Ok(profile) => match ensure_polar_server(profile.clone()).await {
                                                                Ok(result) => {
                                                                    let mut saved_profile = profile;
                                                                    saved_profile.polar_automation = result.profile;
                                                                    saved_profile.connection_status = ConnectionStatus::SavedOffline;
                                                                    saved_profile.last_verified_at = None;
                                                                    setup_profile.set(saved_profile);
                                                                    lab_state.set(None);

                                                                    let message = match result.status {
                                                                        PolarServerEnsureStatus::Existed => "Polar server already exists.",
                                                                        PolarServerEnsureStatus::Created => "Polar server created.",
                                                                    };
                                                                    push_toast(toast, toast_sequence, message, ToastTone::Success);
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
                                            id: "polar-step-demo-nodes".to_string(),
                                            class: wizard_step_class(active_step, PolarWizardStep::DemoNodes).to_string(),
                                            number: 3,
                                            info: "App creates these LND nodes".to_string(),
                                            name: rsx! { "Demo nodes" },
                                            value: Some(rsx! {
                                                label { class: "setup-field-row",
                                                    input {
                                                        id: "polar-demo-nodes-input",
                                                        r#type: "text",
                                                        value: "Alice, Bob, Carol",
                                                        readonly: true,
                                                    }
                                                }
                                            }),
                                            actions: Some(rsx! {
                                                button {
                                                    id: "polar-demo-nodes-submit",
                                                    class: "primary-action",
                                                    r#type: "button",
                                                    disabled: is_busy() || active_step != PolarWizardStep::DemoNodes,
                                                    onclick: move |_| async move {
                                                        create_demo_nodes_step(
                                                            is_busy,
                                                            setup_profile,
                                                            lab_state,
                                                            operation_prompt,
                                                            prompt_sequence,
                                                            amount_text(),
                                                            polar_server_name(),
                                                            polar_bridge_url(),
                                                        ).await;
                                                    },
                                                    "SUBMIT"
                                                }
                                                button {
                                                    id: "polar-demo-nodes-reset",
                                                    class: "secondary-action danger-action",
                                                    r#type: "button",
                                                    disabled: is_busy() || active_step != PolarWizardStep::DemoNodes,
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
                                                                let saved_profile = reset_to_server_name_step(profile);
                                                                setup_profile.set(saved_profile.clone());
                                                                lab_state.set(None);
                                                                polar_bridge_url.set(saved_profile.polar_automation.bridge_url.clone());
                                                                polar_server_name.set(saved_profile.network_name.clone());
                                                                push_toast(toast, toast_sequence, "Returned to step 2.", ToastTone::Success);
                                                                focus_step_control("polar-server-name-input").await;
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
                                            id: "polar-step-complete".to_string(),
                                            class: wizard_step_class(active_step, PolarWizardStep::Complete).to_string(),
                                            number: 4,
                                            info: "Saves setup as connected".to_string(),
                                            name: rsx! { "Unlock routes" },
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
                                                        push_toast(toast, toast_sequence, "Unlock routes sending...", ToastTone::Info);

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
                                                            Ok(profile) => match complete_polar_setup(profile).await {
                                                                Ok(state) => {
                                                                    setup_profile.set(state.profile.clone());
                                                                    lab_state.set(Some(state));
                                                                    push_toast(toast, toast_sequence, "Unlock routes sent", ToastTone::Success);
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
            if show_complete_reset_confirm() {
                CompleteResetConfirmationPrompt {
                    is_busy,
                    show_prompt: show_complete_reset_confirm,
                    setup_profile,
                    lab_state,
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
    for _ in 0..4 {
        gloo_timers::future::TimeoutFuture::new(0).await;

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

#[component]
fn FieldHelpIcon(label: String) -> Element {
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

    if profile.connection_status == ConnectionStatus::PartiallyConnected
        || lab_state_has_status(lab_state, ConnectionStatus::PartiallyConnected)
    {
        return PolarWizardStep::Complete;
    }

    if profile.connection_status == ConnectionStatus::SavedOffline
        && !profile.polar_automation.network_id.trim().is_empty()
    {
        return PolarWizardStep::DemoNodes;
    }

    if profile.connection_status == ConnectionStatus::SavedOffline {
        return PolarWizardStep::ServerName;
    }

    PolarWizardStep::BridgeUrl
}

fn lab_state_has_status(lab_state: Option<&LabState>, status: ConnectionStatus) -> bool {
    match lab_state {
        Some(state) => state.profile.connection_status == status,
        None => false,
    }
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
        .map(|hostname| hostname.eq_ignore_ascii_case("localhost"))
        .unwrap_or(false)
}

#[cfg(not(target_arch = "wasm32"))]
fn browser_origin_allows_polar_bridge() -> bool {
    true
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
    profile.last_verified_at = None;
    profile.polar_automation.network_id.clear();
    profile.polar_automation.bitcoin_backend_name = DEFAULT_BITCOIN_BACKEND_NAME.to_string();
    storage_service::save_setup_profile(&profile);
    storage_service::clear_lab_state_snapshot();
    profile
}

fn reset_to_server_name_step(mut profile: SetupProfile) -> SetupProfile {
    profile.connection_status = ConnectionStatus::SavedOffline;
    profile.last_verified_at = None;
    profile.polar_automation.network_id.clear();
    storage_service::save_setup_profile(&profile);
    storage_service::clear_lab_state_snapshot();
    profile
}

fn reset_to_demo_nodes_step(mut profile: SetupProfile) -> SetupProfile {
    profile.connection_status = ConnectionStatus::SavedOffline;
    profile.last_verified_at = None;

    storage_service::save_setup_profile(&profile);
    storage_service::clear_lab_state_snapshot();

    profile
}

async fn create_demo_nodes_step(
    mut is_busy: Signal<bool>,
    mut setup_profile: Signal<SetupProfile>,
    mut lab_state: Signal<Option<LabState>>,
    operation_prompt: Signal<Option<OperationPrompt>>,
    prompt_sequence: Signal<u64>,
    amount_text: String,
    polar_server_name: String,
    polar_bridge_url: String,
) {
    is_busy.set(true);
    let previous_profile = setup_profile();
    let previous_lab_state = lab_state();
    let operation_id = begin_operation_prompt(
        operation_prompt,
        prompt_sequence,
        "Create demo nodes",
        "Starting demo-node creation...",
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
                                    "Demo node creation canceled. Alice, Bob, and Carol were removed.",
                                    ToastTone::Success,
                                    false,
                                    false,
                                )
                                .await;
                            }
                            Err(message) => {
                                setup_profile.set(state.profile.clone());
                                lab_state.set(Some(state));
                                update_operation_prompt(
                                    operation_prompt,
                                    operation_id,
                                    format!(
                                        "Cancel could not remove the created demo nodes: {message}"
                                    ),
                                    ToastTone::Error,
                                    false,
                                    false,
                                )
                                .await;
                            }
                        }
                    } else {
                        setup_profile.set(state.profile.clone());
                        lab_state.set(Some(state));
                        update_operation_prompt(
                            operation_prompt,
                            operation_id,
                            "Server running. Alice, Bob, and Carol are running and funded.",
                            ToastTone::Success,
                            false,
                            false,
                        )
                        .await;
                    }
                }
                Err(message) => {
                    update_operation_prompt(
                        operation_prompt,
                        operation_id,
                        message,
                        ToastTone::Error,
                        false,
                        false,
                    )
                    .await;
                }
            }
        }
        Err(message) => {
            update_operation_prompt(
                operation_prompt,
                operation_id,
                message,
                ToastTone::Error,
                false,
                false,
            )
            .await;
        }
    }

    is_busy.set(false);
}

async fn reset_complete_step(
    mut is_busy: Signal<bool>,
    mut setup_profile: Signal<SetupProfile>,
    mut lab_state: Signal<Option<LabState>>,
    toast: Signal<Option<Toast>>,
    toast_sequence: Signal<u64>,
) {
    is_busy.set(true);
    let saved_profile = reset_to_demo_nodes_step(setup_profile());
    setup_profile.set(saved_profile);
    lab_state.set(None);
    push_toast(
        toast,
        toast_sequence,
        "Returned to step 3.",
        ToastTone::Success,
    );
    focus_step_control("polar-demo-nodes-submit").await;

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

fn prompt_cancel_requested(prompt: Signal<Option<OperationPrompt>>, operation_id: u64) -> bool {
    prompt
        .peek()
        .as_ref()
        .map(|active_prompt| {
            active_prompt.operation_id == operation_id && active_prompt.cancel_requested
        })
        .unwrap_or(false)
}
