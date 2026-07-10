import {
  AppSettings,
  OrdenRunResult,
  OrdenVisualConfig,
  ScheduleSettings,
} from "../../store/useAppStore";

export type GraceUnit = "seconds" | "minutes" | "hours";
export type OrdenEditorMode = "visual" | "source";
export type OrdenView = "editor" | "preview";
export type McpDraft = Pick<
  AppSettings,
  | "mcp_enabled"
  | "mcp_allow_write"
  | "mcp_transport"
  | "mcp_server_name"
  | "mcp_command"
  | "mcp_args"
  | "mcp_http_url"
  | "mcp_token"
>;

export const GRACE_STEPS = [
  0, 30, 60, 300, 900, 1800, 3600, 7200, 21600, 43200, 86400, 172800, 604800,
];

export const MAX_GRACE_SECONDS = 604800;

export function formatDuration(seconds: number): string {
  if (seconds <= 0) return "0s";
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = seconds % 60;
  const parts: string[] = [];
  if (hours > 0) parts.push(`${hours}h`);
  if (minutes > 0) parts.push(`${minutes}m`);
  if (secs > 0) parts.push(`${secs}s`);
  return parts.join(" ") || "0s";
}

export function secondsToUnit(seconds: number): { value: number; unit: GraceUnit } {
  if (seconds % 3600 === 0 && seconds >= 3600) {
    return { value: seconds / 3600, unit: "hours" };
  }
  if (seconds % 60 === 0 && seconds >= 60) {
    return { value: seconds / 60, unit: "minutes" };
  }
  return { value: seconds, unit: "seconds" };
}

export function unitToSeconds(value: number, unit: GraceUnit): number {
  switch (unit) {
    case "hours":
      return value * 3600;
    case "minutes":
      return value * 60;
    default:
      return value;
  }
}

export function nearestGraceStep(seconds: number): number {
  return GRACE_STEPS.reduce((prev, curr) =>
    Math.abs(curr - seconds) < Math.abs(prev - seconds) ? curr : prev
  );
}

export function getDirectoryFromPath(filePath: string | null): string | null {
  if (!filePath) return null;
  const normalized = filePath.replace(/\\/g, "/");
  const lastSlash = normalized.lastIndexOf("/");
  if (lastSlash <= 0) return normalized;
  return normalized.slice(0, lastSlash);
}

export function defaultSchedule(): ScheduleSettings {
  return {
    schedule_enabled: false,
    schedule_times_per_day: 1,
    schedule_time_1: "08:00",
    schedule_time_2: null,
    schedule_time_3: null,
    schedule_time_4: null,
    schedule_cron_enabled: false,
    schedule_cron_expr: "0 * * * *",
    keepalive_enabled: false,
    keepalive_interval_minutes: 15,
  };
}

export function defaultMcpDraft(): McpDraft {
  return {
    mcp_enabled: false,
    mcp_allow_write: false,
    mcp_transport: "stdio",
    mcp_server_name: "shelfy",
    mcp_command: null,
    mcp_args: "--mcp",
    mcp_http_url: "http://127.0.0.1:8765/mcp",
    mcp_token: null,
  };
}

export function defaultOrdenVisualConfig(): OrdenVisualConfig {
  return {
    rules: [
      {
        id: `rule-${Date.now()}`,
        name: "Back up PDF documents",
        enabled: true,
        targets: "files",
        location: "~/Downloads",
        subfolders: true,
        extensions: "pdf",
        filterMode: "all",
        tags: "backup",
        action: "copy",
        destination: "~/Documents/Shelfy Backups/PDF/",
        archiveFormat: "auto",
        archivePassword: "",
        archivePasswords: "",
        deleteOriginal: false,
        onConflict: "rename_new",
      },
    ],
  };
}

export function yamlQuote(value: string): string {
  const trimmed = value.trim();
  if (!trimmed) return '""';
  if (/^[A-Za-z0-9_./~:@{} -]+$/.test(trimmed) && !trimmed.includes("#")) {
    return trimmed;
  }
  return JSON.stringify(trimmed);
}

