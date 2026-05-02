use dioxus::prelude::*;
use dioxus_i18n::prelude::*;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(AppLayout)]
    #[route("/")]
    Page01 {},

    #[route("/page-02")]
    Page02 {},

    #[route("/page-03")]
    Page03 {},
}

#[component]
fn AppLayout() -> Element {
    let theme = use_signal(services::storage_service::load_theme);
    let language = use_signal(services::storage_service::load_language);
    let initial_language = language();
    use_init_i18n(|| services::localization_service::config(initial_language));

    let data_load_request = use_signal(models::TemplateDataLoadRequest::initial);
    let data_load_cache = use_signal(|| None::<Result<models::TemplateDataLoadResult, String>>);
    let toast = use_signal(|| None::<components::toast::Toast>);

    use_context_provider(|| theme);
    use_context_provider(|| language);
    use_context_provider(|| data_load_request);
    use_context_provider(|| data_load_cache);
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
            Page { route: Route::Page01 {}, will_preload: true,
                Page01 {}
            }
            Page { route: Route::Page02 {}, will_preload: true,
                Page02 {}
            }
            Page { route: Route::Page03 {}, will_preload: true,
                Page03 {}
            }
        }
    }
}

mod app;
pub use app::App;

pub mod pages {
    pub mod page01;
    pub mod page02;
    pub mod page03;
    pub mod template_page;
}
pub use pages::page01::Page01;
pub use pages::page02::Page02;
pub use pages::page03::Page03;

pub mod components {
    pub mod app_error;
    pub mod developer_tools;
    pub mod page;
    pub mod page_footer;
    pub mod page_header;
    pub mod toast;
}
pub use components::app_error::AppErrorFallback;
pub use components::developer_tools::DeveloperTools;
pub use components::page::Page;
pub use components::page_footer::PageFooter;
pub use components::page_header::PageHeader;

pub mod models;
pub use models::{
    TemplateData, TemplateDataLoadRequest, TemplateDataLoadResult, TemplateDataSource,
};

pub mod services;
pub use services::localization_service::AppLanguage;
pub use services::storage_service::Theme;
