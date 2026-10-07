# Desktop diagnostics with Sentry

Sentry reporting is optional and off on first launch. Settings > Diagnostic reporting controls it separately from Maxxit cloud, usage sharing and project sharing. The committed DSN points to the owner's EU Sentry project. A DSN is a public ingestion key, not an administrative credential.

## Implemented checklist

- [x] Pin React and Rust SDKs, initialize before database startup, initialize both WebViews before React render, and capture React boundary, uncaught exception, rejection and Rust panic failures.
- [x] Wrap native commands and background operations. Return a report ID through IPC to prevent React from reporting the same native failure twice. Independent unexpected occurrences are captured at 100 percent.
- [x] Keep operation histories isolated with thread and Tokio task scopes. Attach preceding records inside each error as breadcrumbs and `extra.diagnostic_history`. Logs are independent of the error's history.
- [x] Report storage failures and corrupt rows, provider read/parse/normalization failures, bridge capture/child failures, Keychain failures, cloud retries/status/response failures, updater stages and UI listener failures.
- [x] Include release and source commit, environment, macOS version, architecture, boot and operation IDs, stable error codes, capture-boundary backtraces and safe source locations/codes where available. Never upload raw error messages.
- [x] Use native IPC forwarding under the existing packaged CSP. The browser SDK processes stacks and returns `null` from `beforeSend`; it has no direct network transport to Sentry.
- [x] Persist sanitized envelopes before delivery. Retry transport failures and 5xx with exponential backoff and jitter; handle 429, Retry-After and category rate limits. Discard permanent rejections visibly.
- [x] Bound the outbox to 20 MiB, 2,000 envelopes and seven days. Prioritize fatal reports. Use a private directory and database, show pending/age/drop/delivery health, and preserve event IDs across retries.
- [x] Recheck consent at capture, persistence and delivery. A generation prevents delayed IPC callbacks from replaying events captured under previous consent. Every consent change clears queued envelopes and history.
- [x] Provide a synthetic test action with event ID, support preview and explicit JSON export. Keep endpoint acceptance separate from dashboard verification.
- [x] Capture supported native crashes in a separate reporter, with a separate informed opt-in for memory-bearing minidumps. Reporter startup skips normal app setup. Synchronize safe operation tags and breadcrumbs.
- [x] Generate and inject source-map Debug IDs before Tauri embeds assets. Remove maps from the shipped `dist`. Preserve release manifests and native debug information; gate release creation on private upload success.
- [x] Flush one envelope with a three-second request timeout on normal exit and before updater relaunch. Remaining envelopes stay queued.
- [ ] Owner: configure email alerts and quotas, and verify a real notification.
- [ ] Owner: provide release upload credentials and verify symbolication on signed installed Apple Silicon and Intel builds.

## What is recorded

Ordinary errors contain fixed error codes instead of third-party strings. Attributes are allowlisted numeric counts, timings and OS/SQLite codes; fixed provider, command, stage, method and route names; and random correlation IDs. Up to 50 relevant records accompany an error. The persistent support history keeps the latest 100 records. Console messages, DOM, requests, response bodies, user identity, settings, projects, prompts, transcripts, SQL parameters and arbitrary extras are excluded. Stack filenames contain build filenames, not home directories. Browser assets use `app:///assets/` names with matching Debug IDs.

App start, ready, shutdown, previous unclean exit, pairing, bridge installation, cloud retries/completion, update stages and recovery produce searchable Logs. Routine command success and provider collection remain breadcrumbs. Each terminal error Log carries the error event ID and operation ID. Later successful commands can record a recovery linked to the preceding error. Sentry transport failures stay local to prevent recursion.

The native crash reporter uses the most recently synchronized operation as context. Concurrent operations still have isolated ordinary error histories, but native process crashes cannot identify which concurrent operation caused a signal with certainty.

## Configuration

