import { AccountGate } from "./AccountGate";
import { LocalIdeas } from "./LocalIdeas";
import { version } from "../package.json";
import { useCallback, useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
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
  RefreshCw,
  Search,
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
  type AccountState,
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
export function WindowCard({ provider }: { provider: Provider }) {
  const now = Date.now();
  const windows = provider.observation?.windows ?? [];
  const stale =
    provider.observation &&
    now - Date.parse(provider.observation.observedAt) >= 2 * 3600000;
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
          const value = remaining(w, now);
          return (
            <div className="allowance" key={`${w.bucketId}:${w.windowId}`}>
              <div className="allowance-label">
                <span>{w.label ?? w.windowId}</span>
                <span className="mono">
                  {value === null
                    ? "Unavailable"
                    : `${Math.round(value)}% ${stale ? "last read" : "left"}`}
                </span>
              </div>
              <div className="progress">
                <span style={{ width: `${value ?? 0}%` }} />
              </div>
              <div className="small muted">
                {resetLabel(w.resetsAt, now)}
                {value === null ? " · Usage unavailable" : ""}
                {value !== null && stale
                  ? " · Use your assistant for a fresh reading."
                  : ""}
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
  const [account, setAccount] = useState<AccountState>({ status: "checking" });
  const [tab, setTab] = useState("Overview");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [legacy, setLegacy] = useState<{
    accountId: string;
    projects: { id: string; name: string }[];
    observations: number;
    runs: number;
  } | null>(null);
  const [editing, setEditing] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [projectSearch, setProjectSearch] = useState("");
  const projectQuery = projectSearch.trim().toLocaleLowerCase();
  const visibleProjects = data.projects.filter((project) =>
    `${project.name} ${project.description}`
      .toLocaleLowerCase()
      .includes(projectQuery),
  );
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
  const [shareResults, setShareResults] = useState(false);
  const refresh = useCallback(async () => {
    if (native) {
      try {
        const identity = await invoke<AccountState>("account_status");
        setAccount(identity);
        if (["authenticated", "offline"].includes(identity.status))
          setData(await invoke<Snapshot>("snapshot"));
        else setData(empty);
      } catch (e) {
        setAccount({ status: "error", error: String(e) });
        setData(empty);
        setMessage(String(e));
      }
    }
  }, []);
  useEffect(() => {
    void refresh();
    if (native)
      void isEnabled()
        .then(setAutostart)
        .catch(() => {});
    let cleanup: (() => void) | undefined;
    if (native)
      void listen("usage-updated", () => void refresh()).then((f) => {
        cleanup = f;
      });
    const timer = setInterval(() => void refresh(), 5000);
    const focus = () => void refresh();
    window.addEventListener("focus", focus);
    window.addEventListener("online", focus);
    return () => {
      clearInterval(timer);
      window.removeEventListener("focus", focus);
      window.removeEventListener("online", focus);
      cleanup?.();
    };
  }, [refresh]);
  useEffect(() => {
    if (!native) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen<string>("navigate-to", (event) => setTab(event.payload)).then(
      (fn) => {
        if (disposed) fn();
        else unlisten = fn;
      },
    );
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);
  useEffect(() => {
    document.documentElement.dataset.theme = data.settings.theme;
  }, [data.settings.theme]);
  async function action(callback: () => Promise<void>) {
    setBusy(true);
    setMessage("");
    try {
      await callback();
    } catch (e) {
      setMessage(String(e));
    } finally {
      await refresh();
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
        if (data.cloud.connected) await invoke("cloud_sync");
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
      "Projects discovered automatically from Codex and Claude Code.",
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
  if (!demo && !["authenticated", "offline"].includes(account.status))
    return <AccountGate account={account} refresh={refresh} />;
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
              {label === "Ideas" &&
                data.settings.generationMode === "hosted" && (
                  <span className="pro-pill">PRO</span>
                )}
            </button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <div className="account">
            <div className="avatar">M</div>
            <div>
              <strong>
                {data.cloud.data?.account?.username ||
                  data.cloud.data?.account?.email ||
                  "This Mac"}
              </strong>
              <span>
                {demo
                  ? "Demo workspace"
                  : `${data.cloud.plan === "pro" ? "Pro" : data.cloud.plan === "free" ? "Free" : "Unverified"} plan${account.status === "offline" ? " · Offline" : ""}`}
              </span>
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
                  <p>
                    Choose a discovered project, then prepare a reviewed prompt
                    in Ideas. Hosted suggestions require Pro.
                  </p>
                </div>
                <button
                  className="button secondary"
                  onClick={() => setTab("Projects")}
                >
                  View projects <ArrowUpRight size={15} />
                </button>
              </div>
              {(data.resetEvents?.length ?? 0) > 0 && (
                <section className="card">
                  <h2>Recent reset alerts</h2>
                  <p className="small muted">
                    These readings show reported changes. Claude account
                    identity is unavailable in the statusline, so reconnect
                    after switching accounts. Other limits may still apply.
                  </p>
                  {data.resetEvents?.slice(0, 5).map((event) => (
                    <article key={event.id}>
                      <h3>{event.title}</h3>
                      <p>{event.body}</p>
                      {event.before && (
                        <p className="small muted">
                          Previous reading{" "}
                          {new Date(event.before.observedAt).toLocaleString(
                            undefined,
                            { timeZone: data.settings.timezone },
                          )}
                          .{" "}
                          {event.before.window.usedPercent !== null &&
                            `${Math.round(100 - event.before.window.usedPercent)}% remaining.`}
                        </p>
                      )}
                      <p className="small muted">
                        Desktop status: {event.nativeState ?? "pending"}
                      </p>
                      <p className="small muted">
                        Observed{" "}
                        {new Date(event.observedAt).toLocaleString(undefined, {
                          timeZone: data.settings.timezone,
                        })}
                        .{" "}
                        {event.effectiveAt &&
                          `Reported reset ${new Date(event.effectiveAt).toLocaleString(undefined, { timeZone: data.settings.timezone })}.`}{" "}
                        {data.settings.timezone}
                      </p>
                    </article>
                  ))}
                  <p>
                    Useful next steps: review a recent change for reproducible
                    bugs, or add a regression test for one fixed bug. Allow
                    about 20 to 30 minutes and review the result.
                  </p>
                  <button
                    className="button secondary"
                    onClick={() => setTab("Ideas")}
                  >
                    Review project ideas
                  </button>
                </section>
              )}
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
              <div className="section-heading projects-toolbar">
                <span role="status" aria-live="polite">
                  {projectQuery ? `${visibleProjects.length} of ` : ""}
                  {data.projects.length}{" "}
                  {data.projects.length === 1 ? "project" : "projects"}
                </span>
                <div className="project-search">
                  <Search size={17} aria-hidden="true" />
                  <input
                    type="search"
                    aria-label="Search projects"
                    placeholder="Search projects…"
                    value={projectSearch}
                    onChange={(event) => setProjectSearch(event.target.value)}
                  />
                  {projectSearch && (
                    <button
                      className="icon-button"
                      aria-label="Clear project search"
                      onClick={() => setProjectSearch("")}
                    >
                      <X size={15} />
                    </button>
                  )}
                </div>
              </div>
              {!!data.projects.length && (
                <div
                  className="projects-table-scroll"
                  role="region"
                  aria-label="Projects"
                  tabIndex={0}
                >
                  <table className="projects-table">
                    <caption className="sr-only">Discovered projects</caption>
                    <thead>
                      <tr>
                        <th scope="col">Project</th>
                        <th scope="col">Description</th>
                        <th scope="col">Status</th>
                        <th scope="col" className="project-actions-heading">
                          Actions
                        </th>
                      </tr>
                    </thead>
                    <tbody>
                      {visibleProjects.map((p) => (
                        <tr key={p.id}>
                          <th scope="row">
                            <span className="project-name">{p.name}</span>
                          </th>
                          <td>
                            <span
                              className="project-description"
                              title={p.description || undefined}
                            >
                              {p.description || "No description yet."}
                            </span>
                          </td>
                          <td>
                            <span
                              className={`project-status ${p.archived ? "archived" : ""}`}
                            >
                              {p.archived ? "Archived" : "Active"}
                            </span>
                          </td>
                          <td>
                            <div className="project-row-actions">
                              <button
                                className="button secondary"
                                aria-label={`Edit ${p.name}`}
                                disabled={busy}
                                onClick={() => {
                                  setEditing(p.id);
                                  setName(p.name);
                                  setDescription(p.description);
                                }}
                              >
                                Edit
                              </button>
                              <button
                                className="button secondary"
                                aria-label={`${p.archived ? "Restore" : "Archive"} ${p.name}`}
                                disabled={busy}
                                onClick={() =>
                                  void action(async () => {
                                    if (native)
                                      await invoke("save_project", {
                                        ...p,
                                        archived: !p.archived,
                                      });
                                    else
                                      setData({
                                        ...data,
                                        projects: data.projects.map((x) =>
                                          x.id === p.id
                                            ? { ...x, archived: !x.archived }
                                            : x,
                                        ),
                                      });
                                  })
                                }
                              >
                                {p.archived ? "Restore" : "Archive"}
                              </button>
                              <button
                                className="icon-button"
                                aria-label={`Remove ${p.name}`}
                                disabled={busy}
                                onClick={() =>
                                  void action(async () => {
                                    if (native)
                                      await invoke("remove_project", {
                                        id: p.id,
                                      });
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
                          </td>
                        </tr>
                      ))}
                      {!visibleProjects.length && (
                        <tr>
                          <td colSpan={4} className="project-no-results">
                            <strong>No matching projects</strong>
                            <p>Try another name or description.</p>
                            <button
                              className="button secondary"
                              onClick={() => setProjectSearch("")}
                            >
                              Clear search
                            </button>
                          </td>
                        </tr>
                      )}
                    </tbody>
                  </table>
                </div>
              )}
              {!data.projects.length && (
                <div className="card empty-state">
                  <Folder size={30} />
                  <h3>No projects yet</h3>
                  <p>
                    Projects appear automatically after you use Codex or Claude
                    Code on this Mac.
                  </p>
                  <button
                    className="button"
                    onClick={() => void refresh()}
                    disabled={busy}
                  >
                    Refresh <RefreshCw size={15} />
                  </button>
                </div>
              )}
              <p className="small muted">
                Project descriptions stay local unless you choose to share them
                for ideas.
              </p>
            </>
          )}
          {tab === "Ideas" && data.settings.generationMode === "local" && (
            <LocalIdeas
              data={data}
              native={native}
              busy={busy}
              action={action}
              settings={settings}
            />
          )}
          {tab === "Ideas" &&
            data.settings.generationMode === "hosted" &&
            data.cloud.plan === "pro" && (
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
          {tab === "Ideas" &&
            data.settings.generationMode === "hosted" &&
            data.cloud.plan !== "pro" && (
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
                  disabled={
                    busy ||
                    account.status !== "authenticated" ||
                    !data.cloud.data?.billing?.billingReady
                  }
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
                <h2>Web sync</h2>
                <p>
                  Your desktop owns these records. Web views update
                  automatically after accepted uploads.
                </p>
                <Toggle
                  label="Sync allowance history and analytics"
                  checked={data.settings.cloudSync}
                  disabled={!data.cloud.data?.identity?.consent.usage}
                  onChange={(cloudSync) => void settings({ cloudSync })}
                />
                <Toggle
                  label="Sync projects"
                  detail="Share descriptions even in local idea mode."
                  checked={data.settings.projectSync}
                  disabled={!data.cloud.data?.identity?.consent.metadata}
                  onChange={(projectSync) => void settings({ projectSync })}
                />
                <Toggle
                  label="Sync shared preferences"
                  checked={data.settings.accountSync}
                  disabled={!data.cloud.data?.identity?.consent.accountSync}
                  onChange={(accountSync) => void settings({ accountSync })}
                />
                <Toggle
                  label="Sync ideas and task reports"
                  detail="Includes reviewed prompts and imported reports. Hosted AI stays separate."
                  checked={data.settings.workflowSync}
                  disabled={!data.cloud.data?.identity?.consent.workflowDetails}
                  onChange={(workflowSync) => void settings({ workflowSync })}
                />
                <p className="small muted">
                  To approve more categories, sign out and sign in again with
                  the same account. Your local data stays with that account.
                </p>
                {data.sync?.error && <p role="alert">{data.sync.error}</p>}
              </article>
              <article className="card connection-card">
                <h2>Maxxit account</h2>
                {data.legacyDataAvailable && (
                  <div className="notice">
                    <p>
                      Earlier local data is preserved. Review it before adding
                      it to this account.
                    </p>
                    <button
                      className="button secondary"
                      disabled={busy}
                      onClick={() =>
                        void action(async () => {
                          setLegacy(await invoke("legacy_preview"));
                        })
                      }
                    >
                      Review earlier data
                    </button>
                    {legacy && (
                      <>
                        <p>
                          {legacy.projects.length} projects,{" "}
                          {legacy.observations} observations, and {legacy.runs}{" "}
                          runs will belong to{" "}
                          {data.cloud.data?.account?.email || legacy.accountId}.
                          Sharing follows your current settings.
                        </p>
                        <ul>
                          {legacy.projects.map((p) => (
                            <li key={p.id}>{p.name}</li>
                          ))}
                        </ul>
                        <button
                          className="button"
                          disabled={busy}
                          onClick={() =>
                            void action(async () => {
                              await invoke("legacy_import", {
                                accountId: legacy.accountId,
                              });
                              setLegacy(null);
                            })
                          }
                        >
                          Import into this account
                        </button>
                      </>
                    )}
                  </div>
                )}
                <p>
                  Your account owns this Mac’s synced data, Pro subscription,
                  and email reminders. Choose sharing independently of hosted
                  AI.
                </p>
                {data.cloud.error && (
                  <div className="notice error">{data.cloud.error}</div>
                )}
                {data.cloud.connected ? (
                  <div className="connection-actions">
                    <span className="tag">{data.cloud.plan.toUpperCase()}</span>
                    <span className="small muted">
                      {data.sync?.pending
                        ? `${data.sync.pending} changes waiting to sync`
                        : data.sync?.lastSyncAt
                          ? `Synced ${new Date(data.sync.lastSyncAt).toLocaleTimeString()}`
                          : "Waiting for first sync"}
                    </span>
                    <button
                      className="button secondary"
                      onClick={() =>
                        void action(async () => {
                          setAccount({ status: "signed_out" });
                          setData(empty);
                          await invoke("cloud_disconnect");
                        })
                      }
                    >
                      Sign out
                    </button>
                    <button
                      className="button"
                      disabled={
                        busy ||
                        account.status !== "authenticated" ||
                        !data.cloud.data?.billing?.billingReady
                      }
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
                      detail="Share descriptions for hosted ideas. Only used in hosted mode."
                      checked={shareProjects}
                      onChange={setShareProjects}
                    />
                    <Toggle
                      label="Allow completion notifications"
                      detail="Share run IDs and completion status only. Server delivery requires Pro."
                      checked={shareResults}
                      onChange={setShareResults}
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
                                aiConsent:
                                  data.settings.generationMode === "hosted" &&
                                  shareProjects,
                                resultEvents: shareResults,
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
                              completionEvents: shareResults,
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
                    })
                  }
                />
              </section>
              <section className="card settings-card reset-alert-settings">
                <h2>Reset alerts</h2>
                <p className="small muted">
                  Desktop alerts use fresh local readings. Email requires usage
                  sharing and a connected Maxxit Pro account. macOS controls
                  delivery in System Settings → Notifications.
                </p>
                <div className="reset-options">
                  <Toggle
                    label="Email reset alerts · Pro"
                    detail="Verified email for the reset events you choose. Observed changes require usage sharing."
                    checked={data.settings.email}
                    onChange={(email) => void settings({ email })}
                    disabled={
                      !data.cloud.connected || data.cloud.plan !== "pro"
                    }
                  />
                  <Toggle
                    label="Desktop reset alerts"
                    detail="Show a generic notice on the primary Mac. Open Maxxit for evidence and ideas."
                    checked={data.settings.resetNotifications}
                    onChange={(resetNotifications) =>
                      void settings({ resetNotifications })
                    }
                  />
                  {(
                    [
                      ["scheduled", "Before a scheduled reset"],
                      ["changed", "When a reset time changes"],
                      ["increased", "When fresh readings show more allowance"],
                      ["announcements", "Reviewed official announcements"],
                      ["offers", "Offers I confirm in Maxxit"],
                    ] as const
                  ).map(([key, label]) => (
                    <Toggle
                      key={key}
                      label={label}
                      detail={
                        key === "announcements" || key === "offers"
                          ? "Managed in the web workspace."
                          : "An observed change does not confirm its cause."
                      }
                      checked={data.settings.resetAlerts[key]}
                      onChange={(value) =>
                        void settings({
                          resetAlerts: {
                            ...data.settings.resetAlerts,
                            [key]: value,
                          },
                        })
                      }
                    />
                  ))}
                  {(["codex", "claude"] as const).map((provider) => (
                    <Toggle
                      key={provider}
                      label={`${provider === "codex" ? "Codex" : "Claude"} alerts`}
                      detail="Choose which provider to watch."
                      checked={data.settings.resetAlerts.providers.includes(
                        provider,
                      )}
                      onChange={(value) =>
                        void settings({
                          resetAlerts: {
                            ...data.settings.resetAlerts,
                            providers: value
                              ? [
                                  ...data.settings.resetAlerts.providers,
                                  provider,
                                ]
                              : data.settings.resetAlerts.providers.filter(
                                  (p) => p !== provider,
                                ),
                          },
                        })
                      }
                    />
                  ))}
                </div>
                <label className="field-row">
                  <span>Timezone</span>
                  <input
                    aria-label="Notification timezone"
                    defaultValue={data.settings.timezone}
                    onBlur={(e) => void settings({ timezone: e.target.value })}
                  />
                </label>
                <label className="field-row">
                  <span>Daily alert limit</span>
                  <select
                    value={data.settings.dailyLimit}
                    onChange={(e) =>
                      void settings({ dailyLimit: Number(e.target.value) })
                    }
                  >
                    {[1, 2, 3, 4, 5].map((n) => (
                      <option key={n} value={n}>
                        {n}
                      </option>
                    ))}
                  </select>
                </label>
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
                    min="1"
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
                  Usage and local agent results are stored in an encrypted
                  database on this Mac for 90 days. The database key is held in
                  Mac Keychain.
                </p>
                <label>
                  Suggestion provider
                  <select
                    value={data.settings.generationMode}
                    onChange={(e) =>
                      void settings({
                        generationMode: e.target
                          .value as Settings["generationMode"],
                      })
                    }
                  >
                    <option value="local">My own Codex or Claude</option>
                    <option value="hosted">Maxxit hosted AI</option>
                  </select>
                </label>
                <p className="small muted">
                  Changing suggestion provider updates hosted generation for
                  your Maxxit account after a successful sync. Local mode stops
                  pending hosted work; existing cloud copies remain until you
                  delete them.
                </p>
                <Toggle
                  label="Local result notifications"
                  detail="Show a generic macOS notification after importing results."
                  checked={data.settings.localNotifications}
                  onChange={(localNotifications) =>
                    void settings({ localNotifications })
                  }
                />
                <Toggle
                  label="Send completion status to Maxxit"
                  detail="Requires browser-approved completion scope and Pro. No project text or report content is sent. Reconnect to approve this scope if needed."
                  checked={data.settings.resultEvents}
                  onChange={(resultEvents) => void settings({ resultEvents })}
                  disabled={!data.cloud.connected || data.cloud.plan !== "pro"}
                />
                <Toggle
                  label="Share usage with Maxxit"
                  detail="Needed for server-side reset reminders."
                  checked={data.settings.cloudSync}
                  onChange={(cloudSync) => void settings({ cloudSync })}
                  disabled={!data.cloud.connected}
                />
                <Toggle
                  label="Share descriptions for hosted ideas"
                  detail="Only the descriptions you write here are sent."
                  checked={data.settings.aiConsent}
                  onChange={(aiConsent) => void settings({ aiConsent })}
                  disabled={
                    !data.cloud.connected ||
                    data.settings.generationMode !== "hosted"
                  }
                />
                <button
                  className="button secondary"
                  disabled={!native || !data.cloud.connected || busy}
                  onClick={() =>
                    void action(async () => {
                      await invoke("remove_cloud_copies");
                    })
                  }
                >
                  Delete this device's cloud project copies and all hosted ideas
                </button>
                <p className="small muted">
                  Deleting cloud copies removes this device's uploaded projects
                  and all hosted suggestions for this account, including
                  suggestions derived from other projects.
                </p>
              </section>
            </div>
          )}
          <footer>
            <span>maxxit.</span>
            <span className="mono small">v{version}</span>
          </footer>
        </div>
      </main>
      {editing && (
        <div className="modal-backdrop">
          <form
            className="modal card"
            onSubmit={(e) => {
              e.preventDefault();
              void action(async () => {
                if (native)
                  await invoke("save_project", {
                    description,
                    id: editing,
                  });
                else
                  setData({
                    ...data,
                    projects: data.projects.map((p) =>
                      p.id === editing ? { ...p, description } : p,
                    ),
                  });
                setEditing(null);
                setName("");
                setDescription("");
              });
            }}
          >
            <div className="card-top">
              <h2>Edit project description</h2>
              <button
                type="button"
                className="icon-button"
                aria-label="Close"
                onClick={() => setEditing(null)}
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
                readOnly
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
              Save description <Check size={15} />
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
