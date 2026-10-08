use crate::model::{normalize_window, Observation, ProviderView};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

fn session_files(root: &Path, depth: usize, files: &mut Vec<PathBuf>, examined: &mut usize) {
    if depth > 4 || *examined > 10000 {
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        *examined += 1;
        if *examined > 10000 {
            break;
        }
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_symlink() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if kind.is_dir() && name.chars().all(|c| c.is_ascii_digit()) {
            session_files(&entry.path(), depth + 1, files, examined);
        }
        if kind.is_file() && name.starts_with("rollout-") && name.ends_with(".jsonl") {
            files.push(entry.path());
        }
    }
}

fn records(path: &Path, tail: bool) -> Vec<Value> {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return vec![];
    };
    if !meta.is_file() || meta.file_type().is_symlink() {
        return vec![];
    }
    let Ok(mut file) = fs::File::open(path) else {
        return vec![];
    };
    let start = if tail {
        meta.len().saturating_sub(512000)
    } else {
        0
    };
    if file.seek(SeekFrom::Start(start)).is_err() {
        return vec![];
    }
    let mut bytes = Vec::new();
    if file
        .take(if tail { 512000 } else { 256000 })
        .read_to_end(&mut bytes)
        .is_err()
    {
        return vec![];
    }
    String::from_utf8_lossy(&bytes)
        .lines()
        .skip(if start > 0 { 1 } else { 0 })
        .filter_map(|s| serde_json::from_str(s).ok())
        .collect()
}

pub fn codex_local() -> ProviderView {
    let view = ProviderView::empty("codex");
    let home = std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".codex")));
    let Some(root) = home.map(|h| h.join("sessions")) else {
        return view;
    };
    codex_from_sessions(&root, Utc::now())
}

fn codex_from_sessions(root: &Path, now: DateTime<Utc>) -> ProviderView {
    let mut view = ProviderView::empty("codex");
    let mut files = vec![];
    session_files(root, 0, &mut files, &mut 0);
    files.sort_by_key(|p| std::cmp::Reverse(fs::metadata(p).and_then(|m| m.modified()).ok()));
    files.truncate(200);
    let mut selected_account: Option<Option<String>> = None;
    let mut latest: Option<Observation> = None;
    let mut daily = BTreeMap::<String, u64>::new();
    for path in files {
        let header = records(&path, false);
        let Some(meta) = header
            .iter()
            .find(|r| r.get("type").and_then(Value::as_str) == Some("session_meta"))
            .and_then(|r| r.get("payload"))
        else {
            continue;
        };
        let account = meta
            .get("creator_account_id")
            .and_then(Value::as_str)
            .map(|id| format!("{:x}", Sha256::digest(id.as_bytes())));
        if selected_account.is_none() {
            selected_account = Some(account.clone());
        }
        if selected_account.as_ref() != Some(&account) {
            continue;
        }
        let label = account
            .as_ref()
            .map(|a| format!("Codex {}", &a[..8]))
            .unwrap_or_else(|| "Local Codex, account unverified".into());
        let tail = records(&path, true);
        let mut last_total = 0u64;
        for record in tail {
            let Some(payload) = record.get("payload") else {
                continue;
            };
            if record.get("type").and_then(Value::as_str) != Some("event_msg")
                || payload.get("type").and_then(Value::as_str) != Some("token_count")
            {
                continue;
            }
            let Some(time) = record
                .get("timestamp")
                .and_then(Value::as_str)
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            else {
                continue;
            };
            if time.timestamp() > now.timestamp() + 60 {
                continue;
            }
            if let Some(total) = payload
                .pointer("/info/total_token_usage/total_tokens")
                .and_then(Value::as_u64)
            {
                if total >= last_total {
                    // A bounded tail cannot establish the date of tokens accumulated before it.
                    if last_total > 0 {
                        *daily
                            .entry(time.format("%Y-%m-%d").to_string())
                            .or_default() += total - last_total;
                    }
                }
                last_total = total;
            }
            let Some(raw) = payload.get("rate_limits") else {
                continue;
            };
            let bucket = raw
                .get("limit_id")
                .and_then(Value::as_str)
                .unwrap_or("codex");
            let mut windows = vec![];
            for key in ["primary", "secondary"] {
                let Some(w) = raw.get(key).filter(|w| !w.is_null()) else {
                    continue;
                };
                let normalized = json!({"usedPercent":w.get("used_percent"),"resetsAt":w.get("resets_at"),"windowDurationMins":w.get("window_minutes")});
                windows.push(normalize_window(bucket, key, &normalized, false));
            }
            if windows.is_empty() {
                continue;
            }
            let observed_at = time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
            if latest.as_ref().is_none_or(|o| o.observed_at < observed_at) {
                latest = Some(Observation {
                    provider: "codex".into(),
                    account_label: label.clone(),
                    provider_account_id: account.clone(),
                    source_version: Some("codex-rollout-v1".into()),
                    observed_at,
                    source: "codex-local".into(),
                    windows,
                });
            }
        }
    }
    view.connected = latest.is_some();
    view.status = if latest.is_some() {
        "connected"
    } else {
        "waiting"
    }
    .into();
    view.account_label = latest
        .as_ref()
        .map(|o| o.account_label.clone())
        .unwrap_or_else(|| "Local Codex".into());
    view.observation = latest;
    view.daily_tokens = daily
        .into_iter()
        .map(|(start_date, tokens)| json!({"startDate":start_date,"tokens":tokens}))
        .collect();
    view.summary = json!({"scope":"Observed local session increments","historyComplete":false});
    view
}

