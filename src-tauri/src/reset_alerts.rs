use crate::{
    model::{Observation, UsageWindow},
    storage::Store,
};
use chrono::{DateTime, Timelike, Utc};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const MAX_AGE: i64 = 7200;
const RESET_TOLERANCE: i64 = 120;
const DROP: f64 = 20.0;
#[derive(Clone, Serialize, Deserialize)]
struct Reading {
    window: UsageWindow,
    observed_at: String,
}
#[derive(Serialize, Deserialize)]
struct Watch {
    baseline: Reading,
    candidate_at: Option<String>,
}
fn timestamp(value: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|t| t.timestamp())
}
fn identity(observation: &Observation, window: &UsageWindow) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}",
        observation.provider,
        observation
            .provider_account_id
            .as_deref()
            .unwrap_or(&observation.account_label),
        observation.source,
        observation.source_version.as_deref().unwrap_or("legacy"),
        window.bucket_id,
        window.window_id
    )
}
fn classify(
    watch: Option<Watch>,
    next: Reading,
    now: i64,
) -> (Watch, Vec<(String, Reading, String)>) {
    let fresh = Watch {
        baseline: next.clone(),
        candidate_at: None,
    };
    let Some(old) = watch else {
        return (fresh, vec![]);
    };
    let time = timestamp(&next.observed_at).unwrap_or(0);
    let previous = timestamp(&old.baseline.observed_at).unwrap_or(0);
    if time
        <= previous.max(
            old.candidate_at
                .as_deref()
                .and_then(timestamp)
                .unwrap_or(previous),
        )
    {
        return (old, vec![]);
    }
    let w = &next.window;
    let before = &old.baseline.window;
    let reset = w.resets_at.as_deref().and_then(timestamp).unwrap_or(0);
    let old_reset = before.resets_at.as_deref().and_then(timestamp).unwrap_or(0);
    let used = w.used_percent.unwrap_or(f64::NAN);
    let old_used = before.used_percent.unwrap_or(f64::NAN);
    if time > now + 60
        || now - time > MAX_AGE
        || time - previous > MAX_AGE
        || w.availability != "available"
        || before.availability != "available"
        || !(0.0..=100.0).contains(&used)
        || !(0.0..=100.0).contains(&old_used)
        || w.duration_minutes != before.duration_minutes
        || reset <= time
        || old_reset <= time + RESET_TOLERANCE
    {
        return (fresh, vec![]);
    }
    let mut changes = vec![];
    if (reset - old_reset).abs() > RESET_TOLERANCE && old.candidate_at.is_none() {
        changes.push((
            "changed".into(),
            old.baseline.clone(),
            next.observed_at.clone(),
        ));
    }
    if old_used - used >= DROP {
        if let Some(candidate) = old
            .candidate_at
            .filter(|t| timestamp(t).is_some_and(|t| time > t))
        {
            changes.push(("increased".into(), old.baseline, candidate));
            return (fresh, changes);
        }
        return (
            Watch {
                baseline: old.baseline,
                candidate_at: Some(next.observed_at),
            },
            changes,
        );
    }
    (fresh, changes)
}
impl Store {
    pub fn observe_reset_alerts(
        &self,
        observation: &Observation,
        now: DateTime<Utc>,
    ) -> Result<(), String> {
        let settings = self.settings()?;
        for window in &observation.windows {
            let scope = identity(observation, window);
            let key = format!("{:x}", Sha256::digest(scope.as_bytes()));
            let raw: Option<String> = self
                .connection
                .query_row(
                    "SELECT body FROM reset_watches WHERE id=?1",
                    [&key],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            let watch = raw.and_then(|s| serde_json::from_str::<Watch>(&s).ok());
            let reading = Reading {
                window: window.clone(),
                observed_at: observation.observed_at.clone(),
            };
            let (watch, changes) = classify(watch, reading, now.timestamp());
            self.connection.execute("INSERT INTO reset_watches(id,body,updated_at) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET body=excluded.body,updated_at=excluded.updated_at", params![key, { let mut value = serde_json::to_value(&watch).map_err(|e|e.to_string())?; value["provider"] = json!(observation.provider); value.to_string() }, observation.observed_at]).map_err(|e| e.to_string())?;
            for (kind, before, evidence) in changes {
                if kind == "changed" {
                    self.connection.execute("UPDATE reset_events SET native_state='cancelled' WHERE json_extract(body,'$.scope')=?1 AND json_extract(body,'$.kind')='scheduled' AND native_state='pending'", [&scope]).map_err(|e| e.to_string())?;
                }
                let signature = format!(
                    "{scope}|{kind}|{}|{evidence}",
                    before.window.resets_at.as_deref().unwrap_or("")
                );
                self.save_reset_event(observation, window, &kind, &signature, now, Some(&before))?;
            }
            let Some(reset) = window.resets_at.as_deref().and_then(timestamp) else {
                continue;
            };
            let observed = timestamp(&observation.observed_at).unwrap_or(0);
            let lead = if window.duration_minutes.is_some_and(|m| m <= 300) {
                settings.short_hours_before
            } else {
                settings.hours_before
            } * 3600;
            if window.availability == "available"
                && now.timestamp() - observed <= MAX_AGE
                && observed <= now.timestamp() + 60
                && reset > now.timestamp()
                && reset - now.timestamp() <= lead
                && window
                    .used_percent
                    .is_some_and(|used| 100.0 - used >= settings.min_remaining as f64)
                && window.duration_minutes.is_none_or(|m| lead < m * 60)
            {
                self.save_reset_event(
                    observation,
                    window,
                    "scheduled",
                    &format!("{scope}|scheduled|{reset}"),
                    now,
                    None,
                )?;
            }
        }
        self.connection
            .execute(
                "DELETE FROM reset_watches WHERE julianday(updated_at)<julianday('now','-90 days')",
                [],
            )
            .map_err(|e| e.to_string())?;
        self.connection.execute("DELETE FROM reset_events WHERE julianday(observed_at)<julianday('now','-90 days') OR id IN (SELECT id FROM reset_events ORDER BY observed_at DESC LIMIT -1 OFFSET 500)", []).map_err(|e| e.to_string())?;
        Ok(())
    }
    fn save_reset_event(
        &self,
        observation: &Observation,
        window: &UsageWindow,
        kind: &str,
        signature: &str,
        now: DateTime<Utc>,
        before: Option<&Reading>,
    ) -> Result<(), String> {
        let id = format!("{:x}", Sha256::digest(signature.as_bytes()));
        let name = if observation.provider == "codex" {
            "Codex"
        } else {
            "Claude"
        };
        let title = match kind {
            "increased" => format!("{name} allowance increased earlier than expected"),
            "changed" => format!("{name} reset time changed"),
            _ => format!("{name} allowance resets soon"),
        };
        let reset = window
            .resets_at
            .as_deref()
            .and_then(timestamp)
            .unwrap_or(now.timestamp());
        let expiry = if kind == "scheduled" {
            reset
        } else {
            reset.min(now.timestamp() + MAX_AGE)
        };
        let body = if kind == "increased" {
            "Fresh readings show more allowance. Check other limits before resuming work."
        } else {
            "Review the reported deadline and choose a useful next step in Maxxit."
        };
        let event = json!({"before":before.map(|reading| json!({"observedAt":reading.observed_at,"window":reading.window})),"schemaVersion":1,"confidence":if kind == "increased" {"inferred"} else {"provider-reported"},"id":id,"scope":identity(observation, window),"provider":observation.provider,"accountLabel":observation.account_label,"kind":kind,"title":title,"body":body,"observedAt":observation.observed_at,"effectiveAt":window.resets_at,"expiresAt":DateTime::from_timestamp(expiry,0).map(|t|t.to_rfc3339()),"windowLabel":window.label,"bucketId":window.bucket_id,"windowId":window.window_id,"usedPercent":window.used_percent});
        self.connection
            .execute(
                "INSERT OR IGNORE INTO reset_events(id,body,observed_at) VALUES(?1,?2,?3)",
                params![id, event.to_string(), observation.observed_at],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    pub fn reset_provider_alerts(&self, provider: &str) -> Result<(), String> {
        self.connection.execute("DELETE FROM reset_watches WHERE json_extract(body,'$.provider')=?1 OR json_extract(body,'$.provider') IS NULL",[provider]).map_err(|e|e.to_string())?;
        self.connection.execute("UPDATE reset_events SET native_state='cancelled' WHERE json_extract(body,'$.provider')=?1 AND native_state='pending'",[provider]).map_err(|e|e.to_string())?;
        if provider == "claude" {
            self.set(
                "claudeResetEpoch",
                &json!(uuid::Uuid::new_v4().simple().to_string()[..16]),
            )?;
            self.set("claudeResetAfter", &json!(Utc::now().to_rfc3339()))?;
        }
        Ok(())
    }
    pub fn import_cloud_reset_events(
        &self,
        data: &Value,
        now: DateTime<Utc>,
    ) -> Result<(), String> {
        let events = data["resetEvents"].as_array().cloned().unwrap_or_default();
        for event in events {
            let Some(kind) = event["kind"]
                .as_str()
                .filter(|k| ["announcements", "offers", "offer-expiring"].contains(k))
            else {
                continue;
            };
            let Some(id) = event["id"].as_str() else {
                continue;
            };
            if event["state"] != "notified" {
                self.connection.execute("UPDATE reset_events SET native_state='cancelled' WHERE id=?1 AND native_state='pending'",[id]).map_err(|e|e.to_string())?;
                continue;
            }
            let details = &event["data"];
            let provider = event["provider"].as_str().unwrap_or("provider");
            let title = if kind == "announcements" {
                format!("{provider} reset announcement")
            } else {
                format!("{provider} reset offer")
            };
            let body = json!({"id":id,"provider":provider,"kind":kind,"cloud":true,"title":title,"body":"Open Maxxit to review the evidence and choose a useful next step.","observedAt":details["observedAt"],"effectiveAt":event["effectiveAt"],"expiresAt":event["expiresAt"]});
            self.connection
                .execute(
                    "INSERT OR IGNORE INTO reset_events(id,body,observed_at) VALUES(?1,?2,?3)",
                    params![id, body.to_string(), now.to_rfc3339()],
                )
                .map_err(|e| e.to_string())?;
        }
        self.set("cloudResetCheckedAt", &json!(now.to_rfc3339()))?;
        Ok(())
    }
    pub fn reset_events(&self) -> Result<Vec<Value>, String> {
        let mut query = self
            .connection
            .prepare(
                "SELECT body,native_state FROM reset_events ORDER BY observed_at DESC LIMIT 100",
            )
            .map_err(|e| e.to_string())?;
        let rows = query
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?;
        Ok(rows
            .filter_map(|row| {
                row.ok().and_then(|(s, state)| {
                    let mut value: Value = serde_json::from_str(&s).ok()?;
                    value["nativeState"] = json!(state);
                    Some(value)
                })
            })
            .collect())
    }
    pub fn native_reset_notices(&self, now: DateTime<Utc>) -> Result<Vec<Value>, String> {
        let settings = self.settings()?;
        if !settings.reset_notifications {
            return Ok(vec![]);
        }
        let timezone: chrono_tz::Tz = settings
            .timezone
            .parse()
            .map_err(|_| "Unsupported notification timezone")?;
        let local = now.with_timezone(&timezone);
        let hour = local.hour() as i64;
        let quiet = settings.quiet_start != settings.quiet_end
            && if settings.quiet_start < settings.quiet_end {
                hour >= settings.quiet_start && hour < settings.quiet_end
            } else {
                hour >= settings.quiet_start || hour < settings.quiet_end
            };
        if quiet {
            return Ok(vec![]);
        }
        let day = local.format("%Y-%m-%d").to_string();
        let budget = self
            .get("nativeResetBudget")?
            .unwrap_or(json!({"day":day,"count":0}));
        let count = if budget["day"] == day {
            budget["count"].as_i64().unwrap_or(0)
        } else {
            0
        };
        if count >= settings.daily_limit {
            return Ok(vec![]);
        }
        let mut query = self.connection.prepare("SELECT id,body FROM reset_events WHERE native_state='pending' ORDER BY observed_at DESC LIMIT 100").map_err(|e|e.to_string())?;
        let rows = query
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (id, body) = row.map_err(|e| e.to_string())?;
            let event: Value =
                serde_json::from_str(&body).map_err(|_| "Invalid stored reset event")?;
            if event["expiresAt"]
                .as_str()
                .and_then(timestamp)
                .is_none_or(|t| t <= now.timestamp())
            {
                self.connection
                    .execute(
                        "UPDATE reset_events SET native_state='expired' WHERE id=?1",
                        [&id],
                    )
                    .map_err(|e| e.to_string())?;
                continue;
            }
            let enabled = match event["kind"].as_str() {
                Some("scheduled") => settings.reset_alerts.scheduled,
                Some("changed") => settings.reset_alerts.changed,
                Some("increased") => settings.reset_alerts.increased,
                Some("announcements") => settings.reset_alerts.announcements,
                Some("offers") | Some("offer-expiring") => settings.reset_alerts.offers,
                _ => false,
            };
            if !enabled
                || !settings
                    .reset_alerts
                    .providers
                    .iter()
                    .any(|p| event["provider"] == *p)
            {
                continue;
            }
            if (event["provider"] == "codex" && !settings.codex_enabled)
                || (event["provider"] == "claude" && !settings.claude_enabled)
            {
                continue;
            }
            let current = self.history()?.into_iter().find(|o| {
                o.provider == event["provider"] && o.account_label == event["accountLabel"]
            });
            let remote_valid = event["cloud"] == true
                && self
                    .get("cloudResetCheckedAt")?
                    .and_then(|v| v.as_str().and_then(timestamp))
                    .is_some_and(|t| now.timestamp() - t < 60);
            if !remote_valid
                && !current.is_some_and(|o| {
                    timestamp(&o.observed_at).is_some_and(|t| now.timestamp() - t <= MAX_AGE)
                        && o.windows.iter().any(|w| {
                            identity(&o, w) == event["scope"]
                                && w.bucket_id == event["bucketId"]
                                && w.window_id == event["windowId"]
                                && w.resets_at.as_deref() == event["effectiveAt"].as_str()
                                && w.availability == "available"
                                && (event["kind"] != "scheduled"
                                    || w.used_percent.is_some_and(|used| {
                                        100.0 - used >= settings.min_remaining as f64
                                    }))
                        })
                })
            {
                continue;
            }
            self.connection.execute("UPDATE reset_events SET native_state='claimed' WHERE id=?1 AND native_state='pending'",[&id]).map_err(|e|e.to_string())?;
            self.set("nativeResetBudget", &json!({"day":day,"count":count+1}))?;
            return Ok(vec![event]);
        }
        Ok(vec![])
    }
    pub fn finish_native_reset(&self, id: &str, shown: bool) -> Result<(), String> {
        self.connection
            .execute(
                "UPDATE reset_events SET native_state=?2 WHERE id=?1",
                params![id, if shown { "submitted" } else { "unavailable" }],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn reading(time: i64, used: f64, reset: i64) -> Reading {
        Reading {
            observed_at: DateTime::from_timestamp(time, 0).unwrap().to_rfc3339(),
            window: UsageWindow {
                bucket_id: "codex".into(),
                window_id: "weekly".into(),
                label: None,
                used_percent: Some(used),
                duration_minutes: Some(10080),
                resets_at: Some(DateTime::from_timestamp(reset, 0).unwrap().to_rfc3339()),
                availability: "available".into(),
            },
        }
    }
    #[test]
    fn early_increase_needs_two_distinct_fresh_readings() {
        let (first, _) = classify(None, reading(1000, 80.0, 10000), 1000);
        let (candidate, changes) = classify(Some(first), reading(1100, 5.0, 10000), 1100);
        assert!(changes.is_empty());
        let (same, changes) = classify(Some(candidate), reading(1100, 5.0, 10000), 1100);
        assert!(changes.is_empty());
        let (_, changes) = classify(Some(same), reading(1200, 7.0, 10000), 1200);
        assert_eq!(changes[0].0, "increased");
    }
    #[test]
    fn normal_rollover_and_stale_or_changed_scope_do_not_claim_refill() {
        let (first, _) = classify(None, reading(1000, 90.0, 1100), 1000);
        let (_, changes) = classify(Some(first), reading(1200, 0.0, 2000), 1200);
        assert!(changes.is_empty());
        let (first, _) = classify(None, reading(1000, 90.0, 10000), 1000);
        let (_, changes) = classify(Some(first), reading(9000, 0.0, 10000), 9000);
        assert!(changes.is_empty());
    }
    #[test]
    fn durable_ledger_preserves_deduplication_and_native_budget_after_restart() {
        let directory =
            std::env::temp_dir().join(format!("maxxit-reset-fixture-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("synthetic.sqlite");
        let now = Utc::now();
        let store = Store::open(&path).unwrap();
        let settings = crate::model::Settings {
            codex_enabled: true,
            reset_notifications: true,
            quiet_start: 0,
            quiet_end: 0,
            ..Default::default()
        };
        store.save_settings(&settings).unwrap();
        let observation = Observation {
            provider: "codex".into(),
            account_label: "Fixture".into(),
            provider_account_id: Some("a".repeat(64)),
            source_version: Some("fixture-v1".into()),
            source: "codex-local".into(),
            observed_at: now.to_rfc3339(),
            windows: vec![reading(now.timestamp(), 10.0, now.timestamp() + 1800).window],
        };
        store.record(&observation).unwrap();
        assert_eq!(store.reset_events().unwrap().len(), 1);
        let notices = store.native_reset_notices(now).unwrap();
        assert_eq!(notices.len(), 1);
        store
            .finish_native_reset(notices[0]["id"].as_str().unwrap(), false)
            .unwrap();
        drop(store);
        let reopened = Store::open(&path).unwrap();
        reopened.record(&observation).unwrap();
        assert_eq!(reopened.reset_events().unwrap().len(), 1);
        assert!(reopened.native_reset_notices(now).unwrap().is_empty());
        assert_eq!(
            reopened.get("nativeResetBudget").unwrap().unwrap()["count"],
            1
        );
        drop(reopened);
        std::fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn cloud_withdrawal_cancels_pending_native_offer_without_usage_readings() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let now = Utc::now();
        let settings = crate::model::Settings {
            claude_enabled: true,
            reset_notifications: true,
            quiet_start: 0,
            quiet_end: 0,
            reset_alerts: crate::model::ResetPreferences {
                offers: true,
                ..Default::default()
            },
            ..Default::default()
        };
        store.save_settings(&settings).unwrap();
        let event = json!({"id":"synthetic-offer","kind":"offers","provider":"claude","state":"notified","expiresAt":(now+chrono::Duration::hours(1)).to_rfc3339(),"data":{"observedAt":now.to_rfc3339()}});
        store
            .import_cloud_reset_events(&json!({"resetEvents":[event.clone()]}), now)
            .unwrap();
        let mut cancelled = event;
        cancelled["state"] = json!("cancelled");
        store
            .import_cloud_reset_events(&json!({"resetEvents":[cancelled]}), now)
            .unwrap();
        assert!(store.native_reset_notices(now).unwrap().is_empty());
    }
}
