use crate::model::{Observation, Settings};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::Path;

pub struct Store {
    pub(crate) connection: Connection,
}
impl Store {
    #[cfg(test)]
    pub fn open(path: &Path) -> Result<Self, String> {
        Self::initialize(Connection::open(path).map_err(|_| "Local database could not be opened")?)
    }
    pub fn open_keychain(path: &Path) -> Result<Self, String> {
        let entry = keyring::Entry::new("app.maxxit.desktop", "database")
            .map_err(|_| "Database Keychain could not be opened")?;
        let key = match entry.get_password() {
            Ok(key) => key,
            Err(keyring::Error::NoEntry) => {
                if path.exists() && !is_plaintext(path)? {
                    return Err("The database key is missing. Restore its Keychain item and encrypted database backup; existing data was not changed.".into());
                }
                let key = format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());
                entry.set_password(&key).map_err(|_| "Could not save database key in Keychain")?;
                if entry.get_password().ok().as_deref() != Some(&key) {
                    return Err("Database key could not be verified. Existing data was not changed.".into());
                }
                key
            }
            Err(_) => return Err("Database Keychain is unavailable. Unlock it and retry; existing data was not changed.".into()),
        };
        Self::open_encrypted(path, &key)
    }
    pub fn open_encrypted(path: &Path, key: &str) -> Result<Self, String> {
        if key.len() != 64 || !key.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("Invalid database key".into());
        }
        let backup = path.with_extension("plaintext-backup");
        let next = path.with_extension("encrypted-next");
        if backup.exists() || next.exists() {
            return Err("An interrupted database migration needs recovery. Keep these files and restore the original database or finish the verified encrypted replacement.".into());
        }
        if path.exists() && is_plaintext(path)? {
            let source = Connection::open(path).map_err(|_| "Existing database cannot be read")?;
            source
                .execute_batch("PRAGMA temp_store=MEMORY;")
                .map_err(|_| "Could not prepare migration")?;
            let version: u32 = source
                .query_row("PRAGMA user_version", [], |r| r.get(0))
                .map_err(|_| "Invalid database")?;
            if version > 3 {
                return Err(
                    "Install the newer Maxxit version before migrating this database".into(),
                );
            }
            let mode: String = source
                .query_row("PRAGMA journal_mode=DELETE", [], |r| r.get(0))
                .map_err(|_| "Close other database readers before migrating")?;
            if mode != "delete" {
                return Err("Close other database readers before migrating".into());
            }
            source
                .execute(
                    "ATTACH DATABASE ?1 AS encrypted KEY ?2",
                    params![next.to_string_lossy(), key],
                )
                .map_err(|_| "Could not create encrypted replacement")?;
            source
                .query_row("SELECT sqlcipher_export('encrypted')", [], |_| Ok(()))
                .map_err(|_| "Encryption failed; the original database was retained")?;
            source
                .execute_batch(&format!(
                    "PRAGMA encrypted.user_version={version}; DETACH DATABASE encrypted;"
                ))
                .map_err(|_| "Could not finish encryption")?;
            drop(source);
            let verified = Self::keyed_connection(&next, key)?;
            let check: String = verified
                .query_row("PRAGMA integrity_check", [], |r| r.get(0))
                .map_err(|_| "Encrypted replacement is unreadable")?;
            if check != "ok" {
                return Err("Encrypted replacement failed integrity checks".into());
            }
            // Migrate and validate preferences before replacing any original bytes.
            drop(Self::initialize(verified)?);
            std::fs::rename(path, &backup).map_err(|_| "Could not retain original database")?;
            if std::fs::rename(&next, path).is_err() {
                let _ = std::fs::rename(&backup, path);
                return Err("Could not replace database. Original was retained.".into());
            }
            std::fs::remove_file(&backup).map_err(|_| {
                "Encryption succeeded, but the old plaintext backup still needs removal"
            })?;
        }
        Self::initialize(Self::keyed_connection(path, key)?)
    }
    fn keyed_connection(path: &Path, key: &str) -> Result<Connection, String> {
        if path.exists()
            && std::fs::symlink_metadata(path)
                .map_err(|_| "Database unavailable")?
                .file_type()
                .is_symlink()
        {
            return Err("Database symlinks are not supported".into());
        }
        let connection =
            Connection::open(path).map_err(|_| "Local database could not be opened")?;
        connection
            .pragma_update(None, "key", key)
            .map_err(|_| "Could not unlock database")?;
        let cipher: String = connection
            .query_row("PRAGMA cipher_version", [], |r| r.get(0))
            .map_err(|_| "Database encryption is unavailable")?;
        if cipher.is_empty() {
            return Err("Database encryption is unavailable".into());
        }
        connection
            .execute_batch("PRAGMA temp_store=MEMORY; PRAGMA cipher_memory_security=ON;")
            .map_err(|_| "Could not protect database memory")?;
        Ok(connection)
    }
    fn initialize(mut connection: Connection) -> Result<Self, String> {
        let version: u32 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|_| "Local database is unreadable")?;
        if version > 3 {
            return Err("This database belongs to a newer Maxxit version. Install that version before opening it.".into());
        }
        connection
            .execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|_| "Local database is unreadable")?;
        if version == 0 {
            let transaction = connection
                .transaction()
                .map_err(|_| "Storage migration could not begin")?;
            transaction.execute_batch("CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL); CREATE TABLE IF NOT EXISTS observations (id TEXT PRIMARY KEY, provider TEXT NOT NULL, account TEXT NOT NULL, observed_at TEXT NOT NULL, body TEXT NOT NULL); CREATE INDEX IF NOT EXISTS observation_time ON observations(provider,account,observed_at); CREATE TABLE IF NOT EXISTS projects (id TEXT PRIMARY KEY, body TEXT NOT NULL); PRAGMA user_version=1;").map_err(|e| e.to_string())?;
            transaction
                .commit()
                .map_err(|_| "Storage migration could not be committed")?;
        }
        if version < 2 {
            connection.execute_batch("BEGIN; CREATE TABLE runs (id TEXT PRIMARY KEY, body TEXT NOT NULL); CREATE TABLE outbox (id TEXT PRIMARY KEY, run_id TEXT NOT NULL, body TEXT NOT NULL, credential_hash TEXT NOT NULL, attempts INTEGER NOT NULL DEFAULT 0, due_at INTEGER NOT NULL, expires_at INTEGER NOT NULL); PRAGMA user_version=2; COMMIT;").map_err(|_| "Local workflow migration failed")?;
        }
        if version < 3 {
            connection.execute_batch("BEGIN; CREATE TABLE sync_records(kind TEXT NOT NULL,record_id TEXT NOT NULL,revision INTEGER NOT NULL,body TEXT,pending INTEGER NOT NULL DEFAULT 1,PRIMARY KEY(kind,record_id)); PRAGMA user_version=3; COMMIT;").map_err(|_| "Account sync migration failed")?;
        }
        // Queue writes in the same SQLite transaction as the source mutation.
        connection.execute_batch("CREATE TRIGGER IF NOT EXISTS sync_projects_insert AFTER INSERT ON projects BEGIN INSERT INTO sync_records(kind,record_id,revision,body,pending) VALUES('project',NEW.id,1,NEW.body,1) ON CONFLICT(kind,record_id) DO UPDATE SET revision=revision+1,body=excluded.body,pending=1; END;").map_err(|_| "Sync triggers could not be installed")?;
        connection.execute_batch("CREATE TRIGGER IF NOT EXISTS sync_projects_update AFTER UPDATE ON projects WHEN OLD.body <> NEW.body BEGIN INSERT INTO sync_records(kind,record_id,revision,body,pending) VALUES('project',NEW.id,1,NEW.body,1) ON CONFLICT(kind,record_id) DO UPDATE SET revision=revision+1,body=excluded.body,pending=1; END;").map_err(|_| "Sync triggers could not be installed")?;
        connection.execute_batch("CREATE TRIGGER IF NOT EXISTS sync_projects_delete AFTER DELETE ON projects BEGIN INSERT INTO sync_records(kind,record_id,revision,body,pending) VALUES('project',OLD.id,1,NULL,1) ON CONFLICT(kind,record_id) DO UPDATE SET revision=revision+1,body=excluded.body,pending=1; END;").map_err(|_| "Sync triggers could not be installed")?;
        connection.execute_batch("CREATE TRIGGER IF NOT EXISTS sync_runs_insert AFTER INSERT ON runs BEGIN INSERT INTO sync_records(kind,record_id,revision,body,pending) VALUES('run',NEW.id,1,NEW.body,1) ON CONFLICT(kind,record_id) DO UPDATE SET revision=revision+1,body=excluded.body,pending=1; END;").map_err(|_| "Sync triggers could not be installed")?;
        connection.execute_batch("CREATE TRIGGER IF NOT EXISTS sync_runs_update AFTER UPDATE ON runs WHEN OLD.body <> NEW.body BEGIN INSERT INTO sync_records(kind,record_id,revision,body,pending) VALUES('run',NEW.id,1,NEW.body,1) ON CONFLICT(kind,record_id) DO UPDATE SET revision=revision+1,body=excluded.body,pending=1; END;").map_err(|_| "Sync triggers could not be installed")?;
        connection.execute_batch("CREATE TRIGGER IF NOT EXISTS sync_runs_delete AFTER DELETE ON runs BEGIN INSERT INTO sync_records(kind,record_id,revision,body,pending) VALUES('run',OLD.id,1,NULL,1) ON CONFLICT(kind,record_id) DO UPDATE SET revision=revision+1,body=excluded.body,pending=1; END;").map_err(|_| "Sync triggers could not be installed")?;
        connection.execute_batch("CREATE TRIGGER IF NOT EXISTS sync_observations_insert AFTER INSERT ON observations BEGIN INSERT INTO sync_records(kind,record_id,revision,body,pending) VALUES('observation',NEW.id,1,NEW.body,1) ON CONFLICT(kind,record_id) DO UPDATE SET revision=revision+1,body=excluded.body,pending=1; END;").map_err(|_| "Sync triggers could not be installed")?;
        connection.execute_batch("CREATE TRIGGER IF NOT EXISTS sync_observations_update AFTER UPDATE ON observations WHEN OLD.body <> NEW.body BEGIN INSERT INTO sync_records(kind,record_id,revision,body,pending) VALUES('observation',NEW.id,1,NEW.body,1) ON CONFLICT(kind,record_id) DO UPDATE SET revision=revision+1,body=excluded.body,pending=1; END;").map_err(|_| "Sync triggers could not be installed")?;
        connection.execute_batch("CREATE TRIGGER IF NOT EXISTS sync_observations_delete AFTER DELETE ON observations BEGIN INSERT INTO sync_records(kind,record_id,revision,body,pending) VALUES('observation',OLD.id,1,NULL,1) ON CONFLICT(kind,record_id) DO UPDATE SET revision=revision+1,body=excluded.body,pending=1; END;").map_err(|_| "Sync triggers could not be installed")?;
        connection.execute_batch("CREATE TRIGGER IF NOT EXISTS sync_settings_insert AFTER INSERT ON settings WHEN NEW.key='preferences' OR NEW.key LIKE 'analytics:%' BEGIN INSERT INTO sync_records(kind,record_id,revision,body,pending) VALUES(CASE WHEN NEW.key='preferences' THEN 'preferences' ELSE 'analytics' END,CASE WHEN NEW.key='preferences' THEN 'shared' ELSE substr(NEW.key,11) END,1,NEW.value,1) ON CONFLICT(kind,record_id) DO UPDATE SET revision=revision+1,body=excluded.body,pending=1; END;").map_err(|_| "Sync settings could not be installed")?;
        connection.execute_batch("CREATE TRIGGER IF NOT EXISTS sync_settings_update AFTER UPDATE ON settings WHEN (NEW.key='preferences' OR NEW.key LIKE 'analytics:%') AND OLD.value<>NEW.value BEGIN INSERT INTO sync_records(kind,record_id,revision,body,pending) VALUES(CASE WHEN NEW.key='preferences' THEN 'preferences' ELSE 'analytics' END,CASE WHEN NEW.key='preferences' THEN 'shared' ELSE substr(NEW.key,11) END,1,NEW.value,1) ON CONFLICT(kind,record_id) DO UPDATE SET revision=revision+1,body=excluded.body,pending=1; END;").map_err(|_| "Sync settings could not be installed")?;
        if version < 3 {
            for (table, kind) in [
                ("projects", "project"),
                ("runs", "run"),
                ("observations", "observation"),
            ] {
                connection.execute(&format!("INSERT OR IGNORE INTO sync_records(kind,record_id,revision,body) SELECT ?1,id,1,body FROM {table}"), [kind]).map_err(|_| "Sync backfill failed")?;
            }
        }
        let store = Self { connection };
        store.settings()?;
        Ok(store)
    }
    pub fn settings(&self) -> Result<Settings, String> {
        match self.get("preferences")? {
            None => Ok(Settings::default()),
            Some(value) => serde_json::from_value(value).map_err(|_| "Stored preferences are invalid. Restore a verified backup instead of overwriting them.".into()),
        }
    }
    pub fn save_settings(&self, settings: &Settings) -> Result<(), String> {
        self.set(
            "preferences",
            &serde_json::to_value(settings).map_err(|e| e.to_string())?,
        )
    }
    pub fn get(&self, key: &str) -> Result<Option<Value>, String> {
        let raw: Option<String> = self
            .connection
            .query_row("SELECT value FROM settings WHERE key=?1", [key], |row| {
                row.get(0)
            })
            .optional()
            .map_err(|_| "Stored preferences could not be read")?;
        raw.map(|s| serde_json::from_str(&s).map_err(|_| "Stored preferences are corrupt".into()))
            .transpose()
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
                "DELETE FROM observations WHERE julianday(observed_at) < julianday('now','-90 days')",
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
    pub fn claim_legacy(&self, legacy: &Store, owner: &str) -> Result<(), String> {
        if legacy
            .get("claimedAccountId")?
            .is_some_and(|v| v != serde_json::json!(owner))
        {
            return Err("Earlier local data belongs to another Maxxit account".into());
        }
        // Claim first. If copying fails, this same account can retry safely.
        legacy.set("claimedAccountId", &serde_json::json!(owner))?;
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        for project in legacy.projects()? {
            self.connection
                .execute(
                    "INSERT OR IGNORE INTO projects(id,body) VALUES(?1,?2)",
                    params![project["id"].as_str(), project.to_string()],
                )
                .map_err(|e| e.to_string())?;
        }
        for observation in legacy.history()? {
            self.record(&observation)?;
        }
        for mut run in legacy.runs()? {
            // A historical result must never send a newly queued notification.
            run.delivery = "cancelled".into();
            self.connection
                .execute(
                    "INSERT OR IGNORE INTO runs(id,body) VALUES(?1,?2)",
                    params![
                        run.id,
                        serde_json::to_string(&run).map_err(|e| e.to_string())?
                    ],
                )
                .map_err(|e| e.to_string())?;
        }
        self.set("legacyImported", &serde_json::json!(true))?;
        transaction.commit().map_err(|e| e.to_string())
    }
    pub fn save_project(&self, project: &Value) -> Result<(), String> {
        let id = project
            .get("id")
            .and_then(Value::as_str)
            .ok_or("Missing project ID")?;
        self.connection.execute("INSERT INTO projects(id,body) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET body=excluded.body",params![id,project.to_string()]).map(|_|()).map_err(|e|e.to_string())
    }
    pub fn remove_project(&self, id: &str) -> Result<(), String> {
        let mut dismissed = self
            .get("dismissedProviderProjects")?
            .unwrap_or(serde_json::json!([]));
        if let Some(ids) = dismissed.as_array_mut() {
            if !ids.iter().any(|value| value == id) {
                ids.push(serde_json::json!(id));
            }
        }
        self.set("dismissedProviderProjects", &dismissed)?;
        for run in self.runs()? {
            if run.project_id == id {
                self.remove_run(&run.id)?;
            }
        }
        self.connection
            .execute("DELETE FROM projects WHERE id=?1", [id])
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

fn is_plaintext(path: &Path) -> Result<bool, String> {
    use std::io::Read;
    if std::fs::symlink_metadata(path)
        .map_err(|_| "Database unavailable")?
        .file_type()
        .is_symlink()
    {
        return Err("Database symlinks are not supported".into());
    }
    let mut header = [0u8; 16];
    let mut file = std::fs::File::open(path).map_err(|_| "Database unavailable")?;
    Ok(file.read_exact(&mut header).is_ok() && &header == b"SQLite format 3\0")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encrypted_migration_reopens_and_wrong_key_preserves_data() {
        let directory = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("fixture.sqlite");
        {
            let store = Store::open(&path).unwrap();
            store
                .save_project(&serde_json::json!({"id":"private", "name":"PRIVATE_MARKER"}))
                .unwrap();
        }
        let key = "a".repeat(64);
        let store = Store::open_encrypted(&path, &key).unwrap();
        assert_eq!(store.projects().unwrap()[0]["name"], "PRIVATE_MARKER");
        drop(store);
        for entry in std::fs::read_dir(&directory).unwrap() {
            let bytes = std::fs::read(entry.unwrap().path()).unwrap();
            assert!(!bytes.windows(14).any(|w| w == b"PRIVATE_MARKER"));
        }
        let before = std::fs::read(&path).unwrap();
        assert!(Store::open_encrypted(&path, &"b".repeat(64)).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert_eq!(
            Store::open_encrypted(&path, &key)
                .unwrap()
                .projects()
                .unwrap()
                .len(),
            1
        );
        std::fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn interrupted_migration_retains_all_files() {
        let path = std::env::temp_dir().join(format!("{}.sqlite", uuid::Uuid::new_v4()));
        let backup = path.with_extension("plaintext-backup");
        std::fs::write(&path, b"original").unwrap();
        std::fs::write(&backup, b"backup").unwrap();
        assert!(Store::open_encrypted(&path, &"a".repeat(64)).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"original");
        assert_eq!(std::fs::read(&backup).unwrap(), b"backup");
        std::fs::remove_file(path).unwrap();
        std::fs::remove_file(backup).unwrap();
    }
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
        connection.execute_batch("PRAGMA user_version=3;").unwrap();
        drop(connection);
        assert!(Store::open(&path).is_err());
        let connection = Connection::open(&path).unwrap();
        assert_eq!(
            connection
                .query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0))
                .unwrap(),
            3
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
