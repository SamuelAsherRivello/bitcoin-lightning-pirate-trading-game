use chrono::{DateTime, Utc};

use crate::client::models::TemplateData;

#[derive(Clone, Debug, PartialEq)]
pub struct CachedTemplateData {
    pub data: TemplateData,
    pub db_last_loaded_at: Option<DateTime<Utc>>,
}

pub async fn create_database_if_missing() -> Result<(), String> {
    platform::create_database_if_missing().await
}

pub async fn load_cached_template_data() -> Result<Option<CachedTemplateData>, String> {
    platform::load_cached_template_data().await
}

#[cfg(target_arch = "wasm32")]
mod platform {
    use crate::client::services::database_service::CachedTemplateData;

    pub async fn create_database_if_missing() -> Result<(), String> {
        Ok(())
    }

    pub async fn load_cached_template_data() -> Result<Option<CachedTemplateData>, String> {
        Ok(None)
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use std::fs;
    use std::future::Future;
    use std::path::PathBuf;
    use std::pin::Pin;
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

    use rusqlite::{params, Connection};

    use crate::client::models::TemplateData;

    use super::{create_database_if_missing, load_cached_template_data};

    #[test]
    fn create_database_if_missing_seeds_once_without_overwriting_existing_data() {
        let _guard = CurrentDirGuard::move_to_workspace_root();
        let database_path = PathBuf::from("data").join("bitcoin-lightning-pirate-trading-game.sqlite");

        if database_path.exists() {
            fs::remove_file(&database_path).expect("remove generated template database");
        }

        block_on(create_database_if_missing()).expect("create template database");
        let first_load = block_on(load_cached_template_data())
            .expect("load template data")
            .expect("seed row exists");

        assert_eq!(first_load.data.message, "Hello, World!");

        let connection = Connection::open(&database_path).expect("open generated database");
        connection
            .execute(
                "UPDATE template_data SET message = ?1 WHERE id = ?2",
                params!["custom", 1_i64],
            )
            .expect("update generated database row");
        drop(connection);

        block_on(create_database_if_missing()).expect("leave existing template database alone");
        let second_load = block_on(load_cached_template_data())
            .expect("load template data")
            .expect("existing row remains");

        assert_eq!(second_load.data.message, "custom");

        let seed = TemplateData::seed();
        let connection = Connection::open(&database_path).expect("reopen generated database");
        connection
            .execute(
                "UPDATE template_data SET message = ?1 WHERE id = ?2",
                params![seed.message, seed.id],
            )
            .expect("restore generated database seed row");
    }

    struct CurrentDirGuard {
        original: PathBuf,
    }

    impl CurrentDirGuard {
        fn move_to_workspace_root() -> Self {
            let original = std::env::current_dir().expect("read current dir");
            let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .canonicalize()
                .expect("resolve workspace root");

            std::env::set_current_dir(workspace_root).expect("move to workspace root");

            Self { original }
        }
    }

    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            std::env::set_current_dir(&self.original).expect("restore current dir");
        }
    }

    fn block_on<F: Future>(future: F) -> F::Output {
        let waker = noop_waker();
        let mut context = Context::from_waker(&waker);
        let mut future = Box::pin(future);

        loop {
            match Pin::new(&mut future).poll(&mut context) {
                Poll::Ready(value) => return value,
                Poll::Pending => std::thread::yield_now(),
            }
        }
    }

    fn noop_waker() -> Waker {
        unsafe fn clone(_: *const ()) -> RawWaker {
            RawWaker::new(std::ptr::null(), &VTABLE)
        }

        unsafe fn wake(_: *const ()) {}
        unsafe fn wake_by_ref(_: *const ()) {}
        unsafe fn drop(_: *const ()) {}

        static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

        unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod platform {
    use std::fs;
    use std::path::PathBuf;

    use chrono::Utc;
    use rusqlite::{params, Connection, OptionalExtension};

    use crate::client::models::TemplateData;
    use crate::client::services::database_service::CachedTemplateData;

    pub async fn create_database_if_missing() -> Result<(), String> {
        let path = database_path()?;
        let database_existed = path.exists();
        let connection = open_connection()?;
        let schema_existed = table_exists(&connection, "template_data")?;

        if database_existed && schema_existed {
            return Ok(());
        }

        create_schema(&connection)?;
        seed_template_data(&connection)
    }

    pub async fn load_cached_template_data() -> Result<Option<CachedTemplateData>, String> {
        let path = database_path()?;

        if !path.exists() {
            return Ok(None);
        }

        let connection = open_connection()?;

        if !table_exists(&connection, "template_data")? {
            return Ok(None);
        }

        let data = connection
            .query_row(
                "SELECT id, message FROM template_data WHERE id = ?1",
                params![1_i64],
                |row| {
                    Ok(TemplateData {
                        id: row.get(0)?,
                        message: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(|error| format!("Could not read template data: {error}"))?;

        Ok(data.map(|data| CachedTemplateData {
            data,
            db_last_loaded_at: Some(Utc::now()),
        }))
    }

    fn open_connection() -> Result<Connection, String> {
        let path = database_path()?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Could not create database directory: {error}"))?;
        }

        Connection::open(path).map_err(|error| format!("Could not open template database: {error}"))
    }

    fn database_path() -> Result<PathBuf, String> {
        let root = std::env::current_dir()
            .map_err(|error| format!("Could not find current directory for database: {error}"))?;

        Ok(root
            .join("data")
            .join("bitcoin-lightning-pirate-trading-game.sqlite"))
    }

    fn create_schema(connection: &Connection) -> Result<(), String> {
        connection
            .execute_batch(
                "
            CREATE TABLE IF NOT EXISTS template_data (
                id INTEGER PRIMARY KEY,
                message TEXT NOT NULL
            );
            ",
            )
            .map_err(|error| format!("Could not create template database schema: {error}"))
    }

    fn seed_template_data(connection: &Connection) -> Result<(), String> {
        let seed = TemplateData::seed();

        connection
            .execute(
                "INSERT OR IGNORE INTO template_data (id, message) VALUES (?1, ?2)",
                params![seed.id, seed.message],
            )
            .map_err(|error| format!("Could not seed template data: {error}"))?;

        Ok(())
    }

    fn table_exists(connection: &Connection, table_name: &str) -> Result<bool, String> {
        connection
            .query_row(
                "SELECT EXISTS (
                    SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1
                )",
                params![table_name],
                |row| row.get::<_, bool>(0),
            )
            .map_err(|error| format!("Could not inspect template database schema: {error}"))
    }
}