pub fn claude_local(data_dir: &Path, account: &str) -> ProviderView {
    let mut view = ProviderView::empty("claude");
    view.account_label = account.into();
    view.status = "waiting".into();
    let path = data_dir.join("claude-usage.json");
    if let Ok(text) = fs::read_to_string(path) {
        if let Ok(raw) = serde_json::from_str::<Value>(&text) {
            let mut windows = vec![];
            for key in ["five_hour", "seven_day"] {
                windows.push(normalize_window(
                    "subscription",
                    key,
                    &raw["rate_limits"][key],
                    true,
                ));
            }
            let valid_time = raw
                .get("observedAt")
                .and_then(Value::as_str)
                .filter(|s| DateTime::parse_from_rfc3339(s).is_ok());
            if let Some(observed_at) = valid_time {
                view.connected = true;
                view.status = "connected".into();
                view.observation = Some(Observation {
                    provider: "claude".into(),
                    account_label: account.into(),
                    provider_account_id: None,
                    source_version: Some("claude-statusline-v1".into()),
                    observed_at: observed_at.into(),
                    source: "claude-statusline".into(),
                    windows,
                });
            }
        }
    }
    view.summary = json!({"scope":"Observed on this Mac","historyComplete":false});
    view
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn session_file_walk_excludes_symlinks_and_other_files() {
        let root = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("auth.json"), "secret").unwrap();
        fs::write(root.join("rollout-safe.jsonl"), "{}").unwrap();
        let mut files = vec![];
        session_files(&root, 0, &mut files, &mut 0);
        assert_eq!(files.len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn synthetic_codex_usage_is_partial_account_scoped_and_future_safe() {
        let root = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        fs::create_dir_all(&root).unwrap();
        let now = DateTime::parse_from_rfc3339("2026-10-07T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        for (name, account, modified) in [
            ("rollout-current.jsonl", "current", 3),
            ("rollout-other.jsonl", "other", 2),
        ] {
            let rows = [
                json!({"type":"session_meta","payload":{"creator_account_id":account,"private":"not exported"}}),
                json!({"timestamp":"2026-10-07T10:00:00Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"total_tokens":100}},"rate_limits":{"primary":{"used_percent":25,"resets_at":1791388800i64,"window_minutes":300}}}}),
                json!({"timestamp":"2026-10-07T10:01:00Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"total_tokens":150}}}}),
                json!({"timestamp":"2099-01-01T00:00:00Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"total_tokens":999999}}}}),
            ];
            let path = root.join(name);
            fs::write(
                &path,
                rows.iter()
                    .map(Value::to_string)
                    .collect::<Vec<_>>()
                    .join("\n"),
            )
            .unwrap();
            fs::File::open(&path)
                .unwrap()
                .set_times(
                    fs::FileTimes::new().set_modified(
                        std::time::UNIX_EPOCH + std::time::Duration::from_secs(modified),
                    ),
                )
                .unwrap();
        }
        let view = codex_from_sessions(&root, now);
        assert_eq!(
            view.daily_tokens,
            vec![json!({"startDate":"2026-10-07","tokens":50})]
        );
        assert_eq!(view.observation.as_ref().unwrap().windows.len(), 1);
        assert_eq!(
            view.observation.as_ref().unwrap().windows[0].used_percent,
            Some(25.0)
        );
        assert!(!serde_json::to_string(&view)
            .unwrap()
            .contains("not exported"));
        assert_eq!(view.summary["historyComplete"], false);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn malformed_session_and_absent_claude_windows_remain_unavailable() {
        let root = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("rollout-invalid.jsonl"), "not JSON").unwrap();
        assert!(!codex_from_sessions(&root, Utc::now()).connected);
        fs::write(
            root.join("claude-usage.json"),
            json!({"observedAt":"2026-10-07T12:00:00Z","rate_limits":{}}).to_string(),
        )
        .unwrap();
        let view = claude_local(&root, "synthetic");
        assert!(view
            .observation
            .unwrap()
            .windows
            .iter()
            .all(|window| window.used_percent.is_none()));
        fs::remove_dir_all(root).unwrap();
    }
}
