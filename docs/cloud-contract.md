# Desktop cloud compatibility

The production origin is https://maxxit.app with /v1 API paths. Release builds
reject alternative origins, credentials in URLs, query strings, fragments, and
redirects. Debug builds permit localhost for an isolated fixture server.

The device token is a bearer credential held only by Rust/Mac Keychain.
It is never returned to the renderer. Pairing uses a short approval code and
a separate secret; the hosted service enforces expiry and single redemption.

| Endpoint                                 | Client behavior                                                                                       |
| ---------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| POST pairing/start                       | Sends device name and usage/metadata consent; never consents to excerpts.                             |
| POST pairing/redeem                      | Sends the pairing secret; verifies the returned token's shape before storing it.                      |
| POST collector/disconnect                | Revokes the device before deleting the local credential and sharing preferences.                      |
| GET desktop/state                        | Retrieves entitlement, suggestions, and availability state. Local analytics remain usable on failure. |
| POST desktop/usage                       | Sends an observed allowance snapshot only with usage consent.                                         |
| POST desktop/project                     | Sends typed project metadata only with project sharing enabled.                                       |
| POST desktop/preferences                 | Sends explicit reminder, timezone, email, and processing preferences.                                 |
| POST desktop/checkout and desktop/portal | Gets an approved browser destination. A checkout redirect never grants Pro access.                    |
| POST desktop/generate                    | Requests consented ideas; generated work is never executed by the desktop.                            |

Responses use JSON, a 15-second request timeout, and a two-megabyte streaming limit.
The private backend remains authoritative for ownership, consent, revocation,
billing, generation limits, and email. The renderer cannot choose arbitrary API paths.

Additive fields must remain compatible with supported desktop versions.
Coordinate breaking changes before changing a request or removing a field.
Version a new contract when needed and retain the previous supported contract
through at least one announced desktop upgrade window. Test the oldest supported
desktop against staging before a hosted deployment. Backend fixture/integration
tests and production credentials stay in the private repository.

The account-sync contract in [account-sync.md](account-sync.md) replaces current-state project uploads for desktop 0.1.11. Protocol-1 clients use `desktop/sync` and `desktop/privacy`, require verified login, and explicitly approve usage, projects, shared preferences, and detailed workflows. `desktop/preferences` changes server-owned email, AI, and reminder controls only. Sign-out locks locally before queued server revocation. Legacy endpoints remain for the documented upgrade window.
