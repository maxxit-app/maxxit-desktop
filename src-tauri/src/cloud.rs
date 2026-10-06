use crate::model::Settings;
use serde_json::{json, Value};
pub fn credential() -> Result<Option<String>, String> {
    let entry = keyring::Entry::new("app.maxxit.desktop", "device").map_err(|e| e.to_string())?;
    match entry.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err("Mac Keychain could not be read".into()),
    }
}
pub fn save_credential(token: &str) -> Result<(), String> {
    if token.len() != 43
        || !token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("Invalid device token".into());
    }
    keyring::Entry::new("app.maxxit.desktop", "device")
        .map_err(|e| e.to_string())?
        .set_password(token)
        .map_err(|_| "Could not save the device token in Mac Keychain".into())
}
pub fn forget_credential() -> Result<(), String> {
    let entry = keyring::Entry::new("app.maxxit.desktop", "device").map_err(|e| e.to_string())?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err("Could not remove the device token from Mac Keychain".into()),
    }
}
pub fn origin(settings: &Settings) -> Result<String, String> {
    let url = url::Url::parse(&settings.api_origin).map_err(|_| "Invalid API origin")?;
    if url.username() != ""
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
    {
        return Err("Invalid API origin".into());
    }
    if !(url.scheme() == "https" && url.host_str() == Some("maxxit.app") && url.port().is_none())
        && !(cfg!(debug_assertions)
            && url.scheme() == "http"
            && matches!(url.host_str(), Some("127.0.0.1") | Some("localhost")))
    {
        return Err("Cloud sync requires the Maxxit HTTPS service".into());
    }
    Ok(settings.api_origin.trim_end_matches('/').into())
}
pub async fn request(
    settings: &Settings,
    path: &str,
    body: Option<Value>,
    authenticated: bool,
) -> Result<Value, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| e.to_string())?;
    let mut request = if body.is_some() {
        client.post(format!("{}/v1/{path}", origin(settings)?))
    } else {
        client.get(format!("{}/v1/{path}", origin(settings)?))
    };
    if authenticated {
        request = request.bearer_auth(credential()?.ok_or("Connect your Maxxit account first")?);
    }
    if let Some(payload) = body {
        request = request.json(&payload);
    }
    let response = request
        .send()
        .await
        .map_err(|_| "Maxxit cloud is unavailable. Local analytics still work.")?;
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(|_| "Invalid service response")?;
    if bytes.len() > 2_000_000 {
        return Err("Service response exceeds its limit".into());
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| "Invalid service response")?;
    if !status.is_success() {
        return Err(value["error"]
            .as_str()
            .unwrap_or("Cloud request failed")
            .into());
    }
    Ok(value)
}
pub async fn state(settings: &Settings) -> Value {
    match credential() {
        Ok(Some(_)) => match request(settings, "desktop/state", None, true).await {
            Ok(data) => json!({"connected":true,"plan":data["billing"]["plan"],"data":data}),
            Err(error) => json!({"connected":true,"plan":"free","error":error}),
        },
        Ok(None) => json!({"connected":false,"plan":"free"}),
        Err(error) => json!({"connected":false,"plan":"free","error":error}),
    }
}
