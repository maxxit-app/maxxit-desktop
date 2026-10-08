# Local agent workflow

The desktop prepares prompts for the user's own Codex or Claude. It never launches a provider, reads provider login credentials, executes a suggestion, or exposes its device upload token to the model. This release supports manual prompt handoff and JSON imports. Automatic adapters, a managed executor, repository reading, conversation extraction remain unavailable. Detailed workflow sync is separately opt-in.

## Using the workflow

1. Add a project description in Projects. Include only information you want your agent to receive.
2. Keep Settings > Suggestion provider set to My own Codex or Claude. Allow the local handoff disclosure in Ideas.
3. Prepare an analysis prompt. Review it, then copy or save it. Use a fresh session with tools disabled in your own agent. Maxxit's instructions are advisory; the agent's configuration can load additional context or tools.
4. Paste the agent's JSON into that run. Import validates the schema and originating run ID before changing local records.
5. Review a suggestion and select it to prepare a separate task prompt. Selection does not execute anything. Start it yourself in your agent, using its normal permissions.
6. Import the task's JSON completion report. Maxxit labels this as an agent report; it does not verify edits or tests. Save local result exports if needed.

Manual workflow use requires a verified Maxxit account. It can work offline for seven days after verification. Codex or Claude may require network access and process the pasted text in the provider's cloud. Their retention rules and your installed tool configuration apply separately.

## Data contract

| Data                                                                                                        | Where it goes                                                                                          | Retention                                                                                  |
| ----------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------ |
| Typed project ID, name, description, creation time                                                          | Encrypted local database. The selected project is included in the reviewed analysis prompt.            | Until project deletion                                                                     |
| Run ID, kind, parent ID, project identity, prompt, imported suggestions, report, timestamps, delivery state | Encrypted local database, React memory while displayed, explicit plaintext exports or clipboard copies | 90 days; at most 500 retained runs                                                         |
| Provider login                                                                                              | Handled by the user's official tool. The workflow does not read or proxy it.                           | Provider rules                                                                             |
| Completion schemaVersion, random eventId/runId, fixed kind, occurredAt                                      | Encrypted outbox, then authenticated Maxxit API only when relay is enabled                             | Outbox expires after 24 hours; server events and their generic notices expire after 7 days |
| Usage windows and derived account labels                                                                    | Local database; optional independent allowance upload                                                  | Local observations 90 days; hosted policy applies to uploaded observations                 |
| Device token / database key                                                                                 | Separate native Keychain entries; neither is model-visible or exported                                 | Until explicitly removed                                                                   |
| Generic notice and verified account email                                                                   | Maxxit; Resend if opted-in email is configured                                                         | Server notice 7 days; vendor policy applies                                                |
| Diagnostics                                                                                                 | Version, OS, architecture, installation channel only                                                   | User controls exported diagnostic files                                                    |

Completion payloads have exactly five fields. They cannot include titles, paths, excerpts, report summaries, destinations, or project references. The server derives owner and device from the bearer token. Imported text is rendered as text, never HTML or executable links.

The input is restricted to a selected typed project. The workflow does not scan repositories or histories, so filesystem discovery and evidence expansion are unavailable. Output permits at most 10 suggestions, title 120 bytes, description 2,000 bytes, prompt 10,000 bytes, task summary 10,000 bytes, total JSON 100,000 bytes. Unknown fields and mismatched IDs fail before writing. Each result and its optional event are committed together.

## Sharing choices

Local handoff, usage sharing, native notices, completion relay, and hosted project sharing are separate choices. New handoff and relay choices default off, including for users with old project-sharing consent. Local mode is the default desktop mode. It never calls hosted generation. Browser-approved project and workflow mirroring can operate in local mode without allowing model processing.

Sign in to a Maxxit account with the completion-notification scope approved in the browser. Pro is required for server delivery; local workflow use remains available without it. Email uses the existing verified-recipient and notification preferences. In-app delivery still works with email off. Quiet hours, snooze, daily limits, expiry, revocation, and entitlement apply. Notices disclose only status and direct users to this Mac for details.

Switching generation mode updates the account's server preference after a successful sync. Local mode clears pending hosted generation and prevents future hosted model work and result persistence. Another device can explicitly change this account-wide preference. A mode change cannot recall an already submitted provider request. Offline changes stop local uploads immediately; server cancellation takes effect when the change reaches the server. Old cloud copies remain until explicitly deleted.

Settings has a separate deletion action naming its scope: this device's cloud projects and all hosted ideas for the account. Local data stays available. Revoking relay cancels unsent outbox records; server scope withdrawal or device revocation cancels pending server events. An authorization or scope rejection also disables local relay and cancels its outbox; reconnecting cannot restore those cancelled events. A notice already sent cannot be recalled. Deleting a local run removes it, its task runs, and queued local events. It does not retract an event already accepted by the server.

## Encryption and recovery

SQLCipher is bundled with vendored OpenSSL. Sensitive SQLite pages, WAL and journals use SQLCipher; temporary data uses memory. The database key is a random 64-character hex secret generated from two OS-random UUIDs and stored as app.maxxit.desktop / database. It is separate from app.maxxit.desktop / device. The app data directory is restricted to the user.

Existing plaintext SQLite is checkpointed into DELETE journal mode, exported to a keyed encrypted-next file, checked for integrity and valid preferences, then atomically replaced through a plaintext-backup rename. Successful migration removes the plaintext backup. Erasing a file is not a guarantee of physical erasure on an SSD or in older system backups.

An unavailable Keychain item, wrong key, newer schema, corrupt preferences, or migration remnants stops opening without creating empty replacement records. Unlock Keychain and retry. An absent key for an existing encrypted database requires recovery of that original key. Reinstalling the app does not generate a replacement key for unreadable data.

For an encrypted backup, quit Maxxit before copying the database and back up its original Keychain item using your encrypted system backup. The app does not export the encryption key or provide independent key escrow. Restoring database bytes alone cannot recover a lost key. Keep explicit plaintext exports only in a location you trust; Maxxit does not clear system clipboard history or delete user exports.

For interrupted migration, quit Maxxit and preserve the original database, plaintext-backup, encrypted-next, and any WAL/journal files. Recover from a verified encrypted system backup, or have a maintainer inspect fixture copies and finish the replacement. Never delete remnants or generate a new key just to bypass the startup error. The same installed key can reopen a verified encrypted database restored under its original path.

## Acceptance evidence

Native fixture tests cover encryption migration/reopen, wrong-key preservation, interrupted migration preservation, invalid imports, account-bound events, consent cancellation, separate task runs, and project/run deletion. Backend tests cover schema/authentication, replay conflicts, local-mode cancellation, cloud-copy deletion, consent/revocation, delivery limits, expiry, and email retries. The browser preview verifies layout only.

Before release, use a disposable signed macOS app and test account to complete a real provider analysis and user-started task, test locked Keychain and restore, notification denial and fallback, sleep/wake/offline retry, and real configured email delivery. Never use ordinary CI to touch provider accounts or actual Keychain entries. No automatic provider adapter is claimed supported by these fixture tests.
