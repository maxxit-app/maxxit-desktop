# Account login and desktop authority

Desktop 0.1.11 requires browser account approval before local collection, workflows, exports, and account data access. Rust enforces the gate alongside the React screen and tray. Clerk authenticates the browser; a short-lived pairing secret exchanges its approval for a revocable device credential in Mac Keychain. The renderer receives the approval code and account identity, never the credential.

First login requires a successful `desktop/state` verification. Previously verified credentials permit local use for seven days after the last verification while offline. Sign-out locks local access immediately, removes the active credential, and queues remote revocation for the background worker. A known 401 locks immediately. Other client errors, including a required update, lock access. Network and service failures preserve the last verified plan without claiming fresh billing authorization.

Each account uses `accounts/<sha256-account-id>.sqlite` with SQLCipher. The original `maxxit.sqlite` remains intact. Connections offers a reviewed import of earlier projects, history, and workflows into the current account. The earlier store is claimed by that account, imports preserve newer projects, and historical completion notices are cancelled. Existing web-only projects remain labelled legacy records. They are not silently merged with local projects.

## Sync contract

`POST desktop/sync` accepts schema version 1, a device epoch, and up to 50 operations. Each operation contains `kind`, `recordId`, `revision`, and `body`. A null body is a deletion marker. The stable operation identity is the authenticated device plus kind, record ID, and revision. Batches stay below 400 KB on the desktop and 512 KB at the API.

SQLite triggers write the record and pending operation atomically. The backend derives account and device identity from authorization and locks the user/device before checking consent, epoch, and revocation. Acknowledgements follow transaction commit. Retrying a revision with equal content is safe; conflicting content returns 409. Older revisions cannot replace newer records or deletions. The desktop clears only the exact acknowledged revision, preserving a newer edit made during upload. A restored older backup cannot overwrite a newer server revision; recovery should use the latest account backup or reviewed record recreation.

| Kind        | Browser approval scope | Mirrored fields                                                                 |
| ----------- | ---------------------- | ------------------------------------------------------------------------------- |
| observation | usage                  | Provider, account label, observed time/source, allowance windows                |
| analytics   | usage                  | Observed daily token totals for the last 90 days and scope/completeness         |
| project     | metadata               | Stable ID, name, description, creation time, archive/pin state                  |
| preferences | accountSync            | Theme, generation mode, timezone, reminder thresholds, quiet hours, daily limit |
| run         | workflowDetails        | Idea/task state, reviewed prompt, suggestions, imported report                  |

All scopes default off. Login works with every scope off. These scopes do not authorize hosted AI, completion relay, or paid features. Workflow sharing uploads reviewed text and can include sensitive paths/context supplied by the user. The backend mirror is not end-to-end encrypted. Provider credentials, raw provider sessions, executable paths, autostart, and native permissions are excluded from ordinary sync.

Each Mac owns its records. The first connected Mac owns shared preferences. Secondary Macs read that acknowledged preference record and cannot overwrite it through native or web APIs. Transfer the role explicitly in web Connections. Account identity, verified email, consent withdrawal, revocation, billing, hosted ideas, and notifications remain server-owned. Web views of desktop-owned records are read-only.

The desktop and web refresh every five seconds while active, with one native sync worker. Failed background uploads back off up to 60 seconds with jitter; focus and manual refresh can retry immediately. Local changes enter the durable queue immediately and catch up after reconnect. Transport success and provider observation freshness are separate. The web preserves last successful data on failures and displays original observation times. Local observation/run pruning emits deletion markers, so the web mirror follows local retention after reconnect. Server-owned results are cached in the encrypted account session for offline display.

Deleting this device's cloud copies clears its mirror and usage snapshots, increments its epoch, removes its project copies and derived hosted ideas, and disables local upload categories. Re-enabling approved sharing is an explicit choice. Previously accepted server records and deletion markers remain revisioned; do not reset revisions or epochs manually.

## Rollout and verification

Deploy backend migration `20261008130000_desktop_account_sync`, then the API and web, then desktop 0.1.11. `MIN_DESKTOP_VERSION` defaults to 0.1.11 for protocol-1 remote requests. Older clients retain protocol-0 compatibility during the upgrade window. Older offline binaries cannot be forced to require accounts by changing the server; replace the distributed desktop binary and announce the login requirement.

Back up PostgreSQL and the account databases before rollout. Keep the added schema, account directories, encryption keys, revision records, and tombstones during application rollback. An older desktop may reopen its untouched legacy database, but cannot read new owner-bound files through its old UI. Do not publish a rollback binary as a way to bypass login.

Automated checks use synthetic records, in-memory SQLCipher stores, and a disposable PostgreSQL database. Before production release, verify a signed macOS build with real browser signup/login, Keychain denial/recovery, account switching, checkout with a different browser account, delayed payment webhooks, two Macs, sleep/wake, and a timed desktop-to-web change. Automated tests do not prove those live integrations or the 15-second online acceptance target.
