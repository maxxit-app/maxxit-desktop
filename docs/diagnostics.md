# Safe diagnostics

From a source checkout, run pnpm diagnostics with dmg, homebrew, script, or source.
It outputs only source version, macOS version, architecture, and installation
channel. It reads no provider sessions, Keychain item, environment dump, project
description, email, or local database.

For an installed app, record the version from Settings/About, macOS version,
Mac architecture, and installation method manually. Include exact reproduction
steps and provider CLI version. Describe whether an allowance is missing or stale.

Review every screenshot and log excerpt before attaching it. Remove emails,
account identifiers, private paths, project names/descriptions, prompts,
conversation text, tokens, cookies, and authorization headers. Never attach
Codex rollout files, Claude settings, Maxxit SQLite, or Keychain exports.
A public GitHub attachment is public data.

Use private reporting for vulnerabilities and private support for account or
payment issues. Maintainers should sanitize CI artifacts too. Broad environment
or configuration dumps are not a valid diagnostic collection method.
