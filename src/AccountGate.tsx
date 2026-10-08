import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ArrowUpRight, LoaderCircle, ShieldCheck } from "lucide-react";
import type { AccountState } from "./types";
import { UpdateNotice } from "./UpdateNotice";

type Pairing = { code: string; url: string; expiresAt: string };
export function AccountGate({
  account,
  refresh,
}: {
  account: AccountState;
  refresh: () => Promise<void>;
}) {
  const [pairing, setPairing] = useState<Pairing | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [sharing, setSharing] = useState({
    usage: false,
    metadata: false,
    accountSync: false,
    workflowDetails: false,
  });
  useEffect(() => {
    if (account.pairing) setPairing(account.pairing);
  }, [account.pairing]);
  useEffect(() => {
    if (!pairing) return;
    let disposed = false;
    let pending = false;
    const timer = setInterval(async () => {
      if (pending || disposed) return;
      if (Date.now() >= Date.parse(pairing.expiresAt)) {
        setError("This code expired. Start sign-in again.");
        setPairing(null);
        return;
      }
      pending = true;
      try {
        await invoke("cloud_redeem");
        if (!disposed) await refresh();
      } catch (e) {
        const message = String(e);
        if (!message.startsWith("HTTP 409:")) {
          if (!disposed) setError(message);
        }
      } finally {
        pending = false;
      }
    }, 3000);
    return () => {
      disposed = true;
      clearInterval(timer);
    };
  }, [pairing, refresh]);
  async function start() {
    setBusy(true);
    setError("");
    try {
      const result = await invoke<Pairing>("cloud_start", {
        ...sharing,
        completionEvents: false,
      });
      setPairing(result);
      await invoke("open_link", { url: result.url });
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <main className="account-gate">
      <div className="gate-drag" data-tauri-drag-region />
      <section className="gate-card">
        <div className="brand">
          <img src="/maxxit-mark.png" alt="" />
          <span>
            maxxit<span className="brand-period">.</span>
          </span>
        </div>
        <span className="eyebrow">YOUR MAXXIT ACCOUNT</span>
        <h1>
          {account.status === "checking"
            ? "Checking your account"
            : "Sign in to get started"}
        </h1>
        <p>
          Your account connects this Mac to your web workspace and keeps Pro
          attached to you. Local analytics remain free.
        </p>
        {account.status === "checking" ? (
          <p role="status">
            <LoaderCircle className="spin" size={18} /> Verifying your saved
            sign-in…
          </p>
        ) : (
          <>
            {!pairing && (
              <fieldset className="gate-sharing">
                <legend>Choose what appears on the web</legend>
                {(
                  [
                    ["usage", "Allowance history and token analytics"],
                    ["metadata", "Project names and descriptions"],
                    ["accountSync", "Shared app preferences"],
                    ["workflowDetails", "Ideas, prompts, and task reports"],
                  ] as const
                ).map(([key, label]) => (
                  <label key={key}>
                    <input
                      type="checkbox"
                      checked={sharing[key]}
                      onChange={(e) =>
                        setSharing({ ...sharing, [key]: e.target.checked })
                      }
                    />
                    {label}
                  </label>
                ))}
                <small>
                  These choices do not enable hosted AI or paid notifications.
                  You can sign in with all sharing off.
                </small>
              </fieldset>
            )}
            {pairing ? (
              <div className="gate-pairing">
                <p>Review this device in your browser.</p>
                <strong className="mono">{pairing.code}</strong>
                <p role="status">Waiting for approval…</p>
                <button
                  className="button secondary"
                  onClick={() => void invoke("open_link", { url: pairing.url })}
                >
                  Open browser again <ArrowUpRight size={16} />
                </button>
                <button
                  className="text-button"
                  onClick={() => {
                    void invoke("cloud_cancel");
                    setPairing(null);
                  }}
                >
                  Cancel
                </button>
              </div>
            ) : (
              <button
                className="button gate-submit"
                disabled={busy}
                onClick={() => void start()}
              >
                {busy ? "Opening sign-in…" : "Sign in or create an account"}
                <ArrowUpRight size={16} />
              </button>
            )}
            <button
              className="text-button"
              disabled={busy}
              onClick={() => void refresh()}
            >
              Retry saved sign-in
            </button>
          </>
        )}
        {(error || account.error) && (
          <p className="notice error" role="alert">
            {error || account.error}
          </p>
        )}
        <p className="gate-privacy">
          <ShieldCheck size={16} /> Provider sign-in stays in Codex and Claude
          Code. Maxxit never asks for their passwords.
        </p>
        <UpdateNotice />
      </section>
    </main>
  );
}
