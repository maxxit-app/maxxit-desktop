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
  accountSync: boolean;
  projectSync: boolean;
  workflowSync: boolean;
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
  generationMode: "local" | "hosted";
  localAiConsent: boolean;
  resultEvents: boolean;
  localNotifications: boolean;
}
export interface Project {
  id: string;
  name: string;
  description: string;
  createdAt: string;
  archived?: boolean;
  pinned?: boolean;
}
export interface LocalRun {
  id: string;
  projectId: string;
  projectName: string;
  parentId: string | null;
  kind: "analysis" | "task";
  state: string;
  createdAt: string;
  bundle: string;
  suggestions: { title: string; description: string; prompt: string }[];
  summary: string | null;
  delivery: string;
}
export interface AccountState {
  status:
    | "checking"
    | "signed_out"
    | "authenticated"
    | "offline"
    | "revoked"
    | "error";
  accountId?: string | null;
  deviceId?: string | null;
  error?: string | null;
  offlineUntil?: number;
  pairing?: { code: string; url: string; expiresAt: string };
}
export interface Snapshot {
  legacyDataAvailable?: boolean;
  account?: AccountState;
  sync?: { pending: number; lastSyncAt: string | null; error: string | null };
  localRuns: LocalRun[];
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
      account?: { id: string; email: string | null };
      identity?: {
        consent: {
          usage: boolean;
          metadata: boolean;
          accountSync: boolean;
          workflowDetails: boolean;
          completionEvents: boolean;
        };
        deviceId: string;
      };
      primaryDesktopId?: string;
      billing?: { billingReady: boolean; plan: string };
    };
  };
}
export const defaults: Settings = {
  theme: "system",
  background: true,
  trayProvider: "codex",
  cloudSync: false,
  accountSync: false,
  projectSync: false,
  workflowSync: false,
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
  generationMode: "local",
  localAiConsent: false,
  resultEvents: false,
  localNotifications: false,
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
  localRuns: [],
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
