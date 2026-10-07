# Security policy

Report vulnerabilities privately through
[GitHub private vulnerability reporting](https://github.com/maxxit-app/maxxit-desktop/security/advisories/new)
or email contact@maxxit.app. Include affected versions, impact, and a minimal
reproduction using test data. Never send active credentials or customer records.

The latest stable desktop release receives security fixes. Earlier versions should
upgrade. Versions through 0.1.9 need the DMG upgrade before in-app updates work.
There has been no independent security audit.

The maintainer aims to acknowledge reports within three business days when
available. Fix timing depends on impact and a verified reproduction. Coordinate
disclosure after a corrected release; do not publish exploit details before users
can update. Desktop, updater, and hosted-service vulnerabilities use this same
private contact, while backend details stay private.

## Trust boundaries

Official Codex and Claude CLIs own provider sign-in. Maxxit does not load their
authentication files or browser credentials. Maxxit's optional cloud device token
uses Mac Keychain and is separate from provider credentials. Missing credentials
and Keychain read errors are distinct; failed replacement verification retains the
previous credential when possible.

The main renderer has named Tauri commands and narrowly scoped plugins. The tray
can read a snapshot, resize itself, and navigate or quit; it cannot change settings,
pair devices, or install updates. Rust validates native inputs and cloud origins.
The CSP blocks remote scripts and frames. See docs/threat-model.md.

Claude bridge installation requires an explicit preview/consent. Existing
status-line output is preserved. Disconnect restores only a matching installed
command and approved configuration location. Symlinked configuration is rejected.
Suggestions are never executed automatically.

Apple Developer ID signing and notarization protect Mac distribution. The Tauri
updater separately verifies its signature against the committed public key.
Checksums detect corruption but do not by themselves establish authenticity.
Private signing keys remain outside the repository and ordinary PR jobs.

See docs/privacy.md for collection and retention, docs/releasing.md for release
verification, and docs/recovery.md for bad releases or signing-key compromise.
