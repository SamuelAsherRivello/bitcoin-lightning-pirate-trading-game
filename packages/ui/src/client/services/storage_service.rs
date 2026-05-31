use serde::{Deserialize, Serialize};

use crate::client::models::{LabState, NostrProfile, SetupProfile};
use crate::client::services::localization_service::AppLanguage;

#[cfg(target_arch = "wasm32")]
use crate::client::models::TemplateDataLoadResult;

#[cfg(target_arch = "wasm32")]
const TEMPLATE_DATA_SNAPSHOT_KEY: &str = "bitcoin-lightning-pirate-trading-game:data-snapshot";

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
    debug_assert_setup_profile_is_non_sensitive(profile);
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

pub fn load_nostr_profile_snapshot() -> Option<NostrProfile> {
    platform::load_nostr_profile_snapshot()
}

pub fn save_nostr_profile_snapshot(profile: &NostrProfile) {
    debug_assert_nostr_profile_snapshot_is_non_sensitive(profile);
    platform::save_nostr_profile_snapshot(profile);
}

pub fn clear_nostr_profile_snapshot() {
    platform::clear_nostr_profile_snapshot();
}

fn debug_assert_tra_snapshot_is_non_sensitive(state: &LabState) {
    debug_assert_setup_profile_is_non_sensitive(&state.profile);
    debug_assert_auth_snapshot_is_non_sensitive(state);

    for item in &state.tra_items {
        debug_assert!(
            !looks_sensitive(&item.tra_id)
                && !looks_sensitive(&item.asset_id)
                && !looks_sensitive(&item.unique_name),
            "TRA inventory snapshots may store only non-sensitive identity, item_id, owner, and status fields"
        );
    }
    for entry in &state.game_treasury.recent_entries {
        debug_assert!(
            !looks_sensitive(&entry.description) && !looks_sensitive(&entry.related_action),
            "Treasury history snapshots may not store wallet secrets, credentials, seeds, macaroons, or proof material"
        );
    }
    for resource in &state.game_treasury.owned_items {
        debug_assert!(
            !looks_sensitive(&resource.resource_id) && !looks_sensitive(&resource.display_name),
            "Treasury resource snapshots may store only non-sensitive resource labels and item identities"
        );
    }
}

fn debug_assert_setup_profile_is_non_sensitive(profile: &SetupProfile) {
    if let Some(identity) = &profile.player_identity {
        debug_assert_player_identity_is_non_sensitive(identity);
    }
}

fn debug_assert_auth_snapshot_is_non_sensitive(state: &LabState) {
    if let Some(session) = &state.player_auth_session {
        debug_assert!(
            !looks_sensitive(&session.session_id)
                && !looks_sensitive(&session.challenge_id)
                && !looks_sensitive(&session.lnurl)
                && !looks_sensitive(&session.qr_payload)
                && session
                    .failure_reason
                    .as_ref()
                    .is_none_or(|reason| !looks_sensitive(reason)),
            "Auth session snapshots may store only non-sensitive challenge IDs, QR payloads, statuses, and recoverable failure summaries"
        );
        if let Some(identity) = &session.player_identity {
            debug_assert_player_identity_is_non_sensitive(identity);
        }
    }

    for approval in &state.recent_transaction_approvals {
        debug_assert!(
            !looks_sensitive(&approval.approval_id)
                && !looks_sensitive(&approval.operation_summary)
                && approval
                    .failure_reason
                    .as_ref()
                    .is_none_or(|reason| !looks_sensitive(reason)),
            "Approval history snapshots may not store wallet secrets, credentials, seeds, macaroons, or proof material"
        );
        if let Some(identity) = &approval.player_identity {
            debug_assert_player_identity_is_non_sensitive(identity);
        }
    }

    for warning in &state.auth_warnings {
        debug_assert!(
            !looks_sensitive(warning),
            "Auth warning snapshots may not store wallet secrets, credentials, seeds, macaroons, or proof material"
        );
    }
}

fn debug_assert_player_identity_is_non_sensitive(identity: &crate::client::models::PlayerIdentity) {
    debug_assert!(
        !looks_sensitive(&identity.linking_key_fingerprint)
            && !looks_sensitive(&identity.display_label),
        "Player identity snapshots may store only non-sensitive public fingerprints and display labels"
    );
}

fn debug_assert_nostr_profile_snapshot_is_non_sensitive(profile: &NostrProfile) {
    debug_assert!(
        !looks_sensitive(&profile.public_key)
            && profile
                .username
                .as_ref()
                .is_none_or(|username| !looks_sensitive(username))
            && profile
                .last_error
                .as_ref()
                .is_none_or(|error| !looks_sensitive(error))
            && profile
                .relay_urls
                .iter()
                .all(|relay_url| !looks_sensitive(relay_url)),
        "Nostr profile snapshots may store only non-sensitive public key, username, relay, status, and timestamp fields"
    );
}

