# Privacy and data

Local analytics work without a Maxxit account. Optional cloud sharing is controlled
separately for usage and project descriptions. The app never executes suggested work.

| Data                                          | Source and destination                                                                                                                                                                                                                                                                           | Retention and controls                                                                                                                             |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| Codex allowance and observed token increments | Rollout JSONL files under CODEX_HOME/sessions, or the default ~/.codex/sessions. Maxxit scans numeric subdirectories, skips symlinks, examines at most 200 selected files, and reads bounded header/tail sections. Parsed allowance and daily totals stay local unless usage sharing is enabled. | Observation database prunes older than 90 days during recording. Session files remain owned by Codex. Disconnect stops Maxxit collection.          |
| Claude allowance                              | The consented status-line bridge writes only allowance fields and a timestamp to claude-usage.json. No transcript or provider token is persisted.                                                                                                                                                | The last capture stays until overwritten or local app data is removed. Disconnect restores the status-line configuration when safe.                |
| Local settings and projects                   | SQLite in ~/Library/Application Support/app.maxxit.desktop. Includes typed project names/descriptions and preferences.                                                                                                                                                                           | Retained until explicitly removed. The 90-day observation limit does not delete projects or preferences.                                           |
| Maxxit device credential                      | A revocable device token in Mac Keychain under the stable app identifier.                                                                                                                                                                                                                        | Disconnect asks the hosted service to revoke it, then removes it locally. A network failure is reported; retry revocation before uninstalling.     |
| Optional usage upload                         | Provider, account label, observation time/source, and allowance windows go to Maxxit's HTTPS API.                                                                                                                                                                                                | Disable usage sharing to stop uploads. Hosted retention/deletion is covered by the website's privacy policy.                                       |
| Optional project upload                       | IDs, names, and descriptions you entered go to the HTTPS API when project sharing is enabled.                                                                                                                                                                                                    | Disable sharing to stop uploads. Delete cloud records/account through the hosted account controls. Disabling sharing does not erase prior uploads. |
| Update checks                                 | HTTPS requests to GitHub's release endpoint and signed update downloads.                                                                                                                                                                                                                         | GitHub receives normal network request metadata. Optional crash reporting is controlled separately in Settings.                                    |

No browser cookies, provider passwords, provider authentication files, full project
paths, or conversation text are sent to Maxxit cloud. Codex account identifiers are
hashed locally for grouping; hashes and derived labels are not anonymization guarantees.
The source JSONL files may contain conversations even though only usage fields are kept.

Mac Keychain protects the device credential. Native file dialogs require user
selection; autostart is optional. The Claude bridge can execute the pre-existing
status-line command solely to preserve its output after the user approves installation.

CSV export contains observed daily token totals and unknown dates as blanks.
Local uninstall and optional data removal are explained in install-macos.md.
Hosted services have separate storage, backups, and account deletion rules.

## Optional desktop diagnostics

Sentry receives diagnostics in the EU only after you enable reporting in Settings.
Ordinary reports contain fixed error codes, release and Mac metadata, random operation
IDs, sanitized stack positions and preceding safe events. They exclude project content,
provider data, conversations, credentials and raw request or error strings.

Unsent envelopes remain in the local diagnostics database for up to seven days, bounded
to 20 MiB and 2,000 envelopes. Reporting changes erase pending envelopes and history.
A separate native crash dump option requires explicit consent because dumps can contain
private memory, including credentials. It is off by default and requires restart to enable.
Turning it off stops dump uploads. Disabling reporting does not remove already uploaded
data from Sentry. Use Sentry's deletion controls for that data.

See [desktop diagnostics](sentry.md) for capture limits, release setup, retention and support export.
