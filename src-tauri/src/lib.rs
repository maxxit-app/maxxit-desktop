mod account;
pub mod bridge;
mod cloud;
mod model;
mod providers;
mod storage;
mod sync;
mod workflow;
static SYNC_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

use model::{ProviderView, Settings};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicI64, Ordering},
        Mutex, MutexGuard,
    },
};
use storage::Store;
use tauri::{Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
async fn export_token_csv(
    app: tauri::AppHandle,
    runtime: State<'_, Runtime>,
    csv: String,
    days: u32,
) -> Result<bool, String> {
    drop(runtime.authorized_store()?);
    if ![7, 14, 30].contains(&days)
        || csv.len() > 10_000
        || !csv.starts_with("date_utc,observed_tokens\n")
    {
        return Err("Invalid token export".into());
    }
    for line in csv.lines().skip(1) {
        let (date, tokens) = line.split_once(',').ok_or("Invalid token export")?;
        if date.len() != 10
            || !date.chars().all(|c| c.is_ascii_digit() || c == '-')
            || (!tokens.is_empty() && tokens.parse::<u64>().is_err())
        {
            return Err("Invalid token export".into());
        }
    }
    tauri::async_runtime::spawn_blocking(move || {
        let selected = app
            .dialog()
            .file()
            .add_filter("CSV", &["csv"])
            .set_file_name(format!("maxxit-codex-{days}-days.csv"))
            .blocking_save_file();
        let Some(selected) = selected else {
            return Ok(false);
        };
        let path = selected.into_path().map_err(|e| e.to_string())?;
        std::fs::write(path, csv).map_err(|e| e.to_string())?;
        Ok(true)
    })
    .await
    .map_err(|e| e.to_string())?
}

struct Runtime {
    store: Mutex<Store>,
    session_store: Mutex<Store>,
    account: Mutex<account::Account>,
    directory: PathBuf,
    pairing: Mutex<Option<Value>>,
    panel_hidden_at: AtomicI64,
}
#[cfg(test)]
mod account_gate_tests {
    use super::*;
    #[test]
    fn native_collection_and_storage_require_the_matching_verified_account() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        store.set("ownerAccountId", &json!("A")).unwrap();
        store
            .save_project(&json!({"id":"private","name":"PRIVATE_ACCOUNT_A"}))
            .unwrap();
        let runtime = Runtime {
            store: Mutex::new(store),
            session_store: Mutex::new(Store::open(std::path::Path::new(":memory:")).unwrap()),
            account: Mutex::new(account::Account::default()),
            directory: PathBuf::from("unused-fixture-directory"),
            pairing: Mutex::new(None),
            panel_hidden_at: AtomicI64::new(0),
        };
        assert!(runtime.authorized_store().is_err());
        assert!(collect(&runtime).is_err());
        let now = chrono::Utc::now().timestamp();
        *runtime.account.lock().unwrap() = account::Account::verified(json!({"account":{"id":"B"},"identity":{"accountId":"B","deviceId":"DB"},"billing":{"plan":"free"}}),"fixture-B".into(),now).unwrap();
        assert!(runtime.authorized_store().is_err());
        assert!(collect(&runtime).is_err());
        *runtime.account.lock().unwrap() = account::Account::verified(json!({"account":{"id":"A"},"identity":{"accountId":"A","deviceId":"DA"},"billing":{"plan":"free"}}),"fixture-A".into(),now).unwrap();
        assert_eq!(
            runtime.authorized_store().unwrap().projects().unwrap()[0]["name"],
            "PRIVATE_ACCOUNT_A"
        );
        runtime.reject_revoked("HTTP 401: Revoked").unwrap();
        assert!(runtime.authorized_store().is_err());
        assert_eq!(
            runtime
                .session_store
                .lock()
                .unwrap()
                .get("signedOut")
                .unwrap(),
            Some(json!(true))
        );
    }
    #[test]
    fn legacy_claim_is_owned_and_preserves_newer_projects() {
        let legacy = Store::open(std::path::Path::new(":memory:")).unwrap();
        legacy
            .save_project(&json!({"id":"same","name":"Earlier"}))
            .unwrap();
        let account_a = Store::open(std::path::Path::new(":memory:")).unwrap();
        account_a
            .save_project(&json!({"id":"same","name":"Newer"}))
            .unwrap();
        account_a.claim_legacy(&legacy, "A").unwrap();
        assert_eq!(account_a.projects().unwrap()[0]["name"], "Newer");
        let account_b = Store::open(std::path::Path::new(":memory:")).unwrap();
        assert!(account_b.claim_legacy(&legacy, "B").is_err());
        assert!(account_b.projects().unwrap().is_empty());
    }
}
fn collect(runtime: &Runtime) -> Result<Value, String> {
    let store = runtime.authorized_store()?;
    let settings = store.settings()?;
    let codex = if settings.codex_enabled {
        providers::codex_local()
    } else {
        ProviderView::empty("codex")
    };
    let claude = if settings.claude_enabled {
        providers::claude_local(&runtime.directory, &settings.claude_account)
    } else {
        ProviderView::empty("claude")
    };
    store.prune_runs()?;
    for provider in [&codex, &claude] {
        if let Some(observation) = &provider.observation {
            store.record(observation)?;
        }
    }
    Ok(
        json!({"settings":settings,"providers":[codex,claude],"history":store.history()?,"projects":store.projects()?,"localRuns":store.runs()?,"cloud":{"connected":false,"plan":"free"}}),
    )
}
impl Runtime {
    fn reject_revoked(&self, error: &str) -> Result<(), String> {
        if cloud::relay_revoked(error) {
            self.session_store
                .lock()
                .map_err(|_| "Session storage unavailable")?
                .set("signedOut", &json!(true))?;
            let mut account = self
                .account
                .lock()
                .map_err(|_| "Account state unavailable")?;
            *account = account::Account::default();
            account.status = "revoked".into();
            account.error = Some("This Mac was signed out. Sign in again.".into());
        }
        Ok(())
    }
    fn authorized_store(&self) -> Result<MutexGuard<'_, Store>, String> {
        let store = self.store.lock().map_err(|_| "Storage unavailable")?;
        let account = self
            .account
            .lock()
            .map_err(|_| "Account state unavailable")?;
        if !account.allows_local(chrono::Utc::now().timestamp()) {
            return Err("Sign in to Maxxit before using the app".into());
        }
        if store
            .get("ownerAccountId")?
            .and_then(|v| v.as_str().map(str::to_owned))
            != account.account_id
        {
            return Err("Account storage is locked".into());
        }
        Ok(store)
    }
}
fn runtime_settings(runtime: &Runtime) -> Result<Settings, String> {
    runtime.authorized_store()?.settings()
}
async fn flush_revocations() {
    let settings = Settings::default();
    if let Ok(tokens) = cloud::pending_revocations() {
        let mut remaining = vec![];
        for old in tokens {
            if let Err(error) = cloud::request_typed(
                &settings,
                "collector/disconnect",
                Some(json!({})),
                Some(&old),
            )
            .await
            {
                if !matches!(error, cloud::CloudError::Http(401, _)) {
                    remaining.push(old);
                }
            }
        }
        let _ = cloud::save_revocations(&remaining);
    }
}
async fn refresh_account(runtime: &Runtime) -> Result<Value, String> {
    let _sync = SYNC_LOCK.lock().await;
    let now = chrono::Utc::now().timestamp();
    let signed_out = runtime
        .session_store
        .lock()
        .map_err(|_| "Session storage unavailable")?
        .get("signedOut")?
        == Some(json!(true));
    let token = cloud::credential();
    let mut current = runtime
        .account
        .lock()
        .map_err(|_| "Account state unavailable")?
        .clone();
    let settings = Settings::default();
    current = match token {
        Ok(Some(token)) if !signed_out => {
            let fingerprint = workflow::binding(&token);
            match cloud::request_typed(&settings, "desktop/state", None, Some(&token)).await {
                Ok(data) => {
                    let verified = match account::Account::verified(data, fingerprint, now) {
                        Ok(verified) => verified,
                        Err(error) => {
                            let locked = account::Account {
                                status: "error".into(),
                                error: Some(error.clone()),
                                ..Default::default()
                            };
                            *runtime
                                .account
                                .lock()
                                .map_err(|_| "Account state unavailable")? = locked;
                            return Err(error);
                        }
                    };
                    // Lock access while switching/opening the verified account store.
                    // A storage failure must not leave the previous account admitted.
                    *runtime
                        .account
                        .lock()
                        .map_err(|_| "Account state unavailable")? = account::Account {
                        status: "checking".into(),
                        ..Default::default()
                    };
                    let owner = verified.account_id.as_deref().ok_or("Missing account")?;
                    let mut store = runtime.store.lock().map_err(|_| "Storage unavailable")?;
                    if store.get("ownerAccountId")? != Some(json!(owner)) {
                        let accounts = runtime.directory.join("accounts");
                        std::fs::create_dir_all(&accounts)
                            .map_err(|_| "Account directory unavailable")?;
                        let next = Store::open_keychain(
                            &accounts.join(format!("{}.sqlite", workflow::binding(owner))),
                        )?;
                        if next
                            .get("ownerAccountId")?
                            .is_some_and(|id| id != json!(owner))
                        {
                            return Err("Account storage does not match".into());
                        }
                        next.set("ownerAccountId", &json!(owner))?;
                        *store = next;
                    }
                    let consent = &verified.cloud["data"]["identity"]["consent"];
                    let mut preferences = store.settings()?;
                    // A scope removed in the browser immediately stops that category here.
                    preferences.cloud_sync &= consent["usage"] == true;
                    preferences.project_sync &= consent["metadata"] == true;
                    preferences.account_sync &= consent["accountSync"] == true;
                    preferences.workflow_sync &= consent["workflowDetails"] == true;
                    preferences.result_events &= consent["completionEvents"] == true;
                    let primary = &verified.cloud["data"]["primaryDesktopId"];
                    let shared = &verified.cloud["data"]["sharedDesktopPreferences"];
                    if primary.as_str() != verified.device_id.as_deref() && shared.is_object() {
                        let original = serde_json::to_value(&preferences)
                            .map_err(|_| "Invalid preferences")?;
                        let mut merged = original.as_object().ok_or("Invalid preferences")?.clone();
                        for (key, value) in
                            shared.as_object().ok_or("Invalid shared preferences")?
                        {
                            merged.insert(key.clone(), value.clone());
                        }
                        preferences = serde_json::from_value(Value::Object(merged))
                            .map_err(|_| "Shared preference contract is invalid")?;
                    }
                    if store.get("primaryDesktopId")?.as_ref() != Some(primary) {
                        store.requeue_sync()?;
                        store.set("primaryDesktopId", primary)?;
                    }
                    store.save_settings(&preferences)?;
                    let epoch = verified.cloud["data"]["identity"]["epoch"].clone();
                    if store.get("syncEpoch")? != Some(epoch.clone()) {
                        store.requeue_sync()?;
                        store.set("syncEpoch", &epoch)?;
                    }
                    runtime
                        .session_store
                        .lock()
                        .map_err(|_| "Session storage unavailable")?
                        .set(
                            "account",
                            &serde_json::to_value(&verified)
                                .map_err(|_| "Invalid account state")?,
                        )?;
                    verified
                }
                Err(cloud::CloudError::Http(401, message)) => {
                    let locked = account::Account {
                        status: "revoked".into(),
                        error: Some(message),
                        ..Default::default()
                    };
                    runtime
                        .session_store
                        .lock()
                        .map_err(|_| "Session storage unavailable")?
                        .set("signedOut", &json!(true))?;
                    locked
                }
                Err(cloud::CloudError::Http(status, message)) if (400..500).contains(&status) => {
                    account::Account {
                        status: "error".into(),
                        error: Some(message),
                        ..Default::default()
                    }
                }
                Err(error) => current.unavailable(&fingerprint, now, error.to_string()),
            }
        }
        Ok(_) => account::Account::default(),
        Err(error) => account::Account {
            status: "error".into(),
            error: Some(error),
            ..Default::default()
        },
    };
    *runtime
        .account
        .lock()
        .map_err(|_| "Account state unavailable")? = current.clone();
    let mut public = current.public();
    if let Some(pairing) = runtime
        .session_store
        .lock()
        .map_err(|_| "Session storage unavailable")?
        .get("pairing")?
        .filter(|p| {
            p.is_object()
                && p["expiresAt"]
                    .as_str()
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .is_some_and(|expires| expires.timestamp() > now)
        })
    {
        public["pairing"] = json!({"code":pairing["code"],"expiresAt":pairing["expiresAt"],"url":format!("https://maxxit.app/device?code={}",pairing["code"].as_str().unwrap_or(""))});
    }
    Ok(public)
}
#[tauri::command]
async fn account_status(runtime: State<'_, Runtime>) -> Result<Value, String> {
    refresh_account(&runtime).await
}
#[tauri::command]
async fn snapshot(runtime: State<'_, Runtime>) -> Result<Value, String> {
    let _sync = SYNC_LOCK.lock().await;
    let mut data = collect(&runtime)?;
    let account = runtime
        .account
        .lock()
        .map_err(|_| "Account state unavailable")?
        .clone();
    data["cloud"] = account.cloud.clone();
    data["account"] = account.public();
    data["sync"] = runtime.authorized_store()?.sync_status()?;
    data["legacyDataAvailable"] = json!(
        runtime.directory.join("maxxit.sqlite").exists()
            && runtime.authorized_store()?.get("legacyImported")? != Some(json!(true))
    );
    Ok(data)
}
#[tauri::command]
async fn cloud_start(
    runtime: State<'_, Runtime>,
    usage: bool,
    metadata: bool,
    completion_events: bool,
    account_sync: Option<bool>,
    workflow_details: Option<bool>,
) -> Result<Value, String> {
    let _sync = SYNC_LOCK.lock().await;
    let settings = Settings::default();
    let installation = {
        let session = runtime
            .session_store
            .lock()
            .map_err(|_| "Session storage unavailable")?;
        let installation = session
            .get("installationId")?
            .unwrap_or_else(|| json!(uuid::Uuid::new_v4().to_string()));
        session.set("installationId", &installation)?;
        installation
    };
    let value=cloud::request(&settings,"pairing/start",Some(json!({"name":"Maxxit desktop","syncProtocol":1,"installationId":installation,"consent":{"usage":usage,"metadata":metadata,"excerpts":false,"completionEvents":completion_events,"accountSync":account_sync.unwrap_or(false),"workflowDetails":workflow_details.unwrap_or(false)}})),false).await?;
    runtime
        .session_store
        .lock()
        .map_err(|_| "Session storage unavailable")?
        .set("pairing", &value)?;
    *runtime.pairing.lock().map_err(|_| "Pairing unavailable")? = Some(value.clone());
    Ok(
        json!({"code":value["code"],"expiresAt":value["expiresAt"],"url":format!("{}/device?code={}",cloud::origin(&settings)?,value["code"].as_str().ok_or("Invalid pairing code")?)}),
    )
}
#[tauri::command]
async fn cloud_redeem(runtime: State<'_, Runtime>) -> Result<(), String> {
    let sync = SYNC_LOCK.lock().await;
    let pairing = runtime
        .session_store
        .lock()
        .map_err(|_| "Session storage unavailable")?
        .get("pairing")?
        .ok_or("Start sign-in first")?;
    let value = cloud::request(
        &Settings::default(),
        "pairing/redeem",
        Some(json!({"secret":pairing["secret"]})),
        false,
    )
    .await?;
    if let Some(previous) = cloud::credential()? {
        let mut pending = cloud::pending_revocations()?;
        pending.push(previous);
        cloud::save_revocations(&pending)?;
    }
    cloud::save_credential(value["token"].as_str().ok_or("Invalid device token")?)?;
    runtime
        .session_store
        .lock()
        .map_err(|_| "Session storage unavailable")?
        .set("signedOut", &json!(false))?;
    runtime
        .session_store
        .lock()
        .map_err(|_| "Session storage unavailable")?
        .set("pairing", &Value::Null)?;
    *runtime.pairing.lock().map_err(|_| "Pairing unavailable")? = None;
    drop(sync);
    refresh_account(&runtime).await?;
    let store = runtime.authorized_store()?;
    let mut settings = store.settings()?;
    let consent = &value["consent"];
    settings.cloud_sync = consent["usage"] == true;
    settings.project_sync = consent["metadata"] == true;
    settings.account_sync = consent["accountSync"] == true;
    settings.workflow_sync = consent["workflowDetails"] == true;
    settings.result_events = consent["completionEvents"] == true;
    store.cancel_outbox()?;
    store.save_settings(&settings)?;
    store.requeue_sync()?;
    Ok(())
}
#[tauri::command]
async fn cloud_cancel(runtime: State<'_, Runtime>) -> Result<(), String> {
    runtime
        .session_store
        .lock()
        .map_err(|_| "Session storage unavailable")?
        .set("pairing", &Value::Null)?;
    *runtime.pairing.lock().map_err(|_| "Pairing unavailable")? = None;
    Ok(())
}
#[tauri::command]
async fn cloud_disconnect(
    app: tauri::AppHandle,
    runtime: State<'_, Runtime>,
) -> Result<(), String> {
    let _sync = SYNC_LOCK.lock().await;
    let mut store = runtime.store.lock().map_err(|_| "Storage unavailable")?;
    runtime
        .session_store
        .lock()
        .map_err(|_| "Session storage unavailable")?
        .set("signedOut", &json!(true))?;
    *runtime
        .account
        .lock()
        .map_err(|_| "Account state unavailable")? = account::Account::default();
    let _ = app.emit("usage-updated", ());
    if let Some(tray) = app.tray_by_id("maxxit") {
        let _ = tray.set_title(Some("M"));
        let _ = tray.set_tooltip(Some("Sign in to Maxxit"));
    }
    store.cancel_outbox()?;
    let token = cloud::credential()?;
    if let Some(token) = token {
        let mut pending = cloud::pending_revocations()?;
        if !pending.contains(&token) {
            pending.push(token);
        }
        cloud::save_revocations(&pending)?;
    }
    cloud::forget_credential()?;
    *store = Store::open_keychain(&runtime.directory.join("signed-out.sqlite"))?;
    Ok(())
}
#[tauri::command]
fn legacy_preview(runtime: State<'_, Runtime>) -> Result<Value, String> {
    let _store = runtime.authorized_store()?;
    let legacy = Store::open_keychain(&runtime.directory.join("maxxit.sqlite"))?;
    let owner = runtime
        .account
        .lock()
        .map_err(|_| "Account state unavailable")?
        .account_id
        .clone()
        .ok_or("Sign in first")?;
    if legacy
        .get("claimedAccountId")?
        .is_some_and(|v| v != json!(owner))
    {
        return Err("Earlier local data belongs to another Maxxit account".into());
    }
    Ok(
        json!({"accountId":owner,"projects":legacy.projects()?,"observations":legacy.history()?.len(),"runs":legacy.runs()?.len()}),
    )
}
#[tauri::command]
async fn legacy_import(runtime: State<'_, Runtime>, account_id: String) -> Result<(), String> {
    let _sync = SYNC_LOCK.lock().await;
    let store = runtime.authorized_store()?;
    if store.get("ownerAccountId")? != Some(json!(account_id)) {
        return Err("Account changed. Review the import again".into());
    }
    if store.get("legacyImported")? == Some(json!(true)) {
        return Err("Earlier data is already imported".into());
    }
    let legacy = Store::open_keychain(&runtime.directory.join("maxxit.sqlite"))?;
    store.claim_legacy(&legacy, &account_id)
}
#[tauri::command]
async fn cloud_sync(runtime: State<'_, Runtime>) -> Result<(), String> {
    sync_cloud(&runtime).await
}
fn stop_result_relay(runtime: &Runtime) -> Result<(), String> {
    let store = runtime.authorized_store()?;
    store.cancel_outbox()?;
    let mut settings = store.settings()?;
    settings.result_events = false;
    store.save_settings(&settings)
}
async fn sync_cloud(runtime: &Runtime) -> Result<(), String> {
    let _sync = SYNC_LOCK.lock().await;
    drop(runtime.authorized_store()?);
    let Some(token) = cloud::credential()? else {
        return Ok(());
    };
    let data = collect(runtime)?;
    let settings = runtime_settings(runtime)?;
    let privacy = json!({"generationMode":settings.generation_mode,"completionEvents":settings.result_events,"hostedAiConsent":settings.ai_consent && settings.generation_mode=="hosted"});
    let privacy_key = workflow::binding(&format!("{}{}", token, privacy));
    let previous = runtime.authorized_store()?.get("privacyAcknowledgement")?;
    if previous != Some(json!(privacy_key)) {
        if let Err(error) =
            cloud::request_bound(&settings, "desktop/privacy", Some(privacy), Some(&token)).await
        {
            if cloud::relay_revoked(&error) {
                stop_result_relay(runtime)?;
                runtime.reject_revoked(&error)?;
            }
            return Err(error);
        }
        runtime
            .authorized_store()?
            .set("privacyAcknowledgement", &json!(privacy_key))?;
    }
    for provider in data["providers"].as_array().ok_or("Invalid providers")? {
        let cutoff = (chrono::Utc::now() - chrono::Duration::days(89))
            .format("%Y-%m-%d")
            .to_string();
        let daily_tokens: Vec<Value> = provider["dailyTokens"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|day| {
                day["startDate"]
                    .as_str()
                    .is_some_and(|date| date >= cutoff.as_str())
            })
            .take(90)
            .cloned()
            .collect();
        let analytics = json!({"provider":provider["provider"],"accountLabel":provider["accountLabel"],"dailyTokens":daily_tokens,"summary":{"scope":provider["summary"]["scope"].as_str().unwrap_or("Observed local increments"),"historyComplete":provider["summary"]["historyComplete"].as_bool().unwrap_or(false)}});
        runtime.authorized_store()?.set(
            &format!(
                "analytics:{}",
                provider["provider"].as_str().ok_or("Invalid provider")?
            ),
            &analytics,
        )?;
    }
    for _ in 0..10 {
        let current = runtime_settings(runtime)?;
        let (batch, epoch) = {
            let store = runtime.authorized_store()?;
            (
                store.sync_batch(&current)?,
                store.get("syncEpoch")?.unwrap_or(json!(0)),
            )
        };
        if batch.is_empty() {
            break;
        }
        let result = cloud::request_bound(
            &current,
            "desktop/sync",
            Some(json!({"schemaVersion":1,"epoch":epoch,"operations":batch})),
            Some(&token),
        )
        .await;
        match result {
            Ok(reply) => {
                if reply["epoch"] != epoch {
                    return Err("Sync epoch did not match".into());
                }
                runtime
                    .authorized_store()?
                    .acknowledge_sync(&batch, &reply)?;
                runtime.authorized_store()?.set("syncError", &Value::Null)?;
            }
            Err(error) => {
                runtime
                    .authorized_store()?
                    .set("syncError", &json!(error))?;
                runtime.reject_revoked(&error)?;
                return Err(error);
            }
        }
    }
    for _ in 0..10 {
        if !runtime_settings(runtime)?.result_events {
            break;
        }
        let event = runtime.authorized_store()?.next_event(&token)?;
        let Some(event) = event else {
            break;
        };
        let result = cloud::request_bound(
            &settings,
            "desktop/result-events",
            Some(event.clone()),
            Some(&token),
        )
        .await;
        let accepted = result
            .as_ref()
            .is_ok_and(|ack| ack["ok"] == true && ack["eventId"] == event["eventId"]);
        runtime
            .authorized_store()?
            .event_attempt(&event, accepted)?;
        if !accepted {
            let error = result
                .err()
                .unwrap_or("Completion acknowledgement did not match".into());
            if cloud::relay_revoked(&error) {
                stop_result_relay(runtime)?;
                runtime.reject_revoked(&error)?;
            }
            return Err(error);
        }
    }
    Ok(())
}
#[tauri::command]
async fn cloud_action(runtime: State<'_, Runtime>, action: String) -> Result<Value, String> {
    if !["checkout", "portal", "generate"].contains(&action.as_str()) {
        return Err("Unknown cloud action".into());
    }
    if action == "generate" {
        let settings = runtime_settings(&runtime)?;
        if settings.generation_mode != "hosted" || !settings.ai_consent {
            return Err("Enable project sharing before requesting ideas".into());
        }
        sync_cloud(&runtime).await?;
        cloud::request(
            &settings,
            "desktop/preferences",
            Some(json!({"aiConsent":true,"reminders":settings.cloud_sync})),
            true,
        )
        .await?;
    }
    let result = cloud::request(
        &runtime_settings(&runtime)?,
        &format!("desktop/{action}"),
        Some(json!({})),
        true,
    )
    .await;
    if let Err(error) = &result {
        runtime.reject_revoked(error)?;
    }
    result
}
#[tauri::command]
async fn cloud_preferences(runtime: State<'_, Runtime>) -> Result<(), String> {
    let settings = runtime_settings(&runtime)?;
    sync_cloud(&runtime).await?;
    let result = cloud::request(&settings,"desktop/preferences",Some(json!({"email":settings.email,"aiConsent":settings.ai_consent,"reminders":settings.cloud_sync})),true).await;
    if let Err(error) = &result {
        runtime.reject_revoked(error)?;
    }
    result?;
    Ok(())
}
#[tauri::command]
fn open_link(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let parsed = url::Url::parse(&url).map_err(|_| "Invalid link")?;
    if parsed.scheme() != "https"
        || parsed.username() != ""
        || parsed.password().is_some()
        || !matches!(
            parsed.host_str(),
            Some("maxxit.app") | Some("polar.sh") | Some("sandbox.polar.sh") | Some("github.com")
        )
    {
        return Err("This link is not an allowed Maxxit destination".into());
    }
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn save_settings(runtime: State<'_, Runtime>, mut settings: Settings) -> Result<(), String> {
    let _sync = SYNC_LOCK.lock().await;
    if !["local", "hosted"].contains(&settings.generation_mode.as_str()) {
        return Err("Invalid generation mode".into());
    }
    if settings.generation_mode == "local" {
        settings.ai_consent = false;
    }
    if !["system", "light", "dark"].contains(&settings.theme.as_str())
        || !["codex", "claude"].contains(&settings.tray_provider.as_str())
        || !(1..=100).contains(&settings.min_remaining)
        || !(0..=23).contains(&settings.quiet_start)
        || !(0..=23).contains(&settings.quiet_end)
        || !(1..=5).contains(&settings.daily_limit)
        || !(1..=168).contains(&settings.hours_before)
        || !(1..=4).contains(&settings.short_hours_before)
    {
        return Err("Invalid preferences".into());
    }
    let store = runtime.authorized_store()?;
    if !settings.result_events {
        store.cancel_outbox()?;
    }
    let account = runtime
        .account
        .lock()
        .map_err(|_| "Account state unavailable")?;
    if account.cloud["data"]["primaryDesktopId"].as_str() != account.device_id.as_deref()
        && sync::shared_preferences(&store.settings()?) != sync::shared_preferences(&settings)
    {
        return Err("Change shared preferences on your primary desktop, or transfer that role in the web app".into());
    }
    store.save_settings(&settings)
}
#[tauri::command]
fn claude_preview(runtime: State<'_, Runtime>) -> Result<Value, String> {
    drop(runtime.authorized_store()?);
    bridge::preview(&runtime.directory)
}
#[tauri::command]
fn connect_provider(runtime: State<'_, Runtime>, provider: String) -> Result<(), String> {
    drop(runtime.authorized_store()?);
    if provider == "claude" {
        bridge::install(&runtime.directory)?;
    } else if provider != "codex" {
        return Err("Unknown provider".into());
    }
    let store = runtime.authorized_store()?;
    let mut settings = store.settings()?;
    if provider == "claude" {
        settings.claude_enabled = true;
    } else {
        settings.codex_enabled = true;
    }
    store.save_settings(&settings)
}
#[tauri::command]
fn disconnect_provider(runtime: State<'_, Runtime>, provider: String) -> Result<(), String> {
    drop(runtime.authorized_store()?);
    if provider == "claude" {
        bridge::uninstall(&runtime.directory)?;
    } else if provider != "codex" {
        return Err("Unknown provider".into());
    }
    let store = runtime.authorized_store()?;
    let mut settings = store.settings()?;
    if provider == "claude" {
        settings.claude_enabled = false;
    } else {
        settings.codex_enabled = false;
    }
    store.save_settings(&settings)
}
#[tauri::command]
fn save_project(
    runtime: State<'_, Runtime>,
    name: String,
    description: String,
    id: Option<String>,
    archived: Option<bool>,
    pinned: Option<bool>,
) -> Result<(), String> {
    if name.trim().is_empty() || name.len() > 120 || description.len() > 10000 {
        return Err("Project name or description exceeds its limit".into());
    }
    let store = runtime.authorized_store()?;
    let project = if let Some(id) = id {
        store
            .projects()?
            .into_iter()
            .find(|p| p["id"] == id)
            .ok_or("Project not found")?
    } else {
        json!({"id":uuid::Uuid::new_v4().to_string(),"createdAt":chrono::Utc::now().to_rfc3339()})
    };
    let mut project = project;
    project["name"] = json!(name.trim());
    project["description"] = json!(description);
    project["archived"] = json!(archived.unwrap_or(project["archived"].as_bool().unwrap_or(false)));
    project["pinned"] = json!(pinned.unwrap_or(project["pinned"].as_bool().unwrap_or(false)));
    store.save_project(&project)
}
#[tauri::command]
async fn remove_project(runtime: State<'_, Runtime>, id: String) -> Result<(), String> {
    let _sync = SYNC_LOCK.lock().await;
    runtime.authorized_store()?.remove_project(&id)
}

#[tauri::command]
fn local_analysis(
    runtime: State<'_, Runtime>,
    project_id: String,
) -> Result<workflow::Run, String> {
    runtime.authorized_store()?.create_analysis(&project_id)
}
#[tauri::command]
fn local_task(
    runtime: State<'_, Runtime>,
    parent_id: String,
    index: usize,
) -> Result<workflow::Run, String> {
    runtime.authorized_store()?.create_task(&parent_id, index)
}
#[tauri::command]
async fn local_import(
    app: tauri::AppHandle,
    runtime: State<'_, Runtime>,
    id: String,
    raw: String,
) -> Result<(), String> {
    let _sync = SYNC_LOCK.lock().await;
    let settings = runtime_settings(&runtime)?;
    let token = if settings.result_events {
        cloud::credential()?
    } else {
        None
    };
    runtime
        .authorized_store()?
        .import_result(&id, &raw, token.as_deref())?;
    if settings.local_notifications {
        use tauri_plugin_notification::NotificationExt;
        let _ = app
            .notification()
            .builder()
            .title("Your local result is ready")
            .body("Open Maxxit to review the imported report.")
            .show();
    }
    drop(_sync);
    if settings.result_events {
        tauri::async_runtime::spawn(async move {
            let runtime = app.state::<Runtime>();
            let _ = sync_cloud(&runtime).await;
        });
    }
    Ok(())
}
#[tauri::command]
async fn local_remove(runtime: State<'_, Runtime>, id: String) -> Result<(), String> {
    let _sync = SYNC_LOCK.lock().await;
    runtime.authorized_store()?.remove_run(&id)
}
#[tauri::command]
async fn remove_cloud_copies(runtime: State<'_, Runtime>) -> Result<(), String> {
    let _sync = SYNC_LOCK.lock().await;
    let settings = {
        let store = runtime.authorized_store()?;
        let mut settings = store.settings()?;
        settings.cloud_sync = false;
        settings.project_sync = false;
        settings.workflow_sync = false;
        settings.account_sync = false;
        settings.result_events = false;
        settings.ai_consent = false;
        settings.generation_mode = "local".into();
        store.cancel_outbox()?;
        store.save_settings(&settings)?;
        settings
    };
    cloud::request(&settings,"desktop/privacy",Some(json!({"generationMode":settings.generation_mode,"completionEvents":settings.result_events,"removeCloudCopies":true})),true).await?;
    Ok(())
}
#[tauri::command]
async fn local_export(
    app: tauri::AppHandle,
    runtime: State<'_, Runtime>,
    id: String,
    result: bool,
) -> Result<bool, String> {
    let run = runtime
        .authorized_store()?
        .runs()?
        .into_iter()
        .find(|r| r.id == id)
        .ok_or("Run not found")?;
    let content = if result {
        serde_json::to_string_pretty(&run).map_err(|e| e.to_string())?
    } else {
        run.bundle
    };
    tauri::async_runtime::spawn_blocking(move || {
        let selected = app
            .dialog()
            .file()
            .add_filter("Text", &["txt"])
            .set_file_name(format!("maxxit-{}.txt", run.id))
            .blocking_save_file();
        let Some(selected) = selected else {
            return Ok(false);
        };
        std::fs::write(selected.into_path().map_err(|e| e.to_string())?, content)
            .map_err(|e| e.to_string())?;
        Ok(true)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn tray_resize(window: tauri::WebviewWindow, height: f64) -> Result<(), String> {
    if window.label() != "tray" || !height.is_finite() {
        return Err("Invalid panel size".into());
    }
    let available = window
        .current_monitor()
        .ok()
        .flatten()
        .map(|m| m.work_area().size.height as f64 / m.scale_factor() - 8.0)
        .unwrap_or(860.0);
    let height = height.clamp(350.0, 860.0).min(available.max(200.0));
    window
        .set_size(tauri::LogicalSize::new(440.0, height))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn tray_action(app: tauri::AppHandle, page: String) -> Result<(), String> {
    if ![
        "Overview",
        "Analytics",
        "Connections",
        "Settings",
        "close",
        "quit",
    ]
    .contains(&page.as_str())
    {
        return Err("Unknown panel action".into());
    }
    if let Some(panel) = app.get_webview_window("tray") {
        let _ = panel.hide();
    }
    if page == "quit" {
        app.exit(0);
    } else if page != "close" {
        if let Some(main) = app.get_webview_window("main") {
            main.show().map_err(|e| e.to_string())?;
            main.set_focus().map_err(|e| e.to_string())?;
            main.emit("navigate-to", page).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn show_tray_panel(
    app: &tauri::AppHandle,
    position: tauri::PhysicalPosition<f64>,
    rect: tauri::Rect,
    toggle: bool,
) {
    // macOS can blur the panel on mouse-down before the tray click arrives.
    // Treat that click as dismissal rather than reopening the panel immediately.
    let elapsed = chrono::Utc::now().timestamp_millis()
        - app
            .state::<Runtime>()
            .panel_hidden_at
            .load(Ordering::Relaxed);
    if toggle && (0..200).contains(&elapsed) {
        return;
    }
    if let Some(panel) = app.get_webview_window("tray") {
        if toggle && panel.is_visible().unwrap_or(false) {
            let _ = panel.hide();
            return;
        }
        let monitors = panel.available_monitors().unwrap_or_default();
        let monitor = monitors.iter().find(|m| {
            position.x >= m.position().x as f64
                && position.x < (m.position().x as f64 + m.size().width as f64)
                && position.y >= m.position().y as f64
                && position.y < (m.position().y as f64 + m.size().height as f64)
        });
        let scale = monitor.map(|m| m.scale_factor()).unwrap_or(1.0);
        let anchor = rect.position.to_physical::<f64>(scale);
        let size = rect.size.to_physical::<f64>(scale);
        let mut x = anchor.x + size.width / 2.0 - 220.0 * scale;
        let mut y = anchor.y + size.height + 6.0 * scale;
        if let Some(m) = monitor {
            let work = m.work_area();
            let left = work.position.x as f64 + 8.0 * scale;
            let top = work.position.y as f64 + 4.0 * scale;
            x = x
                .max(left)
                .min((work.position.x as f64 + work.size.width as f64 - 448.0 * scale).max(left));
            let height = (860.0 * scale)
                .min(work.size.height as f64 - 8.0 * scale)
                .max(200.0 * scale);
            let _ = panel.set_size(tauri::PhysicalSize::new(
                (440.0 * scale) as u32,
                height as u32,
            ));
            y = y.max(top).min(
                (work.position.y as f64 + work.size.height as f64 - height - 4.0 * scale).max(top),
            );
        }
        let _ = panel.set_position(tauri::PhysicalPosition::new(x, y));
        let _ = panel.show();
        let _ = panel.set_focus();
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let directory = app.path().app_data_dir()?;
            std::fs::create_dir_all(&directory)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))?;
            }
            let store = Store::open_keychain(&directory.join("signed-out.sqlite"))
                .map_err(std::io::Error::other)?;
            let session_store = Store::open_keychain(&directory.join("account-session.sqlite"))
                .map_err(std::io::Error::other)?;
            let mut cached: account::Account = session_store
                .get("account")
                .map_err(std::io::Error::other)?
                .map(serde_json::from_value)
                .transpose()
                .map_err(std::io::Error::other)?
                .unwrap_or_default();
            let store = if let Some(owner) = cached.account_id.as_deref() {
                let accounts = directory.join("accounts");
                std::fs::create_dir_all(&accounts)?;
                Store::open_keychain(&accounts.join(format!("{}.sqlite", workflow::binding(owner))))
                    .map_err(std::io::Error::other)?
            } else {
                store
            };
            cached.status = "checking".into();
            app.manage(Runtime {
                session_store: Mutex::new(session_store),
                account: Mutex::new(cached),
                store: Mutex::new(store),
                directory,
                pairing: Mutex::new(None),
                panel_hidden_at: AtomicI64::new(0),
            });
            #[cfg(target_os = "macos")]
            if let Some(panel) = app.get_webview_window("tray") {
                use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior as Behavior};
                // Setup runs on AppKit's main thread. Tauri owns this NSWindow;
                // the borrowed pointer is used only while the panel is alive.
                let native = unsafe { &*panel.ns_window()?.cast::<NSWindow>() };
                let behavior = native.collectionBehavior()
                    & !(Behavior::MoveToActiveSpace
                        | Behavior::FullScreenPrimary
                        | Behavior::FullScreenNone
                        | Behavior::Primary
                        | Behavior::Auxiliary);
                native.setCollectionBehavior(
                    behavior
                        | Behavior::CanJoinAllSpaces
                        | Behavior::FullScreenAuxiliary
                        | Behavior::CanJoinAllApplications
                        | Behavior::IgnoresCycle,
                );
            }
            let application_menu = tauri::menu::Menu::default(app.handle())?;
            let panel_item = tauri::menu::MenuItem::with_id(
                app,
                "show-usage",
                "Show usage panel",
                true,
                Some("CmdOrCtrl+Shift+U"),
            )?;
            for item in application_menu.items()? {
                if let tauri::menu::MenuItemKind::Submenu(submenu) = item {
                    if submenu.text()? == "View" {
                        submenu.append(&panel_item)?;
                    }
                }
            }
            app.set_menu(application_menu)?;
            let open =
                tauri::menu::MenuItem::with_id(app, "open", "Open Maxxit", true, None::<&str>)?;
            let refresh = tauri::menu::MenuItem::with_id(
                app,
                "refresh",
                "Refresh usage",
                true,
                None::<&str>,
            )?;
            let quit =
                tauri::menu::MenuItem::with_id(app, "quit", "Quit Maxxit", true, None::<&str>)?;
            let menu = tauri::menu::Menu::with_items(app, &[&open, &refresh, &quit])?;
            let mut tray = tauri::tray::TrayIconBuilder::with_id("maxxit")
                .tooltip("Maxxit · connect a provider")
                .title("M")
                .menu(&menu)
                .show_menu_on_left_click(false);
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.on_menu_event(|app, event| match event.id.as_ref() {
                "open" => {
                    if let Some(w) = app.get_webview_window("main") {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                }
                "refresh" => {
                    let _ = app.emit("usage-updated", ());
                }
                "quit" => app.exit(0),
                _ => {}
            })
            .on_tray_icon_event(|tray, event| {
                if let tauri::tray::TrayIconEvent::Click {
                    button: tauri::tray::MouseButton::Left,
                    button_state: tauri::tray::MouseButtonState::Up,
                    position,
                    rect,
                    ..
                } = event
                {
                    let app = tray.app_handle();
                    show_tray_panel(app, position, rect, true);
                }
            })
            .build(app)?;
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut failures: u32 = 0;
                loop {
                    let runtime = handle.state::<Runtime>();
                    let _ = refresh_account(&runtime).await;
                    if let Ok(data) = collect(&runtime) {
                        let preferred =
                            data["settings"]["trayProvider"].as_str().unwrap_or("codex");
                        let provider = data["providers"]
                            .as_array()
                            .and_then(|p| p.iter().find(|p| p["provider"] == preferred));
                        let observation =
                            provider.and_then(|p| p.get("observation")).and_then(|o| {
                                serde_json::from_value::<model::Observation>(o.clone()).ok()
                            });
                        let (title, tooltip) = model::tray_allowance(
                            preferred,
                            observation.as_ref(),
                            chrono::Utc::now(),
                        );
                        if let Some(tray) = handle.tray_by_id("maxxit") {
                            let _ = tray.set_title(Some(&title));
                            let _ = tray.set_tooltip(Some(tooltip));
                        }
                        let _ = handle.emit("usage-updated", ());
                    }
                    if runtime.authorized_store().is_err() {
                        if let Some(tray) = handle.tray_by_id("maxxit") {
                            let _ = tray.set_title(Some("M"));
                            let _ = tray.set_tooltip(Some("Maxxit · sign in to continue"));
                        }
                        let _ = handle.emit("usage-updated", ());
                    }
                    if runtime.authorized_store().is_ok() {
                        failures = if sync_cloud(&runtime).await.is_err() {
                            (failures + 1).min(4)
                        } else {
                            0
                        };
                    } else {
                        failures = 0;
                    }
                    if runtime.authorized_store().is_err() {
                        if let Some(tray) = handle.tray_by_id("maxxit") {
                            let _ = tray.set_title(Some("M"));
                            let _ = tray.set_tooltip(Some("Maxxit · sign in to continue"));
                        }
                        let _ = handle.emit("usage-updated", ());
                    }
                    flush_revocations().await;
                    let delay = (5_u64 * 2_u64.pow(failures)).min(60);
                    let jitter = if failures > 0 {
                        uuid::Uuid::new_v4().as_u128() as u64 % 1000
                    } else {
                        0
                    };
                    tokio::time::sleep(std::time::Duration::from_millis(delay * 1000 + jitter))
                        .await;
                }
            });
            Ok(())
        })
        .on_menu_event(|app, event| {
            if event.id.as_ref() == "show-usage" {
                if let Some(tray) = app.tray_by_id("maxxit") {
                    if let Ok(Some(rect)) = tray.rect() {
                        let scale = app
                            .get_webview_window("main")
                            .and_then(|w| w.scale_factor().ok())
                            .unwrap_or(1.0);
                        let position = rect.position.to_physical::<f64>(scale);
                        show_tray_panel(app, position, rect, false);
                    }
                }
            }
        })
        .on_window_event(|window, event| {
            if window.label() == "tray" {
                if matches!(event, tauri::WindowEvent::Focused(false)) {
                    window
                        .state::<Runtime>()
                        .panel_hidden_at
                        .store(chrono::Utc::now().timestamp_millis(), Ordering::Relaxed);
                    let _ = window.hide();
                }
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
                return;
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let runtime = window.state::<Runtime>();
                if runtime
                    .store
                    .lock()
                    .map(|s| {
                        s.settings()
                            .map(|settings| settings.background)
                            .unwrap_or(true)
                    })
                    .unwrap_or(true)
                {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            export_token_csv,
            account_status,
            cloud_cancel,
            tray_action,
            tray_resize,
            save_settings,
            claude_preview,
            connect_provider,
            disconnect_provider,
            save_project,
            remove_project,
            cloud_start,
            cloud_redeem,
            cloud_disconnect,
            cloud_sync,
            legacy_preview,
            legacy_import,
            cloud_action,
            cloud_preferences,
            open_link,
            local_analysis,
            local_task,
            local_import,
            local_remove,
            local_export,
            remove_cloud_copies
        ])
        .run(tauri::generate_context!())
        .expect("Maxxit could not start");
}
