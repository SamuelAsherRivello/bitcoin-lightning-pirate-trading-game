use dioxus::prelude::*;
use dioxus_i18n::prelude::*;
use dioxus_i18n::unic_langid::{langid, LanguageIdentifier};
use serde::{Deserialize, Serialize};

const EN_US: &str = include_str!("../../../assets/i18n/en-US.ftl");
const ES_MX: &str = include_str!("../../../assets/i18n/es-MX.ftl");
const PT_BR: &str = include_str!("../../../assets/i18n/pt-BR.ftl");
const FR_FR: &str = include_str!("../../../assets/i18n/fr-FR.ftl");

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AppLanguage {
    En,
    Es,
    Pt,
    Fr,
}

impl AppLanguage {
    pub const ALL: [Self; 4] = [Self::En, Self::Es, Self::Pt, Self::Fr];

    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Es => "es",
            Self::Pt => "pt",
            Self::Fr => "fr",
        }
    }

    pub fn flag_asset(self) -> Asset {
        match self {
            Self::En => asset!("/assets/images/flags/en-us.svg"),
            Self::Es => asset!("/assets/images/flags/es-mx.svg"),
            Self::Pt => asset!("/assets/images/flags/pt-br.svg"),
            Self::Fr => asset!("/assets/images/flags/fr-fr.svg"),
        }
    }

    pub fn language_id(self) -> LanguageIdentifier {
        match self {
            Self::En => langid!("en-US"),
            Self::Es => langid!("es-MX"),
            Self::Pt => langid!("pt-BR"),
            Self::Fr => langid!("fr-FR"),
        }
    }

    pub fn from_code(value: &str) -> Self {
        match value {
            "es" => Self::Es,
            "pt" => Self::Pt,
            "fr" => Self::Fr,
            _ => Self::En,
        }
    }
}

impl Default for AppLanguage {
    fn default() -> Self {
        Self::En
    }
}

pub fn config(initial_language: AppLanguage) -> I18nConfig {
    I18nConfig::new(initial_language.language_id())
        .with_fallback(AppLanguage::En.language_id())
        .with_locale((AppLanguage::En.language_id(), EN_US))
        .with_locale((AppLanguage::Es.language_id(), ES_MX))
        .with_locale((AppLanguage::Pt.language_id(), PT_BR))
        .with_locale((AppLanguage::Fr.language_id(), FR_FR))
}
