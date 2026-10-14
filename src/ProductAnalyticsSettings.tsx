import { useEffect, useState } from "react";
import { invoke as nativeInvoke, isTauri } from "@tauri-apps/api/core";
import { setProductAnalyticsConsent } from "./product-analytics";
type Health = { choice: boolean | null; delivery: string };
export default function ProductAnalyticsSettings({
  prompt = false,
}: {
  prompt?: boolean;
}) {
  const [health, setHealth] = useState<Health>();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  useEffect(() => {
    if (!isTauri()) return;
    let disposed = false;
    const refresh = () =>
      void nativeInvoke<Health>("product_analytics_health")
        .then((value) => {
          if (!disposed) setHealth(value);
        })
        .catch(() => {
          if (!disposed) setError("Analytics preferences are unavailable.");
        });
    refresh();
    const timer = setInterval(refresh, 5000);
    return () => {
      disposed = true;
      clearInterval(timer);
    };
  }, []);
  async function choose(enabled: boolean) {
    setBusy(true);
    setError("");
    if (!enabled) setProductAnalyticsConsent(false);
    try {
      setHealth(
        await nativeInvoke<Health>("product_analytics_consent", { enabled }),
      );
    } catch {
      setError("Analytics preferences could not be saved.");
    } finally {
      setBusy(false);
    }
  }
  if (prompt && (!health || health.choice !== null)) return null;
  return (
    <section className="card settings-card" aria-label="Product analytics">
      <h2>{prompt ? "Help us improve Maxxit" : "Product analytics"}</h2>
      <p>
        Allow Google Analytics to measure screens, feature use, app version,
        action outcomes, and time spent in Maxxit. Events use a random
        identifier and go through the Maxxit service. Project content, prompts,
        emails, account IDs, paths, and credentials are excluded.
      </p>
      <p className="small muted">
        Optional and off until you allow it. This choice is separate from cloud
        sharing and error reports. Turning it off removes the analytics
        identifier. Google may process data outside the EU.
      </p>
      {prompt ? (
        <div className="button-row">
          <button
            className="button secondary"
            disabled={busy}
            onClick={() => void choose(false)}
          >
            Decline analytics
          </button>
          <button
            className="button"
            disabled={busy}
            onClick={() => void choose(true)}
          >
            Allow analytics
          </button>
        </div>
      ) : (
        <>
          <label className="field-row">
            <span>Share product analytics</span>
            <input
              type="checkbox"
              disabled={!health || busy}
              checked={health?.choice === true}
              onChange={(e) => void choose(e.target.checked)}
            />
          </label>
          {health?.choice && (
            <p className="small muted">
              {health.delivery === "not_configured"
                ? "Analytics is waiting for service configuration."
                : health.delivery === "unavailable"
                  ? "Analytics delivery is unavailable. Events are not saved for retry."
                  : health.delivery === "accepted"
                    ? "The latest event reached the ingest endpoint. Dashboard processing may take time."
                    : "Events send while you use Maxxit with an active connection."}
            </p>
          )}
        </>
      )}
      {error && <p role="status">{error}</p>}
    </section>
  );
}
