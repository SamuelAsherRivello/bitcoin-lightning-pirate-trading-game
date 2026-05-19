use dioxus_i18n::fluent::{FluentArgs, FluentBundle, FluentResource};
use dioxus_i18n::unic_langid::LanguageIdentifier;
use ui::{
    get_operation_faq, AppLanguage, ConnectionStatus, SetupMode, SetupProfile, TemplateData,
    TemplateDataLoadRequest, Theme,
};

const LOCALES: [(&str, &str); 4] = [
    ("en-US", include_str!("../assets/i18n/en-US.ftl")),
    ("es-MX", include_str!("../assets/i18n/es-MX.ftl")),
    ("pt-BR", include_str!("../assets/i18n/pt-BR.ftl")),
    ("fr-FR", include_str!("../assets/i18n/fr-FR.ftl")),
];

const TRANSLATION_KEYS: &[&str] = &[
    "nav-setup",
    "nav-setup-short",
    "nav-play-game",
    "nav-play-game-short",
    "nav-debug-network",
    "nav-debug-network-short",
    "view-setup",
    "view-play-game",
    "view-debug-network",
    "locked-play-game",
    "locked-debug-network",
    "setup-title",
    "play-game-title",
    "debug-network-title",
    "nav-page-01",
    "nav-page-01-short",
    "nav-page-02",
    "nav-page-02-short",
    "nav-page-03",
    "nav-page-03-short",
    "view-page-01",
    "view-page-02",
    "view-page-03",
    "open-github-repository",
    "refresh-template-data",
    "toggle-theme",
    "theme",
    "language-selector",
    "language-en",
    "language-es",
    "language-pt",
    "language-fr",
    "loading-template-data",
    "template-data-unavailable",
    "source-database",
    "toast-action-loading",
    "toast-action-loaded",
    "toast-action-error",
    "page-01-title",
    "page-01-body-01",
    "page-01-body-02",
    "page-01-body-03-prefix",
    "page-01-body-03-warning",
    "page-02-title",
    "page-02-body-01",
    "page-02-body-02",
    "page-02-body-03",
    "page-03-title",
    "page-03-body-01",
    "page-03-body-02",
    "page-03-body-03",
    "footer-rights",
];

#[test]
fn theme_labels_match_display_text() {
    assert_eq!(Theme::Light.label(), "Light");
    assert_eq!(Theme::Dark.label(), "Dark");
}

#[test]
fn theme_class_names_match_shell_modifiers() {
    assert_eq!(Theme::Light.class_name(), "app-shell--light");
    assert_eq!(Theme::Dark.class_name(), "app-shell--dark");
}

#[test]
fn theme_default_is_dark() {
    assert_eq!(Theme::default(), Theme::Dark);
}

#[test]
fn theme_toggle_switches_between_light_and_dark() {
    assert_eq!(Theme::Light.toggled(), Theme::Dark);
    assert_eq!(Theme::Dark.toggled(), Theme::Light);
}

#[test]
fn language_default_is_english() {
    assert_eq!(AppLanguage::default(), AppLanguage::En);
}

#[test]
fn template_data_seed_is_the_default_database_row() {
    let seed = TemplateData::seed();

    assert_eq!(seed.id, 1);
    assert_eq!(seed.message, "Hello, World!");
}

#[test]
fn template_data_load_request_starts_at_zero() {
    assert_eq!(TemplateDataLoadRequest::initial().sequence, 0);
}

#[test]
fn setup_profile_defaults_to_polar_connection() {
    let profile = SetupProfile::default();

    assert_eq!(profile.sats_per_transaction, 1_000);
    assert_eq!(profile.setup_mode, SetupMode::ServerConfig);
    assert_eq!(profile.connection_status, ConnectionStatus::NotConfigured);
    assert!(!profile.polar_connection.is_complete());
    assert_eq!(
        profile.polar_automation.bridge_url,
        "http://localhost:37373"
    );
    assert_eq!(
        profile.polar_automation.bitcoin_backend_name,
        "My Bitcoin Node"
    );
    assert!(profile.polar_automation.is_complete());
}

#[test]
fn setup_mode_labels_match_connection_tabs() {
    assert_eq!(
        SetupMode::BrowserRegtestOnly.label(),
        "Mock Connection (Offline)"
    );
    assert_eq!(
        SetupMode::ServerConfig.label(),
        "Polar Connection (Networked)"
    );
}

#[test]
fn operation_faq_covers_block_dependent_actions() {
    let rows = get_operation_faq();

    assert!(rows
        .iter()
        .any(|row| row.operation == "Create invoice" && !row.needs_mined_block));
    assert!(rows
        .iter()
        .any(|row| row.operation == "Pay invoice" && !row.needs_mined_block));
    assert!(rows
        .iter()
        .any(|row| row.operation == "Open channel" && row.needs_mined_block));
    assert!(rows
        .iter()
        .any(|row| row.operation == "Wait for next block" && row.needs_mined_block));
}

#[test]
fn locale_files_are_valid_fluent() {
    for (locale, source) in LOCALES {
        FluentResource::try_new(source.to_string())
            .unwrap_or_else(|errors| panic!("{locale} has invalid Fluent syntax: {errors:#?}"));
    }
}

#[test]
fn locale_files_cover_all_ui_translation_keys() {
    for (locale, source) in LOCALES {
        for key in TRANSLATION_KEYS {
            let has_key = source
                .lines()
                .any(|line| line.trim_start().starts_with(&format!("{key} =")));

            assert!(has_key, "{locale} is missing translation key `{key}`");
        }
    }
}

#[test]
fn locale_files_format_all_ui_translation_keys() {
    let args = FluentArgs::new();

    for (locale, source) in LOCALES {
        let language_id = LanguageIdentifier::from_bytes(locale.as_bytes())
            .unwrap_or_else(|error| panic!("{locale} has an invalid language id: {error}"));
        let resource = FluentResource::try_new(source.to_string())
            .unwrap_or_else(|errors| panic!("{locale} has invalid Fluent syntax: {errors:#?}"));
        let mut bundle = FluentBundle::new(vec![language_id]);

        bundle
            .add_resource(resource)
            .unwrap_or_else(|errors| panic!("{locale} has invalid Fluent resources: {errors:#?}"));

        for key in TRANSLATION_KEYS {
            let message = bundle
                .get_message(key)
                .unwrap_or_else(|| panic!("{locale} is missing translation key `{key}`"));
            let pattern = message
                .value()
                .unwrap_or_else(|| panic!("{locale} translation key `{key}` has no value"));
            let mut errors = Vec::new();
            let formatted = bundle.format_pattern(pattern, Some(&args), &mut errors);

            assert!(
                errors.is_empty(),
                "{locale} translation key `{key}` failed to format: {errors:#?}"
            );
            assert!(
                !formatted.trim().is_empty(),
                "{locale} translation key `{key}` formatted to empty text"
            );
        }
    }
}
