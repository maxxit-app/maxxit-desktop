# Public repository implementation status

Implemented on October 7, 2026 from the QuotaBar/CodexBar review. The original
[checklist](public-repository-checklist.md) retains its source snapshots and
initial settings observations. Checked implementation items mean configuration,
code, or a documented maintenance procedure exists. They do not certify that
all future releases or recurring maintainer tasks have been performed.

## Applied GitHub settings

Main requires Desktop checks, Dependency review, and Rust and npm advisories
from the GitHub Actions app, linear history, and no force pushes/deletion.
Outside PRs need the owner's review and resolved threads. The sole owner's
approval bypass applies only to PR review, with CI enforced separately.
Release tags restrict creation to the owner and block updates/deletion.
Future published releases are immutable. Merge commits are disabled and completed
branches are deleted.

Private vulnerability reporting, secret scanning, push protection, Dependabot
alerts/security updates, selected Action providers, full-SHA enforcement, and
read-only workflow defaults are enabled. Bots cannot approve PRs. The release
environment permits main and requires owner approval; the supported signing path
remains local with keys outside Git. Triage labels, homepage, and topics are set.

The sole organization owner currently uses 2FA. Organization-wide 2FA enforcement
still needs an owner action in GitHub's security settings; the REST update did
not enable it. Security notification subscription preferences are personal
settings and still need the owner's confirmation.

## Delivered in source

Contributor guidance, issue forms, PR template, CODEOWNERS, conduct/support/security
policies, dependency updates, release note categories, public architecture,
provider/privacy/cloud contract docs, diagnostics, installation, testing,
maintenance, release acceptance, and recovery instructions are included.

CI pins Action revisions and toolchains, validates frontend/Rust code, builds a
native unsigned bundle, scans all three CodeQL languages, reviews dependency
changes, audits dependencies, and scans secrets with redacted output. Two
Linux-only lockfile advisory exceptions expire on January 7, 2027 and are rejected
if the crates enter either Mac build graph.

Native commands now use an explicit allowlist for main/tray capabilities.
Keychain writes preserve an unreadable existing item and verify complete values;
failed readback restores a previous value. Claude bridge installation/uninstallation
preserves unrelated and newer user settings and refuses symlinked configuration.
SQLite initializes transactionally, records its schema, rejects future schemas,
and reports corrupt preferences. GET retries are bounded; mutating requests are
not replayed. Responses have a streaming size limit.

Version and release scripts bind a clean reviewed main revision to a strictly
increasing version, notes, both updater targets, expected Apple identity, signed
updater bytes, architectures, notarization, and complete draft assets. The
installer verifies the expected Apple identity and stages the app before replacing
an installation. Binary download failure cannot trigger a source build.
A generated third-party dependency inventory accompanies existing asset licenses.

The companion Homebrew cask pins the real v0.1.10 universal DMG and SHA-256,
requires macOS 14, and preserves user data on uninstall. Its checks never install
or replace the maintainer's application.

## Verification and remaining acceptance

The unsigned native app packaging build also passed.
Local frontend tests (10), script tests (7), Rust tests (20), lint, formatting,
TypeScript/build, and Clippy passed. npm audit found no known vulnerabilities.
The Rust audit passed with the two recorded Mac-inapplicable exceptions.
Full Git history and working-tree secret scans found no leaks.
The published updater signature verified with the embedded key; changing one
archive byte failed verification. Future verification also enforces hardened runtime and rejects debugging or
relaxed library-validation entitlements. Public DMG/archive checksums, Apple team and
bundle identity, Gatekeeper, universal architectures, the archived app's ticket,
and the DMG ticket were verified on this Mac.

The v0.1.10 DMG's app lacks its own stapled ticket. Its container is stapled and
Gatekeeper accepts the app. The stricter future release verifier rejects this
legacy release; use a new patch version with the same final stapled app in both
artifacts. Existing published binaries were not changed or republished.

Physical Intel and oldest-supported macOS installation, a real previous-version
upgrade, native permissions/sleep/autostart, VoiceOver/keyboard/contrast testing,
release screenshots, encrypted key backup/recovery, and retained matching debug
symbols require recorded acceptance before the next stable release. Migration
crash interruption, provider reconnect/concurrency, and additional clock/sleep
regressions need further fixtures. The initial tests do not cover all such paths.
Review downstream license obligations before shipping binaries; the generated
inventory is dependency metadata, not a legal certification.

The protected CI runs must pass on the PR and exact integrated main revision.
Local cask syntax/style and public DMG fetch/hash verification passed. Online
Homebrew audit on this Mac was blocked by installed Xcode 15; the tap's macOS CI
runs the same audit with the runner SDK. A physical Homebrew install/upgrade is
still a release acceptance task.

Beta channels, crash reporting, upstream monitors, Discussions, and a project
board remain optional. Add them only when there is an actual maintenance need.
No beta feed, telemetry service, periodic automation, or placeholder release was
introduced. Hosted backend compatibility tests and customer-data operations
belong in their private repositories.
