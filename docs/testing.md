# Testing Maxxit

pnpm check runs version agreement, frontend lint/formatting/build/tests, release
tooling tests, Rust formatting/Clippy, and native unit tests. The GitHub Desktop
checks job also builds a full unsigned native app bundle without release keys.
Use Node 24.15.0 and the committed Rust toolchain.

Ordinary tests use synthetic records, fake credential stores, in-memory SQLite,
temporary directories, and injected timestamps. They never access the real Mac
Keychain or real provider session/config directories. Do not change HOME or
CODEX_HOME globally to make a test pass. Live provider verification is an explicit
operator task outside the normal suite.

## Automated coverage

Frontend tests cover unknown/expired allowance, UTC token totals, and update
progress/concurrency/retry. Release tests cover synchronized version changes,
malformed versions, missing targets, URL/signature metadata mismatch, incomplete
checksums, tampering, unsafe archive paths, diagnostics, and capability configuration.

Rust tests cover rejected cloud origins, bounded retry classification, unreadable
credentials, verified write/rollback, schema downgrade rejection, corrupt settings,
reopen persistence, provider account isolation/malformed records, injected reset
time, and Claude bridge repeat installation/restore/user-change/symlink safety.

## Native acceptance

Use disposable test accounts and an isolated app/data directory. Record actual
hardware, macOS, CLI versions, and app revision. Never run destructive QA against
a customer's or maintainer's real data.

For each stable release verify tray opening/dismissal, background collection,
sleep/wake freshness, permission denial, Keychain failure, autostart toggles,
CSV dialog behavior, disconnect, safe bridge restoration, and uninstall.
Verify the tray cannot invoke main-only commands or plugin updates at runtime.

On both supported Mac architectures test a browser-downloaded DMG with security
protection enabled, then an installed previous version's update. Verify offline
retry and real signature rejection with a tampered test archive on a staging feed.

Check keyboard navigation, visible focus, dialog focus/return/Escape, meaningful
chart alternatives, contrast, reduced motion, and VoiceOver. Browser preview
automation can assist; it does not prove native WebView or macOS permissions.
Record all untested combinations in the release acceptance record.
