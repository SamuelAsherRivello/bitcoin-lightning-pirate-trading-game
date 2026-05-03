use dioxus::prelude::*;
use dioxus_i18n::prelude::*;

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

    let setup_profile = use_signal(services::storage_service::load_setup_profile);
    let lab_state = use_signal(services::storage_service::load_lab_state_snapshot);
    let toast = use_signal(|| None::<components::toast::Toast>);

    use_context_provider(|| theme);
    use_context_provider(|| language);
    use_context_provider(|| setup_profile);
    use_context_provider(|| lab_state);
    use_context_provider(|| toast);

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
