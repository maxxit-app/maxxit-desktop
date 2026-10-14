# Product analytics

Google Analytics is separate from the local token analytics shown inside Maxxit and
from Sentry error reporting. Product analytics is off until the user allows it.
The website and Mac app use separate streams in one GA4 property owned by
contact@maxxit.app. The Mac stream uses Measurement Protocol through the
Maxxit backend. No Google secret is shipped in the app.

The client sends fixed screen names, operation names and success/failure events,
app version, main/tray window, session ID, random analytics ID, and bounded engagement
time. Project content and paths, Maxxit account IDs, emails, prompts, conversations,
provider data, and credentials never enter the Google payload. The backend requires
a non-revoked device token, validates the payload, and limits delivery to 600 events
per device per hour. Ads consent is denied. The Tauri script and network policies
remain unchanged.

Settings exposes consent and the latest ingest delivery state. Withdrawal removes
the analytics identity and queued frontend events. Native consent and delivery are
serialized, so an in-flight request can finish before withdrawal completes. A later
opt-in creates a new identity. Events are not retried after transport failure and are
not stored offline. Sessions expire after 30 minutes without activity. The frontend
queue holds at most 20 events. These limits mean the reports are a usage overview,
not an audit log of every action.

## Release checks

Before publishing, configure the private service's desktop stream ID and secret,
validate a synthetic payload with Google's debug endpoint, and confirm an opted-in
installed app in GA4 Realtime. Use event dimensions app_platform, app_version,
window_name, action and screen_name to compare app use and feature outcomes.
Dashboard receipt must be verified against the live property for each release.
A successful collect response confirms ingest acceptance only.
