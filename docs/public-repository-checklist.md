# Maxxit public repository checklist

Apply this checklist to [maxxit-app/maxxit-desktop](https://github.com/maxxit-app/maxxit-desktop) and its [Homebrew tap](https://github.com/maxxit-app/homebrew-tap). The desktop uses Tauri 2, Rust, React, and TypeScript. Backend and website source stay in their private repositories.

The first priorities are protecting changes to the public repository and making releases verifiable. Then make contributions easy to submit and maintain. Maxxit already has a binary release and several useful foundations, so the work below builds on those.

Reviewed on October 7, 2026. Source snapshots are [Maxxit e5753fd](https://github.com/maxxit-app/maxxit-desktop/tree/e5753fdd3bb5a716d8185efba7c112e3b793812c), [QuotaBar e1d3e3e](https://github.com/sam-pop/QuotaBar/tree/e1d3e3e551f8daa255a8136de16384cdd07f0152), and [CodexBar 42c7048](https://github.com/steipete/CodexBar/tree/42c7048c9fb117b6ca8ee6d8c8acd7eda0985621). GitHub settings were read through the authenticated API for Maxxit. Settings for the reference repositories were not fully accessible.

P0 means complete before the next stable release. P1 means establish the normal maintenance workflow. P2 means add when demand or release volume justifies it. Checked items describe verified existing files or configuration, not a completed security audit. Unchecked items still need acceptance evidence, owner action, or a justified future need. See [implementation status](implementation-status.md) for current results and limits.

## Existing foundations

- [x] Public desktop source has an MIT [LICENSE](https://github.com/maxxit-app/maxxit-desktop/blob/e5753fdd3bb5a716d8185efba7c112e3b793812c/LICENSE), provider [asset attribution](https://github.com/maxxit-app/maxxit-desktop/blob/e5753fdd3bb5a716d8185efba7c112e3b793812c/public/providers/ATTRIBUTION.md), and the bundled font license.
- [x] A [security policy](https://github.com/maxxit-app/maxxit-desktop/blob/e5753fdd3bb5a716d8185efba7c112e3b793812c/SECURITY.md) gives a private contact and explains provider credentials, pairing, and the Claude bridge.
- [x] [CI](https://github.com/maxxit-app/maxxit-desktop/blob/e5753fdd3bb5a716d8185efba7c112e3b793812c/.github/workflows/check.yml) runs on PRs and main pushes, uses read-only contents permission, cancels superseded runs, and runs the frontend build/tests and locked Rust tests. The [main run](https://github.com/maxxit-app/maxxit-desktop/actions/runs/37604241413) passed.
- [x] pnpm and Cargo lockfiles are committed. Node 24 and pnpm 10.19.0 are documented. Frontend builds include TypeScript checking.
- [x] Tauri has a [CSP](https://github.com/maxxit-app/maxxit-desktop/blob/e5753fdd3bb5a716d8185efba7c112e3b793812c/src-tauri/tauri.conf.json), separate main/tray capabilities, a committed updater public key, and an HTTPS update endpoint. Device credentials use [Mac Keychain](https://github.com/maxxit-app/maxxit-desktop/blob/e5753fdd3bb5a716d8185efba7c112e3b793812c/src-tauri/src/cloud.rs).
- [x] [v0.1.10](https://github.com/maxxit-app/maxxit-desktop/releases/tag/v0.1.10) is published with a universal DMG, updater archive, signature, manifest, and checksums. The [release script](https://github.com/maxxit-app/maxxit-desktop/blob/e5753fdd3bb5a716d8185efba7c112e3b793812c/scripts/release.sh) checks signing, Gatekeeper assessment, stapled tickets, and both architectures before creating a draft. Physical installation and update acceptance still need recorded evidence for each release.

## Initial gaps recorded before implementation

| Area                  | Verified state on October 7                                                                                                                                                                          | Priority                                            |
| --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------- |
| Main protection       | Repository rulesets list is empty; the main branch protection endpoint reports unprotected.                                                                                                          | P0                                                  |
| Release integrity     | v0.1.10 is mutable. The release script does not bind a clean source revision to all versions or verify the expected Apple signing team.                                                              | P0                                                  |
| Security reporting    | GitHub private vulnerability reporting is disabled.                                                                                                                                                  | P0                                                  |
| Security tooling      | Settings API reports secret scanning, push protection, and Dependabot security updates disabled; CodeQL default setup is not configured. Confirm the effective secret protection settings in the UI. | P0                                                  |
| CI completeness       | No Rust formatting/Clippy gate, frontend lint gate, job timeout, fixed Rust toolchain, or full native packaging check. Actions use mutable version tags.                                             | P0                                                  |
| Contribution workflow | No contributing guide, issue forms, PR template, code ownership file, or code of conduct in the public tree. Labels are GitHub's starter set.                                                        | P1                                                  |
| Maintenance           | No dependency update configuration, changelog, or dedicated public release runbook.                                                                                                                  | P1                                                  |
| Distribution          | Homebrew tap contains only a README. Public README/tap wording still mentions waiting for a signed release. Installer can fall back to an ad hoc source build after a binary download failure.       | P0 for installer behavior, P1 for docs and Homebrew |

The settings observations come from [repository metadata](https://api.github.com/repos/maxxit-app/maxxit-desktop), [rulesets](https://api.github.com/repos/maxxit-app/maxxit-desktop/rulesets), [private reporting](https://api.github.com/repos/maxxit-app/maxxit-desktop/private-vulnerability-reporting), and [CodeQL setup](https://api.github.com/repos/maxxit-app/maxxit-desktop/code-scanning/default-setup). Some endpoints require repository access. The [community profile](https://api.github.com/repos/maxxit-app/maxxit-desktop/community/profile), public file tree, and release files establish the file gaps.

## Practices to take from the reference repositories

| Reference practice                                                                                                                                                                                                                                                                                                       | Adaptation for Maxxit                                                                                                                             |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| QuotaBar separates pure logic and injects networking, credentials, storage, and time in [tests](https://github.com/sam-pop/QuotaBar/tree/e1d3e3e551f8daa255a8136de16384cdd07f0152/QuotaBarTests).                                                                                                                        | Test provider decoding, freshness, retries, account separation, and storage failure with synthetic data and temporary directories.                |
| QuotaBar distinguishes unreadable credentials from absent credentials and preserves account slots during [writes](https://github.com/sam-pop/QuotaBar/blob/e1d3e3e551f8daa255a8136de16384cdd07f0152/QuotaBar/Services/AccountCredentialStore.swift).                                                                     | Preserve Maxxit's existing Keychain error distinction. Verify complete migrated values before deleting old storage and test interrupted upgrades. |
| CodexBar publishes [contribution scope](https://github.com/steipete/CodexBar/blob/42c7048c9fb117b6ca8ee6d8c8acd7eda0985621/VISION.md) and [issue labeling rules](https://github.com/steipete/CodexBar/blob/42c7048c9fb117b6ca8ee6d8c8acd7eda0985621/docs/ISSUE_LABELING.md).                                             | Explain which desktop changes belong here and use a small, documented triage system.                                                              |
| CodexBar pins Actions and checks required job outcomes in [CI](https://github.com/steipete/CodexBar/blob/42c7048c9fb117b6ca8ee6d8c8acd7eda0985621/.github/workflows/ci.yml).                                                                                                                                             | Keep Maxxit's read-only PR workflow, pin Actions, expand checks, and make incomplete required jobs block integration.                             |
| CodexBar has a [release runbook](https://github.com/steipete/CodexBar/blob/42c7048c9fb117b6ca8ee6d8c8acd7eda0985621/.agents/skills/release-codexbar/SKILL.md) and [published artifact verification](https://github.com/steipete/CodexBar/blob/42c7048c9fb117b6ca8ee6d8c8acd7eda0985621/Scripts/check-release-assets.sh). | Keep the Tauri release pipeline, verify downloaded artifacts, and test installed upgrades before advancing the stable channel.                    |
| CodexBar gives detailed [privacy explanations](https://github.com/steipete/CodexBar/blob/42c7048c9fb117b6ca8ee6d8c8acd7eda0985621/README.md#privacy-note) and [third-party notices](https://github.com/steipete/CodexBar/blob/42c7048c9fb117b6ca8ee6d8c8acd7eda0985621/docs/THIRD_PARTY_LICENSES.md).                    | Document the exact local reads and optional cloud payloads. Include shipped dependency and asset notices.                                         |

Issue forms, PR templates, CODEOWNERS, dependency automation, and a full disclosure policy are additions to Maxxit. Neither reference snapshot contains all of them. QuotaBar's uploaded release asset lists are empty, so use CodexBar as the stronger binary distribution reference.

## Repository access and GitHub security

P0. These are repository administrator tasks. Rules should work for a solo maintainer and continue to work when outside contributors arrive.

- [x] Protect main with a ruleset requiring a PR, passing checks, resolved review conversations, and linear history. Block deletion and force pushes on main. Require the checks from the expected GitHub App. [GitHub ruleset guidance](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets).
- [x] Require an independent approval for outside contributions and sensitive changes when a second maintainer is available. A sole maintainer cannot approve their own PR; document a narrow, recorded owner bypass rather than an impossible approval requirement.
- [x] Protect release tags matching `v*`. Restrict creation to the release maintainer or release automation and prevent changes to published tags. Enable immutable releases for future releases after preparing the complete draft process. [Immutable releases](https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases).
- [ ] Require organization members to use two-factor authentication. Review owners, collaborators, bots, installed GitHub Apps, and token scopes. Keep daily contributor access below administrator level.
- [ ] Enable GitHub private vulnerability reporting and subscribe the responsible maintainer to security notifications. Add the reporting link to SECURITY.md and issue routing. [Private reporting configuration](https://docs.github.com/en/code-security/how-tos/report-and-fix-vulnerabilities/configure-vulnerability-reporting/configure-for-a-repository).
- [ ] Enable or confirm secret scanning alerts and push protection. Scan existing Git history and release contents as well. Revoke any discovered credential before removing it from history. Define a response owner and avoid posting secrets in the cleanup issue. [Secret scanning configuration](https://docs.github.com/en/code-security/how-tos/secure-your-secrets/detect-secret-leaks/enable-secret-scanning).
- [x] Enable Dependabot alerts/security updates and configure version updates for npm, Cargo, and GitHub Actions. Group routine updates; review security fixes promptly. Keep exceptions dated and owned.
- [x] Configure code scanning for the actual code, including JavaScript/TypeScript, Rust, and Actions where supported. Add dependency review for PRs and Rust advisory checks. Document any temporary exception with its reason and expiry. [Supported CodeQL languages](https://docs.github.com/en/code-security/concepts/code-scanning/codeql/codeql-code-scanning).
- [x] Use a protected release environment with restricted source refs. Keep Apple and updater keys there or in the documented local release secret store. Ordinary PR checks must receive no signing, cloud, billing, or deployment secrets.
- [x] Restrict allowed Actions to reviewed providers, pin each external Action to its full commit SHA, and keep the readable version in a comment. Preserve read-only default workflow permissions and disabled automatic PR approval. Grant write permissions only to the job that needs them. [Actions security guidance](https://docs.github.com/en/actions/reference/security/secure-use).

## Contributors and pull requests

P1. Adopt CodexBar's explicit [scope](https://github.com/steipete/CodexBar/blob/42c7048c9fb117b6ca8ee6d8c8acd7eda0985621/VISION.md) and [development expectations](https://github.com/steipete/CodexBar/blob/42c7048c9fb117b6ca8ee6d8c8acd7eda0985621/docs/DEVELOPMENT.md), with shorter instructions suited to Maxxit.

- [x] Add CONTRIBUTING.md covering a clean clone, supported toolchains, commands, fixture setup, unsigned contributor builds, and contribution scope. Desktop contributions must build without access to the private backend or signing keys.
- [x] State which changes need an issue and design agreement first, especially provider integrations, new permissions, dependencies, storage migrations, billing contracts, and release behavior. Small fixes should remain easy to submit.
- [x] Add a short PR template asking for the problem, resulting behavior, linked issue, actual validation, UI evidence when relevant, and migration/security impact. Require explicit notes about material untested paths.
- [x] Add CODEOWNERS for workflows, release scripts, updater configuration, Tauri capabilities, credentials, provider reads, and cloud synchronization. Keep the file itself owned. Map sensitive review requirements to available maintainers.
- [x] Use branches such as `codex/fix-updater-retry` and Conventional Commit titles such as `fix(updater): preserve pending update after download failure`. Keep changes scoped and describe the final implementation in the PR.
- [x] Resolve conflicts by rebasing onto main. Disable merge commits; choose squash integration by default or rebase integration for intentionally structured commits. Delete completed branches automatically. Keep force-with-lease limited to an owned PR branch after rebase.
- [x] Put agent instructions in the public repository. Reference CONTRIBUTING.md, preserve the rebase rule and clear naming conventions, and include the unslop writing rules in a portable form rather than relying on a maintainer's local filesystem path.
- [x] Make AI-assisted contributions subject to the same review and validation. The contributor remains responsible for understanding the change, its origin, licensing, and evidence. Bots cannot approve their own changes. Release automation may publish only through the configured maintainer-authorized workflow.
- [x] Add a concise code of conduct and a private contact for conduct reports. Use contribution terms compatible with MIT. Avoid adding a CLA unless the project has a concrete licensing need.

## Issues and support

P1. Copy the small taxonomy in CodexBar's [labeling guide](https://github.com/steipete/CodexBar/blob/42c7048c9fb117b6ca8ee6d8c8acd7eda0985621/docs/ISSUE_LABELING.md), not its entire live label list.

- [ ] Add a bug form requiring Maxxit version, macOS version, Apple silicon or Intel, install channel, affected provider, reproduction, expected behavior, actual behavior, and sanitized diagnostic evidence. Make searching existing reports an acknowledgment.
- [ ] Add a feature request form asking for the user's problem, current workaround, and proposed result. Add provider-specific fields to the bug form rather than creating a separate form for every integration.
- [x] Add issue form routing and SUPPORT.md. Send account, payment, email, and personal-data requests to private support; send vulnerabilities to private reporting. Public issues can track sanitized desktop symptoms and cloud compatibility bugs.
- [x] Use one type label from the existing `bug`, `enhancement`, `documentation`, and `question` set. Add one priority system, a few states such as `needs-triage`, `needs-repro`, `accepted`, and `blocked-upstream`, relevant areas, and `provider:codex` or `provider:claude` when useful.
- [x] Document severity and response targets the maintainer can sustain. P0 bugs include credential exposure, data loss, or an unusable release; P1 includes regressions with substantial impact; P2 covers routine improvements. Acknowledgment targets are not promised fix dates.
- [x] Triage regularly: reproduce, find duplicates, label, assign accepted work, and give a reason when declining it. Ask for the missing information once and explain closures. Reopen when a reporter provides useful new evidence.
- [x] Link fixes to issues and identify the release containing the fix. Use milestones for intended releases and a small roadmap for accepted work. Avoid automatically closing confirmed bugs merely because they are quiet.
- [x] Mark `good first issue` only when the scope, expected result, and test approach are clear. Enable Discussions or a project board later if questions or planning volume justify another place to maintain.

## CI and reproducible builds

P0. Maxxit's existing check workflow is a useful start. CodexBar's [CI](https://github.com/steipete/CodexBar/blob/42c7048c9fb117b6ca8ee6d8c8acd7eda0985621/.github/workflows/ci.yml) provides patterns for pins, timeouts, and a required gate.

- [x] Pin a Rust toolchain with rustfmt and Clippy. Choose an explicit macOS runner baseline and record Xcode/SDK versions used for releases. Keep pnpm frozen installs and `--locked` Cargo commands.
- [x] Add frontend lint and formatting checks, `cargo fmt --check`, and Clippy with warnings treated as errors. Run the existing TypeScript build, frontend tests, and Rust tests. Contributors should have equivalent local commands.
- [x] Add a native release build that packages an app without updater/signing secrets. A successful Vite build or `cargo test` does not prove the complete Tauri bundle can be created.
- [x] Give required jobs stable names and enforce them in the main ruleset. If CI later becomes conditional, an aggregate check must fail when a required job fails, is missing, or is unexpectedly skipped. Keep the initial pipeline simple.
- [x] Set job timeouts and cache by OS, toolchain, and lockfile. Cancel old PR checks, but serialize release publication and do not cancel a signing or publishing run halfway through.
- [x] Run fork PRs without secrets or write tokens. Approve first-time workflow execution deliberately. Never check out and execute an untrusted PR revision under `pull_request_target` with privileged credentials.
- [x] Keep tests isolated from the machine's real home directory, Keychain items, provider sessions, and inherited secret variables. Use synthetic fixtures and owned temporary directories. Put any live-provider verification behind explicit opt-in.
- [ ] Validate workflow YAML and release scripts. Add failure tests for the release safeguards, including a wrong version, absent architecture, wrong signing identity, missing signature, and incomplete asset set.

## App security and privacy

P0 for credentials, update trust, and unsafe native access. P1 for the public documentation and continuing review. QuotaBar's [credential handling](https://github.com/sam-pop/QuotaBar/blob/e1d3e3e551f8daa255a8136de16384cdd07f0152/QuotaBar/Services/AccountCredentialStore.swift) and CodexBar's [privacy note](https://github.com/steipete/CodexBar/blob/42c7048c9fb117b6ca8ee6d8c8acd7eda0985621/README.md#privacy-note) are useful models for precision.

- [x] Expand SECURITY.md with supported release versions, the private reporting route, acknowledgment expectations, coordinated disclosure, and emergency update instructions. Name desktop and hosted-service security boundaries.
- [x] Document the threat model and review Tauri commands, capabilities, CSP, filesystem reads, URL opening, and bridge installation against it. Validate command inputs in Rust and keep the tray window's permissions narrow.
- [x] Preserve the official-CLI provider sign-in boundary. Any proposed OAuth or token-reading integration needs documented provider permission and a separate security review before implementation. Do not infer permission from another app using an endpoint.
- [x] Test Keychain access denied separately from no stored credential. On write/migration failure, preserve the previous credential and verify the complete new value before deleting old storage. Keep bundle and Keychain identifiers stable across upgrades and renames.
- [x] Document which provider files Maxxit reads, the Claude configuration change, data retained locally, optional cloud fields, retention, export, deletion, and disconnect behavior. Distinguish provider credentials from Maxxit's device credential and cloud processing from local analytics.
- [x] Provide a sanitized diagnostic export or collection guide. Exclude tokens, authorization headers, emails, private paths, project descriptions, and conversation content by default. Ask users to review the output before posting it publicly.
- [ ] Review public commits, screenshots, source archives, bundles, and debug artifacts for private infrastructure details or user data. Keep production runbooks, customer records, keys, and private repository code outside the public repo.
- [ ] Define safe local-data migrations and backups. Test crash interruption, unreadable/corrupt storage, migration retries, and deletion. Document any downgrade incompatibility before a release changes the schema.

## Behavior and regression tests

P1. Adapt QuotaBar's [injected runtime tests](https://github.com/sam-pop/QuotaBar/blob/e1d3e3e551f8daa255a8136de16384cdd07f0152/QuotaBarTests/AccountRuntimeTests.swift), [migration tests](https://github.com/sam-pop/QuotaBar/blob/e1d3e3e551f8daa255a8136de16384cdd07f0152/QuotaBarTests/AccountMigrationTests.swift), and [retry policy](https://github.com/sam-pop/QuotaBar/blob/e1d3e3e551f8daa255a8136de16384cdd07f0152/QuotaBar/Logic/RetryPolicy.swift).

- [x] Keep sanitized Codex and Claude fixtures for malformed payloads, missing windows, changing formats, unknown fields, and plan-specific limits. Retain a regression fixture when an integration breaks.
- [x] Test that missing data stays unknown, stale observations are visibly stale, past resets are not invented again, and token totals do not imply billing cost or a complete account history.
- [ ] Inject clocks for reset countdowns, expiry, retention, scheduling, and retry tests. Cover UTC dates, local timezones, daylight saving changes, and sleep/wake recovery.
- [x] Test temporary network failures, 429s, 5xx responses, and actual credential revocation separately. Bound retries and add jitter. An outage must preserve local analytics and must not automatically erase credentials.
- [ ] Test account/provider separation, reconnecting as a different account, duplicate telemetry, concurrent refreshes, and collection when the main window is closed.
- [x] Test the Claude bridge's preview, repeated installation, atomic writes, previous-command preservation, and uninstall after the user changes their settings. Verify it never restores over a newer user command.
- [x] Keep native acceptance checks for tray behavior, Keychain, autostart, permissions, export dialogs, and updates. Browser tests and mocked update-controller tests cover different behavior from an installed Mac app.
- [ ] Add accessible UI checks for keyboard navigation, focus, charts, dialogs, contrast, reduced motion, and VoiceOver. Keep a representative manual native pass for each stable release.

## Release tooling to establish

P0 for revision/version/trust checks. P1 for automation and release history. CodexBar's [runbook](https://github.com/steipete/CodexBar/blob/42c7048c9fb117b6ca8ee6d8c8acd7eda0985621/.agents/skills/release-codexbar/SKILL.md) and [changelog validation](https://github.com/steipete/CodexBar/blob/42c7048c9fb117b6ca8ee6d8c8acd7eda0985621/Scripts/validate_changelog.sh) are the useful patterns.

- [x] Add docs/releasing.md and CHANGELOG.md. Describe one supported release command, its prerequisites, exact outputs, verification, publication, and recovery. Include upgrade guidance and known limitations in each release's notes.
- [x] Add one version-bump/check command covering package.json, Cargo.toml, the root package version in Cargo.lock, tauri.conf.json, the Git tag, app bundle, and updater manifest. Define how breaking changes are communicated during the 0.x period.
- [x] Require a clean checkout at a reviewed commit with passing CI. Bind draft creation to that exact commit SHA. Verify that the packaged bundle's version and source revision match the intended release.
- [ ] Extend scripts/release.sh to verify the expected bundle identifier and Apple Team ID, relevant entitlements, and the updater signature against the public key shipped in the app. Keep the existing signing, stapling, and architecture checks.
- [ ] Back up the dedicated Tauri updater private key securely. Document its owner, recovery, rotation, and compromise procedure separately from Apple certificate renewal. Test key continuity with existing installs before changing the embedded public key.
- [x] Keep release tooling inside the repo or pin external tooling to a reviewed version. A local manual release is acceptable if repeatable; a protected CI release workflow can be added when it improves reliability. Clean temporary signing files and keychains on success and failure.
- [ ] Add a rehearsal mode that builds and verifies artifacts without publishing a stable release, updating the stable manifest, or changing Homebrew. Add artifact provenance attestations when builds run in trusted CI. [GitHub artifact attestations](https://docs.github.com/en/actions/concepts/security/artifact-attestations).
- [ ] Retain matching debug symbols and a sanitized release verification record with version, source SHA, toolchains, checksum results, tested Macs, and upgrade results. Restrict access to crash evidence containing personal data.

## Checklist for each stable release

Follow this order. Publishing a stable release can immediately change Maxxit's current `releases/latest/download/latest.json` endpoint, so installation and update checks belong before publication. Apple signing and Tauri updater signing are separate requirements. [Tauri macOS signing](https://v2.tauri.app/distribute/sign/macos/), [Tauri updater documentation](https://v2.tauri.app/plugin/updater/).

1. [ ] Prepare a version/changelog PR with linked fixes, compatibility changes, supported macOS versions, known limitations, and contributor credit. Run required CI on the final revision.
2. [ ] Integrate the reviewed PR and wait for required CI on that exact main commit. Confirm a clean release checkout, version agreement, and a version newer than the current stable release. Create the protected version tag on that revision; reject an existing tag pointing elsewhere.
3. [ ] Build the universal release with pinned toolchains. Sign with Maxxit's Developer ID Application identity and hardened runtime, notarize, and staple the app and DMG as appropriate. Verify both executable architectures.
4. [ ] Build the updater archive from the final notarized app, then sign that exact archive with the existing Tauri updater key. Verify its signature against the embedded public key.
5. [ ] Create a complete draft with `Maxxit-universal.dmg`, `Maxxit-universal.app.tar.gz`, its `.sig`, `latest.json`, and `SHA256SUMS`. Include hashes for the manifest and signature too. Preserve the asset names the website/updater expect.
6. [ ] Check that the manifest's version, notes, publication date, signatures, and `darwin-aarch64`/`darwin-x86_64` targets are correct. Download URLs must use the specific release tag over HTTPS, avoiding mutable archive URLs.
7. [ ] Download the uploaded draft artifacts with maintainer access. Verify checksums, expected Apple identity, bundle ID/version, Gatekeeper acceptance, and notarization. Compare the apps extracted from the DMG and updater archive, including executable and resource hashes, to confirm both contain the same accepted build.
8. [ ] Install the candidate with macOS protections enabled on Apple silicon and Intel, including the oldest supported macOS where available. Verify launch, tray, data collection, cloud disconnect, and uninstall. Record what was physically tested; cross-compilation proves less than installation.
9. [ ] Establish an isolated HTTPS staging feed serving the exact candidate archive and signature. Maxxit's production endpoint is fixed, so use a signed test build from the previous updater-capable tag with only its endpoint changed, or a documented test-only [Rust endpoint override](https://v2.tauri.app/plugin/updater/#runtime-configuration). Preserve the public key and signature enforcement, and record that this differs from testing the unmodified production binary. Verify settings, history, projects, Keychain access, Claude bridge survival, offline retry, and tampered-archive rejection. Versions through 0.1.9 need the one-time DMG upgrade documented in v0.1.10.
10. [ ] Publish the verified draft as the stable/latest release. For immutable releases, all assets must already be attached. Download the public bytes and repeat checksum, updater signature, Apple identity, and version checks; compare them with the accepted draft. Verify the website, stable updater endpoint, and an update from an unmodified previous stable installation. [GitHub draft publication guidance](https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository).
11. [ ] Update the Homebrew cask from the verified public DMG URL and checksum. Wait for tap checks, then test cask installation and upgrade. Update website/repository installation guidance and close the release milestone with links to the shipped fixes.
12. [ ] Observe incoming installation, update, and crash reports after publication. If the release is bad, follow the recovery procedure and issue a new patch version. Never replace the shipped bytes under the same version.

## Installation and distribution

P0 for trustworthy installation. P1 for completing distribution channels. CodexBar [verifies the published archive](https://github.com/steipete/CodexBar/blob/42c7048c9fb117b6ca8ee6d8c8acd7eda0985621/Scripts/check-release-assets.sh) and waits for the [Homebrew update chain](https://github.com/steipete/CodexBar/blob/42c7048c9fb117b6ca8ee6d8c8acd7eda0985621/.github/workflows/release-cli.yml).

- [x] Make the normal install path the verified DMG. Remove the installer script's automatic source-build fallback on any binary download failure. Source builds should be an explicit contributor choice; an outage should return a useful error.
- [x] Verify installer authenticity against the expected signing identity, not only a checksum fetched from the same location. Keep Gatekeeper enabled and never instruct normal users to remove quarantine or disable macOS protection.
- [ ] Publish docs/install-macos.md with real sanitized screenshots, prerequisites, installation, first launch, official CLI setup, update behavior, export, disconnect, uninstall, and troubleshooting. Reconcile `/Applications` versus `~/Applications` instructions with each install path.
- [ ] Add the actual cask to homebrew-tap after release verification. Check version, immutable URL, SHA-256, macOS requirement, and application name. Keep data deletion during uninstall explicit rather than automatic.
- [x] Remove obsolete pending-release language from the public README and tap. Show whether a channel is available, verified, or still planned. Test the download button, stable release link, and update endpoint after releases.
- [ ] Add prereleases and a separate opt-in update channel only when needed. Beta builds must not become GitHub latest or enter the stable Homebrew cask accidentally. Define promotion and opt-out behavior before announcing betas.

## Documentation and legal notices

P1. Follow the specificity of the reference READMEs while keeping Maxxit's own architecture and current release status accurate.

- [x] Keep the README short enough to answer what Maxxit does, platform support, where to download, provider setup, free/local versus paid/cloud behavior, privacy, source builds, and support. Link detailed guides.
- [x] Add a public architecture guide describing renderer/native boundaries, provider adapters, local storage, cloud contract, and update trust. Document the API contract needed for desktop development without disclosing private backend implementation.
- [x] Publish a provider support matrix with data source, official CLI prerequisite, available windows, freshness, known limitations, and last verification. Update it with provider format changes.
- [ ] Add third-party notices for shipped Rust/npm dependencies and vendored assets. Preserve the existing provider attribution and font license. Review any copied code's actual license and required notices before inclusion.
- [x] Put upgrade and troubleshooting instructions next to release notes. Keep historical migration research outside contributor onboarding and remove stale commands when the architecture changes.
- [x] Set the repository website URL and relevant topics, add a clear CI badge and installation link, and keep screenshots free of real emails, paths, or private projects. Use observed sample data that is labeled as such.

## Ongoing maintenance and recovery

P1 for ownership and incident response. P2 for monitoring and additional automation. CodexBar's [upstream monitor](https://github.com/steipete/CodexBar/blob/42c7048c9fb117b6ca8ee6d8c8acd7eda0985621/.github/workflows/upstream-monitor.yml) reuses one existing tracking issue for upstream changes.

- [x] Assign owners for issue triage, dependency updates, private security reports, releases, and support. Review queues regularly and document how to hand over release access without sharing personal credentials.
- [x] Write a bad-release procedure covering affected versions, user guidance, stable/latest promotion, Homebrew, a corrected patch, and data compatibility. Preserve previous verified artifacts. Avoid blind downgrades after local schema changes.
- [x] Write a signing-key compromise procedure. Stop publication, revoke exposed credentials, assess installed clients, and document a trusted manual recovery path when automatic update trust cannot be retained.
- [ ] Define the desktop/backend compatibility window and version any breaking cloud contract. Test old supported desktop clients against backend changes. Keep hosted deploy credentials and database recovery procedures in the private repos.
- [ ] Use opt-in crash reporting only if needed, with a clear payload and retention policy. Keep diagnostic collection separate from usage monitoring, and retain matching symbols for supported versions.
- [ ] Monitor relevant upstream release notes or format changes when provider breakage becomes recurring. Update one existing issue only on actionable changes; avoid automated comment noise and secret-bearing live tests.
- [x] Review repository access, dependency exceptions, supported versions, release keys/certificates, and public documentation periodically. If external contribution volume grows, add a second release maintainer and a lightweight maintainer guide.

## Patterns to avoid copying

- QuotaBar's browser subscription OAuth and CodexBar's broader credential/browser integrations are not authorization for Maxxit to use the same provider paths. Keep Maxxit's documented integration boundary.
- CodexBar's [external mac-release helper](https://github.com/steipete/CodexBar/blob/42c7048c9fb117b6ca8ee6d8c8acd7eda0985621/Scripts/mac-release) depends on tooling outside the checkout. Maxxit's release process should run from its repository and documented toolchain.
- CodexBar's [release configuration](https://github.com/steipete/CodexBar/blob/42c7048c9fb117b6ca8ee6d8c8acd7eda0985621/.mac-release.env) permits forced tags, and its [CLI workflow](https://github.com/steipete/CodexBar/blob/42c7048c9fb117b6ca8ee6d8c8acd7eda0985621/.github/workflows/release-cli.yml) replaces uploaded assets and grants workflow-wide write access. Use immutable releases and narrow publication permissions for Maxxit.
- CodexBar's Linux/CLI assets arrive through workflows triggered after publication. Maxxit can prepare its smaller asset set completely in a draft before exposing it to stable users.
- CodexBar's large label catalog, Swift sharding, Linux matrices, and Sparkle machinery fit that project's scope. Start Maxxit with a small label set, straightforward macOS CI, and its existing Tauri updater.

## Implementation order

1. Protect main and release tags, enable private reporting/security tooling, pin Actions, and fix installer fallback behavior.
2. Expand required CI and release revision/version/signature checks. Establish the complete-draft acceptance process for the next patch.
3. Add contributor guidance, issue forms, a PR template, ownership, and the small triage policy.
4. Add release history, installation/privacy/architecture guides, and publish the verified Homebrew cask.
5. Add regression fixtures, recovery procedures, and only the automation needed to keep these practices maintained.

Each implementation PR should include its actual validation. For settings tasks, record the effective rule or configuration after applying it. This checklist itself changes no GitHub settings, releases, or application code.
