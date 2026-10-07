# Provider support

Verified source behavior on October 7, 2026. Actual quota availability depends on
what the official CLI writes; a local reading does not grant provider API access.

| Provider    | Supported local source                                                                   | Limits                                                                                                                                                                                    |
| ----------- | ---------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Codex       | Official CLI rollout session records under CODEX_HOME/sessions or ~/.codex/sessions.     | Reported primary/secondary windows and observed token increments only. Bounded scans make historical totals partial. Account grouping uses a local hash when a creator account ID exists. |
| Claude Code | Official status-line allowance fields after an explicit bridge preview and installation. | May omit allowance until a new API response. Repeated identical captures do not make an old observation fresh.                                                                            |

Sign in using the official CLI. Maxxit does not implement provider subscription
OAuth, read their token files, or inspect browser sessions. New integration
methods require documented provider permission and a separate security review.

Missing fields remain unknown. Expired resets and readings older than two hours
cannot show spare allowance. Percentages describe individual windows, not billing
credits, and token counts are not a complete account history.

Include CLI/app versions and sanitized format evidence when reporting a provider
regression. Add a synthetic regression fixture rather than committing a real session.
