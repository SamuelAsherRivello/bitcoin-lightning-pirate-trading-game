use serde::{Deserialize, Serialize};

use crate::client::models::{LabState, SetupProfile};
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

pub fn load_setup_profile() -> SetupProfile {
    platform::load_setup_profile().unwrap_or_default()
}

pub fn save_setup_profile(profile: &SetupProfile) {
    platform::save_setup_profile(profile);
}

pub fn clear_setup_profile() {
    platform::clear_setup_profile();
}

pub fn load_lab_state_snapshot() -> Option<LabState> {
    platform::load_lab_state_snapshot()
}

pub fn save_lab_state_snapshot(state: &LabState) {
    debug_assert_tra_snapshot_is_non_sensitive(state);
    platform::save_lab_state_snapshot(state);
}

pub fn clear_lab_state_snapshot() {
    platform::clear_lab_state_snapshot();
}

fn debug_assert_tra_snapshot_is_non_sensitive(state: &LabState) {
    for item in &state.tra_items {
        debug_assert!(
            !looks_sensitive(&item.tra_id)
                && !looks_sensitive(&item.asset_id)
                && !looks_sensitive(&item.unique_name),
            "TRA inventory snapshots may store only non-sensitive identity, item_id, owner, and status fields"
        );
    }
}

fn looks_sensitive(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    [
        "macaroon", "seed", "private", "xprv", "proof", "password", "secret",
    ]
    .iter()
    .any(|marker| value.contains(marker))
}

pub fn load_setup_polar_tab() -> Option<String> {
    platform::load_setup_polar_tab()
}

pub fn save_setup_polar_tab(tab: &str) {
    platform::save_setup_polar_tab(tab);
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
    use crate::client::models::{
        LabState, SetupProfile, TemplateDataLoadResult, TemplateDataSource,
    };

    use super::{AppLanguage, Theme, TEMPLATE_DATA_SNAPSHOT_KEY};

    const THEME_STORAGE_KEY: &str = "dioxus-bitcoin-lightning-game:theme";
    const LANGUAGE_STORAGE_KEY: &str = "dioxus-bitcoin-lightning-game:language";
    const SETUP_PROFILE_STORAGE_KEY: &str = "dioxus-bitcoin-lightning-game:setup-profile";
    const SETUP_POLAR_TAB_STORAGE_KEY: &str = "dioxus-bitcoin-lightning-game:setup-polar-tab";
    const LAB_STATE_STORAGE_KEY: &str = "dioxus-bitcoin-lightning-game:lab-state";

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

    pub fn load_setup_profile() -> Option<SetupProfile> {
        let value = local_storage()?
            .get_item(SETUP_PROFILE_STORAGE_KEY)
            .ok()
            .flatten()?;
        serde_json::from_str(&value).ok()
    }

    pub fn save_setup_profile(profile: &SetupProfile) {
        let Some(storage) = local_storage() else {
            return;
        };
        let Ok(value) = serde_json::to_string(profile) else {
            return;
        };

        let _ = storage.set_item(SETUP_PROFILE_STORAGE_KEY, &value);
    }

    pub fn clear_setup_profile() {
        let Some(storage) = local_storage() else {
            return;
        };

        let _ = storage.remove_item(SETUP_PROFILE_STORAGE_KEY);
    }

    pub fn load_lab_state_snapshot() -> Option<LabState> {
        let value = local_storage()?
            .get_item(LAB_STATE_STORAGE_KEY)
            .ok()
            .flatten()?;
        serde_json::from_str(&value).ok()
    }

    pub fn save_lab_state_snapshot(state: &LabState) {
        let Some(storage) = local_storage() else {
            return;
        };
        let Ok(value) = serde_json::to_string(state) else {
            return;
        };

        let _ = storage.set_item(LAB_STATE_STORAGE_KEY, &value);
    }

    pub fn clear_lab_state_snapshot() {
        let Some(storage) = local_storage() else {
            return;
        };

        let _ = storage.remove_item(LAB_STATE_STORAGE_KEY);
    }

    pub fn load_setup_polar_tab() -> Option<String> {
        local_storage()?
            .get_item(SETUP_POLAR_TAB_STORAGE_KEY)
            .ok()
            .flatten()
    }

    pub fn save_setup_polar_tab(tab: &str) {
        let Some(storage) = local_storage() else {
            return;
        };

        let _ = storage.set_item(SETUP_POLAR_TAB_STORAGE_KEY, tab);
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

    use crate::client::models::{LabState, SetupProfile};

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

    pub fn load_setup_profile() -> Option<SetupProfile> {
        let value = fs::read_to_string(setup_profile_path()).ok()?;
        serde_json::from_str(&value).ok()
    }

    pub fn save_setup_profile(profile: &SetupProfile) {
        write_json(&setup_profile_path(), profile);
    }

    pub fn clear_setup_profile() {
        let _ = fs::remove_file(setup_profile_path());
    }

    pub fn load_lab_state_snapshot() -> Option<LabState> {
        let value = fs::read_to_string(lab_state_path()).ok()?;
        serde_json::from_str(&value).ok()
    }

    pub fn save_lab_state_snapshot(state: &LabState) {
        write_json(&lab_state_path(), state);
    }

    pub fn clear_lab_state_snapshot() {
        let _ = fs::remove_file(lab_state_path());
    }

    pub fn load_setup_polar_tab() -> Option<String> {
        fs::read_to_string(setup_polar_tab_path()).ok()
    }

    pub fn save_setup_polar_tab(tab: &str) {
        let path = setup_polar_tab_path();

        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let _ = fs::write(path, tab);
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

    fn setup_profile_path() -> PathBuf {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("data")
            .join("setup-profile.json")
    }

    fn setup_polar_tab_path() -> PathBuf {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("data")
            .join("setup-polar-tab.txt")
    }

    fn lab_state_path() -> PathBuf {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("data")
            .join("lightning-lab-state.json")
    }

    fn write_json<T: serde::Serialize>(path: &PathBuf, value: &T) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        if let Ok(value) = serde_json::to_string(value) {
            let _ = fs::write(path, value);
        }
    }
}
