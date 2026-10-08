# Changelog

## 0.1.11 October 8 2026

- Require browser account approval before desktop and tray data access. Preserve seven-day offline access for a verified account.
- Isolate encrypted account databases and offer reviewed import of earlier local data.
- Mirror consented usage, analytics, projects, preferences, and workflows with durable revisions and deletion markers. Keep web records read-only and AI consent separate.
- Attach Pro checkout to the verified account and display the current entitlement.

- Add local agent prompt handoff, validated suggestion/task report imports, SQLCipher storage migration, and optional generic completion notifications. See docs/local-workflow.md for recovery and release acceptance limits.

- Keep recorded allowance visible when stale, with its age and a last-reading label. Expired and missing readings remain unavailable.
- Add repository contribution, security, triage, privacy, and release procedures.
- Expand CI with pinned tools, formatting, linting, security checks, and native packaging.
- Restrict the tray's native command permissions.
- Verify credential writes, preserve unreadable storage, and guard local database upgrades.
- Fail installation on untrusted or unavailable release artifacts.
- Bind releases to reviewed revisions and validate complete signed distributions.

## 0.1.10 October 7 2026

- Add automatic update checks, installation progress, retry controls, and manual checks.
- Publish a universal macOS installer and signed Tauri update archive.
- Versions through 0.1.9 need the DMG installer once to receive in-app updates.
- Existing local settings, projects, and observations retain their data directory.

Earlier source tags have no consolidated changelog. See their commit history.
