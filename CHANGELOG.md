# Changelog

## 0.1.18 October 14 2026

- Measure a consented activation milestone when a connected provider first supplies a fresh, usable allowance window.
- Check the observation in native account storage and deduplicate the milestone per random analytics identity. Provider names, allowance values, account labels and source records remain excluded from Google's payload.
- Preserve separate sharing, diagnostics and product analytics choices. Browser approval and download clicks remain separate from activation.
- Support macOS 14 or later on Apple silicon and Intel. Physical Intel and macOS 14 runtime acceptance remain unverified.

## 0.1.17 October 14 2026

- Add optional Google Analytics product measurement for desktop screens, feature actions, update outcomes and engagement. Analytics starts off until you allow it.
- Keep project content, paths, prompts, emails, account IDs and credentials out of the analytics payload. Store Google credentials only on the Maxxit service.
- Add a consent choice and delivery status in Settings. Turning analytics off discards queued events and removes its random identifier.
- Support macOS 14 or later on Apple silicon and Intel. Physical Intel and macOS 14 runtime acceptance remain unverified.

## 0.1.16 October 13 2026

- Detect reported deadline changes after an overnight gap while the previous deadline is still in the future.
- Confirm smaller allowance increases when fresh readings also show a changed deadline. Preserve normal rollover, account isolation, freshness checks and duplicate suppression.
- Keep prior baselines when an upload is stale or incomplete.
- Support macOS 14 or later on Apple silicon and Intel. Physical Intel and macOS 14 runtime acceptance remain unverified.

## 0.1.15 October 12 2026

- Add optional Sentry diagnostic reporting for React, tray, Rust commands, background operations and supported native crashes. Reporting starts off and native memory dumps need separate consent.
- Preserve sanitized reports while offline, show delivery health, and provide a test report and reviewed support export in Settings.
- Upload matching JavaScript source maps and universal native symbols privately before publishing a release.
- Preserve account access checks, encrypted storage, local workflows and reset alerts.
- Support macOS 14 or later on Apple silicon and Intel. Physical Intel, macOS 14 and all WebKit crash paths remain unverified.

## 0.1.14 October 8 2026

- Add durable reset-change detection, event history, provider controls and primary-Mac desktop notices.
- Share additive provider identities and reset preferences; keep email and reviewed offer controls attached to the verified account.
- Fit local idea prompts to recent reported usage windows without automatically starting work. See docs/reset-alerts.md for acceptance and recovery.
- Support macOS 14 or later on Apple silicon and Intel. Physical Intel and macOS 14 runtime acceptance remain unverified.

## 0.1.13 October 8 2026

- Display discovered projects in a compact table with descriptions, archive status, and row actions.
- Search project names and descriptions without changing the stored projects. Show matching counts and a clear-search action.
- Preserve automatic discovery, project editing, and archive/remove behavior.
- Support macOS 14 or later on Apple silicon and Intel. Physical Intel and macOS 14 runtime acceptance remain unverified.

## 0.1.12 October 8 2026

- Discover locally saved Codex and Claude Code projects automatically after sign-in. Deduplicate folders used by both providers and preserve project identity when a Codex CLI folder becomes a saved workspace.
- Remove manual project creation. Project names follow the provider; descriptions and archive status remain editable. Existing local records are retained.
- Display the Clerk username or primary email instead of an internal account ID.
- Exclude macOS metadata files from updater archives and reject them during release verification, fixing installation failures in earlier updates.
- Support macOS 14 or later on Apple silicon and Intel. Remote-only projects without local metadata are unavailable. Physical Intel and macOS 14 acceptance remain unverified.

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
