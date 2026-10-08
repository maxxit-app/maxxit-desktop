import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Copy, Download, Trash2 } from "lucide-react";
import type { LocalRun, Settings, Snapshot } from "./types";

export function LocalIdeas({
  data,
  native,
  busy,
  action,
  settings,
}: {
  data: Snapshot;
  native: boolean;
  busy: boolean;
  action: (fn: () => Promise<void>) => Promise<void>;
  settings: (patch: Partial<Settings>) => Promise<void>;
}) {
  const [project, setProject] = useState("");
  const [selected, setSelected] = useState("");
  const [raw, setRaw] = useState("");
  const [copied, setCopied] = useState(false);
  const run = data.localRuns.find((r) => r.id === selected);
  const command = (fn: () => Promise<void>) => void action(fn);
  return (
    <div className="local-workflow">
      <section className="card settings-card">
        <h2>Use your own Codex or Claude</h2>
        <p>
          Review a project description, copy its analysis prompt into your own
          agent, then import the JSON result here. Select a suggestion when you
          are ready to start a task in that agent.
        </p>
        <p className="small muted">
          Projects, prompts, suggestions, and reports stay in the encrypted
          database on this Mac for 90 days. Your chosen provider receives the
          text you paste and may process it in the cloud. Maxxit does not sign
          in to your provider or start work for you. Use a fresh analysis
          session with tools disabled. Your agent configuration may load extra
          context.
        </p>
        <label className="local-consent">
          <input
            type="checkbox"
            checked={data.settings.localAiConsent}
            onChange={(e) =>
              void settings({ localAiConsent: e.target.checked })
            }
          />{" "}
          I understand that copying a prompt shares its contents with my chosen
          agent.
        </label>
        <div className="connection-actions">
          <label>
            Project
            <select
              value={project}
              onChange={(e) => setProject(e.target.value)}
            >
              <option value="">Choose a project</option>
              {data.projects.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
          </label>
          <button
            className="button"
            disabled={
              busy || !project || !data.settings.localAiConsent || !native
            }
            onClick={() =>
              command(async () => {
                const created = await invoke<LocalRun>("local_analysis", {
                  projectId: project,
                });
                setSelected(created.id);
                setRaw("");
                setCopied(false);
              })
            }
          >
            Prepare analysis prompt
          </button>
        </div>
        {!native && (
          <p className="small muted">
            Open the desktop app to store and import local results.
          </p>
        )}
        {!data.projects.length && (
          <p className="small muted">
            Add a description in Projects first. Include only the context you
            want to share with your agent.
          </p>
        )}
      </section>
      {data.localRuns.length > 0 && (
        <section className="card settings-card">
          <h2>Local runs</h2>
          <label>
            Run
            <select
              value={selected}
              onChange={(e) => {
                setSelected(e.target.value);
                setRaw("");
                setCopied(false);
              }}
            >
              <option value="">Select a run</option>
              {data.localRuns.map((r) => (
                <option key={r.id} value={r.id}>
                  {r.projectName} · {r.kind} · {r.state.replaceAll("_", " ")} ·{" "}
                  {new Date(r.createdAt).toLocaleString()}
                </option>
              ))}
            </select>
          </label>
          {run && (
            <>
              <div className="card-top">
                <div>
                  <h3>{run.projectName}</h3>
                  <p className="small muted">
                    {run.kind === "analysis"
                      ? "Analysis only. No execution is authorized by this prompt."
                      : "You selected this task. Review the prompt and start it in your own agent."}
                  </p>
                </div>
                <button
                  className="icon-button"
                  aria-label="Delete this run and its task reports"
                  disabled={busy}
                  onClick={() =>
                    command(async () => {
                      await invoke("local_remove", { id: run.id });
                      setSelected("");
                    })
                  }
                >
                  <Trash2 size={16} />
                </button>
              </div>
              <details
                className="local-prompt"
                open={run.state === "awaiting_result"}
              >
                <summary>Review prompt before sharing</summary>
                <pre>{run.bundle}</pre>
              </details>
              <div className="connection-actions">
                <button
                  className="button"
                  disabled={busy}
                  onClick={() =>
                    command(async () => {
                      await navigator.clipboard.writeText(run.bundle);
                      setCopied(true);
                    })
                  }
                >
                  <Copy size={15} />
                  {copied ? "Copied" : "Copy reviewed prompt"}
                </button>
                <button
                  className="button secondary"
                  disabled={busy}
                  onClick={() =>
                    command(async () => {
                      await invoke("local_export", {
                        id: run.id,
                        result: false,
                      });
                    })
                  }
                >
                  <Download size={15} />
                  Save prompt
                </button>
              </div>
              {run.state === "awaiting_result" && (
                <form
                  onSubmit={(e) => {
                    e.preventDefault();
                    command(async () => {
                      await invoke("local_import", { id: run.id, raw });
                      setRaw("");
                    });
                  }}
                >
                  <label>
                    Paste the agent's JSON result
                    <textarea
                      rows={8}
                      maxLength={100000}
                      required
                      value={raw}
                      onChange={(e) => setRaw(e.target.value)}
                      placeholder="Paste JSON only, including this run's runId."
                    />
                  </label>
                  <button className="button" disabled={busy || !raw.trim()}>
                    Import result
                  </button>
                </form>
              )}
              {run.state !== "awaiting_result" && (
                <button
                  className="button secondary"
                  disabled={busy}
                  onClick={() =>
                    command(async () => {
                      await invoke("local_export", {
                        id: run.id,
                        result: true,
                      });
                    })
                  }
                >
                  Save local result
                </button>
              )}
              {run.delivery === "pending" && (
                <button
                  className="button secondary"
                  disabled={busy}
                  onClick={() =>
                    command(async () => {
                      await invoke("cloud_sync");
                    })
                  }
                >
                  Sync notifications
                </button>
              )}
              {run.summary && (
                <div className="local-report">
                  <h3>Agent's completion report</h3>
                  <p>{run.summary}</p>
                  <p className="small muted">
                    Reported by your agent. Maxxit has not independently
                    verified the changes or tests.
                  </p>
                </div>
              )}
              {run.suggestions.map((idea, index) => (
                <article className="local-suggestion" key={index}>
                  <h3>{idea.title}</h3>
                  <p>{idea.description}</p>
                  <details>
                    <summary>Review task instructions</summary>
                    <pre>{idea.prompt}</pre>
                  </details>
                  <button
                    className="button secondary"
                    disabled={busy || !data.settings.localAiConsent}
                    onClick={() =>
                      command(async () => {
                        const task = await invoke<LocalRun>("local_task", {
                          parentId: run.id,
                          index,
                        });
                        setSelected(task.id);
                        setRaw("");
                        setCopied(false);
                      })
                    }
                  >
                    Select task and prepare prompt
                  </button>
                </article>
              ))}
              <p className="small muted">
                Notification status:{" "}
                {run.delivery === "accepted"
                  ? "Accepted by server. Delivery follows your notification preferences."
                  : run.delivery}
                . Detailed results stay on this Mac unless you enable workflow
                sharing in Settings.
              </p>
            </>
          )}
        </section>
      )}
    </div>
  );
}
