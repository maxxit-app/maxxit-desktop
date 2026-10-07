import { useEffect, useState } from "react";
import { invoke as nativeInvoke, isTauri } from "@tauri-apps/api/core";
import {
  diagnosticContext,
  recordDiagnostic,
  setDiagnosticHealth,
  type DiagnosticHealth,
} from "./observability";

export default function DiagnosticsSettings() {
  const [health, setHealth] = useState<DiagnosticHealth>();
  const [message, setMessage] = useState("");
  const [preview, setPreview] = useState("");
  const [busy, setBusy] = useState(false);
  const update = (value: DiagnosticHealth) => {
    setHealth(value);
    setDiagnosticHealth(value);
  };
  useEffect(() => {
    if (!isTauri()) return;
    let disposed = false;
    const refresh = () =>
      void nativeInvoke<DiagnosticHealth>("diagnostics_health")
        .then((value) => {
          if (!disposed) update(value);
        })
        .catch(() => {
          if (!disposed) setMessage("Diagnostics are unavailable.");
        });
    refresh();
    const timer = setInterval(refresh, 5000);
    return () => {
      disposed = true;
      clearInterval(timer);
    };
  }, []);
  async function consent(enabled: boolean, nativeCrashes: boolean) {
    setBusy(true);
    setMessage("");
    setPreview("");
    try {
      update(
        await nativeInvoke<DiagnosticHealth>("diagnostics_consent", {
          enabled,
          nativeCrashes,
        }),
      );
      if (nativeCrashes)
        setMessage("Restart Maxxit to enable native crash capture.");
    } catch {
      setMessage("Diagnostic preferences could not be saved.");
    } finally {
      setBusy(false);
    }
  }
  async function test() {
    setBusy(true);
    try {
      const result = await nativeInvoke<{
        eventId?: string;
        health: DiagnosticHealth;
      }>("diagnostics_test");
      update(result.health);
      setMessage(
        result.eventId
          ? `Test report ${result.eventId}. Check this ID in Sentry to confirm it appears.`
          : "The test could not be queued.",
      );
    } catch {
      setMessage("The test report could not be queued.");
    } finally {
      setBusy(false);
    }
  }
  async function showPreview() {
    try {
      setPreview(
        JSON.stringify(await nativeInvoke("diagnostics_preview"), null, 2),
      );
    } catch {
      setMessage("The support preview is unavailable.");
    }
  }
  async function exportPreview() {
    if (!preview) return;
    try {
      await nativeInvoke("diagnostics_export");
    } catch {
      setMessage("The support export could not be saved.");
    }
    recordDiagnostic("ui.command.completed", diagnosticContext("frontend"));
  }
  return (
    <section className="card settings-card">
      <h2>Diagnostic reporting</h2>
      <p>
        Send error reports to Sentry in the EU. Reports include app and Mac
        versions, safe error codes, and the steps before a failure. Project
        content, conversations, and credentials are excluded from ordinary
        reports.
      </p>
      <label className="field-row">
        <span>Share error reports</span>
        <input
          type="checkbox"
          checked={health?.enabled ?? false}
          disabled={!health || busy}
          onChange={(e) => void consent(e.target.checked, false)}
        />
      </label>
      <p className="small muted">
        Optional. Turning reporting off removes queued reports. Local allowance
        monitoring works with it off.
      </p>
      {health?.enabled && (
        <>
          <label className="field-row">
            <span>Include native crash dumps</span>
            <input
              type="checkbox"
              checked={health.nativeCrashes}
              disabled={busy}
              onChange={(e) => void consent(true, e.target.checked)}
            />
          </label>
          <p className="small muted">
            Crash dumps can contain memory, including private content and
            credentials. Enable only if you accept sharing that data. Enabling
            requires a restart. Turning it off stops dump uploads and removes
            queued dumps.
          </p>
          <p className="small">
            Delivery: {health.deliveryState.replaceAll("_", " ")}.{" "}
            {health.pending} queued. {health.dropped} dropped or expired.
            {health.pending > 0 &&
              ` Oldest report: ${Math.floor(health.oldestAgeSeconds / 60)} minutes.`}
          </p>
          {health.lastAcceptedAt && (
            <p className="small muted">
              Last accepted by the ingest endpoint:{" "}
              {new Date(health.lastAcceptedAt * 1000).toLocaleString()}.
              Acceptance does not confirm dashboard processing.
            </p>
          )}
          {!health.configured && (
            <p className="small muted">
              This build has no Sentry DSN. Reports stay queued within the
              retention limit.
            </p>
          )}
          <button
            className="button secondary"
            disabled={busy}
            onClick={() => void test()}
          >
            Send a test report
          </button>
        </>
      )}
      <button
        className="button secondary"
        disabled={!health}
        onClick={() => void showPreview()}
      >
        Preview support diagnostics
      </button>
      {message && <p role="status">{message}</p>}
      {preview && (
        <>
          <pre
            className="code-block"
            style={{ maxHeight: 240, overflow: "auto" }}
          >
            {preview}
          </pre>
          <button
            className="button secondary"
            onClick={() => void exportPreview()}
          >
            Export this preview
          </button>
        </>
      )}
    </section>
  );
}
