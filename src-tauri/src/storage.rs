use crate::model::{Observation, Settings};
use crate::observability::{failure, map_error, record, storage_error};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::json;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::Path;

pub struct Store {
    connection: Connection,
}
impl Store {
    pub fn open(path: &Path) -> Result<Self, String> {
        let mut connection =
            Connection::open(path).map_err(|e| map_error("storage.open.failed", "open", e))?;
        let version: u32 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|e| {
                storage_error(e);
                "Local database is unreadable"
            })?;
        if version > 1 {
            return Err("This database belongs to a newer Maxxit version. Install that version before opening it.".into());
        }
        connection
            .execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| {
                storage_error(e);
                "Local database is unreadable"
            })?;
        if version == 0 {
            record(
                "storage.migration.started",
                json!({"schema_version":version}),
            );
            let transaction = connection.transaction().map_err(|e| {
                storage_error(e);
                "Storage migration could not begin"
            })?;
            transaction.execute_batch("CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL); CREATE TABLE IF NOT EXISTS observations (id TEXT PRIMARY KEY, provider TEXT NOT NULL, account TEXT NOT NULL, observed_at TEXT NOT NULL, body TEXT NOT NULL); CREATE INDEX IF NOT EXISTS observation_time ON observations(provider,account,observed_at); CREATE TABLE IF NOT EXISTS projects (id TEXT PRIMARY KEY, body TEXT NOT NULL); PRAGMA user_version=1;").map_err(storage_error)?;
            transaction.commit().map_err(|e| {
                storage_error(e);
                "Storage migration could not be committed"
            })?;
            record("storage.migration.completed", json!({"schema_version":1}));
        }
        let store = Self { connection };
        store.settings()?;
        record("storage.open.completed", json!({"schema_version":1}));
        Ok(store)
    }
    pub fn settings(&self) -> Result<Settings, String> {
        match self.get("preferences")? {
            None => Ok(Settings::default()),
            Some(value) => serde_json::from_value(value).map_err(|e| { map_error("storage.record.corrupt", "parse", e); "Stored preferences are invalid. Restore a verified backup instead of overwriting them.".into() }),
        }
    }
    pub fn save_settings(&self, settings: &Settings) -> Result<(), String> {
        self.set(
            "preferences",
            &serde_json::to_value(settings)
                .map_err(|e| map_error("settings.save.failed", "write", e))?,
        )
    }
    fn get(&self, key: &str) -> Result<Option<Value>, String> {
        let raw: Option<String> = self
            .connection
            .query_row("SELECT value FROM settings WHERE key=?1", [key], |row| {
                row.get(0)
            })
            .optional()
            .map_err(|e| {
                storage_error(e);
                "Stored preferences could not be read"
            })?;
        raw.map(|s| {
            serde_json::from_str(&s).map_err(|e| {
                map_error("storage.record.corrupt", "parse", e);
                "Stored preferences are corrupt".into()
            })
        })
        .transpose()
    }
    pub fn set(&self, key: &str, value: &Value) -> Result<(), String> {
        self.connection.execute("INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![key,value.to_string()]).map(|_|()).map_err(storage_error)
    }
    pub fn record(&self, observation: &Observation) -> Result<(), String> {
        let body = serde_json::to_string(observation)
            .map_err(|e| map_error("storage.operation.failed", "write", e))?;
        let id = format!("{:x}", Sha256::digest(body.as_bytes()));
        self.connection.execute("INSERT OR IGNORE INTO observations(id,provider,account,observed_at,body) VALUES(?1,?2,?3,?4,?5)",params![id,observation.provider,observation.account_label,observation.observed_at,body]).map_err(storage_error)?;
        self.connection
            .execute(
                "DELETE FROM observations WHERE observed_at < datetime('now','-90 days')",
                [],
            )
            .map_err(storage_error)?;
        Ok(())
    }
    pub fn history(&self) -> Result<Vec<Observation>, String> {
        let mut query = self
            .connection
            .prepare("SELECT body FROM observations ORDER BY observed_at DESC LIMIT 5000")
            .map_err(storage_error)?;
        let records = query
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(storage_error)?;
        Ok(records.filter_map(decode_record).collect())
    }
    pub fn projects(&self) -> Result<Vec<Value>, String> {
        let mut q = self
            .connection
            .prepare("SELECT body FROM projects ORDER BY rowid DESC")
            .map_err(storage_error)?;
        let records = q
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(storage_error)?;
        Ok(records.filter_map(decode_record).collect())
    }
    pub fn save_project(&self, project: &Value) -> Result<(), String> {
        let id = project
            .get("id")
            .and_then(Value::as_str)
            .ok_or("Missing project ID")?;
        self.connection.execute("INSERT INTO projects(id,body) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET body=excluded.body",params![id,project.to_string()]).map(|_|()).map_err(storage_error)
    }
    pub fn remove_project(&self, id: &str) -> Result<(), String> {
        self.connection
            .execute("DELETE FROM projects WHERE id=?1", [id])
            .map(|_| ())
            .map_err(storage_error)
    }
}

fn decode_record<T: serde::de::DeserializeOwned>(
    row: Result<String, rusqlite::Error>,
) -> Option<T> {
    match row {
        Ok(body) => match serde_json::from_str(&body) {
            Ok(value) => Some(value),
            Err(_) => {
                failure(
                    "storage.record.corrupt",
                    json!({"stage":"parse","rejected_count":1}),
                );
                None
            }
        },
        Err(error) => {
            storage_error(error);
            None
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_observation_is_deduplicated() {
        let store = Store::open(Path::new(":memory:")).unwrap();
        let o = Observation {
            provider: "claude".into(),
            account_label: "local".into(),
            observed_at: "2099-01-01T00:00:00Z".into(),
            source: "claude-statusline".into(),
            windows: vec![],
        };
        store.record(&o).unwrap();
        store.record(&o).unwrap();
        assert_eq!(store.history().unwrap().len(), 1);
    }

    #[test]
    fn newer_schema_is_rejected_without_resetting_it() {
        let path = std::env::temp_dir().join(format!("{}.sqlite", uuid::Uuid::new_v4()));
        let connection = Connection::open(&path).unwrap();
        connection.execute_batch("PRAGMA user_version=2;").unwrap();
        drop(connection);
        assert!(Store::open(&path).is_err());
        let connection = Connection::open(&path).unwrap();
        assert_eq!(
            connection
                .query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0))
                .unwrap(),
            2
        );
        drop(connection);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn corrupt_preferences_do_not_become_defaults() {
        let store = Store::open(Path::new(":memory:")).unwrap();
        store
            .connection
            .execute("INSERT INTO settings VALUES('preferences','broken')", [])
            .unwrap();
        assert!(store.settings().is_err());
        let raw: String = store
            .connection
            .query_row(
                "SELECT value FROM settings WHERE key='preferences'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(raw, "broken");
    }

    #[test]
    fn reopening_preserves_settings_and_projects() {
        let path = std::env::temp_dir().join(format!("{}.sqlite", uuid::Uuid::new_v4()));
        {
            let store = Store::open(&path).unwrap();
            let settings = Settings {
                theme: "dark".into(),
                ..Settings::default()
            };
            store.save_settings(&settings).unwrap();
            store
                .save_project(&serde_json::json!({"id":"fixture","name":"Synthetic project"}))
                .unwrap();
        }
        let store = Store::open(&path).unwrap();
        assert_eq!(store.settings().unwrap().theme, "dark");
        assert_eq!(store.projects().unwrap()[0]["id"], "fixture");
        drop(store);
        std::fs::remove_file(path).unwrap();
    }
}
