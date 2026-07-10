import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

export interface Rule {
  id?: number;
  name: string;
  priority: number;
  enabled: boolean;
  extensions: string[];
  pattern: string | null;
  destination: string;
  action: string;
  folder_id: number;
  folder_path?: string | null;
}

export interface WatchedFolder {
  id?: number;
  path: string;
  enabled: boolean;
  mode: string;
}

export interface ActionLog {
  id?: number;
  timestamp: string;
  source_path: string;
  destination_path: string | null;
  action: string;
  file_name: string;
  file_type: string;
   engine: string;
   rule_label?: string | null;
  undone: boolean;
}

export interface AppSettings {
  id?: number;
  language: string;
  theme: string;
  telemetry_enabled: boolean;
  first_run: boolean;
  autostart: boolean;
  grace_period_seconds: number;
  lock_check_enabled: boolean;
  schedule_enabled: boolean;
  schedule_times_per_day: number;
  schedule_time_1: string | null;
  schedule_time_2: string | null;
  schedule_time_3: string | null;
  schedule_time_4: string | null;
  schedule_cron_enabled: boolean;
  schedule_cron_expr: string | null;
  keepalive_enabled: boolean;
  keepalive_interval_minutes: number;
  mcp_enabled: boolean;
  mcp_allow_write: boolean;
  mcp_transport: string;
  mcp_server_name: string;
  mcp_command: string | null;
  mcp_args: string | null;
  mcp_http_url: string | null;
  mcp_token: string | null;
}

export const defaultSettings: AppSettings = {
  language: 'en',
  theme: 'system',
  telemetry_enabled: false,
  first_run: true,
  autostart: true,
  grace_period_seconds: 300,
  lock_check_enabled: true,
  schedule_enabled: false,
  schedule_times_per_day: 1,
  schedule_time_1: '08:00',
  schedule_time_2: null,
  schedule_time_3: null,
  schedule_time_4: null,
  schedule_cron_enabled: false,
  schedule_cron_expr: '0 * * * *',
  keepalive_enabled: false,
  keepalive_interval_minutes: 15,
  mcp_enabled: false,
  mcp_allow_write: false,
  mcp_transport: 'stdio',
  mcp_server_name: 'shelfy',
  mcp_command: null,
  mcp_args: '--mcp',
  mcp_http_url: 'http://127.0.0.1:8765/mcp',
  mcp_token: null,
};

export interface ScheduleSettings {
  schedule_enabled: boolean;
  schedule_times_per_day: number;
  schedule_time_1: string | null;
  schedule_time_2: string | null;
  schedule_time_3: string | null;
  schedule_time_4: string | null;
  schedule_cron_enabled: boolean;
  schedule_cron_expr: string | null;
  keepalive_enabled: boolean;
  keepalive_interval_minutes: number;
}

export interface SchedulerLog {
  id?: number;
  timestamp: string;
  level: string;
  event: string;
  message: string;
  details: string | null;
}

export interface SystemKeepaliveStatus {
  supported: boolean;
  platform: string;
}

// ---- Orden (advanced YAML rules engine) ----

export interface OrdenLog {
  level: string;
  sender: string;
  rule_nr: number;
  path: string;
  msg: string;
}

export interface OrdenRunResult {
  success: number;
  errors: number;
  simulate: boolean;
  logs: OrdenLog[];
}

export interface OrdenRunHistory {
  id?: number;
  config_name: string;
  timestamp: string;
  simulate: boolean;
  success: number;
  errors: number;
  trigger: string;
  logs_json: string;
}

