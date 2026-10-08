use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    pub bucket_id: String,
    pub window_id: String,
    pub label: Option<String>,
    pub used_percent: Option<f64>,
    pub duration_minutes: Option<i64>,
    pub resets_at: Option<String>,
    pub availability: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Observation {
    pub provider: String,
    pub account_label: String,
    pub observed_at: String,
    pub source: String,
    pub windows: Vec<UsageWindow>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderView {
    pub provider: String,
    pub connected: bool,
    pub account_label: String,
    pub status: String,
    pub error: Option<String>,
    pub observation: Option<Observation>,
    pub daily_tokens: Vec<Value>,
    pub summary: Value,
}

impl ProviderView {
    pub fn empty(provider: &str) -> Self {
        Self {
            provider: provider.into(),
            connected: false,
            account_label: "Local account".into(),
            status: "disconnected".into(),
            error: None,
            observation: None,
            daily_tokens: vec![],
            summary: Value::Null,
        }
    }
}

pub fn tray_allowance(
    provider: &str,
    observation: Option<&Observation>,
    now: chrono::DateTime<Utc>,
) -> (String, String) {
    let unavailable = (
        "M —".into(),
        format!("Maxxit · {provider} · allowance unavailable"),
    );
    let Some(observation) = observation else {
        return unavailable;
    };
    let Some(observed_at) = chrono::DateTime::parse_from_rfc3339(&observation.observed_at).ok()
    else {
        return unavailable;
    };
    let Some(window) = observation.windows.first() else {
        return unavailable;
    };
    let Some(used) = window
        .used_percent
        .filter(|used| used.is_finite() && (0.0..=100.0).contains(used))
    else {
        return unavailable;
    };
    let reset = window
        .resets_at
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok());
    if window.availability != "available" || reset.is_none_or(|reset| reset <= now) {
        return unavailable;
    }
    let stale = now.signed_duration_since(observed_at).num_seconds() >= 7200;
    let title = format!(
        "{} {:.0}%{}",
        if provider == "codex" { "C" } else { "A" },
        100.0 - used,
        if stale { "*" } else { "" }
    );
    let tooltip = if stale {
        format!(
            "Maxxit · {provider} · {:.0}% remaining at last reading · observed {} · stale data",
            100.0 - used,
            observation.observed_at
        )
    } else {
        format!("Maxxit · {provider} · remaining allowance")
    };
    (title, tooltip)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub theme: String,
    pub background: bool,
    pub tray_provider: String,
    pub cloud_sync: bool,
    pub account_sync: bool,
    pub project_sync: bool,
    pub workflow_sync: bool,
    pub api_origin: String,
    pub claude_enabled: bool,
    pub codex_enabled: bool,
    pub claude_account: String,
    pub codex_path: Option<String>,
    pub timezone: String,
    pub hours_before: i64,
    pub short_hours_before: i64,
    pub min_remaining: i64,
    pub quiet_start: i64,
    pub quiet_end: i64,
    pub daily_limit: i64,
    pub email: bool,
    pub ai_consent: bool,
    pub generation_mode: String,
    pub local_ai_consent: bool,
    pub result_events: bool,
    pub local_notifications: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            background: true,
            tray_provider: "codex".into(),
            cloud_sync: false,
            account_sync: false,
            project_sync: false,
            workflow_sync: false,
            api_origin: "https://maxxit.app".into(),
            claude_enabled: false,
            codex_enabled: false,
            claude_account: "Local Claude Code".into(),
            codex_path: None,
            timezone: "Europe/Berlin".into(),
            hours_before: 24,
            short_hours_before: 1,
            min_remaining: 30,
            quiet_start: 22,
            quiet_end: 8,
            daily_limit: 1,
            email: false,
            ai_consent: false,
            generation_mode: "local".into(),
            local_ai_consent: false,
            result_events: false,
            local_notifications: false,
        }
    }
}

pub fn normalize_window(bucket: &str, key: &str, value: &Value, claude: bool) -> UsageWindow {
    normalize_window_at(bucket, key, value, claude, Utc::now())
}

