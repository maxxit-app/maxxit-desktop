import {
  diagnosticContext,
  recordDiagnostic,
  captureDiagnostic,
  flushDiagnostics,
} from "./observability";
import { isTauri } from "@tauri-apps/api/core";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type UpdateState = {
  status:
    | "idle"
    | "checking"
    | "current"
    | "available"
    | "downloading"
    | "installing"
    | "ready"
    | "error";
  version?: string;
  notes?: string;
  progress?: number;
  error?: string;
};
type PendingUpdate = Pick<
  Update,
  "version" | "body" | "downloadAndInstall" | "close"
>;

export class UpdateController {
  private state: UpdateState = { status: "idle" };
  private pending: PendingUpdate | null = null;
  private busy = false;
  private listeners = new Set<() => void>();
  constructor(
    private checkUpdate: () => Promise<PendingUpdate | null> = () =>
      check({ timeout: 15000 }),
    private restartApp: () => Promise<void> = relaunch,
  ) {}
  snapshot = () => this.state;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };
  private set(state: UpdateState) {
    this.state = state;
    this.listeners.forEach((listener) => listener());
  }
  async check() {
    if (this.busy || this.state.status === "ready") return;
    this.busy = true;
    this.set({ ...this.state, status: "checking", error: undefined });
    const context = diagnosticContext("updates");
    recordDiagnostic("update.check.started", context, { stage: "check" });
    try {
      const update = await this.checkUpdate();
      await this.pending
        ?.close()
        .catch((error) =>
          captureDiagnostic(error, "update.cleanup.failed", context),
        );
      this.pending = update;
      if (update)
        recordDiagnostic("update.available", context, {
          target_version: update.version,
        });
      this.set(
        update
          ? { status: "available", version: update.version, notes: update.body }
          : { status: "current" },
      );
    } catch (error) {
      captureDiagnostic(error, "update.check.failed", context);
      this.set({
        ...this.state,
        status: "error",
        error:
          "Couldn't check for updates. Check your connection and try again.",
      });
    } finally {
      this.busy = false;
    }
  }
  async install() {
    if (this.busy || !this.pending || this.state.status === "ready") return;
    this.busy = true;
    this.set({
      ...this.state,
      status: "downloading",
      progress: undefined,
      error: undefined,
    });
    const context = diagnosticContext("updates");
    recordDiagnostic("update.download.started", context, {
      stage: "download",
      target_version: this.pending.version,
    });
    let stage = "download";
    let total = 0;
    let downloaded = 0;
    try {
      await this.pending.downloadAndInstall(
        (event) => {
          if (event.event === "Started") {
            total = event.data.contentLength ?? 0;
            downloaded = 0;
          }
          if (event.event === "Progress") downloaded += event.data.chunkLength;
          if (event.event === "Finished") {
            stage = "install";
            recordDiagnostic("update.download.completed", context, {
              stage: "install",
              bytes: downloaded,
            });
          }
          this.set({
            ...this.state,
            status: event.event === "Finished" ? "installing" : "downloading",
            progress:
              total > 0
                ? Math.min(100, Math.round((downloaded / total) * 100))
                : undefined,
          });
        },
        { timeout: 120000 },
      );
      this.set({ ...this.state, status: "ready", progress: 100 });
      recordDiagnostic("update.install.completed", context, {
        stage: "install",
        target_version: this.pending.version,
      });
      // A failure to release the IPC resource must not repeat a successful install.
      await this.pending
        .close()
        .catch((error) =>
          captureDiagnostic(error, "update.cleanup.failed", context),
        );
      this.pending = null;
    } catch (error) {
      captureDiagnostic(
        error,
        String(error).toLowerCase().includes("signature")
          ? "update.verify.failed"
          : stage === "download"
            ? "update.download.failed"
            : "update.install.failed",
        context,
      );
      this.set({
        ...this.state,
        status: "error",
        error:
          "The update couldn't be installed. Try again, or download the latest app from maxxit.app/download.",
      });
    } finally {
      this.busy = false;
    }
  }
  async restart() {
    if (this.busy || this.state.status !== "ready") return;
    this.busy = true;
    const context = diagnosticContext("updates");
    recordDiagnostic("update.relaunch.started", context, { stage: "relaunch" });
    try {
      await flushDiagnostics();
      await this.restartApp();
    } catch (error) {
      captureDiagnostic(error, "update.relaunch.failed", context);
      this.set({
        ...this.state,
        error:
          "Couldn't restart Maxxit. Quit and open the app to finish updating.",
      });
    } finally {
      this.busy = false;
    }
  }
}
export const updates = new UpdateController();
let users = 0;
let timer: ReturnType<typeof setInterval> | undefined;
let lastCheck = 0;
const checkPeriodically = () => {
  if (
    document.visibilityState !== "visible" ||
    Date.now() - lastCheck < 3600000
  )
    return;
  lastCheck = Date.now();
  void updates.check();
};
export function watchUpdates() {
  if (!isTauri()) return () => {};
  if (users++ === 0) {
    checkPeriodically();
    timer = setInterval(checkPeriodically, 60000);
    document.addEventListener("visibilitychange", checkPeriodically);
  }
  return () => {
    if (--users === 0) {
      clearInterval(timer);
      document.removeEventListener("visibilitychange", checkPeriodically);
    }
  };
}
