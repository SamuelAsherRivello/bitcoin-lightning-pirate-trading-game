//! Shared UI crate for the web and desktop packages.

pub mod client;

pub use client::{
    App, AppErrorFallback, AppLanguage, DeveloperTools, Page, Page01, Page02, Page03, PageFooter,
    PageHeader, Route, TemplateData, TemplateDataLoadRequest, TemplateDataLoadResult,
    TemplateDataSource, Theme,
};
