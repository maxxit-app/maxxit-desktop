import { defaults, type Snapshot } from "./types";
export function demoSnapshot(): Snapshot {
  const now = Date.now();
  const providers = ["codex", "claude"].map((provider, index) => ({
    provider,
    connected: true,
    accountLabel: index ? "Claude Code · local" : "Codex · local",
    status: "connected",
    error: null,
    observation: {
      provider,
      accountLabel: "Demo account",
      observedAt: new Date(now - 120000).toISOString(),
      source: index ? "claude-statusline" : "codex-local",
      windows: [
        {
          bucketId: "subscription",
          windowId: "primary",
          label: "Five-hour allowance",
          usedPercent: index ? 42 : 28,
          durationMinutes: 300,
          resetsAt: new Date(now + 110 * 60000).toISOString(),
          availability: "available",
        },
        {
          bucketId: "subscription",
          windowId: "secondary",
          label: "Weekly allowance",
          usedPercent: index ? 64 : 47,
          durationMinutes: 10080,
          resetsAt: new Date(now + 54 * 3600000).toISOString(),
          availability: "available",
        },
      ],
    },
    dailyTokens: Array.from({ length: 14 }, (_, day) => ({
      startDate: new Date(now - (13 - day) * 86400000)
        .toISOString()
        .slice(0, 10),
      tokens: [
        21000, 34000, 15000, 42000, 29000, 48000, 32000, 19000, 56000, 38000,
        61000, 42000, 47000, 35000,
      ][day],
    })),
    summary: {
      scope: "Observed local session increments",
      historyComplete: false,
    },
  }));
  return {
    localRuns: [],
    settings: {
      ...defaults,
      theme: "light",
      codexEnabled: true,
      claudeEnabled: true,
    },
    providers,
    history: providers.flatMap((p) => (p.observation ? [p.observation] : [])),
    projects: [
      {
        id: "sample",
        name: "A better personal website",
        description:
          "Improve accessibility, clean up navigation, and add a writing archive.",
        createdAt: new Date(now).toISOString(),
      },
    ],
    cloud: { connected: false, plan: "free" },
  };
}
