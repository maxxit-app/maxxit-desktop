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
} from "./product-analytics";
afterEach(() => {
  setProductAnalyticsConsent(false);
  vi.clearAllMocks();
  vi.unstubAllGlobals();
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
