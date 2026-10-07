import DiagnosticsSettings from "./DiagnosticsSettings";
import { version } from "../package.json";
import { useCallback, useEffect, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { invoke, captureDiagnostic } from "./observability";
import { listen } from "@tauri-apps/api/event";
import { enable, disable, isEnabled } from "@tauri-apps/plugin-autostart";
import {
  Activity,
  ArrowUpRight,
  Check,
  ChevronRight,
  Folder,
  LayoutDashboard,
  Lightbulb,
  Link2,
  LoaderCircle,
  Plus,
  RefreshCw,
  Settings as SettingsIcon,
  ShieldCheck,
  Terminal,
  Trash2,
  X,
} from "lucide-react";
import {
  empty,
  remaining,
  resetLabel,
  type Snapshot,
  type Provider,
  type Settings,
} from "./types";
import { demoSnapshot } from "./demo";
import { UpdateNotice } from "./UpdateNotice";
import { TokenAnalytics } from "./TokenAnalytics";

const native = isTauri();
const demo = !native && new URLSearchParams(location.search).has("demo");
const tabs = [
  ["Overview", LayoutDashboard],
  ["Analytics", Activity],
  ["Projects", Folder],
  ["Ideas", Lightbulb],
  ["Connections", Link2],
  ["Settings", SettingsIcon],
] as const;
function ProviderMark({ name }: { name: string }) {
  return (
    <span className={`provider-mark ${name}`} aria-label={name}>
      <img src={`/providers/${name}.svg`} alt="" width={23} height={23} />
    </span>
  );
}
function WindowCard({ provider }: { provider: Provider }) {
  const windows = provider.observation?.windows ?? [];
  const stale =
    provider.observation &&
    Date.now() - Date.parse(provider.observation.observedAt) > 2 * 3600000;
  return (
    <article className="card allowance-card">
      <div className="card-top">
        <div className="provider-heading">
          <ProviderMark name={provider.provider} />
          <div>
            <h3>{provider.provider === "codex" ? "Codex" : "Claude Code"}</h3>
            <span className="muted small">{provider.accountLabel}</span>
          </div>
        </div>
        <span className={`status ${provider.connected ? "connected" : ""}`}>
          {stale
            ? "Stale"
            : provider.status === "waiting"
              ? "Waiting"
              : provider.connected
                ? "Connected"
                : "Not connected"}
        </span>
      </div>
      {windows.length ? (
        windows.map((w) => {
          const value = stale ? null : remaining(w);
          return (
            <div className="allowance" key={`${w.bucketId}:${w.windowId}`}>
              <div className="allowance-label">
                <span>{w.label ?? w.windowId}</span>
                <span className="mono">
                  {value === null ? "—" : `${Math.round(value)}% left`}
                </span>
              </div>
              <div className="progress">
                <span style={{ width: `${value ?? 0}%` }} />
              </div>
              <div className="small muted">
                {resetLabel(w.resetsAt)}
                {value === null ? " · Usage unavailable" : ""}
              </div>
            </div>
          );
        })
      ) : (
        <div className="provider-empty">
          <Terminal size={26} />
          <p>
            {provider.status === "waiting"
              ? "Use your coding assistant to record its next usage update."
              : "Connect your local coding assistant to see its allowance."}
          </p>
        </div>
      )}
      <div className="card-foot">
        <span className="dot" />
        {provider.observation
          ? `Observed ${new Date(provider.observation.observedAt).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}`
          : "Stored on your Mac"}
      </div>
    </article>
  );
}
function Toggle({
  checked,
  onChange,
  label,
  detail,
  disabled = false,
}: {
  checked: boolean;
  onChange: (value: boolean) => void;
  label: string;
  detail?: string;
  disabled?: boolean;
}) {
  return (
    <label className="toggle-row">
      <span>
        <strong>{label}</strong>
        {detail && <small>{detail}</small>}
      </span>
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        disabled={disabled}
      />
      <span className="switch" />
    </label>
  );
}
export default function App() {
  const [data, setData] = useState<Snapshot>(demo ? demoSnapshot() : empty);
  const [tab, setTab] = useState("Overview");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [add, setAdd] = useState(false);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [preview, setPreview] = useState<{
    command: string;
    settingsPath: string;
    description: string;
  } | null>(null);
  const [autostart, setAutostart] = useState(false);
  const [pairing, setPairing] = useState<{ code: string; url: string } | null>(
    null,
  );
  const [shareUsage, setShareUsage] = useState(false);
  const [shareProjects, setShareProjects] = useState(false);
  const refresh = useCallback(async () => {
    if (native) {
      try {
        setData(await invoke<Snapshot>("snapshot"));
      } catch (e) {
        captureDiagnostic(e);
        setMessage(String(e));
      }
    }
  }, []);
  useEffect(() => {
    void refresh();
    if (native)
      void isEnabled()
        .then(setAutostart)
        .catch((error) => captureDiagnostic(error, "autostart.read.failed"));
    let cleanup: (() => void) | undefined;
    if (native)
      void listen("usage-updated", () => void refresh())
        .then((f) => {
          cleanup = f;
        })
        .catch((error) => captureDiagnostic(error, "ui.listener.failed"));
    const timer = setInterval(() => void refresh(), 60000);
    return () => {
      clearInterval(timer);
      cleanup?.();
    };
  }, [refresh]);
  useEffect(() => {
    if (!native) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen<string>("navigate-to", (event) => setTab(event.payload))
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch((error) => captureDiagnostic(error, "ui.listener.failed"));
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);
  useEffect(() => {
    document.documentElement.dataset.theme = data.settings.theme;
  }, [data.settings.theme]);
  async function action(
    callback: () => Promise<void>,
    diagnosticName = "ui.command.failed",
  ) {
    setBusy(true);
    setMessage("");
    try {
      await callback();
      await refresh();
    } catch (e) {
      captureDiagnostic(e, diagnosticName);
      setMessage(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function settings(patch: Partial<Settings>) {
    const updated = { ...data.settings, ...patch };
    if (native) {
      await action(async () => {
        await invoke("save_settings", { settings: updated });
        if (data.cloud.connected && data.cloud.plan === "pro")
          await invoke("cloud_preferences");
      });
    } else setData({ ...data, settings: updated });
  }
  async function connect(provider: string) {
    if (!native) {
      setMessage("Open the desktop app to connect a provider.");
      return;
    }
    if (provider === "claude") {
      await action(async () => setPreview(await invoke("claude_preview")));
    } else
      await action(async () => {
        await invoke("connect_provider", { provider });
      });
  }
  const title: Record<string, [string, string]> = {
    Overview: [
      "Usage overview",
      "Remaining usage and reset times for your connected assistants.",
    ],
    Analytics: [
      "Usage analytics",
      "Daily token usage from Codex records on this Mac.",
    ],
    Projects: [
      "Projects",
      "Add project descriptions to get task suggestions with Maxxit Pro.",
    ],
    Ideas: ["Project ideas", "Suggested tasks and prompts for your projects."],
    Connections: [
      "Connections",
      "Connect Codex, Claude Code, or your Maxxit account.",
    ],
    Settings: [
      "Settings",
      "Manage appearance, menu bar settings, and email reminders.",
    ],
  };
  return (
    <div className="app-shell">
      <aside>
        <div className="brand">
          <img src="/maxxit-mark.png" alt="" />
          <span>
            maxxit<span className="brand-period">.</span>
          </span>
        </div>
        <div className="workspace-label">YOUR WORKSPACE</div>
        <nav>
          {tabs.map(([label, Icon]) => (
            <button
              key={label}
              className={tab === label ? "active" : ""}
              onClick={() => setTab(label)}
            >
              <Icon size={18} />
              {label}
              {label === "Ideas" && <span className="pro-pill">PRO</span>}
            </button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <div className="account">
            <div className="avatar">M</div>
            <div>
              <strong>This Mac</strong>
              <span>{demo ? "Demo workspace" : "Free plan"}</span>
            </div>
            <ChevronRight size={15} />
          </div>
        </div>
      </aside>
      <main>
        <div className="topbar" data-tauri-drag-region>
          <span>
            Workspace <ChevronRight size={12} /> {tab}
          </span>
          <button
            className="icon-button"
            aria-label="Refresh usage"
            onClick={() => void action(refresh)}
            disabled={busy}
          >
            {busy ? (
              <LoaderCircle className="spin" size={17} />
            ) : (
              <RefreshCw size={17} />
            )}
          </button>
        </div>
        <div className="page">
          <header>
            <h1>{title[tab][0]}</h1>
            <p>{title[tab][1]}</p>
          </header>
          {tab !== "Settings" && <UpdateNotice />}
          {demo && (
            <div className="notice">
              Demo workspace · sample data for this preview.
            </div>
          )}
          {message && (
            <div className="notice error" role="alert">
              {message}
              <button
                className="icon-button"
                aria-label="Dismiss message"
                onClick={() => setMessage("")}
              >
                <X size={14} />
              </button>
            </div>
          )}
          {tab === "Overview" && (
            <>
              <div className="section-heading">
                <h2>Connected assistants</h2>
                <button
                  className="text-button"
                  onClick={() => setTab("Connections")}
                >
                  Manage connections <ArrowUpRight size={14} />
                </button>
              </div>
              <div className="provider-grid">
                {data.providers.map((p) => (
                  <WindowCard key={p.provider} provider={p} />
                ))}
              </div>
              <div className="insight-strip">
                <div className="insight-icon">
                  <Lightbulb size={24} />
                </div>
                <div>
                  <h3>Project suggestions</h3>
                  <p>Add a project to get task suggestions with Maxxit Pro.</p>
                </div>
                <button
                  className="button secondary"
                  onClick={() => setTab("Projects")}
                >
                  Add a project <Plus size={15} />
                </button>
              </div>
              <div className="section-heading">
                <h2>Recent observations</h2>
              </div>
              <div className="card history">
                {data.history.length ? (
                  data.history.slice(0, 6).map((o, i) => (
                    <div
                      className="history-row"
                      key={`${o.provider}:${o.observedAt}:${i}`}
                    >
                      <ProviderMark name={o.provider} />
                      <div>
                        <strong>
                          {o.provider === "codex" ? "Codex" : "Claude Code"}
                        </strong>
                        <small>{o.accountLabel}</small>
                      </div>
                      <span className="mono small">
                        {o.windows.length} windows
                      </span>
                      <time className="small muted">
                        {new Date(o.observedAt).toLocaleString()}
                      </time>
                    </div>
                  ))
                ) : (
                  <div className="empty-state">
                    <Activity size={27} />
                    <h3>No usage history yet</h3>
                    <p>Connect an assistant, then use it as usual.</p>
                    <button
                      className="button"
                      onClick={() => setTab("Connections")}
                    >
                      Connect an assistant
                    </button>
                  </div>
                )}
              </div>
            </>
          )}
          {tab === "Analytics" && (
            <>
              <div className="provider-grid">
                {data.providers.map((p) => (
                  <WindowCard key={p.provider} provider={p} />
                ))}
              </div>
              <TokenAnalytics
                records={
                  data.providers.find(
                    (provider) => provider.provider === "codex",
                  )?.dailyTokens ?? []
                }
              />
            </>
          )}
          {tab === "Projects" && (
            <>
              <div className="section-heading">
                <span>{data.projects.length} projects</span>
                <button className="button" onClick={() => setAdd(true)}>
                  <Plus size={16} /> Add project
                </button>
              </div>
              <div className="project-grid">
                {data.projects.map((p) => (
                  <article className="card project-card" key={p.id}>
                    <div className="card-top">
                      <span className="project-icon">
                        <Folder size={23} />
                      </span>
                      <button
                        className="icon-button"
                        aria-label={`Remove ${p.name}`}
                        onClick={() =>
                          void action(async () => {
                            if (native)
                              await invoke("remove_project", { id: p.id });
                            else
                              setData({
                                ...data,
                                projects: data.projects.filter(
                                  (x) => x.id !== p.id,
                                ),
                              });
                          })
                        }
                      >
                        <Trash2 size={15} />
                      </button>
                    </div>
                    <h3>{p.name}</h3>
                    <p>{p.description || "No description yet."}</p>
                  </article>
                ))}
              </div>
              {!data.projects.length && (
                <div className="card empty-state">
                  <Folder size={30} />
                  <h3>No projects yet</h3>
                  <p>Add a project name and description.</p>
                  <button className="button" onClick={() => setAdd(true)}>
                    Add your first project <Plus size={15} />
                  </button>
                </div>
              )}
              <p className="small muted">
                Project descriptions stay local unless you choose to share them
                for ideas.
              </p>
            </>
          )}
          {tab === "Ideas" && data.cloud.plan === "pro" && (
            <>
              <div className="section-heading">
                <h2>Suggested tasks</h2>
                <button
                  className="button"
                  disabled={busy || !data.settings.aiConsent}
                  onClick={() =>
                    void action(async () => {
                      await invoke("cloud_action", { action: "generate" });
                      setMessage(
                        "Ideas are being prepared. This view refreshes automatically.",
                      );
                    })
                  }
                >
                  Refresh ideas <RefreshCw size={15} />
                </button>
              </div>
              {!data.settings.aiConsent && (
                <div className="notice">
                  Enable project sharing in Settings to request ideas.
                </div>
              )}
              {data.cloud.data?.suggestions?.map((idea) => (
                <article className="card project-card" key={idea.id}>
                  <h3>{idea.title}</h3>
                  <p>{idea.description}</p>
                  <code className="code-block">{idea.readyToCopyPrompt}</code>
                  <button
                    className="button secondary"
                    onClick={() =>
                      void navigator.clipboard
                        .writeText(idea.readyToCopyPrompt)
                        .then(() => setMessage("Prompt copied."))
                    }
                  >
                    Copy prompt
                  </button>
                </article>
              ))}
            </>
          )}
          {tab === "Ideas" && data.cloud.plan !== "pro" && (
            <div className="card pro-card">
              <div className="pro-symbol">
                <Lightbulb size={35} />
              </div>
              <span className="tag">MAXXIT PRO</span>
              <h2>Project ideas and reset reminders</h2>
              <p>
                Get project ideas and an email before a selected allowance
                window resets. You choose the task and run it in your coding
                assistant.
              </p>
              <ul>
                <li>
                  <Check size={17} /> Ideas based on the projects you share
                </li>
                <li>
                  <Check size={17} /> Reset reminders with quiet hours
                </li>
                <li>
                  <Check size={17} /> Up to 10 idea refreshes per day
                </li>
              </ul>
              <div className="price">
                $9.99 <span>/ month</span>
              </div>
              <button
                className="button"
                onClick={() =>
                  void action(async () => {
                    if (!data.cloud.connected) {
                      setTab("Connections");
                      setMessage("Connect your Maxxit account to subscribe.");
                      return;
                    }
                    const result = await invoke<{ url: string }>(
                      "cloud_action",
                      { action: "checkout" },
                    );
                    await invoke("open_link", { url: result.url });
                  })
                }
              >
                Get Maxxit Pro <ArrowUpRight size={15} />
              </button>
              <small>Cancel anytime. Local analytics stay free.</small>
            </div>
          )}
          {tab === "Connections" && (
            <>
              <div className="notice">
                <ShieldCheck size={18} /> Maxxit never reads or stores your
                Codex or Claude sign-in credentials.
              </div>
              {data.providers.map((p) => (
                <article className="card connection-card" key={p.provider}>
                  <div className="provider-heading">
                    <ProviderMark name={p.provider} />
                    <div>
                      <h2>
                        {p.provider === "codex" ? "Codex" : "Claude Code"}
                      </h2>
                      <p>
                        {p.provider === "codex"
                          ? "Read allowance windows and token increments from local session records."
                          : "Record allowance windows from the documented Claude Code status line."}
                      </p>
                    </div>
                  </div>
                  <div className="connection-detail">
                    <code>
                      {p.provider === "codex" ? "codex login" : "claude"}
                    </code>
                    <span>
                      Sign in using the provider's CLI. Then connect local usage
                      below.
                    </span>
                  </div>
                  <div className="connection-actions">
                    <span className="small muted">
                      {p.provider === "claude"
                        ? "You can review the status-line change before applying it."
                        : "Only usage observations are retained."}
                    </span>
                    <button
                      className={`button ${p.connected || p.status === "waiting" ? "secondary" : ""}`}
                      disabled={busy}
                      onClick={() =>
                        void (p.connected || p.status === "waiting"
                          ? action(async () => {
                              await invoke("disconnect_provider", {
                                provider: p.provider,
                              });
                            })
                          : connect(p.provider))
                      }
                    >
                      {p.connected || p.status === "waiting"
                        ? "Disconnect"
                        : "Connect local usage"}{" "}
                      <Link2 size={15} />
                    </button>
                  </div>
                </article>
              ))}
              <article className="card connection-card">
                <h2>Maxxit account</h2>
                <p>
                  Connect your account for paid ideas and email reminders. Local
                  analytics work without an account.
                </p>
                {data.cloud.error && (
                  <div className="notice error">{data.cloud.error}</div>
                )}
                {data.cloud.connected ? (
                  <div className="connection-actions">
                    <span className="tag">{data.cloud.plan.toUpperCase()}</span>
                    <button
                      className="button secondary"
                      onClick={() =>
                        void action(async () => {
                          await invoke("cloud_disconnect");
                        })
                      }
                    >
                      Disconnect account
                    </button>
                    <button
                      className="button"
                      onClick={() =>
                        void action(async () => {
                          const result = await invoke<{ url: string }>(
                            "cloud_action",
                            {
                              action:
                                data.cloud.plan === "pro"
                                  ? "portal"
                                  : "checkout",
                            },
                          );
                          await invoke("open_link", { url: result.url });
                        })
                      }
                    >
                      {data.cloud.plan === "pro"
                        ? "Manage subscription"
                        : "Get Pro"}
                    </button>
                  </div>
                ) : (
                  <>
                    <Toggle
                      label="Allow allowance sync"
                      detail="Share percentages and reset times for reminders."
                      checked={shareUsage}
                      onChange={setShareUsage}
                    />
                    <Toggle
                      label="Allow project sharing"
                      detail="Share descriptions you write for project ideas."
                      checked={shareProjects}
                      onChange={setShareProjects}
                    />
                    {pairing ? (
                      <div className="connection-actions">
                        <span>
                          Approve code{" "}
                          <strong className="mono">{pairing.code}</strong> in
                          your browser.
                        </span>
                        <button
                          className="button"
                          onClick={() =>
                            void action(async () => {
                              await invoke("cloud_redeem");
                              await settings({
                                cloudSync: shareUsage,
                                aiConsent: shareProjects,
                              });
                              setPairing(null);
                            })
                          }
                        >
                          I've approved it
                        </button>
                      </div>
                    ) : (
                      <button
                        className="button"
                        onClick={() =>
                          void action(async () => {
                            if (!native) {
                              setMessage(
                                "Open Maxxit desktop to connect your account.",
                              );
                              return;
                            }
                            const result = await invoke<{
                              code: string;
                              url: string;
                            }>("cloud_start", {
                              usage: shareUsage,
                              metadata: shareProjects,
                            });
                            setPairing(result);
                            await invoke("open_link", { url: result.url });
                          })
                        }
                      >
                        Connect Maxxit account <ArrowUpRight size={15} />
                      </button>
                    )}
                  </>
                )}
              </article>
              <p className="small muted">
                Direct third-party OAuth is not enabled. Provider policies limit
                how subscription authentication can be used by commercial apps.
              </p>
            </>
          )}
          {tab === "Settings" && (
            <div className="settings-grid">
              <UpdateNotice settings />
              <DiagnosticsSettings />
              <section className="card settings-card">
                <h2>On this Mac</h2>
                <label className="field-row">
                  <span>Appearance</span>
                  <select
                    value={data.settings.theme}
                    onChange={(e) => void settings({ theme: e.target.value })}
                  >
                    <option value="system">System</option>
                    <option value="light">Light</option>
                    <option value="dark">Dark</option>
                  </select>
                </label>
                <label className="field-row">
                  <span>Menu bar provider</span>
                  <select
                    value={data.settings.trayProvider}
                    onChange={(e) =>
                      void settings({ trayProvider: e.target.value })
                    }
                  >
                    <option value="codex">Codex</option>
                    <option value="claude">Claude Code</option>
                  </select>
                </label>
                <Toggle
                  label="Keep running in the menu bar"
                  detail="Closing the window keeps usage up to date."
                  checked={data.settings.background}
                  onChange={(background) => void settings({ background })}
                />
                <Toggle
                  label="Start at login"
                  detail="Open Maxxit when you sign in to your Mac."
                  checked={autostart}
                  onChange={(value) =>
                    void action(async () => {
                      if (!native) return;
                      if (value) await enable();
                      else await disable();
                      setAutostart(value);
                    }, "autostart.change.failed")
                  }
                />
              </section>
              <section className="card settings-card">
                <h2>
                  Reset reminders <span className="pro-pill">PRO</span>
                </h2>
                <p className="small muted">
                  Available after connecting your Maxxit Pro account.
                </p>
                <Toggle
                  label="Email before a reset"
                  detail="Only for windows with fresh usage and spare allowance."
                  checked={data.settings.email}
                  onChange={(email) => void settings({ email })}
                  disabled={!data.cloud.connected || data.cloud.plan !== "pro"}
                />
                <label className="field-row">
                  <span>Short window notice</span>
                  <select
                    value={data.settings.shortHoursBefore}
                    onChange={(e) =>
                      void settings({
                        shortHoursBefore: Number(e.target.value),
                      })
                    }
                  >
                    {[1, 2, 3, 4].map((n) => (
                      <option value={n} key={n}>
                        {n} hours before
                      </option>
                    ))}
                  </select>
                </label>
                <label className="field-row">
                  <span>Weekly notice</span>
                  <select
                    value={data.settings.hoursBefore}
                    onChange={(e) =>
                      void settings({ hoursBefore: Number(e.target.value) })
                    }
                  >
                    {[6, 12, 24, 48].map((n) => (
                      <option value={n} key={n}>
                        {n} hours before
                      </option>
                    ))}
                  </select>
                </label>
                <label className="field-row">
                  <span>Minimum remaining</span>
                  <input
                    type="number"
                    min="0"
                    max="100"
                    value={data.settings.minRemaining}
                    onChange={(e) =>
                      void settings({ minRemaining: Number(e.target.value) })
                    }
                  />
                  <span>%</span>
                </label>
                <label className="field-row">
                  <span>Quiet hours</span>
                  <input
                    aria-label="Quiet hours start"
                    type="number"
                    min="0"
                    max="23"
                    value={data.settings.quietStart}
                    onChange={(e) =>
                      void settings({ quietStart: Number(e.target.value) })
                    }
                  />
                  <span>to</span>
                  <input
                    aria-label="Quiet hours end"
                    type="number"
                    min="0"
                    max="23"
                    value={data.settings.quietEnd}
                    onChange={(e) =>
                      void settings({ quietEnd: Number(e.target.value) })
                    }
                  />
                </label>
              </section>
              <section className="card settings-card">
                <h2>Privacy</h2>
                <p>
                  Usage observations are stored on this Mac for 90 days.
                  Provider credentials and conversation content are excluded.
                </p>
                <Toggle
                  label="Share usage with Maxxit"
                  detail="Needed for server-side reset reminders."
                  checked={data.settings.cloudSync}
                  onChange={(cloudSync) => void settings({ cloudSync })}
                  disabled={!data.cloud.connected}
                />
                <Toggle
                  label="Share project descriptions for ideas"
                  detail="Only the descriptions you write here are sent."
                  checked={data.settings.aiConsent}
                  onChange={(aiConsent) => void settings({ aiConsent })}
                  disabled={!data.cloud.connected}
                />
              </section>
            </div>
          )}
          <footer>
            <span>maxxit.</span>
            <span className="mono small">v{version}</span>
          </footer>
        </div>
      </main>
      {add && (
        <div className="modal-backdrop">
          <form
            className="modal card"
            onSubmit={(e) => {
              e.preventDefault();
              void action(async () => {
                if (native) await invoke("save_project", { name, description });
                else
                  setData({
                    ...data,
                    projects: [
                      {
                        id: crypto.randomUUID(),
                        name,
                        description,
                        createdAt: new Date().toISOString(),
                      },
                      ...data.projects,
                    ],
                  });
                setAdd(false);
                setName("");
                setDescription("");
              });
            }}
          >
            <div className="card-top">
              <h2>Add a project</h2>
              <button
                type="button"
                className="icon-button"
                aria-label="Close"
                onClick={() => setAdd(false)}
              >
                <X size={18} />
              </button>
            </div>
            <label>
              Project name
              <input
                required
                maxLength={120}
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="Project name"
              />
            </label>
            <label>
              What do you want to build?
              <textarea
                maxLength={10000}
                rows={5}
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                placeholder="Describe the goal and what still needs work."
              />
            </label>
            <p className="small muted">
              Saved locally. You control whether this description is shared.
            </p>
            <button className="button" disabled={busy}>
              Save project <Plus size={15} />
            </button>
          </form>
        </div>
      )}
      {preview && (
        <div className="modal-backdrop">
          <div className="modal card">
            <h2>Connect Claude Code</h2>
            <p>{preview.description}</p>
            <label>
              Settings file
              <code className="code-block">{preview.settingsPath}</code>
            </label>
            <label>
              Status-line command
              <code className="code-block">{preview.command}</code>
            </label>
            <p className="small muted">
              Disconnecting restores the previous command if it has not been
              changed since.
            </p>
            <div className="modal-actions">
              <button
                className="button secondary"
                onClick={() => setPreview(null)}
              >
                Cancel
              </button>
              <button
                className="button"
                disabled={busy}
                onClick={() =>
                  void action(async () => {
                    await invoke("connect_provider", { provider: "claude" });
                    setPreview(null);
                  })
                }
              >
                Apply and connect
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
