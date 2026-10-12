use crate::observability::{failure, map_error, record};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub fn config_path() -> Result<PathBuf, String> {
    let root = std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".claude")))
        .ok_or("Home directory unavailable")?;
    Ok(root.join("settings.json"))
}
fn read_json(path: &Path) -> Result<Value, String> {
    if fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink()) {
        return Err("Maxxit will not change a symlinked settings or bridge file".into());
    }
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|_| {
            "Existing Claude settings are not valid JSON. Fix them before connecting.".into()
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(e) => Err(e.to_string()),
    }
}
pub fn atomic_json(path: &Path, value: &Value) -> Result<(), String> {
    let parent = path.parent().ok_or("Invalid storage path")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temporary = parent.join(format!(".maxxit-{}.tmp", uuid::Uuid::new_v4()));
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| {
        let mut file = options.open(&temporary).map_err(|e| e.to_string())?;
        file.write_all(
            serde_json::to_string_pretty(value)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )
        .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        fs::rename(&temporary, path).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
pub fn preview(data_dir: &Path) -> Result<Value, String> {
    preview_at(data_dir, &config_path()?)
}
fn preview_at(data_dir: &Path, config: &Path) -> Result<Value, String> {
    let raw = read_json(config)?;
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let command = format!(
        "{} --capture-claude {}",
        quote(&executable.to_string_lossy()),
        quote(&data_dir.to_string_lossy())
    );
    Ok(
        json!({"settingsPath":config,"previous":raw.get("statusLine"),"command":command,"description":"Record only allowance windows. Existing status-line output is preserved. Provider credentials stay with Claude Code."}),
    )
}
pub fn install(data_dir: &Path) -> Result<(), String> {
    record("bridge.install.started", json!({}));
    install_at(data_dir, &config_path()?)
        .map_err(|e| map_error("bridge.install.failed", "write", e))?;
    record("bridge.install.completed", json!({}));
    Ok(())
}
fn install_at(data_dir: &Path, config: &Path) -> Result<(), String> {
    let preview = preview_at(data_dir, config)?;
    let mut raw = read_json(config)?;
    if !raw.is_object() {
        return Err("Claude settings must be a JSON object".into());
    }
    let backup_path = data_dir.join("claude-bridge.json");
    let previous = read_json(&backup_path)?;
    if let Some(command) = previous.get("installedCommand").and_then(Value::as_str) {
        if raw.pointer("/statusLine/command").and_then(Value::as_str) == Some(command) {
            return Ok(());
        }
    }
    let old = raw.get("statusLine").cloned().unwrap_or(Value::Null);
    if !old.is_null() && old.get("type").and_then(Value::as_str) != Some("command") {
        return Err("This status-line type cannot be safely wrapped".into());
    }
    atomic_json(
        &backup_path,
        &json!({"settingsPath":config,"previous":old,"installedCommand":preview["command"]}),
    )?;
    let mut status_line = if old.is_object() { old } else { json!({}) };
    status_line["type"] = json!("command");
    status_line["command"] = preview["command"].clone();
    raw["statusLine"] = status_line;
    atomic_json(config, &raw)
}
pub fn uninstall(data_dir: &Path) -> Result<(), String> {
    uninstall_at(data_dir, &config_path()?)
        .map_err(|e| map_error("bridge.restore.failed", "restore", e))
}
fn uninstall_at(data_dir: &Path, approved_config: &Path) -> Result<(), String> {
    let backup = read_json(&data_dir.join("claude-bridge.json"))?;
    let Some(path) = backup.get("settingsPath").and_then(Value::as_str) else {
        return Ok(());
    };
    let config = PathBuf::from(path);
    if config != approved_config {
        return Err("Claude configuration location changed. Review the bridge manually.".into());
    }
    let mut raw = read_json(&config)?;
    if raw.pointer("/statusLine/command") != backup.get("installedCommand") {
        return Err("Claude settings changed after connection. Maxxit left them unchanged. Review your status-line command manually.".into());
    }
    if backup["previous"].is_null() {
        raw.as_object_mut()
            .ok_or("Invalid configuration")?
            .remove("statusLine");
    } else {
        raw["statusLine"] = backup["previous"].clone();
    }
    atomic_json(&config, &raw)?;
    let _ = fs::remove_file(data_dir.join("claude-bridge.json"));
    Ok(())
}
pub fn sanitized_capture(data_dir: &Path, raw: &Value) -> Result<bool, String> {
    let mut limits = serde_json::Map::new();
    for key in ["five_hour", "seven_day"] {
        let w = &raw["rate_limits"][key];
        if let (Some(used), Some(reset)) = (
            w.get("used_percentage").and_then(Value::as_f64),
            w.get("resets_at").and_then(Value::as_i64),
        ) {
            if used.is_finite() && (0.0..=100.0).contains(&used) && reset > 0 {
                limits.insert(
                    key.into(),
                    json!({"used_percentage":used,"resets_at":reset}),
                );
            }
        }
    }
    if limits.is_empty() {
        return Ok(false);
    }
    let fingerprint = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&limits).map_err(|e| e.to_string())?)
    );
    let path = data_dir.join("claude-usage.json");
    if read_json(&path)
        .ok()
        .and_then(|v| {
            v.get("fingerprint")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .as_deref()
        == Some(&fingerprint)
    {
        return Ok(false);
    }
    atomic_json(
        &path,
        &json!({"observedAt":chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs,true),"fingerprint":fingerprint,"rate_limits":limits}),
    )?;
    Ok(true)
}
pub fn capture() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let index = args
        .iter()
        .position(|s| s == "--capture-claude")
        .ok_or("Missing mode")?;
    let data_dir = PathBuf::from(args.get(index + 1).ok_or("Missing bridge directory")?);
    let mut input = Vec::new();
    std::io::stdin()
        .take(128001)
        .read_to_end(&mut input)
        .map_err(|e| e.to_string())?;
    if input.len() > 128000 {
        return Err("Payload exceeds 128 KB".into());
    }
    match serde_json::from_slice::<Value>(&input) {
        Ok(raw) => {
            if let Err(error) = sanitized_capture(&data_dir, &raw) {
                map_error("bridge.capture.failed", "write", error);
            }
        }
        Err(_) => failure(
            "provider.parse.failed",
            json!({"provider":"claude","stage":"parse"}),
        ),
    }
    let backup = read_json(&data_dir.join("claude-bridge.json"))?;
    if let Some(command) = backup.pointer("/previous/command").and_then(Value::as_str) {
        let mut child = Command::new("/bin/sh")
            .args(["-c", command])
            .stdin(Stdio::piped())
            .stdout(Stdio::inherit())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(&input).map_err(|e| e.to_string())?;
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            if let Some(status) = child
                .try_wait()
                .map_err(|e| map_error("bridge.previous_command.failed", "read", e))?
            {
                if !status.success() {
                    failure(
                        "bridge.previous_command.failed",
                        json!({"status":status.code()}),
                    );
                }
                break;
            }
            if std::time::Instant::now() >= deadline {
                failure(
                    "bridge.previous_command.timeout",
                    json!({"timeout_ms":2000}),
                );
                let _ = child.kill();
                let _ = child.wait();
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    } else {
        print!("Maxxit · local allowance");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timer_repeats_do_not_refresh_observation() {
        let root = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        let payload = json!({"rate_limits":{"five_hour":{"used_percentage":25,"resets_at":4102444800i64}},"secret":"must not persist"});
        assert!(sanitized_capture(&root, &payload).unwrap());
        let first = fs::read(root.join("claude-usage.json")).unwrap();
        assert!(!sanitized_capture(&root, &payload).unwrap());
        assert_eq!(first, fs::read(root.join("claude-usage.json")).unwrap());
        assert!(!String::from_utf8(first)
            .unwrap()
            .contains("must not persist"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bridge_install_is_repeatable_and_restores_previous_output() {
        let root = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        fs::create_dir_all(&root).unwrap();
        let config = root.join("settings.json");
        let data = root.join("owned-data");
        let previous =
            json!({"statusLine":{"type":"command","command":"printf safe"},"unrelated":true});
        atomic_json(&config, &previous).unwrap();
        install_at(&data, &config).unwrap();
        let first = fs::read(&config).unwrap();
        install_at(&data, &config).unwrap();
        assert_eq!(first, fs::read(&config).unwrap());
        uninstall_at(&data, &config).unwrap();
        assert_eq!(read_json(&config).unwrap(), previous);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn newer_user_settings_and_unreadable_backup_are_preserved() {
        let root = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        fs::create_dir_all(&root).unwrap();
        let config = root.join("settings.json");
        let data = root.join("owned-data");
        atomic_json(&config, &json!({})).unwrap();
        install_at(&data, &config).unwrap();
        let changed = json!({"statusLine":{"type":"command","command":"printf newer"}});
        atomic_json(&config, &changed).unwrap();
        assert!(uninstall_at(&data, &config).is_err());
        assert_eq!(read_json(&config).unwrap(), changed);
        fs::write(data.join("claude-bridge.json"), "broken JSON").unwrap();
        assert!(install_at(&data, &config).is_err());
        assert_eq!(read_json(&config).unwrap(), changed);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn symlinked_settings_are_never_replaced() {
        use std::os::unix::fs::symlink;
        let root = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        fs::create_dir_all(&root).unwrap();
        let original = root.join("original.json");
        let config = root.join("settings.json");
        fs::write(&original, "{}").unwrap();
        symlink(&original, &config).unwrap();
        assert!(install_at(&root.join("data"), &config).is_err());
        assert!(fs::symlink_metadata(&config)
            .unwrap()
            .file_type()
            .is_symlink());
        fs::remove_dir_all(root).unwrap();
    }
}
