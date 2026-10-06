use crate::model::{Observation, Settings};
use rusqlite::{params, Connection};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::Path;

pub struct Store {
    connection: Connection,
}
impl Store {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL); CREATE TABLE IF NOT EXISTS observations (id TEXT PRIMARY KEY, provider TEXT NOT NULL, account TEXT NOT NULL, observed_at TEXT NOT NULL, body TEXT NOT NULL); CREATE INDEX IF NOT EXISTS observation_time ON observations(provider,account,observed_at); CREATE TABLE IF NOT EXISTS projects (id TEXT PRIMARY KEY, body TEXT NOT NULL); PRAGMA user_version=1;").map_err(|e| e.to_string())?;
        Ok(Self { connection })
    }
    pub fn settings(&self) -> Settings {
        self.get("preferences")
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default()
    }
    pub fn save_settings(&self, settings: &Settings) -> Result<(), String> {
        self.set(
            "preferences",
            &serde_json::to_value(settings).map_err(|e| e.to_string())?,
        )
    }
    pub fn get(&self, key: &str) -> Option<Value> {
        self.connection
            .query_row("SELECT value FROM settings WHERE key=?1", [key], |r| {
                r.get::<_, String>(0)
            })
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
    }
    pub fn set(&self, key: &str, value: &Value) -> Result<(), String> {
        self.connection.execute("INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![key,value.to_string()]).map(|_|()).map_err(|e|e.to_string())
    }
    pub fn record(&self, observation: &Observation) -> Result<(), String> {
        let body = serde_json::to_string(observation).map_err(|e| e.to_string())?;
        let id = format!("{:x}", Sha256::digest(body.as_bytes()));
        self.connection.execute("INSERT OR IGNORE INTO observations(id,provider,account,observed_at,body) VALUES(?1,?2,?3,?4,?5)",params![id,observation.provider,observation.account_label,observation.observed_at,body]).map_err(|e|e.to_string())?;
        self.connection
            .execute(
                "DELETE FROM observations WHERE observed_at < datetime('now','-90 days')",
                [],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn history(&self) -> Result<Vec<Observation>, String> {
        let mut query = self
            .connection
            .prepare("SELECT body FROM observations ORDER BY observed_at DESC LIMIT 5000")
            .map_err(|e| e.to_string())?;
        let records = query
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        Ok(records
            .filter_map(|v| v.ok().and_then(|s| serde_json::from_str(&s).ok()))
            .collect())
    }
    pub fn projects(&self) -> Result<Vec<Value>, String> {
        let mut q = self
            .connection
            .prepare("SELECT body FROM projects ORDER BY rowid DESC")
            .map_err(|e| e.to_string())?;
        let records = q
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        Ok(records
            .filter_map(|r| r.ok().and_then(|s| serde_json::from_str(&s).ok()))
            .collect())
    }
    pub fn save_project(&self, project: &Value) -> Result<(), String> {
        let id = project
            .get("id")
            .and_then(Value::as_str)
            .ok_or("Missing project ID")?;
        self.connection.execute("INSERT INTO projects(id,body) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET body=excluded.body",params![id,project.to_string()]).map(|_|()).map_err(|e|e.to_string())
    }
    pub fn remove_project(&self, id: &str) -> Result<(), String> {
        self.connection
            .execute("DELETE FROM projects WHERE id=?1", [id])
            .map(|_| ())
            .map_err(|e| e.to_string())
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
}
