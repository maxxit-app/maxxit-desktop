# Sentry verification record

Local verification on macOS, 2026-10-07:

- Frontend privacy fixtures remove private error text, user data, component props, URL queries and filesystem directories while retaining stack positions and Debug IDs.
- Native fixtures verify complete event scrubbing, native debug-image compatibility, independent category limits, HTTP 429 retention/backoff, restart persistence, consent generation invalidation and queue expiry.
- Synthetic debug subprocess probes persist command failures, panic-hook reports and native abort dumps. They bypass normal app setup, providers and Keychain. No dump or probe content was uploaded. Ordinary payloads contain no `PRIVATE_MARKER` fixture strings. This does not establish that binary dump memory is redacted.
- The explicit ingestion test sent a synthetic error and structured Log to the configured EU project. The ingest endpoint accepted both. Error event ID: `89a64bf28d0b4a26821ed86ac82ab98d`.

Dashboard processing, email alert receipt, private source-map/dSYM upload and signed installed builds on each architecture require owner access and release credentials. Do not count endpoint acceptance as verification of these gates. Segmentation faults, stack overflow and WebKit subprocess crashes were not established by the abort probe.

The local arm64 macOS app bundle built with ad-hoc signing. Its dSYM UUID matches
the app binary. Source maps remain in the ignored private artifact directory and
are absent from the shipped frontend directory. This build was not notarized.

## Owner setup on October 12, 2026

- The EU project is `maxxit-desktop`, ID `4512216749375568`. Its DSN is unchanged.
- An organization token with `org:ci` is stored in a root-only server file. The local release helper retrieves it over SSH into process memory. No administrative credential is committed or shipped, and the local release pipeline needs no GitHub Actions secret. Release creation and source commit association were verified with this credential.
- The production email rule targets contact@maxxit.app, includes new issues, regressions, high-priority changes and repeated error events, and throttles each issue to 30 minutes. A Sentry test email arrived in the owner's inbox.
- Reserved-volume warnings are set at 70 and 90 percent. Personal error, Log and attachment quota notifications are enabled, including the 100-percent limit.
- The account is on a Business trial with nine days remaining. On this date its trial allowances were 1 million errors, 500 GB of Logs and 10 GB of attachments. These are temporary allowances; recheck the subscription after the trial. No paid plan was selected.

Private artifact uploads and installed-build symbolication remain release acceptance checks. Physical Intel and macOS 14 verification require those environments.
