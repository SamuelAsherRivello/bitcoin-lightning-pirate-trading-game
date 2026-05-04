use dioxus::prelude::*;
use dioxus_i18n::prelude::*;

const LAB_POLL_INTERVAL_MS: u32 = 1_000;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(AppLayout)]
    #[route("/")]
    Home {},

    #[route("/setup")]
    SetUp {},

    #[route("/play")]
    PlayGame {},

    #[route("/debug")]
    DebugNetwork {},
}

#[component]
fn AppLayout() -> Element {
    let theme = use_signal(services::storage_service::load_theme);
    let language = use_signal(services::storage_service::load_language);
    let initial_language = language();
    use_init_i18n(|| services::localization_service::config(initial_language));

    let mut setup_profile = use_signal(services::storage_service::load_setup_profile);
    let mut lab_state = use_signal(services::storage_service::load_lab_state_snapshot);
    let toast = use_signal(|| None::<components::toast::Toast>);
    let mut operation_prompt = use_signal(|| None::<components::toast::OperationPrompt>);
    let mut operation_prompt_sequence = use_signal(|| 70_000_u64);
    let mut lab_poll_tick = use_signal(|| 0_u64);
    let navigator = navigator();
    let active_route = use_route::<Route>();

    use_context_provider(|| theme);
    use_context_provider(|| language);
    use_context_provider(|| setup_profile);
    use_context_provider(|| lab_state);
    use_context_provider(|| toast);
    use_context_provider(|| operation_prompt);

    use_effect(move || {
        let profile = setup_profile();
        let route = active_route.clone();
        let _tick = lab_poll_tick();
        if profile.is_connected() {
            spawn(async move {
                match services::lightning_server_functions::get_lab_state_or_recover(profile).await
                {
                    Ok(state) => {
                        let current_profile = setup_profile.peek().clone();
                        if current_profile != state.profile {
                            setup_profile.set(state.profile.clone());
                        }

                        if lab_state.peek().as_ref() != Some(&state) {
                            lab_state.set(Some(state));
                        }
                    }
                    Err(recovery) => {
                        let next_id = *operation_prompt_sequence.peek() + 1;
                        operation_prompt_sequence.set(next_id);
                        setup_profile.set(recovery.profile);
                        lab_state.set(recovery.lab_state);
                        operation_prompt.set(Some(components::toast::OperationPrompt {
                            operation_id: next_id,
                            title: "Polar setup needs attention".to_string(),
                            message: recovery.message,
                            tone: components::toast::ToastTone::Error,
                            is_pending: false,
                            can_cancel: false,
                            cancel_requested: false,
                        }));
                        if route != (Route::Home {}) {
                            let _ = navigator.replace(Route::Home {});
                        }
                    }
                }

                wait_for_lab_poll_interval().await;
                if setup_profile.peek().is_connected() {
                    lab_poll_tick.with_mut(|tick| *tick = tick.wrapping_add(1));
                }
            });
        }
    });

    let shell_class = format!("app-shell {}", theme().class_name());

    rsx! {
        div { class: "{shell_class}",
            ErrorBoundary {
                handle_error: |error_context: ErrorContext| rsx! {
                    AppErrorFallback { error_context }
                },
                PageHeader {}
                PageStack {}
                PageFooter {}
            }
        }
    }
}

#[component]
fn PageStack() -> Element {
    rsx! {
        div {
            class: "page-stack",
            style: "position: relative; isolation: isolate;",
            Page { route: Route::Home {}, will_preload: true,
                Home {}
            }
            Page { route: Route::SetUp {}, will_preload: true,
                SetUp {}
            }
            Page { route: Route::PlayGame {}, will_preload: true,
                PlayGame {}
            }
            Page { route: Route::DebugNetwork {}, will_preload: true,
                DebugNetwork {}
            }
        }
    }
}

mod app;
pub use app::App;

pub mod pages {
    pub mod debug_network;
    pub mod home;
    pub mod play_game;
    pub mod setup;
    pub mod template_page;
}
pub use pages::debug_network::DebugNetwork;
pub use pages::home::Home;
pub use pages::play_game::PlayGame;
pub use pages::setup::SetUp;

pub mod components {
    pub mod app_error;
    pub mod developer_tools;
    pub mod game;
    pub mod network;
    pub mod page;
    pub mod page_footer;
    pub mod page_header;
    pub mod setup;
    pub mod toast;
}
pub use components::app_error::AppErrorFallback;
pub use components::developer_tools::DeveloperTools;
pub use components::page::Page;

#[cfg(target_arch = "wasm32")]
async fn wait_for_lab_poll_interval() {
    gloo_timers::future::TimeoutFuture::new(LAB_POLL_INTERVAL_MS).await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn wait_for_lab_poll_interval() {
    futures_timer::Delay::new(std::time::Duration::from_millis(
        LAB_POLL_INTERVAL_MS.into(),
    ))
    .await;
}
pub use components::page_footer::PageFooter;
pub use components::page_header::PageHeader;

pub mod models;
pub use models::{
    BlockWaitAction, BlockWaitReason, ConnectionStatus, DemoNode, DemoNodeId, InvoiceRequest,
    LabState, OperationFaqRow, PaymentAttempt, PolarAutomationProfile, PolarConnectionProfile,
    PolarNodeConnection, RouteStatus, SetupMode, SetupProfile, TemplateData,
    TemplateDataLoadRequest, TemplateDataLoadResult, TemplateDataSource, TradeRoute,
};

pub mod services;
pub use services::localization_service::AppLanguage;
pub use services::storage_service::Theme;
