import { useCallback, useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  ArrowUpRight,
  Bell,
  Clock3,
  Link2,
  Power,
  RefreshCw,
  Settings,
  X,
} from "lucide-react";
import {
  empty,
  remaining,
  resetLabel,
  type Snapshot,
  type Provider,
} from "./types";
import { demoSnapshot } from "./demo";
import "./tray.css";

const native = isTauri();
const demo = !native && new URLSearchParams(location.search).has("demo");
const count = (n: number) =>
  Intl.NumberFormat(undefined, {
    notation: "compact",
    maximumFractionDigits: 1,
  }).format(n);

function ProviderCard({
  provider,
  now,
  open,
}: {
  provider: Provider;
  now: number;
  open: () => void;
}) {
  const observation = provider.observation;
  const age = observation
    ? Math.max(
        0,
        Math.floor((now - Date.parse(observation.observedAt)) / 60000),
      )
    : null;
  const stale = age !== null && age >= 120;
  const windows = observation?.windows ?? [];
  const status = !provider.connected
    ? "Not connected"
    : stale
      ? "Stale data"
      : windows.some((w) => remaining(w, now) !== null)
        ? "Reporting"
        : "Waiting for usage";
  return (
    <section
      className="tray-provider"
      aria-label={`${provider.provider === "codex" ? "Codex" : "Claude Code"} allowance`}
    >
      <div className="tray-provider-head">
        <img
          src={`/providers/${provider.provider}.svg`}
          alt=""
          width="25"
          height="25"
        />
        <strong>
          {provider.provider === "codex" ? "Codex" : "Claude Code"}
        </strong>
        <span className={`tray-status ${status === "Reporting" ? "live" : ""}`}>
          {status}
        </span>
      </div>
      {windows.length ? (
        windows.map((w) => {
          const value = stale ? null : remaining(w, now);
          const reset = w.resetsAt
            ? new Date(w.resetsAt).toLocaleString(undefined, {
                weekday: "short",
                hour: "numeric",
                minute: "2-digit",
              })
            : "Unknown";
          return (
            <div className="tray-window" key={`${w.bucketId}:${w.windowId}`}>
              <div className="tray-window-head">
                <span>{w.label || "Allowance"}</span>
                <strong>
                  {value === null ? (
                    "Unavailable"
                  ) : (
                    <>
                      {Math.round(value)}
                      <small>% left</small>
                    </>
                  )}
                </strong>
              </div>
              <div
                className={`tray-meter ${value !== null && value < 20 ? "low" : ""}`}
                role="meter"
                aria-label={`${w.label || "Allowance"} remaining`}
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={value ?? undefined}
                aria-valuetext={
                  value === null
                    ? "Usage unavailable"
                    : `${Math.round(value)} percent remaining`
                }
              >
                <span style={{ width: `${value ?? 0}%` }} />
              </div>
              <div className="tray-reset" title={`Reset: ${reset}`}>
                <Clock3 size={12} />
                <span>{resetLabel(w.resetsAt, now)}</span>
                {value !== null && <span>{Math.round(100 - value)}% used</span>}
              </div>
            </div>
          );
        })
      ) : (
        <div className="tray-empty">
          <p>
            {provider.connected
              ? "Use your assistant to record its next allowance window."
              : "Connect local usage to see allowance and reset times."}
          </p>
          <button onClick={open}>
            <Link2 size={13} />
            {provider.connected ? "Connection details" : "Connect assistant"}
            <ArrowUpRight size={12} />
          </button>
        </div>
      )}
      {observation && (
        <div
          className="tray-source"
          title={new Date(observation.observedAt).toLocaleString()}
        >
          Observed{" "}
          {age === 0
            ? "just now"
            : age === 1
              ? "1 minute ago"
              : `${age} minutes ago`}
          {stale && ". Use your assistant for a fresh reading."}
        </div>
      )}
      {provider.error && (
        <p className="tray-error" role="status">
          {provider.error}
        </p>
      )}
    </section>
  );
}

