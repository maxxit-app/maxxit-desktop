import { useEffect, useSyncExternalStore } from "react";
import { ArrowDownToLine, Check, LoaderCircle, RefreshCw } from "lucide-react";
import { isTauri } from "@tauri-apps/api/core";
import { version } from "../package.json";
import { updates, watchUpdates } from "./updates";

export function UpdateNotice({ settings = false }: { settings?: boolean }) {
  const state = useSyncExternalStore(updates.subscribe, updates.snapshot);
  useEffect(watchUpdates, []);
  if (!isTauri())
    return settings ? (
      <section className="card settings-card">
        <h2>App updates</h2>
        <p className="small muted">
          Version {version}. Update checks are available in the Mac app.
        </p>
      </section>
    ) : null;
  const busy = ["checking", "downloading", "installing"].includes(state.status);
  if (!settings && (!state.version || state.status === "checking")) return null;
  const ready = state.status === "ready";
  const available = !!state.version;
  const label = ready
    ? "Restart Maxxit"
    : state.status === "downloading"
      ? `Downloading${state.progress === undefined ? "…" : ` ${state.progress}%`}`
      : state.status === "installing"
        ? "Installing…"
        : state.status === "checking"
          ? "Checking…"
          : available
            ? "Download update"
            : "Check for updates";
  return (
    <section
      className={
        settings ? "card settings-card update-settings" : "update-notice"
      }
      aria-label="App updates"
    >
      <div className="update-heading">
        <span className="update-symbol">
          {ready ? <Check size={20} /> : <ArrowDownToLine size={20} />}
        </span>
        <div>
          <h2>
            {ready
              ? "Your update is ready"
              : available
                ? `Maxxit ${state.version} is available`
                : "App updates"}
          </h2>
          <p>
            {ready
              ? "Restart when you're ready to use the new version."
              : available
                ? "A fresh version for your Mac. Install it here."
                : state.status === "current"
                  ? `You're up to date. Version ${version}.`
                  : `Version ${version}. Maxxit checks for updates automatically.`}
          </p>
        </div>
        <button
          className="button update-action"
          disabled={busy}
          onClick={() =>
            void (ready
              ? updates.restart()
              : available
                ? updates.install()
                : updates.check())
          }
        >
          {busy ? (
            <LoaderCircle className="spin" size={15} />
          ) : ready ? (
            <RefreshCw size={15} />
          ) : (
            <ArrowDownToLine size={15} />
          )}{" "}
          {label}
        </button>
      </div>
      {(state.status === "downloading" || state.status === "installing") && (
        <progress
          aria-label="Update download"
          max={100}
          value={state.progress}
        />
      )}
      {state.error && (
        <p className="update-error" role="alert">
          {state.error}
        </p>
      )}
      {available && state.notes && (
        <details className="update-notes">
          <summary>What's new</summary>
          <p>{state.notes}</p>
        </details>
      )}
      <span className="sr-only" role="status">
        {busy
          ? label
          : ready
            ? "Update installed. Restart Maxxit to finish."
            : available
              ? `Update ${state.version} available`
              : ""}
      </span>
    </section>
  );
}
