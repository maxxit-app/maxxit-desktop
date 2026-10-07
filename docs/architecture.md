# Desktop architecture

Maxxit is a Tauri 2 application. React renders the main window and a separate tray
panel. The Rust command layer owns provider reads, SQLite, Mac Keychain, the
Claude bridge, approved URL opening, and HTTPS requests. The renderer receives
sanitized observations, projects, settings, and hosted state.

AppManifest lists every registered command. The main capability permits the
desktop commands; the tray capability permits snapshot, navigation/quit, resize,
and local events. Remote origins are not granted native access. Plugin update
installation and restart are main-window permissions.

providers.rs reads bounded Codex rollout records and the Claude bridge capture.
model.rs normalizes independent allowance windows without inventing missing data.
storage.rs maintains SQLite schema version 1, rejects newer schemas, and runs
initialization in a transaction. cloud.rs constrains the service origin, disables
redirects, bounds response reads, and keeps device credentials in Keychain.

The app polls locally once a minute, including while its main window is closed.
The TypeScript update controller exposes checks, progress, retry, and restart
after explicit user actions. Tauri verifies signatures natively.

See cloud-contract.md for the public client contract, privacy.md for data flow,
threat-model.md for trust boundaries, and releasing.md for binary distribution.