export default function TrayPanel() {
  const [data, setData] = useState<Snapshot>(demo ? demoSnapshot() : empty);
  const [now, setNow] = useState(Date.now());
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const refresh = useCallback(async () => {
    if (!native) return;
    setBusy(true);
    try {
      setData(await invoke<Snapshot>("snapshot"));
      setError("");
    } catch {
      setError("Could not refresh usage. Try again.");
    } finally {
      setBusy(false);
    }
  }, []);
  const action = useCallback(async (page: string) => {
    if (native) {
      try {
        await invoke("tray_action", { page });
      } catch {
        setError("Could not open Maxxit. Try again.");
      }
    }
  }, []);
  useEffect(() => {
    document.body.classList.add("tray-body");
    void refresh();
    const timer = setInterval(() => setNow(Date.now()), 15000);
    let disposed = false;
    let unlisten: (() => void) | undefined;
    if (native)
      void listen("usage-updated", () => {
        if (!document.hidden) void refresh();
      }).then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      });
    const visible = () => {
      if (!document.hidden) void refresh();
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Escape") void action("close");
    };
    document.addEventListener("visibilitychange", visible);
    window.addEventListener("focus", visible);
    window.addEventListener("keydown", escape);
    return () => {
      disposed = true;
      clearInterval(timer);
      unlisten?.();
      document.body.classList.remove("tray-body");
      document.removeEventListener("visibilitychange", visible);
      window.removeEventListener("focus", visible);
      window.removeEventListener("keydown", escape);
    };
  }, [refresh, action]);
  useEffect(() => {
    document.documentElement.dataset.theme = data.settings.theme;
  }, [data.settings.theme]);
  useEffect(() => {
    if (!native) return;
    const inner = document.querySelector(".tray-content-inner");
    const header = document.querySelector(".tray-header");
    const footer = document.querySelector(".tray-footer");
    if (!inner || !header || !footer) return;
    const resize = () => {
      const height = Math.ceil(
        inner.getBoundingClientRect().height +
          header.getBoundingClientRect().height +
          footer.getBoundingClientRect().height +
          24,
      );
      void invoke("tray_resize", { height }).catch(() => {});
    };
    const observer = new ResizeObserver(resize);
    for (const element of [inner, header, footer]) observer.observe(element);
    return () => observer.disconnect();
  }, []);
  const codex = data.providers.find((p) => p.provider === "codex");
  const days = Array.from({ length: 7 }, (_, i) => {
    const date = new Date(now);
    date.setUTCDate(date.getUTCDate() - 6 + i);
    const key = date.toISOString().slice(0, 10);
    return {
      date,
      tokens: codex?.dailyTokens.find((d) => d.startDate === key)?.tokens ?? 0,
    };
  });
  const total = days.reduce((sum, day) => sum + day.tokens, 0);
  const max = Math.max(1, ...days.map((d) => d.tokens));
  const reminders = !data.cloud.connected
    ? "Account not linked"
    : data.cloud.plan !== "pro"
      ? "Pro required"
      : !data.settings.email
        ? "Off"
        : data.cloud.data?.emailReady
          ? "On"
          : "Email unavailable";
  return (
    <main className="tray-panel" data-theme={data.settings.theme}>
      <header className="tray-header">
        <div>
          <span className="tray-brand">
            maxxit<span>.</span>
          </span>
        </div>
        <div className="tray-header-actions">
          <button
            aria-label="Refresh usage"
            title="Refresh local usage"
            onClick={() => void refresh()}
            disabled={busy}
          >
            <RefreshCw size={16} className={busy ? "tray-spinning" : ""} />
          </button>
          <button
            aria-label="Close usage panel"
            onClick={() => void action("close")}
          >
            <X size={16} />
          </button>
        </div>
      </header>
      <div className="tray-content">
        <div className="tray-content-inner">
          {demo && <div className="tray-demo">Preview with sample data</div>}
          {error && (
            <p role="alert" className="tray-error">
              {error}
            </p>
          )}
          {data.providers.map((provider) => (
            <ProviderCard
              key={provider.provider}
              provider={provider}
              now={now}
              open={() => void action("Connections")}
            />
          ))}
          <section
            className="tray-activity"
            aria-label="Observed Codex token activity"
          >
            <div className="tray-activity-head">
              <div>
                <h2>Local activity</h2>
                <p>Codex tokens · last 7 days</p>
              </div>
              <strong>{total > 0 ? count(total) : "No data"}</strong>
            </div>
            {total > 0 ? (
              <>
                <div className="tray-chart">
                  {days.map((day, i) => (
                    <div
                      key={i}
                      className="tray-day"
                      role="img"
                      aria-label={`${day.date.toLocaleDateString(undefined, { timeZone: "UTC" })}: ${day.tokens.toLocaleString()} observed tokens`}
                      title={`${day.date.toLocaleDateString(undefined, { timeZone: "UTC" })}: ${day.tokens.toLocaleString()} observed tokens`}
                    >
                      <span
                        style={{
                          height: `${Math.max(day.tokens ? 5 : 0, (day.tokens / max) * 44)}px`,
                        }}
                      />
                      <small>
                        {day.date.toLocaleDateString(undefined, {
                          weekday: "narrow",
                          timeZone: "UTC",
                        })}
                      </small>
                    </div>
                  ))}
                </div>
                <p className="tray-chart-note">
                  Observed session increments. Partial history.
                </p>
              </>
            ) : (
              <p className="tray-chart-note">
                Activity appears after local Codex sessions. Claude does not
                report token history through its status line.
              </p>
            )}
          </section>
          <button
            className="tray-reminders"
            onClick={() => void action("Settings")}
          >
            <Bell size={15} />
            <span>Reset reminders</span>
            <small>{reminders}</small>
            <ArrowUpRight size={13} />
          </button>
        </div>
      </div>
      <footer className="tray-footer">
        <button className="tray-open" onClick={() => void action("Overview")}>
          Open dashboard
          <ArrowUpRight size={14} />
        </button>
        <button
          aria-label="Open settings"
          title="Settings"
          onClick={() => void action("Settings")}
        >
          <Settings size={16} />
        </button>
        <button
          aria-label="Quit Maxxit"
          title="Quit Maxxit"
          onClick={() => void action("quit")}
        >
          <Power size={16} />
        </button>
      </footer>
    </main>
  );
}
