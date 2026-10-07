import * as Sentry from "@sentry/react";
import { invoke as nativeInvoke, isTauri } from "@tauri-apps/api/core";
import { version } from "../package.json";

export type DiagnosticHealth = {
  enabled: boolean;
  nativeCrashes: boolean;
  configured: boolean;
  pending: number;
  oldestAgeSeconds: number;
  dropped: number;
  lastAcceptedAt?: number;
  lastEventId?: string;
  deliveryState: string;
  generation: number;
};
type Context = { id: string; command: string; generation: number };
type Input = {
  name: string;
  operationId?: string;
  generation: number;
  attributes?: Record<string, unknown>;
  event?: Record<string, unknown>;
};
const browser = typeof window !== "undefined";
const native = browser && isTauri();
let health: DiagnosticHealth | undefined;
let initialized = false;
const pending: Input[] = [];
let sending = false;
const freshId = () => crypto.randomUUID().replaceAll("-", "");
export const diagnosticOrigin = () =>
  browser && new URLSearchParams(location.search).has("tray") ? "tray" : "main";

export function setDiagnosticHealth(next: DiagnosticHealth) {
  if (health?.generation !== next.generation || !next.enabled)
    pending.length = 0;
  health = next;
}
export function diagnosticContext(command: string): Context {
  return { id: freshId(), command, generation: health?.generation ?? -1 };
}
export function safeAttributes(input: Record<string, unknown>) {
  const result: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(input)) {
    if (
      [
        "elapsed_ms",
        "status",
        "attempt",
        "bytes",
        "rejected_count",
        "rows",
      ].includes(key) &&
      typeof value === "number" &&
      Number.isSafeInteger(value) &&
      value >= 0 &&
      value <= 1e12
    )
      result[key] = value;
    if (
      key === "stage" &&
      typeof value === "string" &&
      [
        "start",
        "check",
        "download",
        "install",
        "verify",
        "cleanup",
        "relaunch",
        "listener",
        "read",
        "write",
      ].includes(value)
    )
      result[key] = value;
    if (key === "provider" && (value === "codex" || value === "claude"))
      result[key] = value;
    if (
      key === "target_version" &&
      typeof value === "string" &&
      /^\d+\.\d+\.\d+$/.test(value)
    )
      result[key] = value;
  }
  return result;
}
const assetName = (value?: string) => {
  const filename = value?.split(/[\\/]/).at(-1)?.split(/[?#]/)[0];
  return filename ? `app:///assets/${filename}` : undefined;
};
// Keep only SDK-generated stack positions. Original exception strings and component props never cross IPC.
export function safeBrowserEvent(event: Sentry.ErrorEvent) {
  const values = event.exception?.values?.slice(-8).map((exception) => ({
    type: ["TypeError", "SyntaxError", "RangeError", "ReferenceError"].includes(
      exception.type ?? "",
    )
      ? exception.type
      : "DesktopError",
    value: "Unexpected desktop failure",
    stacktrace: {
      frames: exception.stacktrace?.frames?.slice(-80).map((frame) => ({
        filename: assetName(frame.filename),
        function: frame.function,
        lineno: frame.lineno,
        colno: frame.colno,
        in_app: frame.in_app,
      })),
    },
  }));
  return {
    event_id: event.event_id ?? freshId(),
    timestamp: event.timestamp,
    platform: "javascript",
    exception: { values },
    debug_meta: {
      images: event.debug_meta?.images
        ?.filter((image) => image.type === "sourcemap")
        .slice(0, 80)
        .map((image) => ({
          type: "sourcemap",
          debug_id: image.debug_id,
          code_file: assetName(image.code_file),
        })),
    },
  };
}
async function drain() {
  if (sending || !native || !health?.enabled) return;
  sending = true;
  try {
    while (pending.length && health?.enabled) {
      const item = pending[0];
      if (item.generation !== health.generation) {
        pending.shift();
        continue;
      }
      try {
        await nativeInvoke("diagnostics_record", { input: item });
        pending.shift();
      } catch {
        break;
      } // Preserve it until the bridge recovers; never recursively report the transport failure.
    }
  } finally {
    sending = false;
  }
}
function enqueue(input: Input) {
  if (!native || !health?.enabled || input.generation !== health.generation)
    return;
  if (pending.length >= 100) pending.shift();
  pending.push(input);
  void drain();
}
export function recordDiagnostic(
  name: string,
  context: Context,
  attributes: Record<string, unknown> = {},
) {
  enqueue({
    name,
    operationId: context.id,
    generation: context.generation,
    attributes: { ...safeAttributes(attributes), command: context.command },
  });
}
export class DesktopCommandError extends Error {
  constructor(
    message: string,
    public reportId?: string,
    public expected = false,
  ) {
    super(message);
  }
  override toString() {
    return this.reportId
      ? `${super.toString()} Report ${this.reportId}.`
      : super.toString();
  }
}
export function captureDiagnostic(
  error: unknown,
  name = "ui.command.failed",
  context = diagnosticContext("frontend"),
) {
  if (
    error instanceof DesktopCommandError &&
    (error.reportId || error.expected)
  )
    return;
  if (!health?.enabled || context.generation !== health.generation) return;
  Sentry.withScope((scope) => {
    scope.setExtra("maxxit_operation", context);
    scope.setTag("maxxit_event", name);
    Sentry.captureException(error);
  });
}
export async function invoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const context = diagnosticContext(command);
  recordDiagnostic("ui.command.started", context);
  try {
    const result = await nativeInvoke<T>(command, {
      ...args,
      operationId: context.id,
    });
    recordDiagnostic("ui.command.completed", context);
    return result;
  } catch (cause) {
    if (cause && typeof cause === "object" && "message" in cause) {
      const value = cause as {
        message: unknown;
        reportId?: string;
        expected?: boolean;
      };
      const error = new DesktopCommandError(
        String(value.message),
        value.reportId,
        value.expected,
      );
      captureDiagnostic(error, "ui.command.failed", context);
      throw error;
    }
    captureDiagnostic(cause, "ui.command.failed", context);
    throw cause;
  }
}
export async function initializeDiagnostics() {
  if (!native || initialized) return;
  initialized = true;
  try {
    setDiagnosticHealth(
      await nativeInvoke<DiagnosticHealth>("diagnostics_health"),
    );
  } catch {
    initialized = false;
    window.setTimeout(() => void initializeDiagnostics(), 5000);
    return;
  }
  Sentry.init({
    // Rust owns network transport. This SDK processes stacks only and never sends to this placeholder.
    dsn: "https://disabled@localhost/1",
    release: `maxxit-desktop@${version}`,
    sendDefaultPii: false,
    defaultIntegrations: false,
    integrations: [Sentry.globalHandlersIntegration()],
    sampleRate: 1,
    tracesSampleRate: 0,
    beforeSend(event, hint) {
      if (
        hint.originalException instanceof DesktopCommandError &&
        (hint.originalException.reportId || hint.originalException.expected)
      )
        return null;
      const context = event.extra?.maxxit_operation as Context | undefined;
      const name =
        event.tags?.maxxit_event === "ui.render.failed"
          ? "ui.render.failed"
          : String(
              event.tags?.maxxit_event ??
                (event.exception?.values?.some(
                  (exception) =>
                    exception.mechanism?.type === "onunhandledrejection",
                )
                  ? "ui.unhandled_rejection"
                  : "ui.unhandled_exception"),
            );
      const input: Input = {
        name,
        operationId: context?.id ?? freshId(),
        generation: context?.generation ?? health?.generation ?? -1,
        event: safeBrowserEvent(event),
        attributes: { command: context?.command ?? "frontend" },
      };
      enqueue(input);
      return null;
    },
  });
  window.setInterval(() => {
    void nativeInvoke<DiagnosticHealth>("diagnostics_health")
      .then(setDiagnosticHealth)
      .then(drain)
      .catch(() => {});
  }, 5000);
}
export async function flushDiagnostics() {
  await drain();
  if (native) await nativeInvoke("diagnostics_flush").catch(() => {});
}
export { Sentry };
