export interface UsageWindow {
  bucketId: string;
  windowId: string;
  label: string | null;
  usedPercent: number | null;
  durationMinutes: number | null;
  resetsAt: string | null;
  availability: string;
}
export interface Observation {
  provider: string;
  accountLabel: string;
  observedAt: string;
  source: string;
  windows: UsageWindow[];
}
export interface Provider {
  provider: string;
  connected: boolean;
  accountLabel: string;
  status: string;
  error: string | null;
  observation: Observation | null;
  dailyTokens: { startDate: string; tokens: number }[];
  summary: { scope?: string; historyComplete?: boolean };
}
export interface Settings {
  theme: string;
  background: boolean;
  trayProvider: string;
  cloudSync: boolean;
  apiOrigin: string;
  claudeEnabled: boolean;
  codexEnabled: boolean;
  claudeAccount: string;
  codexPath: string | null;
  timezone: string;
  hoursBefore: number;
  shortHoursBefore: number;
  minRemaining: number;
  quietStart: number;
  quietEnd: number;
  dailyLimit: number;
  email: boolean;
  aiConsent: boolean;
}
export interface Project {
  id: string;
  name: string;
  description: string;
  createdAt: string;
}
export interface Snapshot {
  settings: Settings;
  providers: Provider[];
  history: Observation[];
  projects: Project[];
  cloud: {
    connected: boolean;
    plan: string;
    error?: string;
    data?: {
      suggestions: {
        id: string;
        title: string;
        description: string;
        readyToCopyPrompt: string;
      }[];
      generations: { id: string; state: string }[];
      emailReady?: boolean;
    };
  };
}
export const defaults: Settings = {
  theme: "system",
  background: true,
  trayProvider: "codex",
  cloudSync: false,
  apiOrigin: "https://maxxit.app",
  claudeEnabled: false,
  codexEnabled: false,
  claudeAccount: "Local Claude Code",
  codexPath: null,
  timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
  hoursBefore: 24,
  shortHoursBefore: 1,
  minRemaining: 30,
  quietStart: 22,
  quietEnd: 8,
  dailyLimit: 1,
  email: false,
  aiConsent: false,
};
export const empty: Snapshot = {
  settings: defaults,
  providers: ["codex", "claude"].map((provider) => ({
    provider,
    connected: false,
    accountLabel: "Local account",
    status: "disconnected",
    error: null,
    observation: null,
    dailyTokens: [],
    summary: {},
  })),
  history: [],
  projects: [],
  cloud: { connected: false, plan: "free" },
};
export function remaining(
  window: UsageWindow,
  now = Date.now(),
): number | null {
  return window.availability === "available" &&
    window.usedPercent !== null &&
    window.resetsAt !== null &&
    Date.parse(window.resetsAt) > now
    ? Math.max(0, 100 - window.usedPercent)
    : null;
}
export function resetLabel(reset: string | null, now = Date.now()): string {
  if (!reset) return "Reset time unavailable";
  const minutes = Math.ceil((Date.parse(reset) - now) / 60000);
  if (minutes <= 0) return "Awaiting next observation";
  return minutes >= 1440
    ? `Resets in ${Math.floor(minutes / 1440)}d ${Math.floor((minutes % 1440) / 60)}h`
    : minutes >= 60
      ? `Resets in ${Math.floor(minutes / 60)}h ${minutes % 60}m`
      : `Resets in ${minutes}m`;
}
