# Release Maxxit

Stable releases use reviewed main and a strictly increasing stable semantic version.
During 0.x, announce breaking behavior explicitly even when the change is small.
Never change an existing release's tag or bytes. Prepare a complete draft before
publication because publishing stable/latest immediately advances the production updater.

## Prepare source

```sh
pnpm version:bump 0.1.11
pnpm version:check
pnpm check
```

Write a dated CHANGELOG.md section, including fixes, compatibility, known limitations,
supported macOS versions, and contributor credit. Submit a PR, integrate it through
the protected process, and wait for CI on the exact main revision. Use a clean
checkout matching origin/main. The script rejects existing tags and versions at
or below the current public stable release.

package.json, Cargo.toml, the root Cargo.lock entry, and tauri.conf.json must agree.
Release binaries expose their compiled version and source revision through --version.
Keep Maxxit's bundle ID, Keychain service/account, updater public key, and database
location stable across updates.

## Build and verify

Use Rust 1.94.1, Node 24.15.0, pnpm 10.19.0, and Xcode 16.4 for the documented
release baseline. Install both Rust Mac targets. Sign with the owner's authorized
Developer ID Application certificate, team DXYF58SJPA. Enable hardened runtime,
notarize, and staple the app and DMG. The app and DMG must contain the same build.

Set TAURI_SIGNING_PRIVATE_KEY_PATH to the existing dedicated updater key outside
the repo. Its public key must match the Tauri configuration. Set its password only
in the protected shell/secret store, without logging it. Apple certificate renewal
is separate from updater key rotation. Back up the updater key in encrypted storage
and verify recovery with a rehearsal before changing either trust configuration.

```sh
pnpm tauri build --target universal-apple-darwin -- --locked
bash scripts/release.sh VERSION APP_PATH DMG_PATH --dry-run /NEW/OUTPUT/DIRECTORY
bash scripts/release.sh VERSION APP_PATH DMG_PATH
```

Tauri/Apple credentials must be configured before building. Follow the
[official macOS signing guide](https://v2.tauri.app/distribute/sign/macos/).
The manual pipeline is the supported publishing path. CI rehearsal artifacts are
unsigned and cannot be promoted directly to consumer installers.

release.sh checks source, main CI, version increase, and release notes. It packages
the final app, signs that exact updater archive, creates both architecture entries,
and hashes the DMG, archive, signature, and manifest. verify-release.sh checks
metadata, all hashes, the embedded updater key, expected Apple identity, Gatekeeper,
notarization, architectures, source revision, and DMG/archive byte agreement.
It then creates a draft bound to the reviewed commit.

v0.1.10 is the legacy baseline: its checksum file covers the DMG/archive only and
its binary predates the source-revision CLI. Metadata verification handles that exact version's legacy checksum format. Its DMG
contains an app without a stapled ticket, although the container is stapled and
Gatekeeper accepts it. The stricter release verifier rejects this baseline.
New releases require stapled tickets on both copies of the app and all full checks.

## Acceptance and publication

Complete release-checklist.md with actual evidence for Apple silicon, Intel,
the oldest supported macOS, native permissions, and the previous version's update.
Cross-compilation does not prove an Intel installation.

For a pre-publication update test, serve the exact signed candidate bytes on an
isolated HTTPS staging feed. Use a release-equivalent signed previous-tag build
with only its test endpoint changed, or a documented test-only Rust endpoint override.
Preserve signature enforcement and the public key. Record this difference from the
unmodified production binary. Private draft URLs are not a consumer staging feed.

The draft must contain Maxxit-universal.dmg, Maxxit-universal.app.tar.gz,
Maxxit-universal.app.tar.gz.sig, latest.json, and SHA256SUMS. The updater targets are
darwin-aarch64 and darwin-x86_64 with versioned HTTPS URLs. The website expects the DMG name.

After maintainer review and acceptance, publish the complete stable/latest draft.
Future releases become immutable at publication. Repeat verification on public bytes:

```sh
bash scripts/verify-published-release.sh VERSION SOURCE_SHA
```

Confirm the website, production latest.json, and a real prior stable installation
see the intended release. Update the Homebrew cask with the verified public DMG
URL/hash, wait for tap checks, and test install/upgrade. Close release issues with
the shipped version. Follow recovery.md immediately if public checks fail.

Retain matching debug symbols and a sanitized record of source SHA, toolchain,
checksums, signatures, machines, installs, upgrades, and known limitations.
Private crash evidence and key recovery details belong in restricted storage.
