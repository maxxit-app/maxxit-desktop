import { invoke as nativeInvoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { remaining, type Provider } from "./types";
const screens = [
  "Overview",
  "Analytics",
  "Projects",
  "Ideas",
  "Connections",
  "Settings",
  "Tray",
];
export const productActions = new Set([
  "connect_provider",
  "disconnect_provider",
  "save_project",
  "remove_project",
  "cloud_start",
  "cloud_redeem",
  "cloud_disconnect",
  "cloud_sync",
  "cloud_action",
  "cloud_preferences",
  "save_settings",
  "local_analysis",
  "local_task",
  "local_import",
  "local_remove",
  "local_export",
  "remove_cloud_copies",
  "export_token_csv",
  "tray_action",
  "legacy_import",
  "update_check",
  "update_install",
]);
type EventName =
  | "screen_view"
  | "app_open"
  | "user_engagement"
  | "desktop_action"
  | "action_failed";
type ActivationEvent = "activation";
type Input = {
  name: EventName | ActivationEvent;
  screen: string;
  engagementMs: number;
  action?: string;
};
let screen = "Overview";
let previous = Date.now();
let enabled = false;
let epoch = 0;
let sending = false;
let activationCandidate = false;
let activationQueued = false;
const queue: { input: Input; epoch: number }[] = [];
export function setProductAnalyticsConsent(value: boolean) {
  enabled = value;
  epoch++;
  queue.length = 0;
  previous = Date.now();
  activationQueued = false;
  if (value && activationCandidate) recordActivation();
}
export function hasUsableObservation(providers: Provider[], now = Date.now()) {
  return providers.some((provider) => {
    const observation = provider.observation;
    if (!provider.connected || !observation) return false;
    const age = now - Date.parse(observation.observedAt);
    return (
      age >= 0 &&
      age < 2 * 60 * 60 * 1000 &&
      observation.windows.some(
        (window) =>
          window.usedPercent !== null &&
          Number.isFinite(window.usedPercent) &&
          window.usedPercent >= 0 &&
          window.usedPercent <= 100 &&
          remaining(window, now) !== null,
      )
    );
  });
}
function recordActivation() {
  if (!enabled || activationQueued || !isTauri() || document.hidden) return;
  activationQueued = true;
  trackProductEvent("activation");
}
export function productActivation(providers: Provider[], authorized: boolean) {
  activationCandidate = authorized && hasUsableObservation(providers);
  if (activationCandidate) recordActivation();
}
export function productScreen(value: string) {
  if (!screens.includes(value) || screen === value) return;
  screen = value;
  trackProductEvent("screen_view");
}
async function drain() {
  if (sending) return;
  sending = true;
  try {
    while (enabled && queue.length) {
      const item = queue.shift()!;
      if (item.epoch !== epoch) continue;
      await nativeInvoke("product_analytics_record", {
        input: item.input,
      }).catch(() => {});
    }
  } finally {
    sending = false;
  }
}
export function trackProductEvent(
  name: EventName | ActivationEvent,
  action?: string,
) {
  if (
    !enabled ||
    !isTauri() ||
    document.hidden ||
    (action && !productActions.has(action))
  )
    return;
  const now = Date.now();
  const engagementMs = Math.max(0, Math.min(60000, now - previous));
  previous = now;
  if (queue.length >= 20) queue.shift();
  queue.push({
    epoch,
    input: { name, screen, engagementMs, ...(action ? { action } : {}) },
  });
  void drain();
}
export function initializeProductAnalytics() {
  if (!isTauri()) return;
  if (new URLSearchParams(location.search).has("tray")) screen = "Tray";
  void nativeInvoke<{ choice: boolean | null }>("product_analytics_health")
    .then((health) => {
      setProductAnalyticsConsent(health.choice === true);
      trackProductEvent("app_open");
    })
    .catch(() => {});
  void listen<boolean>("product-analytics-consent", (event) => {
    setProductAnalyticsConsent(event.payload);
    if (event.payload) trackProductEvent("screen_view");
  }).catch(() => {});
  document.addEventListener("visibilitychange", () => {
    previous = Date.now();
    if (!document.hidden) trackProductEvent("screen_view");
  });
  window.setInterval(() => trackProductEvent("user_engagement"), 60000);
}
