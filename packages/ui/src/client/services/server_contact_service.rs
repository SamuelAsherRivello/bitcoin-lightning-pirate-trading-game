#[cfg(target_arch = "wasm32")]
pub async fn contact_server() -> Result<(), String> {
    let window = web_sys::window().ok_or_else(|| "Browser window is unavailable.".to_string())?;

    wasm_bindgen_futures::JsFuture::from(window.fetch_with_str("/"))
        .await
        .map(|_| ())
        .map_err(|_| "Server cannot be contacted".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn contact_server() -> Result<(), String> {
    Err("Server cannot be contacted".to_string())
}
