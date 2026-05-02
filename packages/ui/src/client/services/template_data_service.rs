#[cfg(target_arch = "wasm32")]
use crate::client::models::TemplateData;
use crate::client::models::{TemplateDataLoadResult, TemplateDataSource};
#[cfg(not(target_arch = "wasm32"))]
use crate::client::services::database_service;
#[cfg(target_arch = "wasm32")]
use crate::client::services::storage_service;

pub async fn load_template_data() -> Result<TemplateDataLoadResult, String> {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(result) = storage_service::load_template_data_snapshot() {
            return Ok(result);
        }

        let result = seed_browser_snapshot();
        storage_service::save_template_data_snapshot(&result);
        return Ok(result);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(result) = load_from_database().await? {
            return Ok(result);
        }

        database_service::create_database_if_missing().await?;
        load_from_database()
            .await?
            .ok_or_else(|| "Template database was created without seed data.".to_string())
    }
}

pub async fn refresh_template_data() -> Result<TemplateDataLoadResult, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let result = seed_browser_snapshot();
        storage_service::save_template_data_snapshot(&result);
        Ok(result)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        database_service::create_database_if_missing().await?;
        load_from_database()
            .await?
            .ok_or_else(|| "Template database has no seed data.".to_string())
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn load_from_database() -> Result<Option<TemplateDataLoadResult>, String> {
    let Some(cached) = database_service::load_cached_template_data().await? else {
        return Ok(None);
    };

    Ok(Some(TemplateDataLoadResult {
        data: cached.data,
        source: TemplateDataSource::Database,
        db_last_loaded_at: cached.db_last_loaded_at,
    }))
}

#[cfg(target_arch = "wasm32")]
fn seed_browser_snapshot() -> TemplateDataLoadResult {
    TemplateDataLoadResult {
        data: TemplateData::seed(),
        source: TemplateDataSource::BrowserSnapshot,
        db_last_loaded_at: None,
    }
}
