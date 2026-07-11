import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  DEFAULT_ORDEN_EXAMPLE,
  McpClientConfig,
  OrdenVisualConfig,
  OrdenVisualRule,
  OrdenTemplate,
  OrdenRunResult,
  OrdenJob,
  useAppStore,
  Rule,
  ScheduleSettings,
} from "../store/useAppStore";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { save, open } from "@tauri-apps/plugin-dialog";
import { BrandMark } from "./BrandMark";
import { GeneralTab } from "./settings/GeneralTab";
import { IgnoreTab } from "./settings/IgnoreTab";
import { RulesTab } from "./settings/RulesTab";
import { OrdenPreview } from "./settings/OrdenPreview";
import { OrdenRunHistoryTable } from "./settings/OrdenRunHistoryTable";
import { OrdenVisualRuleCard } from "./settings/OrdenVisualRuleCard";
import { OrdenTemplateCenter } from "./settings/OrdenTemplateCenter";
import { TopNavButton } from "./settings/TopNavButton";
import {
  defaultMcpDraft,
  defaultOrdenVisualConfig,
  defaultSchedule,
  formatDuration,
  getDirectoryFromPath,
  GRACE_STEPS,
  GraceUnit,
  MAX_GRACE_SECONDS,
  McpDraft,
  mergePathText,
  nearestGraceStep,
  normalizeDialogSelection,
  OrdenEditorMode,
  OrdenView,
  secondsToUnit,
  unitToSeconds,
  visualToOrdenYaml,
  yamlQuote,
} from "./settings/utils";
import { Badge } from "./ui/badge";
import { Table, TableBody, TableCell, TableFooter, TableHead, TableHeader, TableRow } from "./ui/table";
import { Menu, MenuGroup, MenuGroupLabel, MenuItem, MenuPopup, MenuSeparator, MenuTrigger } from "./ui/menu";
import { Button } from "./ui/button";
import { Card } from "./ui/card";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./ui/select";
import { Switch } from "./ui/switch";
import { AnimatedIcon } from "./ui/animated-icon";
import { Tooltip, TooltipContent, TooltipTrigger } from "./ui/tooltip";
import {
  FolderOpen,
  List,
  History,
  Inbox,
  Globe,
  Plus,
  Trash2,
  Save,
  X,
  Check,
  ChevronLeft,
  RotateCcw,
  Code2,
  FileCheck2,
  Play,
  ScanSearch,
  Braces,
  Eye,
  ChevronDown,
  ChevronRight,
  MoreHorizontal,
  Pause,
  Pencil,
  Search,
  StickyNote,
  ShieldAlert,
  LayoutGrid,
} from "lucide-react";


type Tab = "rules" | "history" | "ignore" | "advanced" | "templates" | "general";
const SETTINGS_TABS: Tab[] = ["rules", "history", "ignore", "advanced", "templates", "general"];

function initialSettingsTab(): Tab {
  const query = window.location.hash.split("?", 2)[1] || "";
  const requested = new URLSearchParams(query).get("tab") as Tab | null;
  return requested && SETTINGS_TABS.includes(requested) ? requested : "rules";
}

