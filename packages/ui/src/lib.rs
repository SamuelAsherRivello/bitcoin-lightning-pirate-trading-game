//! Shared UI crate for the web and desktop packages.

pub mod client;

pub use client::{
    App, AppErrorFallback, AppLanguage, ConnectionStatus, DebugNetwork, DeveloperTools, Home,
    LabState, Page, PageFooter, PageHeader, PlayGame, PolarAutomationProfile,
    PolarConnectionProfile, PolarNodeConnection, Route, SetUp, SetupMode, SetupProfile,
    TemplateData, TemplateDataLoadRequest, TemplateDataLoadResult, TemplateDataSource, Theme,
};
pub use lightning_service::get_operation_faq;
