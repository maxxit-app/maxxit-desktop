# Contributing to Maxxit

Bug fixes, accessibility improvements, tests, and documentation are welcome.
Discuss new providers, native permissions, dependencies, storage changes, cloud
contracts, and release behavior in an issue before starting a large change.
Backend and website source are private. Desktop development must work without them.

## Development

Use macOS 14 or later, Apple's command-line tools, Node 24.15.0, pnpm 10.19.0,
and Rust 1.94.1. rust-toolchain.toml installs rustfmt and Clippy.

```sh
corepack enable
pnpm install --frozen-lockfile
pnpm desktop:dev
pnpm check
APPLE_SIGNING_IDENTITY=- pnpm tauri build --bundles app --config '{"bundle":{"createUpdaterArtifacts":false}}' -- --locked
```

No provider account, Maxxit account, Apple certificate, or private repository is
needed for tests or unsigned builds. The browser preview runs with pnpm dev at
http://127.0.0.1:1420/?demo. It never sends demo records to production.

Use synthetic fixtures and temporary directories in tests. Inject paths and clocks.
Do not point tests at your real provider files, change HOME globally, or access
your real Keychain. See docs/testing.md for native acceptance checks.

## Pull requests

Create a branch such as codex/fix-updater-retry. Rebase on main to resolve conflicts.
Use titles such as "fix(updater): preserve pending update after download failure".
Keep the PR focused on one problem. Explain the resulting behavior, link the issue,
and list checks actually run. Attach sanitized UI evidence when useful.
State migration, security, dependency, and release impact, including untested paths.

AI-assisted work follows the same rules. You are responsible for understanding
the code, its source/license, and validation. Submitted changes use the MIT license.
No CLA is required. Read CODE_OF_CONDUCT.md and SECURITY.md before posting reports.
