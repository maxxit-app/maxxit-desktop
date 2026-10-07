# Agent instructions

Read CONTRIBUTING.md and docs/maintaining.md before changing this repository.

- Use descriptive Conventional Commit names and codex/ branches.
- Resolve conflicts by rebasing. Never create a merge commit.
- Run the checks that cover the change and report material untested paths.
- Keep ordinary tests away from real provider sessions and Keychain credentials.
- Do not add provider OAuth or credential reads without approved provider access.
- Keep release keys, user data, and private service implementation out of this repo.
- Use the unslop skill when available. Otherwise apply the portable writing rules below.

## Writing rules

Write the concrete change and its result in plain language. Cut promotional claims,
AI filler, decorative emojis, forced lists of three, and generic conclusions.
Avoid em dashes, repeated bold labels, and vague claims about significance.
Do not describe unfinished, mocked, or untested behavior as verified.
Keep release notes, UI copy, support replies, and documentation accurate.
