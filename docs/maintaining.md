# Maintaining Maxxit

Akhouad is currently the sole repository owner and handles releases, private
security reports, issue triage, dependency updates, and support. Add a second
maintainer when contribution volume makes independent release review practical.
Use least-privilege contributor roles and review collaborators, apps, and tokens
quarterly. Never transfer ownership by sharing personal credentials.

## Branch and release rules

Main uses PRs, passing Desktop checks, resolved review threads, linear history,
and blocked force pushes/deletion. Review requirements are a separate ruleset:
outside contributions need the code owner's approval. The sole owner has a
PR-only bypass for the approval rule, with GitHub recording its use. This bypass
must not bypass the separate CI and history rules.

Resolve conflicts by rebase. Use squash integration by default, or rebase for a
deliberately structured commit series. Delete completed branches. Never force
push main. Force-with-lease is limited to a PR branch you own.

Version tags may be created by the designated release owner. A separate rule
blocks tag updates/deletions. Future published releases are immutable. Use a new
patch version for corrected binaries.

## Triage

Review new reports weekly. Assign one type, one priority, and one state.
Type labels are bug, enhancement, documentation, or question. Priorities are
P0 for credential exposure/data loss/unusable releases, P1 for major regressions,
and P2 for routine improvements. States are needs-triage, needs-repro, accepted,
and blocked-upstream. Add an area or provider label when it helps routing.

Ask for missing evidence once. Confirm a reproduction before marking accepted.
Link duplicates to the original report. Explain declined requests and closures.
Do not close confirmed bugs automatically because they are quiet. Link fixes to
PRs and name the release containing them. Use milestones for agreed releases.
Only label good first issue when scope and a test approach are clear.

Keep billing, personal information, and security reports private. Public issues
may describe sanitized cloud compatibility problems. Do not promise fix dates.
Discussions, projects, upstream monitoring, and crash reporting remain optional
until demand justifies their maintenance cost.

## Dependency and security maintenance

Review Dependabot and security findings weekly, sooner for exploitable issues.
Group routine updates. Avoid unattended integration for native/updater changes.
Each temporary exception needs an owner, reason, affected version, and expiry.
Never suppress an advisory without verifying its relevance.
The two Linux-only lockfile exceptions are recorded in security-exceptions.json,
expire on January 7, 2027, and fail CI if either crate enters a Mac compilation graph.

For a second maintainer, enable independent approval for owner PRs and release
environment approvals. Keep bots unable to approve their own work. A publishing
workflow requires maintainer authorization, protected refs, and scoped tokens.
