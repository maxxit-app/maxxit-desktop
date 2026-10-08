use crate::{model::Settings, storage::Store};
use rusqlite::params;
use serde_json::{json, Value};

pub fn allowed(settings: &Settings, kind: &str) -> bool {
    match kind {
        "observation" | "analytics" => settings.cloud_sync,
        "project" => settings.project_sync,
        "run" => settings.workflow_sync,
        "preferences" => settings.account_sync,
        _ => false,
    }
}
pub fn shared_preferences(settings: &Settings) -> Value {
    json!({"theme":settings.theme,"generationMode":settings.generation_mode,"timezone":settings.timezone,
        "hoursBefore":settings.hours_before,"shortHoursBefore":settings.short_hours_before,"minRemaining":settings.min_remaining,
        "quietStart":settings.quiet_start,"quietEnd":settings.quiet_end,"dailyLimit":settings.daily_limit,"resetAlerts":settings.reset_alerts})
}
impl Store {
    pub fn sync_batch(&self, settings: &Settings) -> Result<Vec<Value>, String> {
        let mut query = self.connection.prepare("SELECT kind,record_id,revision,body FROM sync_records WHERE pending=1 ORDER BY revision,kind,record_id").map_err(|e|e.to_string())?;
        let rows = query
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut operations = vec![];
        let mut bytes = 0;
        for row in rows {
            let (kind, id, revision, body) = row.map_err(|e| e.to_string())?;
            if !allowed(settings, &kind) {
                continue;
            }
            let body = if kind == "preferences" {
                shared_preferences(settings)
            } else {
                body.map(|s| serde_json::from_str(&s))
                    .transpose()
                    .map_err(|_| "Invalid queued record")?
                    .unwrap_or(Value::Null)
            };
            let operation = json!({"kind":kind,"recordId":id,"revision":revision,"body":body});
            let size = operation.to_string().len();
            if !operations.is_empty() && (bytes + size > 400_000 || operations.len() >= 50) {
                break;
            }
            if size > 400_000 {
                return Err("A sync record exceeds its limit".into());
            }
            bytes += size;
            operations.push(operation);
        }
        Ok(operations)
    }
    pub fn acknowledge_sync(&self, operations: &[Value], reply: &Value) -> Result<(), String> {
        if reply["schemaVersion"] != 1 {
            return Err("Invalid sync acknowledgement".into());
        }
        let acknowledgements = reply["acknowledgements"]
            .as_array()
            .ok_or("Missing sync acknowledgements")?;
        if operations.len() != acknowledgements.len()
            || operations.iter().any(|op| {
                !acknowledgements.iter().any(|ack| {
                    ack["kind"] == op["kind"]
                        && ack["recordId"] == op["recordId"]
                        && ack["revision"] == op["revision"]
                })
            })
        {
            return Err("Sync acknowledgements did not match".into());
        }
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        for op in operations {
            self.connection.execute("UPDATE sync_records SET pending=0 WHERE kind=?1 AND record_id=?2 AND revision=?3",params![op["kind"].as_str(),op["recordId"].as_str(),op["revision"].as_i64()]).map_err(|e|e.to_string())?;
        }
        self.set("lastSyncAt", &json!(chrono::Utc::now().to_rfc3339()))?;
        transaction.commit().map_err(|e| e.to_string())
    }
    pub fn requeue_sync(&self) -> Result<(), String> {
        self.connection
            .execute("UPDATE sync_records SET pending=1,revision=revision+1", [])
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    pub fn sync_status(&self) -> Result<Value, String> {
        let settings = self.settings()?;
        let mut query = self
            .connection
            .prepare("SELECT kind,COUNT(*) FROM sync_records WHERE pending=1 GROUP BY kind")
            .map_err(|e| e.to_string())?;
        let rows = query
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, usize>(1)?))
            })
            .map_err(|e| e.to_string())?;
        let mut pending = 0;
        for row in rows {
            let (kind, count) = row.map_err(|e| e.to_string())?;
            if allowed(&settings, &kind) {
                pending += count;
            }
        }
        Ok(
            json!({"pending":pending,"lastSyncAt":self.get("lastSyncAt")?,"error":self.get("syncError")?}),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    #[test]
    fn atomic_mutations_survive_missing_ack_and_tombstones_survive_replay() {
        let store = Store::open(Path::new(":memory:")).unwrap();
        let settings = Settings {
            project_sync: true,
            ..Default::default()
        };
        store.save_settings(&settings).unwrap();
        store.save_project(&json!({"id":"project","name":"A","description":"Private","createdAt":"2026-10-08T00:00:00Z"})).unwrap();
        let first = store.sync_batch(&settings).unwrap();
        assert_eq!(first.len(), 1);
        assert!(store
            .acknowledge_sync(&first, &json!({"schemaVersion":1,"acknowledgements":[]}))
            .is_err());
        store.remove_project("project").unwrap();
        let deleted = store.sync_batch(&settings).unwrap();
        assert!(deleted[0]["body"].is_null());
        assert!(deleted[0]["revision"].as_i64() > first[0]["revision"].as_i64());
        store
            .acknowledge_sync(&first, &json!({"schemaVersion":1,"acknowledgements":first}))
            .unwrap();
        assert_eq!(store.sync_batch(&settings).unwrap(), deleted);
        store
            .acknowledge_sync(
                &deleted,
                &json!({"schemaVersion":1,"acknowledgements":deleted}),
            )
            .unwrap();
        assert!(store.sync_batch(&settings).unwrap().is_empty());
    }
    #[test]
    fn sync_scope_is_independent_of_ai_and_never_sends_native_paths() {
        let store = Store::open(Path::new(":memory:")).unwrap();
        let settings = Settings {
            account_sync: true,
            codex_path: Some("PRIVATE_PATH".into()),
            ..Default::default()
        };
        store.save_settings(&settings).unwrap();
        let batch = store.sync_batch(&settings).unwrap();
        assert!(!serde_json::to_string(&batch)
            .unwrap()
            .contains("PRIVATE_PATH"));
        assert_eq!(batch[0]["body"]["generationMode"], "local");
        assert!(store.sync_batch(&Settings::default()).unwrap().is_empty());
    }
}