export function listFromCsv(value: string): string[] {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

export function listFromPathText(value: string): string[] {
  const byLine = value
    .split(/\r?\n/)
    .map((item) => item.trim())
    .filter(Boolean);
  if (byLine.length > 1) return byLine;
  return byLine.length === 1 ? byLine : [];
}

export function mergePathText(current: string, additions: string[]): string {
  const paths = [...listFromPathText(current), ...additions]
    .map((item) => item.trim())
    .filter(Boolean);
  return Array.from(new Set(paths)).join("\n");
}

export function normalizeDialogSelection(selection: string | string[] | null): string[] {
  if (!selection) return [];
  return Array.isArray(selection) ? selection : [selection];
}

export function parseOrdenPreviewDestination(message: string): string | null {
  const match = message.match(/(?:Copy|Move|Rename|Write|Symlink|Hardlink) to (.+)$/i);
  return match ? match[1].trim() : null;
}

export function parseOrdenPreviewAction(sender: string, message: string): string {
  const normalized = sender.trim().toLowerCase();
  if (normalized) return normalized;
  if (/copy/i.test(message)) return "copy";
  if (/move/i.test(message)) return "move";
  if (/rename/i.test(message)) return "rename";
  return "rule";
}

export function buildOrdenPreviewRows(result: OrdenRunResult | null) {
  if (!result) return [];
  return result.logs.map((log, idx) => ({
    id: `${idx}-${log.path}-${log.msg}`,
    source: log.path || "(standalone)",
    action: parseOrdenPreviewAction(log.sender, log.msg),
    destination: parseOrdenPreviewDestination(log.msg),
    message: log.msg,
    level: log.level,
  }));
}

export function visualToOrdenYaml(config: OrdenVisualConfig): string {
  const lines = ["rules:"];
  const rules = config.rules.length > 0 ? config.rules : defaultOrdenVisualConfig().rules;
  rules.forEach((rule) => {
    lines.push(`  - name: ${yamlQuote(rule.name || "Untitled rule")}`);
    if (!rule.enabled) lines.push("    enabled: false");
    lines.push(`    targets: ${rule.targets || "files"}`);
    const tags = listFromCsv(rule.tags);
    if (tags.length > 0) {
      lines.push("    tags:");
      tags.forEach((tag) => lines.push(`      - ${yamlQuote(tag)}`));
    }
    lines.push("    locations:");
    const locations = listFromPathText(rule.location || "~/Downloads");
    (locations.length > 0 ? locations : ["~/Downloads"]).forEach((location) => {
      lines.push(`      - ${yamlQuote(location)}`);
    });
    lines.push(`    subfolders: ${rule.subfolders ? "true" : "false"}`);
    const extensions = listFromCsv(rule.extensions);
    if (extensions.length > 0) {
      lines.push(`    filter_mode: ${rule.filterMode || "all"}`);
      lines.push("    filters:");
      lines.push(`      - extension: [${extensions.map(yamlQuote).join(", ")}]`);
    }
    lines.push("    actions:");
    const destinations = listFromPathText(rule.destination || "~/Documents/Shelfy Backups/");
    if (rule.action === "copy") {
      const actionDestinations = destinations.length > 0 ? destinations : ["~/Documents/Shelfy Backups/"];
      if (actionDestinations.length === 1) {
        lines.push(`      - copy: ${yamlQuote(actionDestinations[0])}`);
      } else {
        lines.push("      - copy:");
        lines.push("          dest:");
        actionDestinations.forEach((destination) => lines.push(`            - ${yamlQuote(destination)}`));
        lines.push("          continue_with: original");
      }
    } else if (rule.action === "move") {
      lines.push(`      - move: ${yamlQuote(destinations[0] || "~/Documents/Shelfy Backups/")}`);
    } else if (rule.action === "rename") {
      lines.push(`      - rename: ${yamlQuote(destinations[0] || "{name}")}`);
    } else {
      lines.push(`      - ${rule.action || "echo"}: ${yamlQuote(destinations[0] || "matched {path}")}`);
    }
  });
  return `${lines.join("\n")}\n`;
}