`sentry.config.json` holds the public DSN. `MAXXIT_SENTRY_DSN` overrides it at compile time; an empty value builds without a remote destination. `MAXXIT_SENTRY_ENVIRONMENT` accepts `development`, `staging` or `production`. Debug builds default to development; release builds default to production. Errors remain queued when reporting is enabled but no valid HTTPS DSN is configured. No credential is required to run checks or build locally.

Release upload needs these environment variables outside the app:

```sh
export SENTRY_URL=https://de.sentry.io/
export SENTRY_ORG=your-organization-slug
export SENTRY_PROJECT=your-project-slug
# Set SENTRY_AUTH_TOKEN through your secret manager or CI secrets.
pnpm sentry:upload /absolute/path/Maxxit.app/Contents/MacOS/maxxit
```

Use an organization token authorized for release and debug-file uploads. Never use a VITE variable for it. `pnpm build` injects Debug IDs and preserves JS/maps plus a digest manifest under ignored `release-artifacts/sentry/<release>/`. Build the native app from those exact assets. Upload generates a dSYM from the signed binary, validates debug files, waits for upload processing, validates source maps, and records the release commit. Release builds retain DWARF with `debug = 1`. Preserve the native target objects until `dsymutil` completes. Build universal symbols for the universal binary; do not substitute symbols from an arm64-only local build.

`scripts/release.sh` invokes this upload gate before creating the GitHub release draft. Dry runs remain offline and do not prove symbol upload or dashboard processing. Rebuild after committing: the source revision changes both release identity and artifact directory.

## Owner setup and notifications

In the Sentry project, restrict notifications to production. Add issue alerts for new issues and regressions with an email action to yourself. Add a fatal/startup/database/update-verification rule using `event_name` and `error_code` tags. Group frequent provider parsing and cloud sync notifications by issue rather than emailing every poll. Include the issue link and inspect its release, operation ID and history. Send a test event and verify that the email arrives. The DSN alone cannot configure alert rules or prove delivery to your inbox.

Check the account's current error, Logs and attachment quotas and retention. Set usage warnings where supported, or check usage manually. Keep tracing and replay off initially. The app shows rate limiting and discarded reports, but cannot read account quota usage with a public DSN. Error history remains in the error payload even when Logs are limited.

## Verification and limitations

`pnpm check` runs ordinary checks offline apart from dependency tooling. Rust transport fixtures use loopback HTTP; privacy fixtures use recognizable private markers. The explicitly ignored `sentry_ingestion_smoke` test sends only synthetic data to the configured project. Run it deliberately with:

```sh
cargo test --locked --manifest-path src-tauri/Cargo.toml sentry_ingestion_smoke -- --ignored --nocapture
```

An accepted envelope is not proof of processed, symbolicated data. Search the printed event ID in Sentry and confirm history and alerts. For native subprocess probes, build the debug binary and use `--diagnostics-probe error|panic|abort /absolute/temporary/directory`. These probes bypass all provider, Keychain and normal app setup. They persist locally and do not upload. Remove the temporary directory afterward. Probes do not exist in release builds.

Reporting cannot cover errors before diagnostics storage initializes, a full or inaccessible disk, a WebView failure before its IPC message persists, force quit/SIGKILL, all WebKit process crashes, or arbitrary hangs. The WebView keeps a bounded 100-item memory retry queue until native acceptance; it is lost if that process terminates and can drop oldest entries when full. Native dumps share the 20 MiB envelope budget, so oversized dumps are discarded and counted. Dumps may contain private memory; ordinary redaction cannot sanitize them. Native capture requires restart when enabled and must be verified on each supported signed architecture. Segmentation faults and stack overflow are not claimed verified by the abort probe.

Turning reporting off removes queued diagnostics and history. It does not delete data already accepted by Sentry. Delete uploaded data through the Sentry project controls. Support exports contain safe history and delivery metadata, never dump bytes, and remain wherever the user saves them.