fn looks_sensitive(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    [
        "macaroon", "seed", "private", "xprv", "proof", "password", "secret", "nsec", "bearer",
        "token", "cookie",
    ]
    .iter()
    .any(|marker| value.contains(marker))
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::client::models::{
        ApprovalOperationKind, DemoNodeId, NostrProfile, NostrProfilePublishStatus,
        NostrProfileSource, SetupProfile, TransactionApproval, TransactionApprovalStatus,
    };

    use super::*;

    #[test]
    fn setup_profile_auth_identity_allows_public_fingerprint() {
        let mut profile = SetupProfile::default();
        profile.player_identity = Some(crate::client::models::PlayerIdentity {
            linking_key_fingerprint: "pubkey-fingerprint-123".to_string(),
            display_label: "Player wallet".to_string(),
            authenticated_at: Utc::now(),
            last_seen_at: None,
        });

        debug_assert_setup_profile_is_non_sensitive(&profile);
    }

    #[test]
    #[should_panic(expected = "Player identity snapshots may store only non-sensitive")]
    fn setup_profile_auth_identity_rejects_secret_like_values() {
        let mut profile = SetupProfile::default();
        profile.player_identity = Some(crate::client::models::PlayerIdentity {
            linking_key_fingerprint: "private-key-material".to_string(),
            display_label: "Player wallet".to_string(),
            authenticated_at: Utc::now(),
            last_seen_at: None,
        });

        debug_assert_setup_profile_is_non_sensitive(&profile);
    }

    #[test]
    #[should_panic(expected = "Approval history snapshots may not store wallet secrets")]
    fn approval_history_rejects_secret_like_values() {
        let profile = SetupProfile::default();
        let mut state = lightning_service::default_lab_state(profile);
        state
            .recent_transaction_approvals
            .push(TransactionApproval {
                approval_id: "approval-1".to_string(),
                operation_kind: ApprovalOperationKind::SendSats,
                operation_summary: "pay with macaroon".to_string(),
                player_identity: None,
                amount_sats: Some(1_000),
                status: TransactionApprovalStatus::Pending,
                created_at: Utc::now(),
                expires_at: None,
                approved_at: None,
                failure_reason: None,
            });
        state
            .nodes
            .retain(|node| node.node_id != DemoNodeId::GameTreasury);

        debug_assert_auth_snapshot_is_non_sensitive(&state);
    }

    #[test]
    fn nostr_profile_snapshot_allows_public_summary() {
        let profile = NostrProfile {
            public_key: "abcdef123456".to_string(),
            username: Some("jack".to_string()),
            source: NostrProfileSource::Mock,
            publish_status: NostrProfilePublishStatus::Published,
            updated_at: Some(Utc::now()),
            relay_urls: vec!["wss://relay.example".to_string()],
            last_error: None,
        };

        debug_assert_nostr_profile_snapshot_is_non_sensitive(&profile);
    }

    #[test]
    #[should_panic(expected = "Nostr profile snapshots may store only non-sensitive")]
    fn nostr_profile_snapshot_rejects_secret_like_values() {
        let profile = NostrProfile {
            public_key: "nsec1private".to_string(),
            username: Some("jack".to_string()),
            source: NostrProfileSource::Mock,
            publish_status: NostrProfilePublishStatus::Published,
            updated_at: Some(Utc::now()),
            relay_urls: Vec::new(),
            last_error: None,
        };

        debug_assert_nostr_profile_snapshot_is_non_sensitive(&profile);
    }
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
        LabState, NostrProfile, SetupProfile, TemplateDataLoadResult, TemplateDataSource,
    };

    use super::{AppLanguage, Theme, TEMPLATE_DATA_SNAPSHOT_KEY};

    const THEME_STORAGE_KEY: &str = "bitcoin-lightning-pirate-trading-game:theme";
    const LANGUAGE_STORAGE_KEY: &str = "bitcoin-lightning-pirate-trading-game:language";
    const SETUP_PROFILE_STORAGE_KEY: &str = "bitcoin-lightning-pirate-trading-game:setup-profile";
    const SETUP_POLAR_TAB_STORAGE_KEY: &str = "bitcoin-lightning-pirate-trading-game:setup-polar-tab";
    const LAB_STATE_STORAGE_KEY: &str = "bitcoin-lightning-pirate-trading-game:lab-state";
    const NOSTR_PROFILE_STORAGE_KEY: &str = "bitcoin-lightning-pirate-trading-game:nostr-profile";

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

    pub fn load_nostr_profile_snapshot() -> Option<NostrProfile> {
        let value = local_storage()?
            .get_item(NOSTR_PROFILE_STORAGE_KEY)
            .ok()
            .flatten()?;
        serde_json::from_str(&value).ok()
    }

    pub fn save_nostr_profile_snapshot(profile: &NostrProfile) {
        let Some(storage) = local_storage() else {
            return;
        };
        let Ok(value) = serde_json::to_string(profile) else {
            return;
        };

        let _ = storage.set_item(NOSTR_PROFILE_STORAGE_KEY, &value);
    }

    pub fn clear_nostr_profile_snapshot() {
        let Some(storage) = local_storage() else {
            return;
        };

        let _ = storage.remove_item(NOSTR_PROFILE_STORAGE_KEY);
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

    use crate::client::models::{LabState, NostrProfile, SetupProfile};

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

    pub fn load_nostr_profile_snapshot() -> Option<NostrProfile> {
        let value = fs::read_to_string(nostr_profile_path()).ok()?;
        serde_json::from_str(&value).ok()
    }

    pub fn save_nostr_profile_snapshot(profile: &NostrProfile) {
        write_json(&nostr_profile_path(), profile);
    }

    pub fn clear_nostr_profile_snapshot() {
        let _ = fs::remove_file(nostr_profile_path());
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

    fn nostr_profile_path() -> PathBuf {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("data")
            .join("nostr-profile.json")
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
