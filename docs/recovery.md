# Release and security recovery

## Bad desktop release

Stop promoting the affected version as stable/latest and pause Homebrew updates.
Identify affected versions, impact, and a safe workaround in a sanitized public
notice. Preserve diagnostics and previous verified artifacts. Keep user data and
security details private as needed.

Use a corrected, strictly newer patch version. Do not replace an existing tag or
release asset. Check schema compatibility before recommending an older app.
Verify a fixed update from the affected version, data preservation, the public
endpoint, website, and Homebrew. Record the cause and add a regression test.

## Signing key or credential compromise

Stop publication. Revoke exposed Apple or service credentials with their issuers.
Rotate the compromised credential and review repository access, CI logs, and
published artifacts. Notify users of affected versions without exposing secret values.

A compromised/lost updater key cannot be fixed by simply changing the public key
in the newest source. Existing installs trust their embedded key. Determine whether
a securely authorized transitional update is possible. Otherwise provide a trusted
manual DMG recovery path signed by the verified Apple identity and explain why
automatic updates cannot safely continue. Use private coordinated disclosure first.

Keep updater and Apple key backups separate, encrypted, and access controlled.
Test recovery in an isolated environment. Record access owners and rotation dates
in restricted storage, never in this public repository.

## Local data

Quit Maxxit before an offline copy of its complete app data directory. A SQLite
WAL may contain recent changes, so retain the database and companion files together.
Restore only a verified copy after quitting; preserve the current directory
until recovery is confirmed. Never erase unreadable data to hide a startup error.

Schema version 1 initialization is transactional and repeatable. A newer schema
is rejected instead of reset. Back up before future migrations, announce downgrade
limits, and test interruption/retry on a disposable copy.

Cloud records, backups, deletion tombstones, consent, and entitlement reconciliation
remain the hosted operator's responsibility in the private backend. Restoring an
old backend backup must not revive deleted accounts or revoked devices.
