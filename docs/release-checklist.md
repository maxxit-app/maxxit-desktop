# Stable release acceptance

Copy this checklist into a release record and replace each unchecked item only
with evidence. The release owner must not treat a build or mocked updater test as
proof of native installation.

- [ ] Version/changelog PR accepted; required CI passed on the integrated main SHA.
- [ ] Clean source checkout, version agreement, version increase, and unused tag.
- [ ] Universal build, expected Apple identity, hardened runtime, notarization, stapling.
- [ ] Final updater archive signed and verified with the shipped public key.
- [ ] DMG and updater contain the same executable/resources/version/source revision.
- [ ] Complete draft metadata and checksums verified after downloading the uploaded files.
- [ ] Apple silicon install with normal macOS protection, startup, tray, collection, and export.
- [ ] Intel install and behavior, recording actual hardware and macOS version.
- [ ] Oldest supported macOS check or explicit unsupported/untested release limitation.
- [ ] Upgrade preserves preferences, projects, observations, device credential, and Claude bridge.
- [ ] Tampered update rejected; offline check/download/restart retry works.
- [ ] Native capabilities, Keychain errors, autostart, disconnect, and uninstall checked.
- [ ] Keyboard focus, dialogs, contrast, reduced motion, and VoiceOver checked.
- [ ] Stable publication approved; public bytes and production updater verified again.
- [ ] Website and Homebrew installation/upgrade verified.
- [ ] Symbols and sanitized acceptance record retained; release issues closed with version links.