export default function Settings() {
  const { t, i18n } = useTranslation();
  const {
    rules,
    folders,
    logs,
    loadRules,
    loadFolders,
    loadLogs,
    addFolder,
    removeFolder,
    updateFolderMode,
    addRule,
    updateRule,
    deleteRule,
    clearLogs,
    deleteHistoryLog,
    undoAction,
    undoAll,
    settings,
    saveSettings,
    setAutostart,
    schedule,
    schedulerLogs,
    getSchedule,
    updateSchedule,
    validateCron,
    loadSchedulerLogs,
    clearSchedulerLogs,
    getSystemKeepaliveStatus,
    validateFolderAccess,
    openFullDiskAccessSettings,
    installSystemKeepalive,
    uninstallSystemKeepalive,
    exportRules,
    importRules,
    exportConfig,
    importConfig,
    ordenList,
    ordenLoad,
    ordenSave,
    ordenDelete,
    ordenTemplateList,
    ordenTemplateSave,
    ordenTemplateDelete,
    ordenCheck,
    ordenRun,
    ordenVisualFromYaml,
    ordenHistory,
    ordenDeleteHistory,
    ordenClearHistory,
    ordenJobs,
    ordenSaveJob,
    ordenDeleteJob,
    ordenRunJob,
    getMcpClientConfig,
    loadStats,
  } = useAppStore();

  const [tab, setTab] = useState<Tab>(initialSettingsTab);
  const loadedTabs = useRef(new Set<Tab>());
  const [editingRule, setEditingRule] = useState<Rule | null>(null);
  const [newFolderPath, setNewFolderPath] = useState("");

  const [graceValue, setGraceValue] = useState(300);
  const [graceUnit, setGraceUnit] = useState<GraceUnit>("seconds");
  const [graceError, setGraceError] = useState<string | null>(null);

  const [localSchedule, setLocalSchedule] = useState<ScheduleSettings>(defaultSchedule());
  const [ruleToast, setRuleToast] = useState<{ message: string; type: "success" | "error" } | null>(null);
  const [replaceOnImport, setReplaceOnImport] = useState(false);
  const [configToast, setConfigToast] = useState<{ message: string; type: "success" | "error" } | null>(null);
  const [replaceConfigOnImport, setReplaceConfigOnImport] = useState(false);
  const [scheduleToast, setScheduleToast] = useState<{ message: string; type: "success" | "error" } | null>(null);
  const [systemKeepaliveSupported, setSystemKeepaliveSupported] = useState(false);
  const [historyFilter, setHistoryFilter] = useState("all");
  const [localMcp, setLocalMcp] = useState<McpDraft>(defaultMcpDraft());
  const [mcpClientConfig, setMcpClientConfig] = useState<McpClientConfig | null>(null);
  const [mcpToast, setMcpToast] = useState<{ message: string; type: "success" | "error" } | null>(null);
  const [ordenConfigs, setOrdenConfigs] = useState<string[]>([]);
  const [ordenTemplates, setOrdenTemplates] = useState<OrdenTemplate[]>([]);
  const [ordenName, setOrdenName] = useState("main");
  const [ordenYaml, setOrdenYaml] = useState(DEFAULT_ORDEN_EXAMPLE);
  const [ordenEditorMode, setOrdenEditorMode] = useState<OrdenEditorMode>("visual");
  const [ordenSourceExpanded, setOrdenSourceExpanded] = useState(false);
  const [ordenView, setOrdenView] = useState<OrdenView>("list");
  const [ordenPreviewError, setOrdenPreviewError] = useState<string | null>(null);
  const [ordenVisual, setOrdenVisual] = useState<OrdenVisualConfig>(defaultOrdenVisualConfig());
  const [ordenTags, setOrdenTags] = useState("");
  const [ordenSkipTags, setOrdenSkipTags] = useState("");
  const [ordenResult, setOrdenResult] = useState<OrdenRunResult | null>(null);
  const [ordenHistoryRows, setOrdenHistoryRows] = useState<import("../store/useAppStore").OrdenRunHistory[]>([]);
  const [ordenHistoryByConfig, setOrdenHistoryByConfig] = useState<Record<string, import("../store/useAppStore").OrdenRunHistory[]>>({});
  const [ordenSearch, setOrdenSearch] = useState("");
  const [ordenNotes, setOrdenNotes] = useState<Record<string, string>>(() => {
    try { return JSON.parse(localStorage.getItem("shelfy.orden.notes") || "{}"); } catch { return {}; }
  });
  const [ordenDetailName, setOrdenDetailName] = useState<string | null>(null);
  const [ordenJobsRows, setOrdenJobsRows] = useState<OrdenJob[]>([]);
  const [editingOrdenJob, setEditingOrdenJob] = useState<OrdenJob | null>(null);
  const [ordenBusy, setOrdenBusy] = useState(false);
  const [ordenToast, setOrdenToast] = useState<{ message: string; type: "success" | "error" } | null>(null);
  const [folderAccessError, setFolderAccessError] = useState<{ path: string; error: string; permission_denied: boolean } | null>(null);
  const isMacOS = /Macintosh|Mac OS X/i.test(navigator.userAgent);

  useEffect(() => {
    const unlisten = listen<string>("settings-navigate", (event) => {
      const nextTab = event.payload as Tab;
      if (SETTINGS_TABS.includes(nextTab)) setTab(nextTab);
    });
    return () => {
      unlisten.then((dispose) => dispose());
    };
  }, []);

  useEffect(() => {
    const unlisten = listen<{ path: string; error: string; permission_denied: boolean }>(
      "folder-access-error",
      (event) => setFolderAccessError(event.payload)
    );
    return () => {
      unlisten.then((dispose) => dispose());
    };
  }, []);

  useEffect(() => {
    if (!folderAccessError?.path) return;

    const refreshAccess = async () => {
      try {
        const access = await validateFolderAccess(folderAccessError.path);
        if (access.readable) setFolderAccessError(null);
      } catch {
        // Keep the existing error visible until the path can be verified again.
      }
    };

    window.addEventListener("focus", refreshAccess);
    return () => window.removeEventListener("focus", refreshAccess);
  }, [folderAccessError?.path, validateFolderAccess]);

  const loadOrdenConfigs = async (preferredName?: string) => {
    const names = await ordenList();
    setOrdenConfigs(names);
    Promise.all(names.map(async (name) => [name, await ordenHistory(name, 1)] as const))
      .then((rows) => setOrdenHistoryByConfig(Object.fromEntries(rows)))
      .catch(console.error);
    const nameToLoad = preferredName || (ordenName && names.includes(ordenName) ? ordenName : names[0]);
    if (nameToLoad) {
      setOrdenName(nameToLoad);
    } else {
      setOrdenName("main");
      setOrdenYaml(DEFAULT_ORDEN_EXAMPLE);
      setOrdenHistoryRows([]);
    }
  };

  useEffect(() => {
    if (loadedTabs.current.has(tab)) return;
    loadedTabs.current.add(tab);

    const loadTabData = async () => {
      if (tab === "rules") {
        await Promise.all([loadRules(), loadFolders()]);
      } else if (tab === "history") {
        await loadLogs();
      } else if (tab === "ignore") {
        await loadFolders();
      } else if (tab === "advanced") {
        const [, jobs] = await Promise.all([loadOrdenConfigs(), ordenJobs()]);
        setOrdenJobsRows(jobs);
      } else if (tab === "templates") {
        const [names, templates] = await Promise.all([ordenList(), ordenTemplateList()]);
        setOrdenConfigs(names);
        setOrdenTemplates(templates);
      } else if (tab === "general") {
        const [, , keepalive] = await Promise.all([
          getSchedule(),
          loadSchedulerLogs(),
          getSystemKeepaliveStatus().catch(() => ({ supported: false, platform: "unknown" })),
        ]);
        setSystemKeepaliveSupported(keepalive.supported);
      }
    };

    loadTabData().catch((error) => {
      loadedTabs.current.delete(tab);
      console.error(`Failed to load ${tab} settings:`, error);
    });
  }, [
    tab,
    loadRules,
    loadFolders,
    loadLogs,
    getSchedule,
    loadSchedulerLogs,
    getSystemKeepaliveStatus,
    ordenJobs,
  ]);

  const updateOrdenNote = (name: string, note: string) => {
    setOrdenNotes((previous) => {
      const next = { ...previous, [name]: note };
      localStorage.setItem("shelfy.orden.notes", JSON.stringify(next));
      return next;
    });
  };

  const ordenJobsByConfig = useMemo(() => {
    const grouped = new Map<string, OrdenJob[]>();
    ordenJobsRows.forEach((job) => {
      grouped.set(job.config_name, [...(grouped.get(job.config_name) || []), job]);
    });
    return grouped;
  }, [ordenJobsRows]);
  const configJobs = (name: string) => ordenJobsByConfig.get(name) || [];
  const setConfigJobsEnabled = async (name: string, enabled: boolean) => {
    const jobs = configJobs(name);
    await Promise.all(jobs.map((job) => ordenSaveJob({ ...job, enabled })));
    setOrdenJobsRows(await ordenJobs());
  };
  const filteredOrdenConfigs = useMemo(() => {
    const query = ordenSearch.trim().toLowerCase();
    if (!query) return ordenConfigs;
    return ordenConfigs.filter((name) =>
      `${name} ${ordenNotes[name] || ""}`.toLowerCase().includes(query)
    );
  }, [ordenConfigs, ordenNotes, ordenSearch]);

  // Sync local grace editor with loaded settings
  useEffect(() => {
    if (settings) {
      const clamped = Math.min(settings.grace_period_seconds, MAX_GRACE_SECONDS);
      const converted = secondsToUnit(clamped);
      setGraceValue(converted.value);
      setGraceUnit(converted.unit);
      setGraceError(null);
    }
  }, [settings?.grace_period_seconds]);

  // Sync local schedule editor with loaded schedule
  useEffect(() => {
    if (schedule) {
      setLocalSchedule(schedule);
    }
  }, [schedule]);

  useEffect(() => {
    if (settings) {
      setLocalMcp({
        mcp_enabled: settings.mcp_enabled,
        mcp_allow_write: settings.mcp_allow_write,
        mcp_transport: settings.mcp_transport || "stdio",
        mcp_server_name: settings.mcp_server_name || "shelfy",
        mcp_command: settings.mcp_command || null,
        mcp_args: settings.mcp_args || "--mcp",
        mcp_http_url: settings.mcp_http_url || "http://127.0.0.1:8765/mcp",
        mcp_token: settings.mcp_token || null,
      });
    }
  }, [
    settings?.mcp_enabled,
    settings?.mcp_allow_write,
    settings?.mcp_transport,
    settings?.mcp_server_name,
    settings?.mcp_command,
    settings?.mcp_args,
    settings?.mcp_http_url,
    settings?.mcp_token,
  ]);

  useEffect(() => {
    if (tab !== "general") return;
    getMcpClientConfig()
      .then(setMcpClientConfig)
      .catch(() => setMcpClientConfig(null));
  }, [
    getMcpClientConfig,
    tab,
    settings?.mcp_enabled,
    settings?.mcp_transport,
    settings?.mcp_server_name,
    settings?.mcp_command,
    settings?.mcp_args,
    settings?.mcp_http_url,
    settings?.mcp_token,
  ]);

  const handleAddFolder = async () => {
    if (!newFolderPath.trim()) return;
    await addFolder(newFolderPath.trim(), "silent");
    setNewFolderPath("");
  };

  const handleChooseFolder = async () => {
    try {
      const selected = await open({ directory: true, multiple: false });
      const path = Array.isArray(selected) ? selected[0] : selected;
      if (path) {
        const access = await validateFolderAccess(path);
        if (!access.readable) {
          setFolderAccessError({
            path,
            error: access.error || t("settings.permissions.folderDenied"),
            permission_denied: access.permission_denied,
          });
          return;
        }
        setFolderAccessError(null);
        setNewFolderPath(path);
      }
    } catch (error) {
      setFolderAccessError({
        path: "",
        error: String(error || t("settings.permissions.folderDenied")),
        permission_denied: true,
      });
    }
  };

  const handleChooseDestination = async () => {
    if (!editingRule) return;
    try {
      const selected = await open({ directory: true, multiple: false });
      const path = Array.isArray(selected) ? selected[0] : selected;
      if (path) setEditingRule({ ...editingRule, destination: path });
    } catch (error) {
      setFolderAccessError({ path: "", error: String(error), permission_denied: true });
    }
  };

  const handleChooseRuleScopeFolder = async () => {
    if (!editingRule) return;
    try {
      const selected = await open({ directory: true, multiple: false });
      const path = Array.isArray(selected) ? selected[0] : selected;
      if (path) setEditingRule({ ...editingRule, folder_id: 0, folder_path: path });
    } catch (error) {
      setFolderAccessError({ path: "", error: String(error), permission_denied: true });
    }
  };

  const handleSaveRule = async () => {
    if (!editingRule) return;
    const normalizedRule = {
      ...editingRule,
      name: editingRule.name.trim(),
      extensions: editingRule.extensions
        .map((extension) => extension.trim().replace(/^\./, "").toLowerCase())
        .filter(Boolean),
      pattern: editingRule.pattern?.trim() || null,
      destination: editingRule.destination.trim(),
      folder_path: editingRule.folder_path?.trim() ? editingRule.folder_path.trim() : null,
    };
    if (editingRule.id) {
      await updateRule(normalizedRule);
    } else {
      await addRule(normalizedRule);
    }
    setEditingRule(null);
  };

  const handleChangeLanguage = async (lang: string) => {
    if (!settings) return;
    await i18n.changeLanguage(lang);
    await saveSettings({ ...settings, language: lang });
  };

  const handleGraceSliderChange = (stepIndex: number) => {
    if (!settings) return;
    const seconds = GRACE_STEPS[stepIndex];
    const converted = secondsToUnit(seconds);
    setGraceValue(converted.value);
    setGraceUnit(converted.unit);
    setGraceError(null);
    saveSettings({ ...settings, grace_period_seconds: seconds });
  };

  const handleGraceNumberChange = (value: number, unit: GraceUnit) => {
    if (!settings) return;
    const seconds = unitToSeconds(value, unit);
    if (seconds > MAX_GRACE_SECONDS) {
      setGraceError(t("settings.general.gracePeriodMaxError"));
      setGraceValue(value);
      setGraceUnit(unit);
      return;
    }
    setGraceError(null);
    setGraceValue(value);
    setGraceUnit(unit);
    saveSettings({ ...settings, grace_period_seconds: Math.max(0, seconds) });
  };

  const handleScheduleChange = (patch: Partial<ScheduleSettings>) => {
    setLocalSchedule((prev) => {
      const next = { ...prev, ...patch };
      const times = Math.max(1, Math.min(4, next.schedule_times_per_day || 1));
      // Ensure required time slots have defaults when increasing count
      if (times >= 1 && !next.schedule_time_1) next.schedule_time_1 = "08:00";
      if (times >= 2 && !next.schedule_time_2) next.schedule_time_2 = "14:00";
      if (times >= 3 && !next.schedule_time_3) next.schedule_time_3 = "20:00";
      if (times >= 4 && !next.schedule_time_4) next.schedule_time_4 = "23:00";
      return {
        ...next,
        schedule_times_per_day: times,
        keepalive_interval_minutes: Math.max(1, Math.min(1440, next.keepalive_interval_minutes || 15)),
      };
    });
  };

  const showScheduleToast = (message: string, type: "success" | "error") => {
    setScheduleToast({ message, type });
    setTimeout(() => setScheduleToast(null), 3000);
  };

  const handleSaveSchedule = async () => {
    try {
      await updateSchedule(localSchedule);
      showScheduleToast(t("settings.scheduler.saveSuccess"), "success");
    } catch (e) {
      console.error("Failed to save schedule:", e);
      showScheduleToast(String(e || t("settings.scheduler.saveError")), "error");
    }
  };

  const handleValidateCron = async () => {
    try {
      await validateCron(localSchedule.schedule_cron_expr || "");
      showScheduleToast(t("settings.scheduler.cronValid"), "success");
    } catch (e) {
      console.error("Cron validation failed:", e);
      showScheduleToast(String(e || t("settings.scheduler.cronInvalid")), "error");
    }
  };

  const handleInstallSystemKeepalive = async () => {
    try {
      await installSystemKeepalive(localSchedule.keepalive_interval_minutes);
      await loadSchedulerLogs();
      showScheduleToast(t("settings.scheduler.keepaliveInstallSuccess"), "success");
    } catch (e) {
      console.error("Install system keepalive failed:", e);
      showScheduleToast(String(e || t("settings.scheduler.keepaliveInstallError")), "error");
    }
  };

  const handleUninstallSystemKeepalive = async () => {
    try {
      await uninstallSystemKeepalive();
      await loadSchedulerLogs();
      showScheduleToast(t("settings.scheduler.keepaliveUninstallSuccess"), "success");
    } catch (e) {
      console.error("Uninstall system keepalive failed:", e);
      showScheduleToast(String(e || t("settings.scheduler.keepaliveUninstallError")), "error");
    }
  };

  const showMcpToast = (message: string, type: "success" | "error") => {
    setMcpToast({ message, type });
    setTimeout(() => setMcpToast(null), 3000);
  };

  const handleSaveMcp = async () => {
    if (!settings) return;
    try {
      await saveSettings({
        ...settings,
        ...localMcp,
        mcp_transport: localMcp.mcp_transport === "http" ? "http" : "stdio",
        mcp_server_name: localMcp.mcp_server_name.trim() || "shelfy",
        mcp_command: localMcp.mcp_command?.trim() || null,
        mcp_args: localMcp.mcp_args?.trim() || "--mcp",
        mcp_http_url: localMcp.mcp_http_url?.trim() || null,
        mcp_token: localMcp.mcp_token?.trim() || null,
      });
      const config = await getMcpClientConfig();
      setMcpClientConfig(config);
      showMcpToast(t("settings.mcp.saveSuccess"), "success");
    } catch (e) {
      console.error("Save MCP settings failed:", e);
      showMcpToast(String(e || t("settings.mcp.saveError")), "error");
    }
  };

  const handleCopyMcpConfig = async () => {
    if (!mcpClientConfig) return;
    try {
      await navigator.clipboard.writeText(mcpClientConfig.config_json);
      showMcpToast(t("settings.mcp.copySuccess"), "success");
    } catch (e) {
      console.error("Copy MCP config failed:", e);
      showMcpToast(t("settings.mcp.copyError"), "error");
    }
  };

  const handleExportRules = async () => {
    try {
      const path = await save({
        filters: [{ name: "JSON", extensions: ["json"] }],
        defaultPath: "shelfy-rules.json",
      });
      if (path) {
        await exportRules(path);
        setRuleToast({ message: t("settings.rules.exportSuccess"), type: "success" });
      }
    } catch (e) {
      console.error("Export rules failed:", e);
      setRuleToast({ message: t("settings.rules.exportError"), type: "error" });
    }
    setTimeout(() => setRuleToast(null), 3000);
  };

  const handleImportRules = async () => {
    try {
      const selected = await open({
        filters: [{ name: "JSON", extensions: ["json"] }],
        multiple: false,
      });
      const path = Array.isArray(selected) ? selected[0] : selected;
      if (path) {
        const count = await importRules(path, replaceOnImport);
        setRuleToast({
          message: t("settings.rules.importSuccess", { count }),
          type: "success",
        });
      }
    } catch (e) {
      console.error("Import rules failed:", e);
      setRuleToast({ message: t("settings.rules.importError"), type: "error" });
    }
    setTimeout(() => setRuleToast(null), 3000);
  };

  const handleExportConfig = async () => {
    try {
      const path = await save({
        filters: [{ name: "JSON", extensions: ["json"] }],
        defaultPath: "shelfy-config.json",
      });
      if (path) {
        await exportConfig(path);
        setConfigToast({ message: t("settings.config.exportSuccess"), type: "success" });
      }
    } catch (e) {
      console.error("Export config failed:", e);
      setConfigToast({ message: t("settings.config.exportError"), type: "error" });
    }
    setTimeout(() => setConfigToast(null), 3000);
  };

  const handleImportConfig = async () => {
    try {
      const selected = await open({
        filters: [{ name: "JSON", extensions: ["json"] }],
        multiple: false,
      });
      const path = Array.isArray(selected) ? selected[0] : selected;
      if (path) {
        await importConfig(path, replaceConfigOnImport);
        setConfigToast({ message: t("settings.config.importSuccess"), type: "success" });
      }
    } catch (e) {
      console.error("Import config failed:", e);
      setConfigToast({ message: t("settings.config.importError"), type: "error" });
    }
    setTimeout(() => setConfigToast(null), 3000);
  };

  const parseTagList = (value: string) =>
    value
      .split(",")
      .map((tag) => tag.trim())
      .filter(Boolean);

  const showOrdenToast = (message: string, type: "success" | "error") => {
    setOrdenToast({ message, type });
    setTimeout(() => setOrdenToast(null), 3000);
  };

  const currentOrdenYaml = () =>
    ordenEditorMode === "visual" ? visualToOrdenYaml(ordenVisual) : ordenYaml;

  const parseOrdenVisual = async (yamlText: string) => {
    const parsed = await ordenVisualFromYaml(yamlText);
    setOrdenVisual(parsed.rules.length > 0 ? parsed : defaultOrdenVisualConfig());
  };

  const handleOrdenEditorModeChange = async (mode: OrdenEditorMode) => {
    if (mode === ordenEditorMode) return;
    if (mode === "visual") {
      try {
        await parseOrdenVisual(ordenYaml);
        setOrdenEditorMode("visual");
      } catch (e) {
        console.error("Visual parse failed:", e);
        showOrdenToast(String(e || t("settings.orden.visualParseError")), "error");
      }
      return;
    }

    const yamlText = visualToOrdenYaml(ordenVisual);
    setOrdenYaml(yamlText);
    setOrdenEditorMode("source");
  };

  const updateOrdenVisualRule = (id: string, patch: Partial<OrdenVisualRule>) => {
    setOrdenVisual((prev) => {
      const next = {
        rules: prev.rules.map((rule) => (rule.id === id ? { ...rule, ...patch } : rule)),
      };
      setOrdenYaml(visualToOrdenYaml(next));
      return next;
    });
  };

  const handleAddOrdenVisualRule = () => {
    setOrdenVisual((prev) => {
      const next = {
        rules: [
          ...prev.rules,
          {
            ...defaultOrdenVisualConfig().rules[0],
            id: `rule-${Date.now()}`,
            name: `Rule ${prev.rules.length + 1}`,
          },
        ],
      };
      setOrdenYaml(visualToOrdenYaml(next));
      return next;
    });
  };

  const handleRemoveOrdenVisualRule = (id: string) => {
    setOrdenVisual((prev) => {
      const next = { rules: prev.rules.filter((rule) => rule.id !== id) };
      setOrdenYaml(visualToOrdenYaml(next));
      return next;
    });
  };

  const handleChooseOrdenLocations = async (id: string, directory: boolean) => {
    try {
      const selected = await open({ directory, multiple: true });
      const paths = normalizeDialogSelection(selected);
      if (paths.length === 0) return;
      if (directory) {
        const checks = await Promise.all(paths.map(validateFolderAccess));
        const denied = checks.find((check) => !check.readable);
        if (denied) {
          setFolderAccessError({
            path: denied.path,
            error: denied.error || t("settings.permissions.folderDenied"),
            permission_denied: denied.permission_denied,
          });
          return;
        }
      }
      setFolderAccessError(null);
      const rule = ordenVisual.rules.find((item) => item.id === id);
      updateOrdenVisualRule(id, {
        location: mergePathText(rule?.location || "", paths),
      });
    } catch (error) {
      setFolderAccessError({ path: "", error: String(error), permission_denied: true });
    }
  };

  const handleChooseOrdenDestinations = async (id: string, stepId: string) => {
    try {
      const selected = await open({ directory: true, multiple: true });
      const paths = normalizeDialogSelection(selected);
      if (paths.length === 0) return;
      const checks = await Promise.all(paths.map(validateFolderAccess));
      const denied = checks.find((check) => !check.readable);
      if (denied) {
        setFolderAccessError({
          path: denied.path,
          error: denied.error || t("settings.permissions.folderDenied"),
          permission_denied: denied.permission_denied,
        });
        return;
      }
      setFolderAccessError(null);
      const rule = ordenVisual.rules.find((item) => item.id === id);
      const actionSteps = (rule?.actionSteps || []).map((step) => {
        if (step.id !== stepId) return step;
        const value = step.kind === "copy" && paths.length > 1
          ? `dest:\n${paths.map((path) => `  - ${yamlQuote(path)}`).join("\n")}\ncontinue_with: original`
          : yamlQuote(paths[0]);
        return { ...step, value };
      });
      updateOrdenVisualRule(id, {
        destination: mergePathText(rule?.destination || "", paths),
        actionSteps,
      });
    } catch (error) {
      setFolderAccessError({ path: "", error: String(error), permission_denied: true });
    }
  };

  const handleOrdenSelect = async (name: string) => {
    try {
      const [yaml, history] = await Promise.all([ordenLoad(name), ordenHistory(name, 100)]);
      setOrdenName(name);
      setOrdenYaml(yaml);
      setOrdenHistoryRows(history);
      setOrdenHistoryByConfig((previous) => ({ ...previous, [name]: history }));
      if (ordenEditorMode === "visual") {
        await parseOrdenVisual(yaml);
      }
      setOrdenResult(null);
      setOrdenPreviewError(null);
      setOrdenView("editor");
    } catch (e) {
      console.error("Load orden config failed:", e);
      showOrdenToast(t("settings.orden.loadError"), "error");
    }
  };

  const handleOrdenPreview = async (name: string) => {
    try {
      const [yaml, history] = await Promise.all([ordenLoad(name), ordenHistory(name, 20)]);
      const visual = await ordenVisualFromYaml(yaml);
      setOrdenName(name);
      setOrdenYaml(yaml);
      setOrdenVisual(visual.rules.length > 0 ? visual : defaultOrdenVisualConfig());
      setOrdenHistoryRows(history);
      setOrdenHistoryByConfig((previous) => ({ ...previous, [name]: history }));
      setOrdenDetailName(name);
      setOrdenView("detail");
    } catch (e) {
      console.error("Preview orden config failed:", e);
      showOrdenToast(t("settings.orden.loadError"), "error");
    }
  };

  const handleOrdenSave = async () => {
    const name = ordenName.trim();
    if (!name) {
      showOrdenToast(t("settings.orden.nameRequired"), "error");
      return;
    }
    try {
      const yaml = currentOrdenYaml();
      await ordenCheck(yaml);
      await ordenSave(name, yaml);
      setOrdenYaml(yaml);
      await loadOrdenConfigs(name.replace(/\.ya?ml$/i, ""));
      showOrdenToast(t("settings.orden.saveSuccess"), "success");
    } catch (e) {
      console.error("Save orden config failed:", e);
      showOrdenToast(String(e || t("settings.orden.saveError")), "error");
    }
  };

  const handleUseOrdenTemplate = async (template: OrdenTemplate) => {
    const sourceName = template.id.replace(/^custom-/, "template-").replace(/[^a-zA-Z0-9_-]/g, "-");
    const baseName = sourceName || `template-${Date.now()}`;
    let configName = baseName;
    let suffix = 2;
    while (ordenConfigs.includes(configName)) {
      configName = `${baseName}-${suffix}`;
      suffix += 1;
    }
    await ordenCheck(template.yaml);
    await ordenSave(configName, template.yaml);
    const visual = await ordenVisualFromYaml(template.yaml);
    setOrdenName(configName);
    setOrdenYaml(template.yaml);
    setOrdenVisual(visual.rules.length > 0 ? visual : defaultOrdenVisualConfig());
    setOrdenEditorMode("visual");
    setOrdenView("editor");
    await loadOrdenConfigs(configName);
    setTab("advanced");
    setOrdenToast({ message: t("settings.orden.templates.addedNotice"), type: "success" });
    setTimeout(() => setOrdenToast(null), 3000);
  };

  const handleSaveOrdenTemplate = async (name: string, yaml: string) => {
    await ordenCheck(yaml);
    await ordenTemplateSave(name, yaml);
    setOrdenTemplates(await ordenTemplateList());
  };

  const handleDeleteOrdenTemplate = async (template: OrdenTemplate) => {
    await ordenTemplateDelete(template.name);
    setOrdenTemplates(await ordenTemplateList());
  };

  const handleOrdenDelete = async () => {
    if (!ordenConfigs.includes(ordenName)) {
      setOrdenName("main");
      setOrdenYaml(DEFAULT_ORDEN_EXAMPLE);
      setOrdenResult(null);
      setOrdenPreviewError(null);
      setOrdenView("editor");
      return;
    }
    if (!window.confirm(t("settings.orden.deleteConfirm", { name: ordenName }))) return;
    try {
      await ordenDelete(ordenName);
      await loadOrdenConfigs();
      setOrdenResult(null);
      setOrdenPreviewError(null);
      setOrdenView("editor");
      showOrdenToast(t("settings.orden.deleteSuccess"), "success");
    } catch (e) {
      console.error("Delete orden config failed:", e);
      showOrdenToast(t("settings.orden.deleteError"), "error");
    }
  };

  const handleOrdenCheck = async () => {
    try {
      const yaml = currentOrdenYaml();
      await ordenCheck(yaml);
      setOrdenYaml(yaml);
      showOrdenToast(t("settings.orden.checkSuccess"), "success");
    } catch (e) {
      console.error("Check orden config failed:", e);
      showOrdenToast(String(e || t("settings.orden.checkError")), "error");
    }
  };

  const newOrdenJob = (configName = ordenName || ordenConfigs[0] || "main"): OrdenJob => ({
    name: `${configName}-task`,
    config_name: configName,
    enabled: true,
    mode: "manual",
    cron_expr: "0 * * * *",
    fixed_time: "08:00",
    interval_minutes: 60,
    watch_paths: "~/Downloads",
    tags: ordenTags,
    skip_tags: ordenSkipTags,
    simulate: false,
    min_file_count: 0,
    path_exists: null,
    time_window_start: null,
    time_window_end: null,
    last_run_at: null,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  });

  const handleNewOrdenConfig = () => {
    setOrdenName("main");
    setOrdenYaml(DEFAULT_ORDEN_EXAMPLE);
    setOrdenVisual(defaultOrdenVisualConfig());
    setOrdenEditorMode("visual");
    setOrdenResult(null);
    setOrdenPreviewError(null);
    setOrdenView("editor");
  };

  const handleNewOrdenJob = (configName?: string) => {
    setEditingOrdenJob(newOrdenJob(configName));
  };

  const handleSaveOrdenJob = async () => {
    if (!editingOrdenJob) return;
    try {
      await ordenSaveJob(editingOrdenJob);
      setOrdenJobsRows(await ordenJobs());
      setEditingOrdenJob(null);
      showOrdenToast(t("settings.orden.jobSaved"), "success");
    } catch (e) {
      showOrdenToast(String(e || t("settings.orden.jobSaveError")), "error");
    }
  };

  const handleDeleteOrdenJob = async (job: OrdenJob) => {
    if (!job.id) return;
    await ordenDeleteJob(job.id);
    setOrdenJobsRows(await ordenJobs());
  };

  const handleRunOrdenJob = async (job: OrdenJob) => {
    setOrdenBusy(true);
    try {
      const result = await ordenRunJob(job);
      setOrdenResult(result);
      setOrdenView("preview");
      setOrdenJobsRows(await ordenJobs());
      await refreshOrdenHistory(job.config_name);
    } catch (error) {
      setOrdenResult(null);
      setOrdenPreviewError(String(error || t("settings.orden.runError")));
      setOrdenView("preview");
      await refreshOrdenHistory(job.config_name).catch(console.error);
    } finally {
      setOrdenBusy(false);
    }
  };

  const refreshOrdenHistory = async (name: string) => {
    const history = await ordenHistory(name, 100);
    setOrdenHistoryRows(history);
    setOrdenHistoryByConfig((previous) => ({ ...previous, [name]: history }));
  };

  const handleDeleteOrdenHistory = async (name: string, id: number) => {
    await ordenDeleteHistory(id);
    await refreshOrdenHistory(name);
  };

  const handleClearOrdenHistory = async (name: string) => {
    await ordenClearHistory(name);
    setOrdenHistoryRows([]);
    setOrdenHistoryByConfig((previous) => ({ ...previous, [name]: [] }));
  };

  const runOrdenConfigByName = async (name: string, simulate: boolean) => {
    setOrdenBusy(true);
    try {
      const yaml = await ordenLoad(name);
      setOrdenName(name);
      setOrdenYaml(yaml);
      const result = await ordenRun(yaml, simulate, parseTagList(ordenTags), parseTagList(ordenSkipTags));
      setOrdenResult(result);
      const history = await ordenHistory(name, 20);
      setOrdenHistoryRows(history);
      setOrdenHistoryByConfig((previous) => ({ ...previous, [name]: history }));
      setOrdenView("preview");
      if (!simulate) await Promise.all([loadLogs(), loadStats()]);
    } catch (error) {
      setOrdenResult(null);
      setOrdenPreviewError(String(error || t("settings.orden.runError")));
      setOrdenView("preview");
      await refreshOrdenHistory(name).catch(console.error);
    } finally {
      setOrdenBusy(false);
    }
  };

  const deleteOrdenConfigByName = async (name: string) => {
    if (!window.confirm(t("settings.orden.deleteConfirm", { name }))) return;
    setOrdenBusy(true);
    try {
      await ordenDelete(name);
      await loadOrdenConfigs();
      showOrdenToast(t("settings.orden.deleteSuccess"), "success");
    } catch {
      showOrdenToast(t("settings.orden.deleteError"), "error");
    } finally {
      setOrdenBusy(false);
    }
  };

  const handleOrdenRun = async (simulate: boolean) => {
    setOrdenBusy(true);
    try {
      const yaml = currentOrdenYaml();
      setOrdenYaml(yaml);
      setOrdenPreviewError(null);
      const result = await ordenRun(
        yaml,
        simulate,
        parseTagList(ordenTags),
        parseTagList(ordenSkipTags)
      );
      setOrdenResult(result);
      ordenHistory(ordenName, 20).then(setOrdenHistoryRows).catch(console.error);
      setOrdenView("preview");
      if (!simulate) {
        await Promise.all([loadLogs(), loadStats()]);
      }
      showOrdenToast(
        simulate ? t("settings.orden.simulateSuccess") : t("settings.orden.runSuccess"),
        result.errors > 0 ? "error" : "success"
      );
    } catch (e) {
      console.error("Run orden config failed:", e);
      setOrdenResult(null);
      setOrdenPreviewError(String(e || t("settings.orden.runError")));
      setOrdenView("preview");
      await refreshOrdenHistory(ordenName).catch(console.error);
      showOrdenToast(String(e || t("settings.orden.runError")), "error");
    } finally {
      setOrdenBusy(false);
    }
  };

  const currentGraceSeconds = useMemo(
    () => unitToSeconds(graceValue, graceUnit),
    [graceValue, graceUnit]
  );

  const sliderIndex = useMemo(() => {
    const clamped = Math.min(currentGraceSeconds, MAX_GRACE_SECONDS);
    const nearest = nearestGraceStep(clamped);
    return GRACE_STEPS.indexOf(nearest);
  }, [currentGraceSeconds]);

  const historyFilterOptions = useMemo(() => {
    const labels = Array.from(
      new Set(logs.map((log) => log.rule_label).filter((value): value is string => Boolean(value?.trim())))
    ).sort((a, b) => a.localeCompare(b));
    return labels;
  }, [logs]);

  const filteredHistoryLogs = useMemo(() => {
    if (historyFilter === "all") return logs;
    if (historyFilter === "engine:rules") return logs.filter((log) => log.engine === "rules");
    if (historyFilter === "engine:orden") return logs.filter((log) => log.engine === "orden");
    if (historyFilter.startsWith("label:")) {
      const label = historyFilter.slice("label:".length);
      return logs.filter((log) => log.rule_label === label);
    }
    return logs;
  }, [logs, historyFilter]);

  const openHistoryFilter = (filter: string) => {
    setHistoryFilter(filter);
    setTab("history");
  };

  return (
    <div className="relative flex h-full flex-col overflow-hidden rounded-xl bg-background/88 text-foreground">
      <div aria-hidden="true" className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_12%_0%,color-mix(in_srgb,var(--primary)_12%,transparent),transparent_34%),radial-gradient(circle_at_88%_4%,color-mix(in_srgb,var(--accent)_42%,transparent),transparent_30%)]" />

      <header className="relative z-20 shrink-0 px-4 pt-3">
        <div data-tauri-drag-region className="flex h-12 items-center gap-2 pl-20">
          <div className="glass-panel flex h-11 shrink-0 items-center rounded-2xl px-3">
            <BrandMark showLabel iconClassName="size-7 rounded-md" />
          </div>
          <nav className="glass-panel no-scrollbar mx-auto flex h-11 min-w-0 items-center gap-1 overflow-x-auto rounded-2xl p-1">
          <TopNavButton
            active={tab === "rules"}
            onClick={() => setTab("rules")}
            icon={<List size={16} />}
            label={t("settings.rules.title")}
          />
          <TopNavButton
            active={tab === "advanced"}
            onClick={() => setTab("advanced")}
            icon={<Code2 size={16} />}
            label={t("settings.orden.title")}
          />
          <TopNavButton
            active={tab === "templates"}
            onClick={() => setTab("templates")}
            icon={<LayoutGrid size={16} />}
            label={t("settings.orden.templates.tab")}
          />
          <TopNavButton
            active={tab === "history"}
            onClick={() => setTab("history")}
            icon={<History size={16} />}
            label={t("settings.history.title")}
          />
          <TopNavButton
            active={tab === "ignore"}
            onClick={() => setTab("ignore")}
            icon={<X size={16} />}
            label={t("settings.ignore.title")}
          />
          <TopNavButton
            active={tab === "general"}
            onClick={() => setTab("general")}
            icon={<Globe size={16} />}
            label={t("settings.general.title")}
          />
          </nav>
          <Button
            type="button"
            onClick={() => invoke("close_settings")}
            variant="ghost"
            size="icon"
            className="glass-panel size-10 shrink-0 rounded-2xl text-muted-foreground"
            aria-label={t("settings.close")}
          >
            <X size={15} />
          </Button>
        </div>
      </header>

      {/* Content */}
      <main className="relative z-10 min-w-0 flex-1 overflow-y-auto overscroll-contain px-4 pb-4 pt-3 [scrollbar-gutter:stable]">
        <div className="mx-auto w-full max-w-[1280px]">
        {isMacOS && folderAccessError && (tab === "rules" || tab === "advanced") && (
          <div className="mb-3 flex items-center justify-between gap-3 rounded-xl border border-border bg-card/75 px-3 py-2 shadow-sm">
            <div className="flex min-w-0 items-center gap-2.5">
              <ShieldAlert size={16} className={folderAccessError ? "shrink-0 text-destructive" : "shrink-0 text-muted-foreground"} />
              <div className="min-w-0">
                <div className="text-xs font-medium">
                  {folderAccessError ? t("settings.permissions.accessError") : t("settings.permissions.title")}
                </div>
                <div className="truncate text-xs text-muted-foreground" title={folderAccessError?.error}>
                  {folderAccessError
                    ? `${folderAccessError.path ? `${folderAccessError.path} · ` : ""}${folderAccessError.error}`
                    : t("settings.permissions.desc")}
                </div>
              </div>
            </div>
            <Button type="button" onClick={() => openFullDiskAccessSettings()} variant="outline" size="sm" className="shrink-0">
              {t("settings.permissions.openFullDiskAccess")}
            </Button>
          </div>
        )}
        {tab === "rules" && (
          <RulesTab
            rules={rules}
            folders={folders}
            editingRule={editingRule}
            setEditingRule={setEditingRule}
            newFolderPath={newFolderPath}
            setNewFolderPath={setNewFolderPath}
            replaceOnImport={replaceOnImport}
            setReplaceOnImport={setReplaceOnImport}
            ruleToast={ruleToast}
            handleChooseFolder={handleChooseFolder}
            handleAddFolder={handleAddFolder}
            updateFolderMode={updateFolderMode}
            removeFolder={removeFolder}
            handleExportRules={handleExportRules}
            handleImportRules={handleImportRules}
            handleChooseDestination={handleChooseDestination}
            handleChooseRuleScopeFolder={handleChooseRuleScopeFolder}
            handleSaveRule={handleSaveRule}
            updateRule={updateRule}
            deleteRule={deleteRule}
            handleViewHistory={(ruleLabel) => openHistoryFilter(`label:${ruleLabel}`)}
          />
        )}

        {tab === "history" && (
          <div className="flex flex-col gap-4">
            <div className="flex items-center justify-between">
              <h2 className="text-lg font-semibold">{t("settings.history.title")}</h2>
              <div className="flex items-center gap-2">
                <Select value={historyFilter} onValueChange={setHistoryFilter}>
                  <SelectTrigger className="w-[220px]">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">{t("settings.history.filterAll")}</SelectItem>
                    <SelectItem value="engine:rules">{t("settings.history.filterRules")}</SelectItem>
                    <SelectItem value="engine:orden">{t("settings.history.filterOrden")}</SelectItem>
                    {historyFilterOptions.map((label) => (
                      <SelectItem key={label} value={`label:${label}`}>
                        {label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <span>
                      <Button
                        onClick={async () => { await undoAll(); }}
                        disabled={logs.length === 0 || logs.every((log) => log.undone)}
                        variant="outline"
                      >
                        <AnimatedIcon icon={RotateCcw} size={14} motion="tilt" />
                        {t("settings.history.revertAll")}
                      </Button>
                    </span>
                  </TooltipTrigger>
                  {(logs.length === 0 || logs.every((log) => log.undone)) && (
                    <TooltipContent>{t("settings.history.revertAllDisabled")}</TooltipContent>
                  )}
                </Tooltip>
                <Button
                  onClick={() => {
                    if (window.confirm(t("settings.history.clearConfirm"))) void clearLogs();
                  }}
                  variant="destructive"
                >
                  <Trash2 size={14} />
                  {t("settings.history.clear")}
                </Button>
              </div>
            </div>
            {filteredHistoryLogs.length === 0 ? (
              <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
                <Inbox size={48} className="mb-3 opacity-50" />
                <span>{t("settings.history.empty")}</span>
              </div>
            ) : (
              <div className="flex flex-col gap-2">
                {filteredHistoryLogs.map((log) => (
                  <Card
                    key={log.id}
                    className={`grid gap-3 px-4 py-3 md:grid-cols-[minmax(0,1fr)_minmax(15rem,0.85fr)_auto] md:items-center ${
                      log.undone ? "bg-muted opacity-70" : ""
                    }`}
                  >
                    <div className="min-w-0 flex-1">
                      <div className="flex flex-wrap items-center gap-2">
                        <div className="truncate text-sm font-medium">{log.file_name}</div>
                        <Badge variant="outline">{log.engine}</Badge>
                        <Badge variant="secondary">{log.action}</Badge>
                      </div>
                      <div className="mt-1 text-xs text-muted-foreground">
                        {new Date(log.timestamp).toLocaleString()} · {log.rule_label || log.file_type}
                      </div>
                    </div>
                    <div className="min-w-0 text-xs">
                      <div className="break-all text-muted-foreground" title={log.source_path}>
                        {t("settings.history.source")}: {log.source_path}
                      </div>
                      <div className="mt-1 break-all" title={log.destination_path || undefined}>
                        {t("settings.history.destination")}: {log.destination_path || "—"}
                      </div>
                    </div>
                    <div className="flex shrink-0 items-center gap-2">
                      {log.destination_path && (
                        <Tooltip>
                          <TooltipTrigger asChild>
                            <Button
                              onClick={async () => {
                                const folderPath = getDirectoryFromPath(log.destination_path);
                                if (folderPath) {
                                  try {
                                    await invoke("open_folder_cmd", { path: folderPath });
                                  } catch (e) {
                                    console.error("Failed to open folder:", e);
                                  }
                                }
                              }}
                              variant="ghost"
                              size="icon"
                              className="text-muted-foreground"
                              aria-label={t("popup.openActionFolder")}
                            >
                              <FolderOpen size={16} />
                            </Button>
                          </TooltipTrigger>
                          <TooltipContent>{t("popup.openActionFolder")}</TooltipContent>
                        </Tooltip>
                      )}
                      {log.undone ? (
                        <span className="flex items-center gap-1 text-xs text-muted-foreground">
                          <Check size={12} />
                          {t("settings.history.undone")}
                        </span>
                      ) : (
                        <Button
                          onClick={() => log.id && undoAction(log.id)}
                          variant="link"
                          className="h-auto px-0 text-xs"
                        >
                          {t("settings.history.undo")}
                        </Button>
                      )}
                      {log.id != null && (
                        <Button
                          type="button"
                          onClick={() => {
                            if (window.confirm(t("settings.history.deleteConfirm"))) void deleteHistoryLog(log.id!);
                          }}
                          variant="ghost"
                          size="icon"
                          className="size-8 text-destructive hover:bg-destructive/10 hover:text-destructive"
                          aria-label={t("settings.history.delete")}
                        >
                          <Trash2 size={14} />
                        </Button>
                      )}
                    </div>
                  </Card>
                ))}
              </div>
            )}
          </div>
        )}

        {tab === "advanced" && (
          <div className="space-y-4">
            {ordenView === "preview" ? (
              <OrdenPreview
                ordenResult={ordenResult}
                ordenPreviewError={ordenPreviewError}
                onBack={() => setOrdenView("list")}
              />
            ) : ordenView === "detail" && ordenDetailName ? (() => {
              const history = ordenHistoryByConfig[ordenDetailName] || [];
              const jobs = configJobs(ordenDetailName);
              return <div className="space-y-4">
                <div className="flex items-center justify-between gap-3">
                  <div className="flex items-center gap-3">
                    <Button type="button" onClick={() => setOrdenView("list")} variant="ghost" size="icon" aria-label={t("settings.orden.backToList")}><ChevronLeft size={17} /></Button>
                    <div><h2 className="text-lg font-semibold">{ordenDetailName}</h2><p className="text-xs text-muted-foreground">{t("settings.orden.detailDesc")}</p></div>
                  </div>
                  <div className="flex items-center gap-2">
                    <Button type="button" onClick={() => runOrdenConfigByName(ordenDetailName, true)} variant="outline" disabled={ordenBusy}><ScanSearch size={14} />{t("settings.orden.tryRun")}</Button>
                    <Button type="button" onClick={() => handleOrdenSelect(ordenDetailName)}><Pencil size={14} />{t("settings.orden.edit")}</Button>
                  </div>
                </div>
                <div className="grid gap-3 md:grid-cols-3">
                  <Card className="p-4"><div className="text-xs text-muted-foreground">{t("settings.orden.runs")}</div><div className="mt-1 text-2xl font-semibold">{history.length}</div></Card>
                  <Card className="p-4"><div className="text-xs text-muted-foreground">{t("settings.orden.schedules")}</div><div className="mt-1 text-2xl font-semibold">{jobs.length}</div></Card>
                  <Card className="p-4"><div className="text-xs text-muted-foreground">{t("settings.orden.lastResult")}</div><div className="mt-1 text-sm font-medium">{history[0] ? `${history[0].success} / ${history[0].errors}` : "—"}</div></Card>
                </div>
                <Card className="space-y-3 p-4">
                  <div><h3 className="font-medium">{t("settings.orden.previewRules")}</h3><p className="text-xs text-muted-foreground">{t("settings.orden.previewRulesDesc")}</p></div>
                  <Table>
                    <TableHeader><TableRow><TableHead>{t("settings.orden.rule")}</TableHead><TableHead>{t("settings.orden.locations")}</TableHead><TableHead>{t("settings.orden.filter")}</TableHead><TableHead>{t("settings.orden.action")}</TableHead><TableHead>{t("settings.orden.destination")}</TableHead></TableRow></TableHeader>
                    <TableBody>
                      {ordenVisual.rules.map((rule) => <TableRow key={rule.id}>
                        <TableCell><div className="font-medium">{rule.name}</div><div className="text-xs text-muted-foreground">{rule.enabled ? t("settings.orden.enabled") : t("settings.orden.stopped")}</div></TableCell>
                        <TableCell className="max-w-48 truncate text-xs text-muted-foreground" title={rule.location}>{rule.location || "—"}</TableCell>
                        <TableCell className="text-xs">{rule.filterSteps?.map((step) => `${step.inverted ? "not " : ""}${step.kind}`).join(", ") || rule.extensions || t("settings.orden.noFilter")} · {rule.filterMode || "all"}</TableCell>
                        <TableCell><div className="flex flex-wrap gap-1">{(rule.actionSteps?.length ? rule.actionSteps : [{ kind: rule.action }]).map((step, index) => <Badge key={`${step.kind}-${index}`} variant="outline">{step.kind}</Badge>)}</div></TableCell>
                        <TableCell className="max-w-48 truncate text-xs text-muted-foreground" title={rule.actionSteps?.map((step) => step.value).join("\n") || rule.destination}>{rule.actionSteps?.[0]?.value || rule.destination || "—"}</TableCell>
                      </TableRow>)}
                    </TableBody>
                  </Table>
                </Card>
                <Card className="space-y-2 p-4"><Label>{t("settings.orden.note")}</Label><textarea value={ordenNotes[ordenDetailName] || ""} onChange={(event) => updateOrdenNote(ordenDetailName, event.target.value)} placeholder={t("settings.orden.notePlaceholder")} className="min-h-24 w-full resize-y rounded-lg border border-border bg-background px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-ring/20" /></Card>
                <Card className="p-4">
                  <OrdenRunHistoryTable
                    rows={history}
                    onRefresh={() => refreshOrdenHistory(ordenDetailName)}
                    onDelete={(id) => handleDeleteOrdenHistory(ordenDetailName, id)}
                    onClear={() => handleClearOrdenHistory(ordenDetailName)}
                  />
                </Card>
              </div>;
            })() : (
              <>

            <div className="flex items-center justify-between gap-3">
              <div className="flex min-w-0 items-center gap-3">
                {ordenView === "editor" && (
                  <Button onClick={() => setOrdenView("list")} variant="ghost" size="icon" aria-label={t("settings.orden.backToList")}>
                    <ChevronLeft size={17} />
                  </Button>
                )}
                <div className="min-w-0">
                  <h2 className="truncate text-lg font-semibold">
                    {ordenView === "list" ? t("settings.orden.title") : ordenName}
                  </h2>
                  <p className="text-xs text-muted-foreground">
                    {ordenView === "list" ? t("settings.orden.centerDesc") : t("settings.orden.editorDesc")}
                  </p>
                </div>
              </div>
              <div className="flex items-center gap-2">
                {ordenView === "list" ? (
                  <>
                    <Button onClick={() => openHistoryFilter("engine:orden")} variant="outline">
                      <History size={14} />
                      {t("settings.history.title")}
                    </Button>
                    <Button onClick={handleNewOrdenConfig}>
                      <Plus size={14} />
                      {t("settings.orden.newConfig")}
                    </Button>
                  </>
                ) : (
                  <>
                    <Button onClick={handleOrdenCheck} variant="outline" disabled={ordenBusy}>
                      <FileCheck2 size={14} />
                      {t("settings.orden.check")}
                    </Button>
                    <Button onClick={() => handleOrdenRun(true)} variant="outline" disabled={ordenBusy}>
                      <ScanSearch size={14} />
                      {t("settings.orden.simulate")}
                    </Button>
                    <Button onClick={() => handleOrdenRun(false)} disabled={ordenBusy}>
                      <Play size={14} />
                      {t("settings.orden.run")}
                    </Button>
                  </>
                )}
              </div>
            </div>

            <Card className="space-y-3 p-4">
              {ordenView === "editor" && (
                <>
              <div className="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto_auto] items-end gap-2">
                <div>
                  <Label className="mb-1 block text-xs text-muted-foreground">
                    {t("settings.orden.savedConfigs")}
                  </Label>
                  <Select
                    value={ordenConfigs.includes(ordenName) ? ordenName : "__draft__"}
                    onValueChange={(value) => {
                      if (value === "__draft__") return;
                      handleOrdenSelect(value);
                    }}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {!ordenConfigs.includes(ordenName) && (
                        <SelectItem value="__draft__">{t("settings.orden.unsaved")}</SelectItem>
                      )}
                      {ordenConfigs.map((name) => (
                        <SelectItem key={name} value={name}>
                          {name}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
                <div>
                  <Label className="mb-1 block text-xs text-muted-foreground">
                    {t("settings.orden.configName")}
                  </Label>
                  <Input
                    value={ordenName}
                    onChange={(e) => setOrdenName(e.target.value)}
                    placeholder="main"
                  />
                </div>
                <Button onClick={handleOrdenSave} variant="outline" disabled={ordenBusy}>
                  <Save size={14} />
                  {t("settings.orden.save")}
                </Button>
                <Button
                  onClick={handleOrdenDelete}
                  variant="ghost"
                  disabled={ordenBusy}
                  className="text-destructive hover:bg-destructive/10 hover:text-destructive"
                >
                  <Trash2 size={14} />
                  {t("settings.orden.delete")}
                </Button>
              </div>

              <div className="grid gap-2 md:grid-cols-2">
                <div>
                  <Label className="mb-1 block text-xs text-muted-foreground">
                    {t("settings.orden.tags")}
                  </Label>
                  <Input
                    value={ordenTags}
                    onChange={(e) => setOrdenTags(e.target.value)}
                    placeholder="work, invoices"
                  />
                </div>
                <div>
                  <Label className="mb-1 block text-xs text-muted-foreground">
                    {t("settings.orden.skipTags")}
                  </Label>
                  <Input
                    value={ordenSkipTags}
                    onChange={(e) => setOrdenSkipTags(e.target.value)}
                    placeholder="never"
                  />
                </div>
              </div>
                </>
              )}

              {ordenView === "list" && (
              <div className="space-y-3">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <div>
                    <Label className="text-sm font-medium">{t("settings.orden.configTable")}</Label>
                    <p className="text-xs text-muted-foreground">{t("settings.orden.configTableDesc")}</p>
                  </div>
                  <div className="relative">
                    <Search className="absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
                    <Input value={ordenSearch} onChange={(event) => setOrdenSearch(event.target.value)} placeholder={t("settings.orden.searchConfigs")} className="h-8 w-56 pl-8" />
                  </div>
                </div>
                <div className="overflow-visible rounded-lg border border-border">
                  <Table>
                    <TableHeader><TableRow><TableHead>{t("settings.orden.config")}</TableHead><TableHead>{t("settings.orden.status")}</TableHead><TableHead>{t("settings.orden.schedule")}</TableHead><TableHead>{t("settings.orden.lastRun")}</TableHead><TableHead className="text-right">{t("settings.orden.actions")}</TableHead></TableRow></TableHeader>
                    <TableBody>
                      {filteredOrdenConfigs.map((name) => {
                        const history = ordenHistoryByConfig[name] || [];
                        const last = history[0];
                        const jobs = configJobs(name);
                        const scheduled = jobs.some((job) => job.enabled);
                        const status = !last ? "ready" : last.errors > 0 ? "failed" : "success";
                        return (
                          <TableRow key={name} data-state={name === ordenName ? "selected" : undefined}>
                            <TableCell><button type="button" className="text-left" onClick={() => handleOrdenPreview(name)}><div className="font-medium">{name}</div><div className="max-w-64 truncate text-xs text-muted-foreground">{ordenNotes[name] || t("settings.orden.noNote")}</div></button></TableCell>
                            <TableCell><Badge variant="outline" className="gap-1.5"><span aria-hidden="true" className={`size-1.5 rounded-full ${status === "success" ? "bg-primary" : status === "failed" ? "bg-destructive" : "bg-muted-foreground/64"}`} />{t(`settings.orden.status_${status}`)}</Badge></TableCell>
                            <TableCell><Badge variant="outline" className="gap-1.5"><span aria-hidden="true" className={`size-1.5 rounded-full ${scheduled ? "bg-primary" : "bg-muted-foreground/64"}`} />{scheduled ? t("settings.orden.running") : t("settings.orden.stopped")}</Badge></TableCell>
                            <TableCell className="text-xs text-muted-foreground">{last ? new Date(last.timestamp).toLocaleString() : "—"}</TableCell>
                            <TableCell className="text-right">
                              <div className="flex flex-wrap justify-end gap-1">
                                <Button type="button" onClick={() => runOrdenConfigByName(name, true)} variant="outline" size="sm" className="max-[900px]:px-2" disabled={ordenBusy}>
                                  <ScanSearch size={13} />
                                  {t("settings.orden.tryRun")}
                                </Button>
                                <Button type="button" onClick={() => handleOrdenPreview(name)} variant="ghost" size="sm" className="max-[900px]:px-2">
                                  <Eye size={13} />
                                  {t("settings.orden.preview")}
                                </Button>
                                <Menu>
                                  <MenuTrigger render={<Button type="button" variant="ghost" size="icon" aria-label={t("settings.orden.moreActions")} />}>
                                    <MoreHorizontal size={15} />
                                  </MenuTrigger>
                                  <MenuPopup>
                                    <MenuGroup>
                                      <MenuGroupLabel>{t("settings.orden.configManagement")}</MenuGroupLabel>
                                      <MenuItem onClick={() => handleOrdenSelect(name)}><Pencil />{t("settings.orden.edit")}</MenuItem>
                                      <MenuItem onClick={() => runOrdenConfigByName(name, false)}><Play />{t("settings.orden.run")}</MenuItem>
                                      <MenuItem onClick={() => handleNewOrdenJob(name)}><Plus />{t("settings.orden.newTask")}</MenuItem>
                                      <MenuItem onClick={() => { const note = window.prompt(t("settings.orden.notePrompt"), ordenNotes[name] || ""); if (note !== null) updateOrdenNote(name, note); }}><StickyNote />{t("settings.orden.addNote")}</MenuItem>
                                    </MenuGroup>
                                    <MenuSeparator />
                                    <MenuGroup>
                                      <MenuGroupLabel>{t("settings.orden.taskManagement")}</MenuGroupLabel>
                                      <MenuItem disabled={jobs.length === 0 || !scheduled} onClick={() => setConfigJobsEnabled(name, false)}><Pause />{t("settings.orden.stopSchedules")}</MenuItem>
                                      <MenuItem disabled={jobs.length === 0 || scheduled} onClick={() => setConfigJobsEnabled(name, true)}><Play />{t("settings.orden.startSchedules")}</MenuItem>
                                    </MenuGroup>
                                    <MenuSeparator />
                                    <MenuItem variant="destructive" onClick={() => deleteOrdenConfigByName(name)}><Trash2 />{t("settings.orden.delete")}</MenuItem>
                                  </MenuPopup>
                                </Menu>
                              </div>
                            </TableCell>
                          </TableRow>
                        );
                      })}
                    </TableBody>
                    {filteredOrdenConfigs.length === 0 && <TableBody><TableRow><TableCell colSpan={5} className="py-8 text-center text-muted-foreground">{t("settings.orden.noConfigs")}</TableCell></TableRow></TableBody>}
                    <TableFooter><TableRow><TableCell colSpan={4}>{t("settings.orden.totalConfigs")}</TableCell><TableCell className="text-right">{filteredOrdenConfigs.length}</TableCell></TableRow></TableFooter>
                  </Table>
                </div>
              </div>
              )}
            </Card>

            {ordenToast && (
              <div
                className={`rounded-xl border px-3 py-2 text-xs shadow-sm ${
                  ordenToast.type === "success"
                    ? "border-primary/25 bg-primary/8 text-primary"
                    : "border-destructive/20 bg-destructive/10 text-destructive"
                }`}
              >
                {ordenToast.message}
              </div>
            )}

            {ordenView === "list" && <Card className="space-y-3 p-4">
              <div className="flex items-center justify-between gap-2">
                <div>
                  <Label className="text-sm font-medium">{t("settings.orden.tasks")}</Label>
                  <p className="text-xs text-muted-foreground">{t("settings.orden.tasksDesc")}</p>
                </div>
                <Button onClick={() => handleNewOrdenJob()} variant="outline" size="sm" disabled={ordenConfigs.length === 0}>
                  <Plus size={14} />
                  {t("settings.orden.newTask")}
                </Button>
              </div>
              {editingOrdenJob && (
                <div className="grid gap-3 rounded-xl border border-border bg-muted/30 p-3 md:grid-cols-2">
                  <div>
                    <Label className="mb-1 block text-xs text-muted-foreground">{t("settings.orden.taskName")}</Label>
                    <Input value={editingOrdenJob.name} onChange={(e) => setEditingOrdenJob({ ...editingOrdenJob, name: e.target.value })} />
                  </div>
                  <div>
                    <Label className="mb-1 block text-xs text-muted-foreground">{t("settings.orden.config")}</Label>
                    <Select value={editingOrdenJob.config_name} onValueChange={(value) => setEditingOrdenJob({ ...editingOrdenJob, config_name: value })}>
                      <SelectTrigger><SelectValue /></SelectTrigger>
                      <SelectContent>{ordenConfigs.map((name) => <SelectItem key={name} value={name}>{name}</SelectItem>)}</SelectContent>
                    </Select>
                  </div>
                  <div>
                    <Label className="mb-1 block text-xs text-muted-foreground">{t("settings.orden.triggerMode")}</Label>
                    <Select value={editingOrdenJob.mode} onValueChange={(value) => setEditingOrdenJob({ ...editingOrdenJob, mode: value })}>
                      <SelectTrigger><SelectValue /></SelectTrigger>
                      <SelectContent>
                        <SelectItem value="manual">{t("settings.orden.modeManual")}</SelectItem>
                        <SelectItem value="fixed">{t("settings.orden.modeFixed")}</SelectItem>
                        <SelectItem value="cron">Cron</SelectItem>
                        <SelectItem value="interval">{t("settings.orden.modeInterval")}</SelectItem>
                        <SelectItem value="monitor">{t("settings.orden.modeMonitor")}</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                  <div className="flex items-end gap-4">
                    <Label className="flex items-center gap-2 text-sm"><Switch checked={editingOrdenJob.enabled} onCheckedChange={(checked) => setEditingOrdenJob({ ...editingOrdenJob, enabled: checked })} /> {t("settings.orden.enabled")}</Label>
                    <Label className="flex items-center gap-2 text-sm"><Switch checked={editingOrdenJob.simulate} onCheckedChange={(checked) => setEditingOrdenJob({ ...editingOrdenJob, simulate: checked })} /> {t("settings.orden.simulate")}</Label>
                  </div>
                  {editingOrdenJob.mode === "cron" && <div>
                    <Label className="mb-1 block text-xs text-muted-foreground">Cron</Label>
                    <Input value={editingOrdenJob.cron_expr || ""} onChange={(e) => setEditingOrdenJob({ ...editingOrdenJob, cron_expr: e.target.value })} placeholder="0 * * * *" />
                  </div>}
                  {editingOrdenJob.mode === "fixed" && <div>
                      <Label className="mb-1 block text-xs text-muted-foreground">{t("settings.orden.fixedTime")}</Label>
                      <Input type="time" value={editingOrdenJob.fixed_time || ""} onChange={(e) => setEditingOrdenJob({ ...editingOrdenJob, fixed_time: e.target.value || null })} />
                  </div>}
                  {editingOrdenJob.mode === "interval" && <div>
                      <Label className="mb-1 block text-xs text-muted-foreground">{t("settings.orden.intervalMinutes")}</Label>
                      <Input type="number" min={1} value={editingOrdenJob.interval_minutes} onChange={(e) => setEditingOrdenJob({ ...editingOrdenJob, interval_minutes: parseInt(e.target.value, 10) || 60 })} />
                  </div>}
                  <div>
                    <Label className="mb-1 block text-xs text-muted-foreground">Tags</Label>
                    <Input value={editingOrdenJob.tags} onChange={(e) => setEditingOrdenJob({ ...editingOrdenJob, tags: e.target.value })} />
                  </div>
                  <div>
                    <Label className="mb-1 block text-xs text-muted-foreground">{t("settings.orden.skipTags")}</Label>
                    <Input value={editingOrdenJob.skip_tags} onChange={(e) => setEditingOrdenJob({ ...editingOrdenJob, skip_tags: e.target.value })} />
                  </div>
                  <div>
                    <Label className="mb-1 block text-xs text-muted-foreground">{t("settings.orden.pathExists")}</Label>
                    <Input value={editingOrdenJob.path_exists || ""} onChange={(e) => setEditingOrdenJob({ ...editingOrdenJob, path_exists: e.target.value || null })} placeholder="~/Downloads" />
                  </div>
                  <div className="grid grid-cols-3 gap-2">
                    <div>
                      <Label className="mb-1 block text-xs text-muted-foreground">{t("settings.orden.minFiles")}</Label>
                      <Input type="number" min={0} value={editingOrdenJob.min_file_count} onChange={(e) => setEditingOrdenJob({ ...editingOrdenJob, min_file_count: parseInt(e.target.value, 10) || 0 })} />
                    </div>
                    <div>
                      <Label className="mb-1 block text-xs text-muted-foreground">{t("settings.orden.windowStart")}</Label>
                      <Input type="time" value={editingOrdenJob.time_window_start || ""} onChange={(e) => setEditingOrdenJob({ ...editingOrdenJob, time_window_start: e.target.value || null })} />
                    </div>
                    <div>
                      <Label className="mb-1 block text-xs text-muted-foreground">{t("settings.orden.windowEnd")}</Label>
                      <Input type="time" value={editingOrdenJob.time_window_end || ""} onChange={(e) => setEditingOrdenJob({ ...editingOrdenJob, time_window_end: e.target.value || null })} />
                    </div>
                  </div>
                  <div className="md:col-span-2">
                    <Label className="mb-1 block text-xs text-muted-foreground">{t("settings.orden.monitorPaths")}</Label>
                    <textarea value={editingOrdenJob.watch_paths} onChange={(e) => setEditingOrdenJob({ ...editingOrdenJob, watch_paths: e.target.value })} className="min-h-20 w-full resize-y rounded-lg border border-border bg-background px-3 py-2 text-xs leading-5 text-foreground outline-none focus:border-ring focus:ring-2 focus:ring-ring/20" />
                  </div>
                  <div className="flex gap-2 md:col-span-2">
                    <Button onClick={handleSaveOrdenJob}><Save size={14} /> {t("settings.orden.saveTask")}</Button>
                    <Button onClick={() => setEditingOrdenJob(null)} variant="outline"><X size={14} /> {t("common.cancel")}</Button>
                  </div>
                </div>
              )}
              <div className="overflow-hidden rounded-lg border border-border">
                <Table>
                  <TableHeader><TableRow><TableHead>{t("settings.orden.task")}</TableHead><TableHead>{t("settings.orden.config")}</TableHead><TableHead>{t("settings.orden.triggerMode")}</TableHead><TableHead>{t("settings.orden.status")}</TableHead><TableHead>{t("settings.orden.lastRun")}</TableHead><TableHead className="text-right">{t("settings.orden.actions")}</TableHead></TableRow></TableHeader>
                  <TableBody>
                    {ordenJobsRows.map((job) => (
                      <TableRow key={job.id || job.name}>
                        <TableCell><div className="font-medium">{job.name}</div>{job.simulate && <div className="mt-1 text-xs text-muted-foreground">{t("settings.orden.simulate")}</div>}</TableCell>
                        <TableCell className="text-muted-foreground">{job.config_name}</TableCell>
                        <TableCell><Badge variant="outline">{job.mode}</Badge></TableCell>
                        <TableCell><div className="flex items-center gap-2"><Switch checked={job.enabled} onCheckedChange={(enabled) => ordenSaveJob({ ...job, enabled }).then(() => ordenJobs()).then(setOrdenJobsRows)} aria-label={t("settings.orden.toggleTask", { name: job.name })} /><span className="text-xs text-muted-foreground">{job.enabled ? t("settings.orden.running") : t("settings.orden.stopped")}</span></div></TableCell>
                        <TableCell className="text-xs text-muted-foreground">{job.last_run_at ? new Date(job.last_run_at).toLocaleString() : "—"}</TableCell>
                        <TableCell><div className="flex justify-end gap-1"><Button onClick={() => handleRunOrdenJob(job)} disabled={ordenBusy} variant="ghost" size="icon" aria-label={t("settings.orden.runTask", { name: job.name })}><Play size={14} /></Button><Button onClick={() => setEditingOrdenJob(job)} variant="ghost" size="icon" aria-label={t("settings.orden.editTask", { name: job.name })}><Pencil size={14} /></Button><Button onClick={() => handleDeleteOrdenJob(job)} variant="ghost" size="icon" aria-label={t("settings.orden.deleteTask", { name: job.name })} className="text-destructive hover:bg-destructive/10 hover:text-destructive"><Trash2 size={14} /></Button></div></TableCell>
                      </TableRow>
                    ))}
                    {ordenJobsRows.length === 0 && <TableRow><TableCell colSpan={6} className="py-8 text-center text-muted-foreground">{t("settings.orden.noTasks")}</TableCell></TableRow>}
                  </TableBody>
                </Table>
              </div>
            </Card>}

            {ordenView === "editor" && <>
            <Card className="p-4">
              <OrdenRunHistoryTable
                rows={ordenHistoryRows}
                onRefresh={() => refreshOrdenHistory(ordenName)}
                onDelete={(id) => handleDeleteOrdenHistory(ordenName, id)}
                onClear={() => handleClearOrdenHistory(ordenName)}
              />
            </Card>

            <div className="space-y-3">
              <div className="flex items-center justify-between gap-2">
                <Label className="text-xs text-muted-foreground">{t("settings.orden.yaml")}</Label>
                <div className="flex rounded-xl border border-border bg-muted/30 p-0.5">
                  <Button
                    onClick={() => handleOrdenEditorModeChange("visual")}
                    variant={ordenEditorMode === "visual" ? "secondary" : "ghost"}
                    size="sm"
                    className="h-7"
                  >
                    <Eye size={14} />
                    {t("settings.orden.visual")}
                  </Button>
                  <Button
                    onClick={() => handleOrdenEditorModeChange("source")}
                    variant={ordenEditorMode === "source" ? "secondary" : "ghost"}
                    size="sm"
                    className="h-7"
                  >
                    <Braces size={14} />
                    {t("settings.orden.source")}
                  </Button>
                </div>
              </div>

              {ordenEditorMode === "source" ? (
                <div className="overflow-hidden rounded-xl border border-border bg-card shadow-sm">
                  <button
                    type="button"
                    onClick={() => setOrdenSourceExpanded((prev) => !prev)}
                    className="flex w-full items-center justify-between px-3 py-2 text-left text-sm"
                  >
                    <span className="font-medium">{t("settings.orden.source")}</span>
                    <span className="flex items-center gap-2 text-xs text-muted-foreground">
                      {ordenSourceExpanded ? t("settings.orden.hideYaml") : t("settings.orden.showYaml")}
                      {ordenSourceExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                    </span>
                  </button>
                  {ordenSourceExpanded && (
                    <div className="border-t border-border p-3">
                      <textarea
                        value={ordenYaml}
                        onChange={(e) => setOrdenYaml(e.target.value)}
                        spellCheck={false}
                        className="min-h-[320px] w-full resize-y rounded-lg border border-border bg-background px-3 py-2 font-mono text-xs leading-5 text-foreground outline-none focus:border-ring focus:ring-2 focus:ring-ring/20"
                      />
                    </div>
                  )}
                </div>
              ) : (
                <div className="space-y-3">
                  <div className="flex justify-end">
                    <Button onClick={handleAddOrdenVisualRule} variant="outline" size="sm">
                      <Plus size={14} />
                      {t("settings.orden.addRule")}
                    </Button>
                  </div>
                  {ordenVisual.rules.map((rule, idx) => (
                    <OrdenVisualRuleCard
                      key={rule.id}
                      rule={rule}
                      index={idx}
                      onUpdate={updateOrdenVisualRule}
                      onRemove={handleRemoveOrdenVisualRule}
                      onChooseLocations={handleChooseOrdenLocations}
                      onChooseDestinations={handleChooseOrdenDestinations}
                    />
                  ))}
                </div>
              )}
            </div>
            </>}

              </>
            )}
          </div>
        )}

        {tab === "templates" && (
          <OrdenTemplateCenter
            templates={ordenTemplates}
            configNames={ordenConfigs}
            onUseTemplate={handleUseOrdenTemplate}
            onLoadConfig={ordenLoad}
            onSaveTemplate={handleSaveOrdenTemplate}
            onDeleteTemplate={handleDeleteOrdenTemplate}
          />
        )}

        {tab === "general" && (
          <GeneralTab
            settings={settings}
            saveSettings={saveSettings}
            setAutostart={setAutostart}
            handleChangeLanguage={handleChangeLanguage}
            graceValue={graceValue}
            graceUnit={graceUnit}
            graceError={graceError}
            currentGraceSeconds={currentGraceSeconds}
            graceSteps={GRACE_STEPS}
            sliderIndex={sliderIndex}
            formatDuration={formatDuration}
            handleGraceSliderChange={handleGraceSliderChange}
            handleGraceNumberChange={handleGraceNumberChange}
            localSchedule={localSchedule}
            handleScheduleChange={handleScheduleChange}
            handleValidateCron={handleValidateCron}
            systemKeepaliveSupported={systemKeepaliveSupported}
            handleInstallSystemKeepalive={handleInstallSystemKeepalive}
            handleUninstallSystemKeepalive={handleUninstallSystemKeepalive}
            scheduleToast={scheduleToast}
            handleSaveSchedule={handleSaveSchedule}
            schedulerLogs={schedulerLogs}
            loadSchedulerLogs={loadSchedulerLogs}
            clearSchedulerLogs={clearSchedulerLogs}
            localMcp={localMcp}
            setLocalMcp={setLocalMcp}
            handleSaveMcp={handleSaveMcp}
            handleCopyMcpConfig={handleCopyMcpConfig}
            mcpClientConfig={mcpClientConfig}
            mcpToast={mcpToast}
            handleExportConfig={handleExportConfig}
            handleImportConfig={handleImportConfig}
            replaceConfigOnImport={replaceConfigOnImport}
            setReplaceConfigOnImport={setReplaceConfigOnImport}
            configToast={configToast}
          />
        )}

        {tab === "ignore" && (
          <IgnoreTab />
        )}

        </div>
      </main>
    </div>
  );
}
