import { describe, expect, it, vi } from "vitest";
import type { DownloadEvent } from "@tauri-apps/plugin-updater";
import { UpdateController } from "./updates";

function release() {
  return {
    version: "0.2.0",
    body: "Better usage charts.",
    close: vi.fn().mockResolvedValue(undefined),
    downloadAndInstall: vi
      .fn()
      .mockImplementation(async (progress: (event: DownloadEvent) => void) => {
        progress({ event: "Started", data: { contentLength: 200 } });
        progress({ event: "Progress", data: { chunkLength: 100 } });
        progress({ event: "Finished" });
      }),
  };
}
describe("desktop updates", () => {
  it("keeps installation and restart behind user actions", async () => {
    const update = release();
    const restart = vi.fn().mockResolvedValue(undefined);
    const controller = new UpdateController(async () => update, restart);
    await controller.check();
    expect(controller.snapshot()).toMatchObject({
      status: "available",
      version: "0.2.0",
    });
    expect(update.downloadAndInstall).not.toHaveBeenCalled();
    const states: unknown[] = [];
    controller.subscribe(() => states.push(controller.snapshot()));
    await controller.install();
    expect(states).toContainEqual(
      expect.objectContaining({ status: "downloading", progress: 50 }),
    );
    expect(states).toContainEqual(
      expect.objectContaining({ status: "installing" }),
    );
    expect(controller.snapshot().status).toBe("ready");
    expect(restart).not.toHaveBeenCalled();
    await controller.install();
    expect(update.downloadAndInstall).toHaveBeenCalledTimes(1);
    await controller.restart();
    expect(restart).toHaveBeenCalledOnce();
  });
  it("reports no update and recovers from offline checks", async () => {
    const check = vi
      .fn()
      .mockRejectedValueOnce(new Error("offline"))
      .mockResolvedValueOnce(null);
    const controller = new UpdateController(check);
    await controller.check();
    expect(controller.snapshot().status).toBe("error");
    await controller.check();
    expect(controller.snapshot()).toEqual({ status: "current" });
  });
  it("retains the pending update after a failed download so it can be retried", async () => {
    const update = release();
    update.downloadAndInstall.mockRejectedValueOnce(
      new Error("signature rejected"),
    );
    const controller = new UpdateController(async () => update);
    await controller.check();
    await controller.install();
    expect(controller.snapshot()).toMatchObject({
      status: "error",
      version: "0.2.0",
    });
    expect(update.close).not.toHaveBeenCalled();
    await controller.install();
    expect(controller.snapshot().status).toBe("ready");
  });
  it("prevents overlapping checks and disposes replaced resources", async () => {
    const update = release();
    let resolve!: (value: typeof update) => void;
    const check = vi
      .fn()
      .mockImplementationOnce(
        () =>
          new Promise((done) => {
            resolve = done;
          }),
      )
      .mockResolvedValueOnce(null);
    const controller = new UpdateController(check);
    const first = controller.check();
    await controller.check();
    expect(check).toHaveBeenCalledOnce();
    resolve(update);
    await first;
    await controller.check();
    expect(update.close).toHaveBeenCalledOnce();
    expect(controller.snapshot().status).toBe("current");
  });
  it("allows a restart retry without downloading again", async () => {
    const restart = vi
      .fn()
      .mockRejectedValueOnce(new Error("restart failed"))
      .mockResolvedValueOnce(undefined);
    const controller = new UpdateController(async () => release(), restart);
    await controller.check();
    await controller.install();
    await controller.restart();
    expect(controller.snapshot()).toMatchObject({
      status: "ready",
      error: expect.stringContaining("Quit and open"),
    });
    await controller.restart();
    expect(restart).toHaveBeenCalledTimes(2);
  });
});
