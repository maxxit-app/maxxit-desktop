# Desktop threat model

Protect Maxxit device credentials, local observations/projects, provider files,
Claude configuration, update trust, and user consent.

| Boundary                          | Threat                                                                                           | Control and verification                                                                                                                                                       |
| --------------------------------- | ------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Renderer to Rust                  | A compromised view requests a native mutation.                                                   | Named command permissions, separate tray capability, CSP, and input checks. Tooling tests verify the permission boundary; native acceptance must confirm rejection at runtime. |
| Provider files to parser          | Malformed, huge, future-dated, cross-account, or symlinked records distort usage or expose data. | Bounded scans/reads, account grouping, timestamp checks, field normalization, and synthetic provider fixtures.                                                                 |
| Claude settings to bridge         | A settings change or unreadable backup gets overwritten.                                         | Preview, atomic writes, unreadable-backup rejection, matching-command restoration, and approved configuration path. Symlinked files are rejected.                              |
| Local SQLite to app               | Corrupt preferences or a newer schema are silently replaced.                                     | Transactional schema initialization, readable preferences, and newer-schema rejection. Restore a verified backup rather than resetting automatically.                          |
| App to Keychain                   | An access error is mistaken for missing data or a new token fails to persist.                    | Separate absence/read failure, serialized access, full write readback, and restoration of the prior value. Tests use fake stores.                                              |
| App to hosted service             | Credential leakage through a redirect or unapproved destination.                                 | Fixed HTTPS origin, no redirects, native token attachment, bounded streaming responses, and URL validation tests.                                                              |
| Release infrastructure to install | Changed assets, a wrong signing team, or a substituted archive installs.                         | Protected tags, immutable releases, Apple Team ID checks, Tauri signatures, complete checksums, and public artifact verification.                                              |
| Reports to public GitHub          | Diagnostics reveal credentials, paths, or user content.                                          | Metadata allowlist, private support/security routes, synthetic screenshots, and manual review before attachment.                                                               |

A user-controlled existing Claude status-line command is deliberately executed to
preserve its output. Maxxit does not grant the renderer arbitrary command execution.
A malicious process already running as the same macOS user can change user files;
Maxxit's checks do not claim protection from a fully compromised account.

Review these boundaries when changing providers, capabilities, filesystem paths,
cloud contracts, storage, or signing configuration. Keep user secrets out of tests.
