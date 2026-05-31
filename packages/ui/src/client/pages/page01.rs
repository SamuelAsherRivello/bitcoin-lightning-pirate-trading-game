use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::client::components::toast::{Toast, ToastTone};
use crate::client::models::{TemplateDataLoadRequest, TemplateDataLoadResult, TemplateDataSource};
use crate::client::services::template_data_service::{load_template_data, refresh_template_data};

#[component]
pub fn Page01() -> Element {
    let data_load_request = use_context::<Signal<TemplateDataLoadRequest>>();
    let mut data_load_cache =
        use_context::<Signal<Option<Result<TemplateDataLoadResult, String>>>>();
    let initial_request_sequence = use_signal(|| data_load_request().sequence);
    let mut toast = use_context::<Signal<Option<Toast>>>();
    let mut toast_sequence = use_signal(|| 0_u64);
    let mut last_toast_key = use_signal(|| None::<String>);
    let data = use_resource(move || async move {
        let request = data_load_request();

        if request.sequence > initial_request_sequence() {
            refresh_template_data().await
        } else if let Some(cached) = data_load_cache.peek().clone() {
            cached
        } else {
            load_template_data().await
        }
    });

    use_effect(move || {
        if let Some(result) = data() {
            data_load_cache.set(Some(result));
        }
    });

    use_effect(move || {
        let request = data_load_request();
        let toast_details = match data() {
            Some(Ok(result)) => (
                format!("success:{}:{:?}", request.sequence, result.source),
                format!(
                    "{} {}",
                    template_data_source_label(&result.source),
                    t!("toast-action-loaded")
                ),
                ToastTone::Success,
            ),
            Some(Err(message)) => (
                format!("error:{}:{message}", request.sequence),
                format!("{} {}", t!("source-database"), t!("toast-action-error")),
                ToastTone::Error,
            ),
            None => (
                format!("loading:{}", request.sequence),
                format!("{} {}", t!("source-database"), t!("toast-action-loading")),
                ToastTone::Info,
            ),
        };

        if last_toast_key.peek().as_ref() == Some(&toast_details.0) {
            return;
        }

        let next_id = *toast_sequence.peek() + 1;
        toast_sequence.set(next_id);
        last_toast_key.set(Some(toast_details.0));
        toast.set(Some(Toast {
            id: next_id,
            message: toast_details.1,
            tone: toast_details.2,
        }));
    });

    let data_text = match data() {
        Some(Ok(result)) => format!("DB data is {}", result.data.message),
        Some(Err(message)) => message,
        None => t!("loading-template-data"),
    };

    rsx! {
        main { class: "page-content template-page",
            h1 { {t!("page-01-title")} }
            p { class: "template-page__data", "{data_text}" }
            div { class: "template-page__body",
                p { {t!("page-01-body-01")} }
                p { {t!("page-01-body-02")} }
                p {
                    {t!("page-01-body-03-prefix")}
                    " "
                    span { class: "template-page__warning", {t!("page-01-body-03-warning")} }
                }
            }
            a {
                class: "template-page__readme-link",
                href: "https://github.com/SamuelAsherRivello/bitcoin-lightning-pirate-trading-game#readme",
                target: "_blank",
                rel: "noopener noreferrer",
                {t!("readme-details-link")}
            }
        }
    }
}

fn template_data_source_label(source: &TemplateDataSource) -> String {
    match source {
        TemplateDataSource::BrowserSnapshot | TemplateDataSource::Database => t!("source-database"),
    }
}
