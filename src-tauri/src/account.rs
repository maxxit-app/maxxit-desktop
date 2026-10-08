use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const OFFLINE_SECONDS: i64 = 7 * 86400;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Account {
    pub status: String,
    pub account_id: Option<String>,
    pub device_id: Option<String>,
    pub verified_at: i64,
    pub credential_hash: String,
    pub cloud: Value,
    pub error: Option<String>,
}
impl Default for Account {
    fn default() -> Self {
        Self {
            status: "signed_out".into(),
            account_id: None,
            device_id: None,
            verified_at: 0,
            credential_hash: String::new(),
            cloud: json!({"connected":false,"plan":"unknown"}),
            error: None,
        }
    }
}
impl Account {
    pub fn allows_local(&self, now: i64) -> bool {
        ["authenticated", "offline"].contains(&self.status.as_str())
            && self.account_id.as_ref().is_some_and(|id| !id.is_empty())
            && self.verified_at <= now
            && now - self.verified_at < OFFLINE_SECONDS
    }
    pub fn verified(data: Value, fingerprint: String, now: i64) -> Result<Self, String> {
        let account_id = data["identity"]["accountId"]
            .as_str()
            .ok_or("Update the Maxxit service before signing in")?
            .to_owned();
        let device_id = data["identity"]["deviceId"]
            .as_str()
            .ok_or("Invalid device identity")?
            .to_owned();
        if account_id.is_empty()
            || device_id.is_empty()
            || data["account"]["id"].as_str() != Some(account_id.as_str())
        {
            return Err("Account identity did not match".into());
        }
        Ok(Self {
            status: "authenticated".into(),
            account_id: Some(account_id),
            device_id: Some(device_id),
            verified_at: now,
            credential_hash: fingerprint,
            cloud: json!({"connected":true,"plan":data["billing"]["plan"],"data":data}),
            error: None,
        })
    }
    pub fn unavailable(mut self, fingerprint: &str, now: i64, error: String) -> Self {
        self.status = if self.credential_hash == fingerprint
            && self.account_id.as_ref().is_some_and(|id| !id.is_empty())
            && self.verified_at <= now
            && now - self.verified_at < OFFLINE_SECONDS
            && ["checking", "authenticated", "offline"].contains(&self.status.as_str())
        {
            "offline"
        } else {
            "error"
        }
        .into();
        self.error = Some(error);
        self
    }
    pub fn public(&self) -> Value {
        json!({"status":self.status,"accountId":self.account_id,"deviceId":self.device_id,
            "verifiedAt":self.verified_at,"offlineUntil":self.verified_at+OFFLINE_SECONDS,"error":self.error})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn account() -> Account {
        Account::verified(json!({"account":{"id":"A"},"identity":{"accountId":"A","deviceId":"D"},"billing":{"plan":"pro"}}),"credential-A".into(),100).unwrap()
    }
    #[test]
    fn offline_admission_requires_verified_identity_and_matching_credential() {
        assert!(!Account::default().allows_local(100));
        let cached = account().unavailable("credential-A", 101, "Offline".into());
        assert!(cached.allows_local(101));
        assert_eq!(cached.cloud["plan"], "pro");
        assert!(!cached.allows_local(100 + OFFLINE_SECONDS));
        assert!(!account()
            .unavailable("credential-B", 101, "Offline".into())
            .allows_local(101));
        assert!(!account()
            .unavailable("credential-A", 99, "Clock moved back".into())
            .allows_local(99));
    }
    #[test]
    fn mismatched_server_identity_is_rejected_and_public_state_contains_no_secrets() {
        assert!(Account::verified(
            json!({"account":{"id":"B"},"identity":{"accountId":"A","deviceId":"D"}}),
            "secret".into(),
            100
        )
        .is_err());
        assert!(!account().public().to_string().contains("credential-A"));
        let mut revoked = account();
        revoked.status = "revoked".into();
        assert!(!revoked.allows_local(101));
    }
}
