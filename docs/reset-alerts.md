# Reset alerts

Maxxit records changes in allowance and reported reset times. A 20-point percentage drop needs a second genuinely newer reading before Maxxit calls it an unexpected increase. It does not establish the cause or predict an unannounced reset. A normal reset crossing, stale capture, different account, incompatible source or duration change establishes a baseline instead.

Open Settings to choose providers and event types, desktop notices, lead times, timezone, quiet hours and a daily limit. Email requires Pro, usage sharing and an explicit email choice. Shared email and event choices follow the verified account. Local changes remain pending until the service accepts them. Email does not depend on macOS notification permission or AI generation consent.

The primary Mac shows native reset notices. Each logical local event has a stable encrypted record and persistent delivery state. The native daily budget survives restarts. Native submission is not proof that the OS displayed it. The notification plugin cannot reliably query macOS permission, so manage delivery in System Settings > Notifications. The history remains available if delivery fails. Remote reviewed announcements and user-confirmed offers appear after a fresh account refresh and do not need a local usage reading.

Scheduled and observed notices need evidence at most two hours old. Late reminders expire instead of claiming fresh allowance. Backend scheduling can send an eligible email while the Mac is closed; new observed changes need passive evidence and an accepted usage upload. Collection normally polls about every five seconds, with bounded backoff after failures. The polling time does not refresh an old capture's observation time.

Codex records supply a full SHA-256 account identity. Display labels and shortened hashes do not group observations. Claude's statusline does not establish account identity or free-reset offers. Reconnect Claude after switching accounts. Reconnection gives the source a new epoch and waits for a capture newer than disconnection.

The overview shows recent events and their dates. Ideas opens reviewed, user-started prompts. Local analysis receives recent reported windows and requests task sizes that fit before the earliest applicable reset; unknown limits cannot promise a token budget. Generic reviews and regression tests remain available when no model output exists. Work never starts from an alert automatically. Confirm offers and mark them redeemed in the web workspace after checking the provider. Maxxit never redeems a reset itself.

## Storage and compatibility

Schema 4 adds reset watches and an event ledger in the existing encrypted account database. Recording an observation, its sync revision, watch state and events is atomic. Local history retains at most 500 events for 90 days. Only consented observation summaries sync; credentials, raw session text and filesystem paths remain local. The service derives its own events from those observations.

Observation additions are optional providerAccountId and sourceVersion. Shared preferences add resetAlerts. Server event envelopes use schemaVersion 1. Deploy a service supporting these additive fields before distributing this desktop build. Keep the older supported-client tests in service staging.

For rollback, keep the encrypted database and its Keychain key. Earlier binaries reject schema 4. Use a schema-4-compatible recovery build; do not change user_version or delete delivery history to force an old application to open it.

## Acceptance

Synthetic tests cover confirmation, normal rollover, stale readings, persisted deduplication and budgets after restart, cloud withdrawal, account storage and sync replay. Release still needs a signed native build checked with macOS delivery allowed and denied, verified email delivery while closed, and provider approval before automatic offer collection. Native and service daily budgets are separate so offline alerts remain possible.
