use dioxus::prelude::*;

const APP_ERROR_CSS: Asset = asset!("/assets/styling/app_error.css");

#[component]
pub fn AppErrorFallback(error_context: ErrorContext) -> Element {
    let error_message = error_context
        .error()
        .map(|error| error.to_string())
        .unwrap_or_else(|| "The app encountered an unexpected rendering error.".to_string());

    rsx! {
        document::Link { rel: "stylesheet", href: APP_ERROR_CSS }

        main { class: "app-error",
            section { class: "app-error__panel",
                h1 { "Something went wrong" }
                p { "{error_message}" }
                button {
                    class: "app-error__button",
                    r#type: "button",
                    onclick: move |_| {
                        error_context.clear_errors();
                    },
                    "Try again"
                }
            }
        }
    }
}