export interface OrdenJob {
  id?: number;
  name: string;
  config_name: string;
  enabled: boolean;
  mode: string;
  cron_expr: string | null;
  fixed_time: string | null;
  interval_minutes: number;
  watch_paths: string;
  tags: string;
  skip_tags: string;
  simulate: boolean;
  min_file_count: number;
  path_exists: string | null;
  time_window_start: string | null;
  time_window_end: string | null;
  last_run_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface OrdenVisualConfig {
  rules: OrdenVisualRule[];
}

export interface OrdenVisualRule {
  id: string;
  name: string;
  enabled: boolean;
  targets: string;
  location: string;
  subfolders: boolean;
  extensions: string;
  filterMode: string;
  tags: string;
  action: string;
  destination: string;
  archiveFormat: string;
  archivePassword: string;
  archivePasswords: string;
  deleteOriginal: boolean;
  onConflict: string;
}

export interface McpClientConfig {
  enabled: boolean;
  transport: string;
  config_json: string;
}

export interface OrdenQuickTask {
  configName: string;
  ruleId: string;
  ruleName: string;
  enabled: boolean;
  action: string;
  location: string;
  destination: string;
  tags: string;
  yaml: string;
}

// The default example orden config: scan downloads for .html files and copy
// them into a "网页" (web pages) subfolder.
export const DEFAULT_ORDEN_EXAMPLE = `rules:
  - name: "扫描下载目录的 HTML 文件并复制到网页文件夹"
    locations:
      - ~/Downloads
    subfolders: true
    filters:
      - extension: html
    actions:
      - copy: ~/Downloads/网页/
`;

interface AppState {
  rules: Rule[];
  folders: WatchedFolder[];
  logs: ActionLog[];
  stats: { file_type: string; count: number }[];
  settings: AppSettings | null;
  schedule: ScheduleSettings | null;
  schedulerLogs: SchedulerLog[];
  pendingFiles: [string, string][];
  isLoading: boolean;
  currentView: 'popup' | 'settings';

