use serde::{Deserialize, Serialize};

use crate::client::services::localization_service::AppLanguage;

#[cfg(target_arch = "wasm32")]
use crate::client::models::TemplateDataLoadResult;

#[cfg(target_arch = "wasm32")]
const TEMPLATE_DATA_SNAPSHOT_KEY: &str = "dioxus-bitcoin-lightning-game:data-snapshot";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    pub fn label(self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }

    pub fn class_name(self) -> &'static str {
        match self {
            Self::Light => "app-shell--light",
            Self::Dark => "app-shell--dark",
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::Dark
    }
}

pub fn load_theme() -> Theme {
    platform::load_theme().unwrap_or_default()
}

pub fn save_theme(theme: Theme) {
    platform::save_theme(theme);
}

pub fn load_language() -> AppLanguage {
    platform::load_language().unwrap_or_default()
}

pub fn save_language(language: AppLanguage) {
    platform::save_language(language);
}

#[cfg(target_arch = "wasm32")]
pub fn load_template_data_snapshot() -> Option<TemplateDataLoadResult> {
    platform::load_template_data_snapshot()
}

#[cfg(target_arch = "wasm32")]
pub fn save_template_data_snapshot(result: &TemplateDataLoadResult) {
    platform::save_template_data_snapshot(result);
}

#[cfg(target_arch = "wasm32")]
mod platform {
    use crate::client::models::{TemplateDataLoadResult, TemplateDataSource};

    use super::{AppLanguage, Theme, TEMPLATE_DATA_SNAPSHOT_KEY};

    const THEME_STORAGE_KEY: &str = "dioxus-bitcoin-lightning-game:theme";
    const LANGUAGE_STORAGE_KEY: &str = "dioxus-bitcoin-lightning-game:language";

    pub fn load_theme() -> Option<Theme> {
        let value = local_storage()?
            .get_item(THEME_STORAGE_KEY)
            .ok()
            .flatten()?;
        serde_json::from_str(&value).ok()
    }

    pub fn save_theme(theme: Theme) {
        let Some(storage) = local_storage() else {
            return;
        };
        let Ok(value) = serde_json::to_string(&theme) else {
            return;
        };

        let _ = storage.set_item(THEME_STORAGE_KEY, &value);
    }

    pub fn load_language() -> Option<AppLanguage> {
        let value = local_storage()?
            .get_item(LANGUAGE_STORAGE_KEY)
            .ok()
            .flatten()?;
        serde_json::from_str(&value).ok()
    }

    pub fn save_language(language: AppLanguage) {
        let Some(storage) = local_storage() else {
            return;
        };
        let Ok(value) = serde_json::to_string(&language) else {
            return;
        };

        let _ = storage.set_item(LANGUAGE_STORAGE_KEY, &value);
    }

    pub fn load_template_data_snapshot() -> Option<TemplateDataLoadResult> {
        let storage = local_storage()?;
        let value = storage
            .get_item(TEMPLATE_DATA_SNAPSHOT_KEY)
            .ok()
            .flatten()?;
        let mut result = serde_json::from_str::<TemplateDataLoadResult>(&value).ok()?;

        result.source = TemplateDataSource::BrowserSnapshot;
        Some(result)
    }

    pub fn save_template_data_snapshot(result: &TemplateDataLoadResult) {
        let Some(storage) = local_storage() else {
            return;
        };
        let Ok(value) = serde_json::to_string(result) else {
            return;
        };

        let _ = storage.set_item(TEMPLATE_DATA_SNAPSHOT_KEY, &value);
    }

    fn local_storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok().flatten()
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod platform {
    use std::fs;
    use std::path::PathBuf;

    use super::{AppLanguage, Theme};

    pub fn load_theme() -> Option<Theme> {
        let value = fs::read_to_string(settings_path()).ok()?;
        serde_json::from_str(&value).ok()
    }

    pub fn save_theme(theme: Theme) {
        let path = settings_path();

        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        if let Ok(value) = serde_json::to_string(&theme) {
            let _ = fs::write(path, value);
        }
    }

    pub fn load_language() -> Option<AppLanguage> {
        let value = fs::read_to_string(language_settings_path()).ok()?;
        serde_json::from_str(&value).ok()
    }

    pub fn save_language(language: AppLanguage) {
        let path = language_settings_path();

        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        if let Ok(value) = serde_json::to_string(&language) {
            let _ = fs::write(path, value);
        }
    }

    fn settings_path() -> PathBuf {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("data")
            .join("user-settings.json")
    }

    fn language_settings_path() -> PathBuf {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("data")
            .join("language-settings.json")
    }
}
