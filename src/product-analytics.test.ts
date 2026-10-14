import { afterEach, expect, it, vi } from "vitest";
const bridge = vi.hoisted(() => ({
  invoke: vi.fn().mockResolvedValue(undefined),
  isTauri: vi.fn(() => true),
}));
vi.mock("@tauri-apps/api/core", () => bridge);
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));
import {
  productScreen,
  setProductAnalyticsConsent,
  trackProductEvent,
  hasUsableObservation,
  productActivation,
} from "./product-analytics";
import { empty, type Provider } from "./types";
afterEach(() => {
  productActivation([], false);
  setProductAnalyticsConsent(false);
  vi.clearAllMocks();
  vi.unstubAllGlobals();
});
function usableProvider(now: number): Provider {
  return {
    ...structuredClone(empty.providers[0]),
    connected: true,
    observation: {
      provider: "codex",
      accountLabel: "PRIVATE",
      source: "fixture",
      observedAt: new Date(now - 1000).toISOString(),
      windows: [
        {
          bucketId: "fixture",
          windowId: "fixture",
          label: null,
          usedPercent: 100,
          durationMinutes: 300,
          resetsAt: new Date(now + 1000).toISOString(),
          availability: "available",
        },
      ],
    },
  };
}
it("requires a connected, fresh, finite and unexpired reported window for activation", () => {
  const now = Date.now();
  const provider = usableProvider(now);
  expect(hasUsableObservation([provider], now)).toBe(true);
  provider.connected = false;
  expect(hasUsableObservation([provider], now)).toBe(false);
  provider.connected = true;
  provider.observation!.observedAt = new Date(now - 7200001).toISOString();
  expect(hasUsableObservation([provider], now)).toBe(false);
  provider.observation!.observedAt = new Date(now + 1000).toISOString();
  expect(hasUsableObservation([provider], now)).toBe(false);
  provider.observation!.observedAt = new Date(now).toISOString();
  provider.observation!.windows[0].resetsAt = new Date(now).toISOString();
  expect(hasUsableObservation([provider], now)).toBe(false);
  provider.observation!.windows[0].resetsAt = new Date(
    now + 1000,
  ).toISOString();
  provider.observation!.windows[0].usedPercent = NaN;
  expect(hasUsableObservation([provider], now)).toBe(false);
});
it("queues activation once after consent and excludes provider details", async () => {
  vi.stubGlobal("document", { hidden: false });
  const providers = [usableProvider(Date.now())];
  productActivation(providers, true);
  expect(bridge.invoke).not.toHaveBeenCalled();
  setProductAnalyticsConsent(true);
  await vi.waitFor(() => expect(bridge.invoke).toHaveBeenCalledTimes(1));
  expect(bridge.invoke.mock.calls[0][1].input.name).toBe("activation");
  expect(JSON.stringify(bridge.invoke.mock.calls)).not.toContain("PRIVATE");
  productActivation(providers, true);
  productActivation(providers, false);
  expect(bridge.invoke).toHaveBeenCalledTimes(1);
});
it("never invokes the analytics bridge before consent", () => {
  vi.stubGlobal("document", { hidden: false });
  trackProductEvent("desktop_action", "local_import");
  expect(bridge.invoke).not.toHaveBeenCalled();
});
it("sends fixed metadata without project args and rejects unknown actions", async () => {
  vi.stubGlobal("document", { hidden: false });
  setProductAnalyticsConsent(true);
  productScreen("Projects");
  await vi.waitFor(() => expect(bridge.invoke).toHaveBeenCalledTimes(1));
  expect(bridge.invoke.mock.calls[0][1].input).toMatchObject({
    name: "screen_view",
    screen: "Projects",
  });
  trackProductEvent("desktop_action", "PRIVATE_PROMPT");
  expect(bridge.invoke).toHaveBeenCalledTimes(1);
});
it("discards queued events when analytics is withdrawn", async () => {
  vi.stubGlobal("document", { hidden: false });
  let resolve: () => void = () => {};
  bridge.invoke.mockImplementationOnce(
    () =>
      new Promise<void>((done) => {
        resolve = done;
      }),
  );
  setProductAnalyticsConsent(true);
  trackProductEvent("desktop_action", "save_project");
  trackProductEvent("desktop_action", "local_import");
  setProductAnalyticsConsent(false);
  resolve();
  await new Promise((done) => setTimeout(done, 0));
  expect(bridge.invoke).toHaveBeenCalledTimes(1);
  trackProductEvent("desktop_action", "local_task");
  expect(bridge.invoke).toHaveBeenCalledTimes(1);
});
it("excludes hidden windows and browser previews", () => {
  vi.stubGlobal("document", { hidden: true });
  setProductAnalyticsConsent(true);
  trackProductEvent("app_open");
  expect(bridge.invoke).not.toHaveBeenCalled();
  vi.stubGlobal("document", { hidden: false });
  bridge.isTauri.mockReturnValueOnce(false);
  trackProductEvent("app_open");
  expect(bridge.invoke).not.toHaveBeenCalled();
});
