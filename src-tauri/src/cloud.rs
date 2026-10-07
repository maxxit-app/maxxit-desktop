use crate::model::Settings;
use serde_json::{json, Value};
static CREDENTIAL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
trait Credentials {
    fn read(&self) -> Result<Option<String>, String>;
    fn write(&self, token: &str) -> Result<(), String>;
    fn remove(&self) -> Result<(), String>;
}
struct Keychain;
impl Credentials for Keychain {
    fn read(&self) -> Result<Option<String>, String> {
        let entry = keyring::Entry::new("app.maxxit.desktop", "device")
            .map_err(|_| "Mac Keychain could not be opened")?;
        match entry.get_password() {
            Ok(token) => Ok(Some(token)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err("Mac Keychain could not be read".into()),
        }
    }
    fn write(&self, token: &str) -> Result<(), String> {
        keyring::Entry::new("app.maxxit.desktop", "device")
            .map_err(|_| "Mac Keychain could not be opened")?
            .set_password(token)
            .map_err(|_| "Could not save the device token in Mac Keychain".into())
    }
    fn remove(&self) -> Result<(), String> {
        let entry = keyring::Entry::new("app.maxxit.desktop", "device")
            .map_err(|_| "Mac Keychain could not be opened")?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err("Could not remove the device token from Mac Keychain".into()),
        }
    }
}
fn save_verified(store: &impl Credentials, token: &str) -> Result<(), String> {
    if token.len() != 43
        || !token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("Invalid device token".into());
    }
    // An unreadable existing item is never treated as an absent item.
    let previous = store.read()?;
    store.write(token)?;
    if store
        .read()
        .is_ok_and(|saved| saved.as_deref() == Some(token))
    {
        return Ok(());
    }
    if let Some(previous) = previous {
        store
            .write(&previous)
            .map_err(|_| "Keychain verification and credential restoration failed")?;
    }
    Err(
        "Keychain write could not be verified. Previous credentials were retained when available."
            .into(),
    )
}
pub fn credential() -> Result<Option<String>, String> {
    let _lock = CREDENTIAL_LOCK
        .lock()
        .map_err(|_| "Credential storage unavailable")?;
    Keychain.read()
}
pub fn save_credential(token: &str) -> Result<(), String> {
    let _lock = CREDENTIAL_LOCK
        .lock()
        .map_err(|_| "Credential storage unavailable")?;
    save_verified(&Keychain, token)
}
pub fn forget_credential() -> Result<(), String> {
    let _lock = CREDENTIAL_LOCK
        .lock()
        .map_err(|_| "Credential storage unavailable")?;
    Keychain.remove()
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
    let production =
        url.scheme() == "https" && url.host_str() == Some("maxxit.app") && url.port().is_none();
    let development = cfg!(debug_assertions)
        && url.scheme() == "http"
        && matches!(url.host_str(), Some("127.0.0.1") | Some("localhost"));
    if !(production || development) {
        return Err("Cloud sync requires the Maxxit HTTPS service".into());
    }
    Ok(settings.api_origin.trim_end_matches('/').into())
}
fn retry_delay(attempt: usize, status: Option<u16>, jitter_ms: u64) -> Option<std::time::Duration> {
    if attempt >= 2 || status.is_some_and(|code| code != 429 && !(500..=599).contains(&code)) {
        return None;
    }
    Some(std::time::Duration::from_millis(
        (1000 << attempt) + jitter_ms.min(255),
    ))
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
    let retryable = body.is_none();
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
    let mut attempt = 0;
    let mut response = loop {
        let result = request
            .try_clone()
            .ok_or("Could not prepare service request")?
            .send()
            .await;
        let status = result
            .as_ref()
            .ok()
            .map(|response| response.status().as_u16());
        let jitter = u64::from(uuid::Uuid::new_v4().as_bytes()[0]);
        if retryable {
            if let Some(delay) = retry_delay(attempt, status, jitter) {
                attempt += 1;
                tokio::time::sleep(delay).await;
                continue;
            }
        }
        break result.map_err(|_| "Maxxit cloud is unavailable. Local analytics still work.")?;
    };
    let status = response.status();
    const LIMIT: usize = 2_000_000;
    if response
        .content_length()
        .is_some_and(|size| size > LIMIT as u64)
    {
        return Err("Service response exceeds its limit".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Invalid service response")?
    {
        if bytes.len() + chunk.len() > LIMIT {
            return Err("Service response exceeds its limit".into());
        }
        bytes.extend_from_slice(&chunk);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    struct Fake {
        value: RefCell<Option<String>>,
        denied: bool,
        readback_fails: bool,
        writes: RefCell<Vec<String>>,
    }
    impl Credentials for Fake {
        fn read(&self) -> Result<Option<String>, String> {
            if self.denied || (self.readback_fails && self.writes.borrow().len() == 1) {
                return Err("Access denied".into());
            }
            Ok(self.value.borrow().clone())
        }
        fn write(&self, value: &str) -> Result<(), String> {
            self.writes.borrow_mut().push(value.into());
            *self.value.borrow_mut() = Some(value.into());
            Ok(())
        }
        fn remove(&self) -> Result<(), String> {
            *self.value.borrow_mut() = None;
            Ok(())
        }
    }
    fn fake(denied: bool, readback_fails: bool) -> Fake {
        Fake {
            value: RefCell::new(Some("previous".into())),
            denied,
            readback_fails,
            writes: RefCell::new(vec![]),
        }
    }
    #[test]
    fn transient_read_retries_are_bounded_and_never_retry_auth_rejections() {
        assert_eq!(retry_delay(0, Some(503), 10).unwrap().as_millis(), 1010);
        assert_eq!(retry_delay(1, Some(429), 10).unwrap().as_millis(), 2010);
        assert!(retry_delay(0, None, 10).is_some());
        assert!(retry_delay(2, Some(503), 0).is_none());
        for status in [200, 400, 401, 403, 404] {
            assert!(retry_delay(0, Some(status), 0).is_none());
        }
    }
    #[test]
    fn unreadable_credentials_are_never_overwritten() {
        let store = fake(true, false);
        assert!(save_verified(&store, &"n".repeat(43)).is_err());
        assert!(store.writes.borrow().is_empty());
    }
    #[test]
    fn failed_readback_restores_previous_value() {
        let store = fake(false, true);
        assert!(save_verified(&store, &"n".repeat(43)).is_err());
        assert_eq!(store.value.borrow().as_deref(), Some("previous"));
    }
    #[test]
    fn successful_write_verifies_the_complete_value() {
        let store = fake(false, false);
        let token = "n".repeat(43);
        save_verified(&store, &token).unwrap();
        assert_eq!(store.read().unwrap(), Some(token));
    }
    #[test]
    fn cloud_origin_rejects_untrusted_destinations() {
        for origin in [
            "https://maxxit.app.evil.test",
            "https://user@maxxit.app",
            "https://maxxit.app/private",
            "https://maxxit.app?token=x",
            "http://maxxit.app",
        ] {
            let settings = Settings {
                api_origin: origin.into(),
                ..Settings::default()
            };
            assert!(super::origin(&settings).is_err(), "{origin}");
        }
    }
}