  loadSettings: () => Promise<void>;
  saveSettings: (settings: AppSettings) => Promise<void>;
  setAutostart: (enabled: boolean) => Promise<void>;
  loadRules: () => Promise<void>;
  loadFolders: () => Promise<void>;
  loadLogs: () => Promise<void>;
  loadStats: () => Promise<void>;
  scanFolder: (path: string) => Promise<{ file: string; rule: string; destination: string }[]>;
  undoAction: (id: number) => Promise<boolean>;
  undoAll: () => Promise<number>;
  addFolder: (path: string, mode: string) => Promise<void>;
  removeFolder: (id: number) => Promise<void>;
  updateFolderMode: (id: number, mode: string) => Promise<void>;
  addRule: (rule: Rule) => Promise<void>;
  updateRule: (rule: Rule) => Promise<void>;
  deleteRule: (id: number) => Promise<void>;
  clearLogs: () => Promise<void>;
  getPendingFiles: () => Promise<void>;
  getSchedule: () => Promise<void>;
  updateSchedule: (schedule: ScheduleSettings) => Promise<void>;
  validateCron: (expr: string) => Promise<void>;
  loadSchedulerLogs: () => Promise<void>;
  clearSchedulerLogs: () => Promise<void>;
  getSystemKeepaliveStatus: () => Promise<SystemKeepaliveStatus>;
  installSystemKeepalive: (intervalMinutes: number) => Promise<void>;
  uninstallSystemKeepalive: () => Promise<void>;
  exportRules: (path: string) => Promise<void>;
  importRules: (path: string, replace: boolean) => Promise<number>;
  exportConfig: (path: string) => Promise<void>;
  importConfig: (path: string, replace: boolean) => Promise<void>;
  // orden
  ordenList: () => Promise<string[]>;
  ordenLoad: (name: string) => Promise<string>;
  ordenSave: (name: string, yaml: string) => Promise<void>;
  ordenDelete: (name: string) => Promise<void>;
  ordenCheck: (yaml: string) => Promise<void>;
  ordenRun: (yaml: string, simulate: boolean, tags: string[], skipTags: string[]) => Promise<OrdenRunResult>;
  ordenVisualFromYaml: (yaml: string) => Promise<OrdenVisualConfig>;
  ordenHistory: (name: string, limit: number) => Promise<OrdenRunHistory[]>;
  ordenJobs: () => Promise<OrdenJob[]>;
  ordenSaveJob: (job: OrdenJob) => Promise<number>;
  ordenDeleteJob: (id: number) => Promise<void>;
  ordenRunJob: (job: OrdenJob) => Promise<OrdenRunResult>;
  getMcpClientConfig: () => Promise<McpClientConfig>;
  getOrdenQuickTasks: () => Promise<OrdenQuickTask[]>;
  runOrdenQuickTask: (yaml: string, simulate: boolean) => Promise<OrdenRunResult>;
}

export const useAppStore = create<AppState>((set, get) => ({
  rules: [],
  folders: [],
  logs: [],
  stats: [],
  settings: null,
  schedule: null,
  schedulerLogs: [],
  pendingFiles: [],
  isLoading: false,
  currentView: 'popup',

  loadSettings: async () => {
    const settings = await invoke<AppSettings>('get_settings_cmd');
    set({ settings });
  },

  saveSettings: async (settings) => {
    await invoke('update_settings_cmd', { settings });
    set({ settings });
  },

  setAutostart: async (enabled) => {
    const { settings } = get();
    if (!settings) return;
    if (enabled) {
      await invoke('enable_autostart_cmd');
    } else {
      await invoke('disable_autostart_cmd');
    }
    const updated = { ...settings, autostart: enabled };
    await invoke('update_settings_cmd', { settings: updated });
    set({ settings: updated });
  },

  loadRules: async () => {
    const rules = await invoke<Rule[]>('get_rules_cmd');
    set({ rules });
  },

  loadFolders: async () => {
    const folders = await invoke<WatchedFolder[]>('get_folders_cmd');
    set({ folders });
  },

  loadLogs: async () => {
    const logs = await invoke<ActionLog[]>('get_logs_cmd', { limit: 50 });
    set({ logs });
  },

  loadStats: async () => {
    const raw = await invoke<[string, number][]>('get_stats_cmd');
    set({ stats: raw.map(([file_type, count]) => ({ file_type, count })) });
  },

  scanFolder: async (path) => {
    set({ isLoading: true });
    try {
      const results = await invoke<[string, string, string][]>('scan_folder_cmd', { path });
      await get().loadLogs();
      await get().loadStats();
      return results.map(([file, rule, destination]) => ({ file, rule, destination }));
    } finally {
      set({ isLoading: false });
    }
  },

  undoAction: async (id) => {
    const success = await invoke<boolean>('undo_action_cmd', { id });
    if (success) {
      await get().loadLogs();
      await get().loadStats();
    }
    return success;
  },

  undoAll: async () => {
    const count = await invoke<number>('undo_all_cmd');
    if (count > 0) {
      await get().loadLogs();
      await get().loadStats();
    }
    return count;
  },

  addFolder: async (path, mode) => {
    try {
      await invoke('add_folder_cmd', { path, mode });
      await get().loadFolders();
    } catch (e) {
      console.error('addFolder failed:', e);
      throw e;
    }
  },

  removeFolder: async (id) => {
    try {
      await invoke('remove_folder_cmd', { id });
      await get().loadFolders();
    } catch (e) {
      console.error('removeFolder failed:', e);
      throw e;
    }
  },

  updateFolderMode: async (id, mode) => {
    try {
      await invoke('update_folder_mode_cmd', { id, mode });
      await get().loadFolders();
    } catch (e) {
      console.error('updateFolderMode failed:', e);
      throw e;
    }
  },

  addRule: async (rule) => {
    await invoke('add_rule_cmd', { rule });
    await get().loadRules();
  },

  updateRule: async (rule) => {
    await invoke('update_rule_cmd', { rule });
    await get().loadRules();
  },

  deleteRule: async (id) => {
    await invoke('delete_rule_cmd', { id });
    await get().loadRules();
  },

  clearLogs: async () => {
    await invoke('clear_logs_cmd');
    set({ logs: [], stats: [] });
  },

  getPendingFiles: async () => {
    try {
      const pendingFiles = await invoke<[string, string][]>('get_pending_files_cmd');
      set({ pendingFiles });
    } catch (e) {
      console.error('getPendingFiles failed:', e);
    }
  },

  getSchedule: async () => {
    try {
      const schedule = await invoke<ScheduleSettings>('get_schedule_cmd');
      set({ schedule });
    } catch (e) {
      console.error('getSchedule failed:', e);
    }
  },

  updateSchedule: async (schedule) => {
    await invoke('update_schedule_cmd', { schedule });
    set({ schedule });
  },

  validateCron: async (expr) => {
    await invoke('validate_cron_cmd', { expr });
  },

  loadSchedulerLogs: async () => {
    const schedulerLogs = await invoke<SchedulerLog[]>('get_scheduler_logs_cmd', { limit: 100 });
    set({ schedulerLogs });
  },

  clearSchedulerLogs: async () => {
    await invoke('clear_scheduler_logs_cmd');
    set({ schedulerLogs: [] });
  },

  getSystemKeepaliveStatus: async () => {
    return await invoke<SystemKeepaliveStatus>('system_keepalive_status_cmd');
  },

  installSystemKeepalive: async (intervalMinutes) => {
    await invoke('install_system_keepalive_cmd', { intervalMinutes });
  },

  uninstallSystemKeepalive: async () => {
    await invoke('uninstall_system_keepalive_cmd');
  },

  exportRules: async (path) => {
    await invoke('export_rules_cmd', { path });
  },

  importRules: async (path, replace) => {
    const count = await invoke<number>('import_rules_cmd', { path, replace });
    await get().loadRules();
    return count;
  },

  exportConfig: async (path) => {
    await invoke('export_config_cmd', { path });
  },

  importConfig: async (path, replace) => {
    await invoke('import_config_cmd', { path, replace });
    await Promise.all([
      get().loadSettings(),
      get().loadFolders(),
      get().loadRules(),
      get().loadLogs(),
      get().loadStats(),
      get().getSchedule(),
    ]);
  },

  // ---- orden ----
  ordenList: async () => {
    return await invoke<string[]>('orden_list_cmd');
  },
  ordenLoad: async (name) => {
    return await invoke<string>('orden_load_cmd', { name });
  },
  ordenSave: async (name, yaml) => {
    await invoke('orden_save_cmd', { name, yaml });
  },
  ordenDelete: async (name) => {
    await invoke('orden_delete_cmd', { name });
  },
  ordenCheck: async (yaml) => {
    await invoke('orden_check_cmd', { yaml });
  },
  ordenRun: async (yaml, simulate, tags, skipTags) => {
    return await invoke<OrdenRunResult>('orden_run_cmd', { yaml, simulate, tags, skipTags });
  },
  ordenVisualFromYaml: async (yaml) => {
    return await invoke<OrdenVisualConfig>('orden_visual_from_yaml_cmd', { yaml });
  },
  ordenHistory: async (name, limit) => {
    return await invoke<OrdenRunHistory[]>('orden_history_cmd', { name, limit });
  },
  ordenJobs: async () => {
    return await invoke<OrdenJob[]>('orden_jobs_cmd');
  },
  ordenSaveJob: async (job) => {
    return await invoke<number>('orden_save_job_cmd', { job });
  },
  ordenDeleteJob: async (id) => {
    await invoke('orden_delete_job_cmd', { id });
  },
  ordenRunJob: async (job) => {
    return await invoke<OrdenRunResult>('orden_run_job_cmd', { job });
  },
  getMcpClientConfig: async () => {
    return await invoke<McpClientConfig>('mcp_client_config_cmd');
  },
  getOrdenQuickTasks: async () => {
    const names = await get().ordenList();
    const taskGroups = await Promise.all(
      names.map(async (configName) => {
        try {
          const yaml = await get().ordenLoad(configName);
          const visual = await get().ordenVisualFromYaml(yaml);
          return visual.rules.map((rule, index) => ({
            configName,
            ruleId: `${configName}:${rule.id || index}`,
            ruleName: rule.name || `Rule ${index + 1}`,
            enabled: rule.enabled,
            action: rule.action || 'run',
            location: rule.location,
            destination: rule.destination,
            tags: rule.tags,
            yaml: buildSingleOrdenRuleYaml(rule),
          }));
        } catch (e) {
          console.error('getOrdenQuickTasks failed for config:', configName, e);
          return [] as OrdenQuickTask[];
        }
      })
    );
    return taskGroups.flat();
  },
  runOrdenQuickTask: async (yaml, simulate) => {
    return await get().ordenRun(yaml, simulate, [], []);
  },
}));

function yamlQuote(value: string): string {
  const trimmed = value.trim();
  if (!trimmed) return '""';
  if (/^[A-Za-z0-9_./~:@{} -]+$/.test(trimmed) && !trimmed.includes('#')) {
    return trimmed;
  }
  return JSON.stringify(trimmed);
}

function csvList(value: string): string[] {
  return value.split(',').map((item) => item.trim()).filter(Boolean);
}

function pathList(value: string): string[] {
  const lines = value.split(/\r?\n/).map((item) => item.trim()).filter(Boolean);
  return lines.length > 0 ? lines : [];
}

function buildSingleOrdenRuleYaml(rule: OrdenVisualRule): string {
  const lines = ['rules:'];
  lines.push(`  - name: ${yamlQuote(rule.name || 'Quick task')}`);
  if (!rule.enabled) lines.push('    enabled: false');
  lines.push(`    targets: ${rule.targets || 'files'}`);
  const tags = csvList(rule.tags || '');
  if (tags.length > 0) {
    lines.push('    tags:');
    tags.forEach((tag) => lines.push(`      - ${yamlQuote(tag)}`));
  }
  lines.push('    locations:');
  const locations = pathList(rule.location || '~/Downloads');
  (locations.length > 0 ? locations : ['~/Downloads']).forEach((location) => {
    lines.push(`      - ${yamlQuote(location)}`);
  });
  lines.push(`    subfolders: ${rule.subfolders ? 'true' : 'false'}`);
  const extensions = csvList(rule.extensions || '');
  if (extensions.length > 0) {
    lines.push(`    filter_mode: ${rule.filterMode || 'all'}`);
    lines.push('    filters:');
    lines.push(`      - extension: [${extensions.map(yamlQuote).join(', ')}]`);
  }
  lines.push('    actions:');
  const action = rule.action || 'copy';
  const destinations = pathList(rule.destination || '~/Documents/Shelfy Backups/');
  if (action === 'copy') {
    const actionDestinations = destinations.length > 0 ? destinations : ['~/Documents/Shelfy Backups/'];
    if (actionDestinations.length === 1) {
      lines.push(`      - copy: ${yamlQuote(actionDestinations[0])}`);
    } else {
      lines.push('      - copy:');
      lines.push('          dest:');
      actionDestinations.forEach((destination) => lines.push(`            - ${yamlQuote(destination)}`));
      lines.push('          continue_with: original');
    }
  } else if (action === 'move') {
    lines.push(`      - move: ${yamlQuote(destinations[0] || '~/Documents/Shelfy Backups/')}`);
  } else if (action === 'rename') {
    lines.push(`      - rename: ${yamlQuote(rule.destination || '{name}')}`);
  } else if (['extract', 'compress'].includes(action)) {
    lines.push(`      - ${action}:`);
    lines.push(`          dest: ${yamlQuote(rule.destination || '~/Documents/Shelfy Archives/')}`);
    lines.push(`          format: ${yamlQuote(rule.archiveFormat || 'auto')}`);
    if (rule.archivePassword.trim()) lines.push(`          password: ${yamlQuote(rule.archivePassword)}`);
    const passwords = csvList(rule.archivePasswords || '');
    if (passwords.length > 0) {
      lines.push('          passwords:');
      passwords.forEach((password) => lines.push(`            - ${yamlQuote(password)}`));
    }
    if (rule.deleteOriginal) lines.push('          delete_original: true');
    if (rule.onConflict) lines.push(`          on_conflict: ${yamlQuote(rule.onConflict)}`);
  } else if (action === 'echo') {
    lines.push(`      - echo: ${yamlQuote(rule.destination || 'Matched {path}')}`);
  } else {
    lines.push(`      - ${action}: ${yamlQuote(rule.destination || '')}`);
  }
  return `${lines.join('\n')}\n`;
}
