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