pub fn normalize_window_at(
    bucket: &str,
    key: &str,
    value: &Value,
    claude: bool,
    now: chrono::DateTime<Utc>,
) -> UsageWindow {
    let used = value
        .get(if claude {
            "used_percentage"
        } else {
            "usedPercent"
        })
        .and_then(Value::as_f64)
        .filter(|n| n.is_finite() && *n >= 0.0 && *n <= 100.0);
    let reset = value
        .get(if claude { "resets_at" } else { "resetsAt" })
        .and_then(Value::as_i64)
        .and_then(|n| chrono::DateTime::from_timestamp(n, 0));
    let duration = if claude {
        Some(if key == "five_hour" { 300 } else { 10080 })
    } else {
        value
            .get("windowDurationMins")
            .and_then(Value::as_i64)
            .filter(|n| *n > 0)
    };
    let availability = if reset.is_some_and(|r| r <= now) {
        "expired"
    } else if used.is_some() && reset.is_some() {
        "available"
    } else {
        "missing"
    };
    UsageWindow {
        bucket_id: bucket.into(),
        window_id: key.into(),
        label: Some(
            match duration {
                Some(300) => "Five-hour allowance",
                Some(10080) => "Weekly allowance",
                _ if key == "primary" => "Primary allowance",
                _ => "Secondary allowance",
            }
            .into(),
        ),
        used_percent: used,
        duration_minutes: duration,
        resets_at: reset.map(|r| r.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
        availability: availability.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tray_retains_stale_readings_but_expires_elapsed_resets() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-10-07T19:47:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let mut observation = Observation {
            provider: "codex".into(),
            account_label: "synthetic".into(),
            observed_at: (now - chrono::Duration::minutes(316)).to_rfc3339(),
            source: "codex-local".into(),
            windows: vec![normalize_window_at(
                "codex",
                "primary",
                &serde_json::json!({"usedPercent":7,"resetsAt":now.timestamp()+86400,"windowDurationMins":10080}),
                false,
                now,
            )],
        };
        let (title, tooltip) = tray_allowance("codex", Some(&observation), now);
        assert_eq!(title, "C 93%*");
        assert!(tooltip.contains("93% remaining at last reading"));
        assert!(tooltip.contains(&observation.observed_at));
        observation.observed_at = (now - chrono::Duration::minutes(119)).to_rfc3339();
        assert_eq!(tray_allowance("codex", Some(&observation), now).0, "C 93%");
        observation.observed_at = (now - chrono::Duration::minutes(120)).to_rfc3339();
        assert_eq!(tray_allowance("codex", Some(&observation), now).0, "C 93%*");
        observation.windows[0].resets_at = Some(now.to_rfc3339());
        assert_eq!(tray_allowance("codex", Some(&observation), now).0, "M —");
        observation.windows[0].resets_at = Some((now + chrono::Duration::days(1)).to_rfc3339());
        observation.windows[0].used_percent = None;
        assert_eq!(tray_allowance("codex", Some(&observation), now).0, "M —");
        assert_eq!(tray_allowance("codex", None, now).0, "M —");
    }

    #[test]
    fn missing_quota_stays_missing() {
        let w = normalize_window("subscription", "five_hour", &serde_json::json!({}), true);
        assert_eq!(w.used_percent, None);
        assert_eq!(w.availability, "missing");
    }
    #[test]
    fn elapsed_reset_expires() {
        let w = normalize_window(
            "codex",
            "primary",
            &serde_json::json!({"usedPercent":42,"resetsAt":1,"windowDurationMins":300}),
            false,
        );
        assert_eq!(w.availability, "expired");
        assert_eq!(w.used_percent, Some(42.0));
    }
    #[test]
    fn invalid_percent_is_unavailable() {
        let w = normalize_window(
            "codex",
            "primary",
            &serde_json::json!({"usedPercent":120,"resetsAt":4102444800i64}),
            false,
        );
        assert_eq!(w.availability, "missing");
    }
    #[test]
    fn reset_expiry_uses_the_injected_utc_clock() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-10-25T01:30:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let value = serde_json::json!({"usedPercent":30,"resetsAt":now.timestamp()+60,"windowDurationMins":300});
        assert_eq!(
            normalize_window_at("codex", "primary", &value, false, now).availability,
            "available"
        );
        assert_eq!(
            normalize_window_at(
                "codex",
                "primary",
                &value,
                false,
                now + chrono::Duration::minutes(2)
            )
            .availability,
            "expired"
        );
    }
}
