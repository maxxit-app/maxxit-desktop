use crate::{cloud, model::Settings, Runtime};
use serde::Deserialize;
use serde_json::{json, Value};
use tauri::{Emitter, State};

// Serializing consent and transport means withdrawal completes after any in-flight
// request, and no request from the old identifier can begin after withdrawal.
static DELIVERY: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnalyticsInput {
    name: String,
    screen: String,
    engagement_ms: u64,
    action: Option<String>,
}
fn valid(input: &AnalyticsInput) -> bool {
    [
        "screen_view",
        "app_open",
        "user_engagement",
        "desktop_action",
        "action_failed",
    ]
    .contains(&input.name.as_str())
        && [
            "Overview",
            "Analytics",
            "Projects",
            "Ideas",
            "Connections",
            "Settings",
            "Tray",
        ]
        .contains(&input.screen.as_str())
        && input.engagement_ms <= 60_000
        && input.action.as_deref().is_none_or(|action| {
            [
                "connect_provider",
                "disconnect_provider",
                "save_project",
                "remove_project",
                "cloud_start",
                "cloud_redeem",
                "cloud_disconnect",
                "cloud_sync",
                "cloud_action",
                "cloud_preferences",
                "save_settings",
                "local_analysis",
                "local_task",
                "local_import",
                "local_remove",
                "local_export",
                "remove_cloud_copies",
                "export_token_csv",
                "tray_action",
                "legacy_import",
                "update_check",
                "update_install",
            ]
            .contains(&action)
        })
}
fn client_id() -> String {
    let uuid = uuid::Uuid::new_v4();
    let bytes = uuid.as_bytes();
    format!(
        "{}.{}",
        u32::from_le_bytes(bytes[..4].try_into().unwrap()).max(1),
        u32::from_le_bytes(bytes[4..8].try_into().unwrap()).max(1)
    )
}
fn health(runtime: &Runtime) -> Result<Value, String> {
    let store = runtime
        .session_store
        .lock()
        .map_err(|_| "Analytics preferences unavailable")?;
    Ok(json!({
        "choice": store.get("productAnalyticsChoice")?,
        "delivery": store.get("productAnalyticsDelivery")?.unwrap_or(json!("idle"))
    }))
}
#[tauri::command]
pub fn product_analytics_health(runtime: State<'_, Runtime>) -> Result<Value, String> {
    health(&runtime)
}
#[tauri::command]
pub async fn product_analytics_consent(
    app: tauri::AppHandle,
    runtime: State<'_, Runtime>,
    enabled: bool,
) -> Result<Value, String> {
    let _delivery = DELIVERY.lock().await;
    {
        let store = runtime
            .session_store
            .lock()
            .map_err(|_| "Analytics preferences unavailable")?;
        // Always rotate the random identity. Withdrawal retains no analytics identifier.
        store.set(
            "productAnalyticsClientId",
            &if enabled {
                json!(client_id())
            } else {
                Value::Null
            },
        )?;
        store.set("productAnalyticsSession", &Value::Null)?;
        store.set("productAnalyticsDelivery", &json!("idle"))?;
        store.set("productAnalyticsChoice", &json!(enabled))?;
    }
    let _ = app.emit("product-analytics-consent", enabled);
    health(&runtime)
}
#[tauri::command]
pub async fn product_analytics_record(
    window: tauri::WebviewWindow,
    runtime: State<'_, Runtime>,
    input: AnalyticsInput,
) -> Result<(), String> {
    if !valid(&input) || !["main", "tray"].contains(&window.label()) {
        return Err("Invalid analytics event".into());
    }
    let _delivery = DELIVERY.lock().await;
    let payload = {
        let store = runtime
            .session_store
            .lock()
            .map_err(|_| "Analytics preferences unavailable")?;
        if store.get("productAnalyticsChoice")? != Some(json!(true)) {
            return Ok(());
        }
        // No account IDs, project data, paths, or provider credentials enter the payload.
        let Some(client_id) = store
            .get("productAnalyticsClientId")?
            .and_then(|v| v.as_str().map(str::to_owned))
        else {
            return Ok(());
        };
        let now = chrono::Utc::now().timestamp();
        let previous = store.get("productAnalyticsSession")?.unwrap_or(Value::Null);
        let session = if previous["last"]
            .as_i64()
            .is_some_and(|last| now >= last && now - last < 1800)
        {
            previous["id"].as_i64().unwrap_or(now)
        } else {
            now
        };
        store.set(
            "productAnalyticsSession",
            &json!({"id": session, "last": now}),
        )?;
        json!({"clientId": client_id, "sessionId": session, "name": input.name,
            "screen": input.screen, "window": window.label(), "version": env!("CARGO_PKG_VERSION"),
            "engagementMs": input.engagement_ms, "action": input.action})
    };
    // Local use is authorized separately; missing/revoked cloud credentials never
    // fall back to an unauthenticated ingest route.
    drop(runtime.authorized_store()?);
    let mut payload = payload;
    if payload["action"].is_null() {
        payload.as_object_mut().unwrap().remove("action");
    }
    let result = cloud::request(
        &Settings::default(),
        "desktop/analytics",
        Some(payload),
        true,
    )
    .await;
    let delivery = match result {
        Ok(value) if value["accepted"] == true => "accepted",
        Ok(value) if value["configured"] == false => "not_configured",
        _ => "unavailable",
    };
    runtime
        .session_store
        .lock()
        .map_err(|_| "Analytics preferences unavailable")?
        .set("productAnalyticsDelivery", &json!(delivery))?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn analytics_ids_are_random_numeric_pairs_without_account_identity() {
        let first = client_id();
        assert_ne!(first, client_id());
        let parts: Vec<_> = first.split('.').collect();
        assert_eq!(parts.len(), 2);
        assert!(parts
            .iter()
            .all(|part| part.parse::<u32>().is_ok_and(|value| value > 0)));
    }
    #[test]
    fn only_fixed_event_metadata_is_accepted() {
        let mut input = AnalyticsInput {
            name: "screen_view".into(),
            screen: "Overview".into(),
            engagement_ms: 1500,
            action: None,
        };
        assert!(valid(&input));
        input.screen = "/Users/private/project".into();
        assert!(!valid(&input));
        input.screen = "Projects".into();
        input.action = Some("PRIVATE_SECRET".into());
        assert!(!valid(&input));
        assert!(serde_json::from_value::<AnalyticsInput>(
            json!({"name":"screen_view","screen":"Overview","engagementMs":0,"email":"PRIVATE"})
        )
        .is_err());
    }
}
