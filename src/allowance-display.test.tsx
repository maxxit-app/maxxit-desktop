import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it, vi } from "vitest";
import { demoSnapshot } from "./demo";
import type { Provider } from "./types";

vi.hoisted(() => vi.stubGlobal("location", new URL("http://localhost/")));
vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => false,
  invoke: vi.fn(),
}));

import { WindowCard } from "./App";
import { ProviderCard } from "./TrayPanel";

const now = Date.parse("2026-10-07T19:47:00Z");

function provider(ageMinutes: number): Provider {
  const view = demoSnapshot().providers[0];
  view.observation!.observedAt = new Date(
    now - ageMinutes * 60000,
  ).toISOString();
  view.observation!.windows = [
    {
      bucketId: "codex",
      windowId: "primary",
      label: "Weekly allowance",
      usedPercent: 7,
      durationMinutes: 10080,
      resetsAt: "2026-10-14T08:06:30Z",
      availability: "available",
    },
  ];
  return view;
}

function cards(view: Provider) {
  return {
    tray: renderToStaticMarkup(
      <ProviderCard provider={view} now={now} open={() => {}} />,
    ),
    dashboard: renderToStaticMarkup(<WindowCard provider={view} />),
  };
}

describe("recorded allowance display", () => {
  afterEach(() => vi.useRealTimers());

  it("retains an old percentage with stale labels and the original observation age", () => {
    vi.useFakeTimers().setSystemTime(now);
    const view = provider(316);
    const before = JSON.stringify(view);
    const { tray, dashboard } = cards(view);
    expect(tray).toContain("Stale data");
    expect(tray).toContain('aria-valuenow="93"');
    expect(tray).toContain("93 percent remaining at last reading");
    expect(tray).toContain("316 minutes ago");
    expect(tray).toContain("% last read");
    expect(dashboard).toContain("93% last read");
    expect(dashboard).toContain("Stale");
    expect(tray).not.toContain("Unavailable");
    expect(dashboard).not.toContain("Unavailable");
    expect(JSON.stringify(view)).toBe(before);
  });

  it("marks both views stale at exactly two hours while fresh readings show allowance left", () => {
    vi.useFakeTimers().setSystemTime(now);
    const fresh = cards(provider(119));
    expect(fresh.tray).toContain("% left");
    expect(fresh.dashboard).toContain("93% left");
    const stale = cards(provider(120));
    expect(stale.tray).toContain("% last read");
    expect(stale.dashboard).toContain("93% last read");
  });

  it.each(["missing", "expired"])(
    "keeps %s allowance unavailable even in an old observation",
    (state) => {
      vi.useFakeTimers().setSystemTime(now);
      const view = provider(316);
      const window = view.observation!.windows[0];
      if (state === "missing") window.usedPercent = null;
      else window.resetsAt = new Date(now).toISOString();
      const { tray, dashboard } = cards(view);
      expect(tray).toContain("Unavailable");
      expect(dashboard).toContain("Unavailable");
      expect(tray).not.toContain('aria-valuenow="93"');
      expect(dashboard).not.toContain("93% last read");
    },
  );
});
