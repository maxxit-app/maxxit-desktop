import { describe, it, expect } from "vitest";
import { remaining, resetLabel, type UsageWindow } from "./types";
const window: UsageWindow = {
  bucketId: "codex",
  windowId: "primary",
  label: "Primary",
  usedPercent: 34,
  durationMinutes: 300,
  resetsAt: "2099-01-01T00:00:00Z",
  availability: "available",
};
describe("allowance availability", () => {
  it("never converts absent usage into a full allowance", () => {
    expect(remaining({ ...window, usedPercent: null })).toBeNull();
    expect(remaining({ ...window, availability: "missing" })).toBeNull();
  });
  it("expires old snapshots instead of showing spare allowance", () => {
    expect(
      remaining({ ...window, resetsAt: "2000-01-01T00:00:00Z" }),
    ).toBeNull();
    expect(resetLabel("2000-01-01T00:00:00Z")).toBe(
      "Awaiting next observation",
    );
  });
  it("reports observed remaining percentage", () =>
    expect(remaining(window)).toBe(66));
});
