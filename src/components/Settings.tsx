import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  DEFAULT_ORDEN_EXAMPLE,
  McpClientConfig,
  OrdenVisualConfig,
  OrdenVisualRule,
  OrdenRunResult,
  OrdenJob,
  useAppStore,
  Rule,
  ScheduleSettings,
} from "../store/useAppStore";
import { invoke } from "@tauri-apps/api/core";
import { save, open } from "@tauri-apps/plugin-dialog";
import { BrandMark } from "./BrandMark";
import { GeneralTab } from "./settings/GeneralTab";
import { IgnoreTab } from "./settings/IgnoreTab";
import { RulesTab } from "./settings/RulesTab";
import { OrdenPreview } from "./settings/OrdenPreview";
import { SidebarButton } from "./settings/SidebarButton";
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
} from "./settings/utils";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { Card } from "./ui/card";
import { Checkbox } from "./ui/checkbox";
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
} from "lucide-react";


type Tab = "rules" | "history" | "ignore" | "advanced" | "general";

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
    ordenCheck,
    ordenRun,
    ordenVisualFromYaml,
    ordenHistory,
    ordenJobs,
    ordenSaveJob,
    ordenDeleteJob,
    ordenRunJob,
    getMcpClientConfig,
    loadStats,
  } = useAppStore();

  const [tab, setTab] = useState<Tab>("rules");
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
  const [ordenName, setOrdenName] = useState("main");
  const [ordenYaml, setOrdenYaml] = useState(DEFAULT_ORDEN_EXAMPLE);
  const [ordenEditorMode, setOrdenEditorMode] = useState<OrdenEditorMode>("source");
  const [ordenSourceExpanded, setOrdenSourceExpanded] = useState(false);
  const [ordenView, setOrdenView] = useState<OrdenView>("editor");
  const [ordenPreviewError, setOrdenPreviewError] = useState<string | null>(null);
  const [ordenVisual, setOrdenVisual] = useState<OrdenVisualConfig>(defaultOrdenVisualConfig());
  const [ordenTags, setOrdenTags] = useState("");
  const [ordenSkipTags, setOrdenSkipTags] = useState("");
  const [ordenResult, setOrdenResult] = useState<OrdenRunResult | null>(null);
  const [ordenHistoryRows, setOrdenHistoryRows] = useState<import("../store/useAppStore").OrdenRunHistory[]>([]);
  const [ordenJobsRows, setOrdenJobsRows] = useState<OrdenJob[]>([]);
  const [editingOrdenJob, setEditingOrdenJob] = useState<OrdenJob | null>(null);
  const [ordenBusy, setOrdenBusy] = useState(false);
  const [ordenToast, setOrdenToast] = useState<{ message: string; type: "success" | "error" } | null>(null);

  useEffect(() => {
    loadRules();
    loadFolders();
    loadLogs();
    getSchedule();
    loadSchedulerLogs();
  }, [loadRules, loadFolders, loadLogs, getSchedule, loadSchedulerLogs]);

  useEffect(() => {
    getSystemKeepaliveStatus()
      .then((status) => setSystemKeepaliveSupported(status.supported))
      .catch(() => setSystemKeepaliveSupported(false));
  }, [getSystemKeepaliveStatus]);

  const loadOrdenConfigs = async (preferredName?: string) => {
    const names = await ordenList();
    setOrdenConfigs(names);
    const nameToLoad = preferredName || (ordenName && names.includes(ordenName) ? ordenName : names[0]);
    if (nameToLoad) {
      const yaml = await ordenLoad(nameToLoad);
      setOrdenName(nameToLoad);
      setOrdenYaml(yaml);
      ordenHistory(nameToLoad, 20).then(setOrdenHistoryRows).catch(console.error);
    } else {
      setOrdenName("main");
      setOrdenYaml(DEFAULT_ORDEN_EXAMPLE);
      setOrdenHistoryRows([]);
    }
  };

  useEffect(() => {
    loadOrdenConfigs().catch((e) => console.error("Failed to load orden configs:", e));
    ordenJobs().then(setOrdenJobsRows).catch((e) => console.error("Failed to load orden jobs:", e));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

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
    getMcpClientConfig()
      .then(setMcpClientConfig)
      .catch(() => setMcpClientConfig(null));
  }, [
    getMcpClientConfig,
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
    const selected = await open({ directory: true, multiple: false });
    const path = Array.isArray(selected) ? selected[0] : selected;
    if (path) {
      setNewFolderPath(path);
    }
  };

  const handleChooseDestination = async () => {
    if (!editingRule) return;
    const selected = await open({ directory: true, multiple: false });
    const path = Array.isArray(selected) ? selected[0] : selected;
    if (path) {
      setEditingRule({ ...editingRule, destination: path });
    }
  };

  const handleChooseRuleScopeFolder = async () => {
    if (!editingRule) return;
    const selected = await open({ directory: true, multiple: false });
    const path = Array.isArray(selected) ? selected[0] : selected;
    if (path) {
      setEditingRule({ ...editingRule, folder_id: 0, folder_path: path });
    }
  };

  const handleSaveRule = async () => {
    if (!editingRule) return;
    const normalizedRule = {
      ...editingRule,
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
    const selected = await open({ directory, multiple: true });
    const paths = normalizeDialogSelection(selected);
    if (paths.length === 0) return;
    const rule = ordenVisual.rules.find((item) => item.id === id);
    updateOrdenVisualRule(id, {
      location: mergePathText(rule?.location || "", paths),
    });
  };

  const handleChooseOrdenDestinations = async (id: string) => {
    const selected = await open({ directory: true, multiple: true });
    const paths = normalizeDialogSelection(selected);
    if (paths.length === 0) return;
    const rule = ordenVisual.rules.find((item) => item.id === id);
    updateOrdenVisualRule(id, {
      destination: mergePathText(rule?.destination || "", paths),
    });
  };

  const handleOrdenSelect = async (name: string) => {
    try {
      const yaml = await ordenLoad(name);
      setOrdenName(name);
      setOrdenYaml(yaml);
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

  const handleOrdenDelete = async () => {
    if (!ordenConfigs.includes(ordenName)) {
      setOrdenName("main");
      setOrdenYaml(DEFAULT_ORDEN_EXAMPLE);
      setOrdenResult(null);
      setOrdenPreviewError(null);
      setOrdenView("editor");
      return;
    }
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

  const newOrdenJob = (): OrdenJob => ({
    name: `${ordenName || "main"}-job`,
    config_name: ordenName || "main",
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
    <div className="flex h-full overflow-hidden rounded-xl border border-border/80 bg-background text-foreground shadow-lg">
      {/* Sidebar */}
      <div className="flex w-60 flex-col border-r border-border/70 bg-card/95">
        <div data-tauri-drag-region className="flex min-h-14 items-center gap-2 px-4 py-4 pl-20">
          <Button
            onClick={() => {
              invoke("close_settings");
            }}
            variant="ghost"
            size="icon"
            className="size-7"
          >
            <ChevronLeft size={16} />
          </Button>
          <BrandMark showLabel />
        </div>
        <nav className="flex flex-1 flex-col gap-0.5 px-2">
          <SidebarButton
            active={tab === "rules"}
            onClick={() => setTab("rules")}
            icon={<List size={16} />}
            label={t("settings.rules.title")}
          />
          <SidebarButton
            active={tab === "history"}
            onClick={() => setTab("history")}
            icon={<History size={16} />}
            label={t("settings.history.title")}
          />
          <SidebarButton
            active={tab === "ignore"}
            onClick={() => setTab("ignore")}
            icon={<X size={16} />}
            label={t("settings.ignore.title")}
          />
          <SidebarButton
            active={tab === "advanced"}
            onClick={() => setTab("advanced")}
            icon={<Code2 size={16} />}
            label={t("settings.orden.title")}
          />
          <SidebarButton
            active={tab === "general"}
            onClick={() => setTab("general")}
            icon={<Globe size={16} />}
            label={t("settings.general.title")}
          />
        </nav>
      </div>

      {/* Content */}
      <div className="min-w-0 flex-1 overflow-y-auto overscroll-contain px-7 py-6 [scrollbar-gutter:stable]">
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
                  onClick={clearLogs}
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
                    className={`flex items-center justify-between gap-4 px-4 py-3 ${
                      log.undone ? "bg-muted opacity-70" : ""
                    }`}
                  >
                    <div className="min-w-0 flex-1">
                      <div className="truncate text-sm font-medium">{log.file_name}</div>
                      <div className="truncate text-xs text-muted-foreground">
                        {(log.rule_label || log.file_type)} · {log.action} → {log.destination_path || "-"}
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
                onBack={() => setOrdenView("editor")}
              />
            ) : (
              <>

            <div className="flex items-center justify-between gap-3">
              <div>
                <h2 className="text-lg font-semibold">{t("settings.orden.title")}</h2>
                <p className="text-xs text-muted-foreground">{t("settings.orden.desc")}</p>
              </div>
              <div className="flex items-center gap-2">
                <Button onClick={() => openHistoryFilter("engine:orden")} variant="outline">
                  <History size={14} />
                  {t("settings.history.title")}
                </Button>
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
              </div>
            </div>

            <Card className="p-4 space-y-3">
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

              <div className="space-y-2 border-t border-border pt-3">
                <div className="flex items-center justify-between gap-2">
                  <div>
                    <Label className="text-sm font-medium">Orden configs</Label>
                    <p className="text-xs text-muted-foreground">Card list for create, read, update, and delete.</p>
                  </div>
                  <Button
                    onClick={() => {
                      setOrdenName("main");
                      setOrdenYaml(DEFAULT_ORDEN_EXAMPLE);
                      setOrdenVisual(defaultOrdenVisualConfig());
                      setOrdenEditorMode("visual");
                    }}
                    variant="outline"
                    size="sm"
                  >
                    <Plus size={14} />
                    New config
                  </Button>
                </div>
                <div className="grid gap-2 md:grid-cols-2 xl:grid-cols-3">
                  {ordenConfigs.length === 0 ? (
                    <Card className="p-3 text-xs text-muted-foreground">No saved configs yet.</Card>
                  ) : (
                    ordenConfigs.map((name) => (
                      <Card
                        key={name}
                        className={`space-y-2 p-3 transition-colors ${name === ordenName ? "border-primary/50 bg-primary/5" : ""}`}
                      >
                        <div className="flex items-start justify-between gap-2">
                          <div className="min-w-0">
                            <div className="truncate text-sm font-medium">{name}</div>
                            <div className="text-xs text-muted-foreground">YAML / Visual rules</div>
                          </div>
                          {name === ordenName && <Badge variant="secondary">open</Badge>}
                        </div>
                        <div className="flex flex-wrap gap-1">
                          <Button onClick={() => handleOrdenSelect(name)} variant="outline" size="sm" className="h-7 text-xs">
                            <Eye size={13} />
                            View/Edit
                          </Button>
                          <Button
                            onClick={async () => {
                              await handleOrdenSelect(name);
                              await handleOrdenRun(true);
                            }}
                            variant="ghost"
                            size="sm"
                            className="h-7 text-xs"
                            disabled={ordenBusy}
                          >
                            <ScanSearch size={13} />
                            Sim
                          </Button>
                          <Button
                            onClick={async () => {
                              await handleOrdenSelect(name);
                              await handleOrdenDelete();
                            }}
                            variant="ghost"
                            size="sm"
                            className="h-7 text-xs text-destructive hover:bg-destructive/10 hover:text-destructive"
                            disabled={ordenBusy}
                          >
                            <Trash2 size={13} />
                            Delete
                          </Button>
                        </div>
                      </Card>
                    ))
                  )}
                </div>
              </div>
            </Card>

            {ordenToast && (
              <div
                className={`rounded-xl border px-3 py-2 text-xs shadow-sm ${
                  ordenToast.type === "success"
                    ? "border-emerald-200 bg-emerald-50 text-emerald-700 dark:border-emerald-900 dark:bg-emerald-950 dark:text-emerald-300"
                    : "border-destructive/20 bg-destructive/10 text-destructive"
                }`}
              >
                {ordenToast.message}
              </div>
            )}

            <Card className="space-y-3 p-4">
              <div className="flex items-center justify-between gap-2">
                <div>
                  <Label className="text-sm font-medium">Automation jobs</Label>
                  <p className="text-xs text-muted-foreground">Manual, fixed time, cron, interval, condition, and monitor execution for each Orden config.</p>
                </div>
                <Button onClick={() => setEditingOrdenJob(newOrdenJob())} variant="outline" size="sm">
                  <Plus size={14} />
                  New job
                </Button>
              </div>
              {editingOrdenJob && (
                <div className="grid gap-3 rounded-xl border border-border bg-muted/30 p-3 md:grid-cols-2">
                  <div>
                    <Label className="mb-1 block text-xs text-muted-foreground">Name</Label>
                    <Input value={editingOrdenJob.name} onChange={(e) => setEditingOrdenJob({ ...editingOrdenJob, name: e.target.value })} />
                  </div>
                  <div>
                    <Label className="mb-1 block text-xs text-muted-foreground">Config</Label>
                    <Select value={editingOrdenJob.config_name} onValueChange={(value) => setEditingOrdenJob({ ...editingOrdenJob, config_name: value })}>
                      <SelectTrigger><SelectValue /></SelectTrigger>
                      <SelectContent>{ordenConfigs.map((name) => <SelectItem key={name} value={name}>{name}</SelectItem>)}</SelectContent>
                    </Select>
                  </div>
                  <div>
                    <Label className="mb-1 block text-xs text-muted-foreground">Mode</Label>
                    <Select value={editingOrdenJob.mode} onValueChange={(value) => setEditingOrdenJob({ ...editingOrdenJob, mode: value })}>
                      <SelectTrigger><SelectValue /></SelectTrigger>
                      <SelectContent>
                        <SelectItem value="manual">Manual only</SelectItem>
                        <SelectItem value="fixed">Fixed time</SelectItem>
                        <SelectItem value="cron">Cron</SelectItem>
                        <SelectItem value="interval">Interval</SelectItem>
                        <SelectItem value="monitor">Continuous monitor</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                  <div className="flex items-end gap-4">
                    <Label className="flex items-center gap-2 text-sm"><Switch checked={editingOrdenJob.enabled} onCheckedChange={(checked) => setEditingOrdenJob({ ...editingOrdenJob, enabled: checked })} /> Enabled</Label>
                    <Label className="flex items-center gap-2 text-sm"><Switch checked={editingOrdenJob.simulate} onCheckedChange={(checked) => setEditingOrdenJob({ ...editingOrdenJob, simulate: checked })} /> Simulate</Label>
                  </div>
                  <div>
                    <Label className="mb-1 block text-xs text-muted-foreground">Cron</Label>
                    <Input value={editingOrdenJob.cron_expr || ""} onChange={(e) => setEditingOrdenJob({ ...editingOrdenJob, cron_expr: e.target.value })} placeholder="0 * * * *" />
                  </div>
                  <div className="grid grid-cols-2 gap-2">
                    <div>
                      <Label className="mb-1 block text-xs text-muted-foreground">Fixed time</Label>
                      <Input type="time" value={editingOrdenJob.fixed_time || ""} onChange={(e) => setEditingOrdenJob({ ...editingOrdenJob, fixed_time: e.target.value || null })} />
                    </div>
                    <div>
                      <Label className="mb-1 block text-xs text-muted-foreground">Interval min</Label>
                      <Input type="number" min={1} value={editingOrdenJob.interval_minutes} onChange={(e) => setEditingOrdenJob({ ...editingOrdenJob, interval_minutes: parseInt(e.target.value, 10) || 60 })} />
                    </div>
                  </div>
                  <div>
                    <Label className="mb-1 block text-xs text-muted-foreground">Tags</Label>
                    <Input value={editingOrdenJob.tags} onChange={(e) => setEditingOrdenJob({ ...editingOrdenJob, tags: e.target.value })} />
                  </div>
                  <div>
                    <Label className="mb-1 block text-xs text-muted-foreground">Skip tags</Label>
                    <Input value={editingOrdenJob.skip_tags} onChange={(e) => setEditingOrdenJob({ ...editingOrdenJob, skip_tags: e.target.value })} />
                  </div>
                  <div>
                    <Label className="mb-1 block text-xs text-muted-foreground">Condition: path exists</Label>
                    <Input value={editingOrdenJob.path_exists || ""} onChange={(e) => setEditingOrdenJob({ ...editingOrdenJob, path_exists: e.target.value || null })} placeholder="~/Downloads" />
                  </div>
                  <div className="grid grid-cols-3 gap-2">
                    <div>
                      <Label className="mb-1 block text-xs text-muted-foreground">Min files</Label>
                      <Input type="number" min={0} value={editingOrdenJob.min_file_count} onChange={(e) => setEditingOrdenJob({ ...editingOrdenJob, min_file_count: parseInt(e.target.value, 10) || 0 })} />
                    </div>
                    <div>
                      <Label className="mb-1 block text-xs text-muted-foreground">Window start</Label>
                      <Input type="time" value={editingOrdenJob.time_window_start || ""} onChange={(e) => setEditingOrdenJob({ ...editingOrdenJob, time_window_start: e.target.value || null })} />
                    </div>
                    <div>
                      <Label className="mb-1 block text-xs text-muted-foreground">Window end</Label>
                      <Input type="time" value={editingOrdenJob.time_window_end || ""} onChange={(e) => setEditingOrdenJob({ ...editingOrdenJob, time_window_end: e.target.value || null })} />
                    </div>
                  </div>
                  <div className="md:col-span-2">
                    <Label className="mb-1 block text-xs text-muted-foreground">Monitor paths, one per line</Label>
                    <textarea value={editingOrdenJob.watch_paths} onChange={(e) => setEditingOrdenJob({ ...editingOrdenJob, watch_paths: e.target.value })} className="min-h-20 w-full resize-y rounded-lg border border-border bg-background px-3 py-2 text-xs leading-5 text-foreground outline-none focus:border-ring focus:ring-2 focus:ring-ring/20" />
                  </div>
                  <div className="flex gap-2 md:col-span-2">
                    <Button onClick={handleSaveOrdenJob}><Save size={14} /> Save job</Button>
                    <Button onClick={() => setEditingOrdenJob(null)} variant="outline"><X size={14} /> Cancel</Button>
                  </div>
                </div>
              )}
              <div className="space-y-2">
                {ordenJobsRows.length === 0 ? <div className="text-xs text-muted-foreground">No automation jobs yet.</div> : ordenJobsRows.map((job) => (
                  <Card key={job.id || job.name} className="p-3 text-xs">
                    <div className="flex items-start justify-between gap-3">
                      <div className="min-w-0 space-y-1">
                        <div className="flex flex-wrap items-center gap-2">
                          <span className="font-medium">{job.name}</span>
                          <Badge variant={job.enabled ? "secondary" : "outline"}>{job.mode}</Badge>
                          {job.simulate && <Badge variant="secondary">sim</Badge>}
                        </div>
                        <div className="truncate text-muted-foreground">{job.config_name} · {job.cron_expr || job.fixed_time || `${job.interval_minutes}m`}</div>
                        <div className="line-clamp-2 text-muted-foreground">{job.watch_paths || "no monitor paths"}</div>
                      </div>
                      <div className="flex shrink-0 gap-1">
                        <Button onClick={() => handleRunOrdenJob(job)} disabled={ordenBusy} variant="ghost" size="icon"><Play size={14} /></Button>
                        <Button onClick={() => setEditingOrdenJob(job)} variant="ghost" size="icon"><Save size={14} /></Button>
                        <Button onClick={() => handleDeleteOrdenJob(job)} variant="ghost" size="icon" className="text-destructive hover:bg-destructive/10 hover:text-destructive"><Trash2 size={14} /></Button>
                      </div>
                    </div>
                  </Card>
                ))}
              </div>
            </Card>

            <Card className="space-y-2 p-4">
              <div className="flex items-center justify-between">
                <Label className="text-sm font-medium">{t("settings.orden.history")}</Label>
                <Button onClick={() => ordenHistory(ordenName, 20).then(setOrdenHistoryRows)} variant="outline" size="sm">
                  {t("settings.scheduler.refreshLogs")}
                </Button>
              </div>
              {ordenHistoryRows.length === 0 ? (
                <div className="text-xs text-muted-foreground">{t("settings.orden.noHistory")}</div>
              ) : (
                <div className="max-h-40 overflow-auto rounded-xl border border-border bg-muted/30">
                  {ordenHistoryRows.map((row) => (
                    <div key={row.id} className="grid grid-cols-[150px_80px_70px_minmax(0,1fr)] gap-2 border-b border-border px-3 py-2 text-xs last:border-b-0">
                      <span className="text-muted-foreground">{new Date(row.timestamp).toLocaleString()}</span>
                      <span>{row.simulate ? t("settings.orden.simulated") : t("settings.orden.applied")}</span>
                      <span>{row.success}/{row.errors}</span>
                      <span className="truncate text-muted-foreground" title={row.logs_json}>{row.trigger}</span>
                    </div>
                  ))}
                </div>
              )}
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
                    <Card key={rule.id} className="space-y-3 p-4">
                      <div className="flex items-center justify-between gap-3">
                        <div className="min-w-0 flex-1">
                          <Label className="mb-1 block text-xs text-muted-foreground">
                            {t("settings.orden.ruleName", { number: idx + 1 })}
                          </Label>
                          <Input
                            value={rule.name}
                            onChange={(e) => updateOrdenVisualRule(rule.id, { name: e.target.value })}
                          />
                        </div>
                        <Switch
                          checked={rule.enabled}
                          onCheckedChange={(checked) => updateOrdenVisualRule(rule.id, { enabled: checked })}
                        />
                        <Button
                          onClick={() => handleRemoveOrdenVisualRule(rule.id)}
                          variant="ghost"
                          size="icon"
                          className="text-destructive hover:bg-destructive/10 hover:text-destructive"
                        >
                          <Trash2 size={14} />
                        </Button>
                      </div>

                      <div className="grid gap-3 md:grid-cols-2">
                        <div>
                          <Label className="mb-1 block text-xs text-muted-foreground">
                            {t("settings.orden.locations")}
                          </Label>
                          <textarea
                            value={rule.location}
                            onChange={(e) => updateOrdenVisualRule(rule.id, { location: e.target.value })}
                            placeholder="~/Downloads"
                            className="min-h-20 w-full resize-y rounded-lg border border-border bg-background px-3 py-2 text-xs leading-5 text-foreground outline-none focus:border-ring focus:ring-2 focus:ring-ring/20"
                          />
                          <div className="mt-2 flex flex-wrap gap-2">
                            <Button
                              onClick={() => handleChooseOrdenLocations(rule.id, false)}
                              variant="outline"
                              size="sm"
                            >
                              <FileCheck2 size={14} />
                              {t("settings.orden.chooseFiles")}
                            </Button>
                            <Button
                              onClick={() => handleChooseOrdenLocations(rule.id, true)}
                              variant="outline"
                              size="sm"
                            >
                              <FolderOpen size={14} />
                              {t("settings.orden.chooseFolders")}
                            </Button>
                          </div>
                        </div>
                        <div className="grid gap-3 sm:grid-cols-[1fr_10rem]">
                          <div>
                            <Label className="mb-1 block text-xs text-muted-foreground">
                              {t("settings.orden.extensions")}
                            </Label>
                            <Input
                              value={rule.extensions}
                              onChange={(e) => updateOrdenVisualRule(rule.id, { extensions: e.target.value })}
                              placeholder="pdf, docx, xlsx"
                            />
                          </div>
                          <div>
                            <Label className="mb-1 block text-xs text-muted-foreground">
                              {t("settings.orden.filterMode")}
                            </Label>
                            <Select
                              value={rule.filterMode || "all"}
                              onValueChange={(value) => updateOrdenVisualRule(rule.id, { filterMode: value })}
                            >
                              <SelectTrigger>
                                <SelectValue />
                              </SelectTrigger>
                              <SelectContent>
                                <SelectItem value="all">{t("settings.orden.filterAll")}</SelectItem>
                                <SelectItem value="any">{t("settings.orden.filterAny")}</SelectItem>
                                <SelectItem value="none">{t("settings.orden.filterNone")}</SelectItem>
                              </SelectContent>
                            </Select>
                          </div>
                        </div>
                        <div>
                          <Label className="mb-1 block text-xs text-muted-foreground">
                            {t("settings.orden.action")}
                          </Label>
                          <Select
                            value={rule.action}
                            onValueChange={(value) => updateOrdenVisualRule(rule.id, { action: value })}
                          >
                            <SelectTrigger>
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              <SelectItem value="copy">{t("settings.orden.actionCopy")}</SelectItem>
                              <SelectItem value="move">{t("settings.orden.actionMove")}</SelectItem>
                              <SelectItem value="rename">{t("settings.orden.actionRename")}</SelectItem>
                              <SelectItem value="extract">{t("settings.orden.actionExtract")}</SelectItem>
                              <SelectItem value="compress">{t("settings.orden.actionCompress")}</SelectItem>
                              <SelectItem value="echo">{t("settings.orden.actionEcho")}</SelectItem>
                            </SelectContent>
                          </Select>
                        </div>
                        <div>
                          <Label className="mb-1 block text-xs text-muted-foreground">
                            {t("settings.orden.destinations")}
                          </Label>
                          <textarea
                            value={rule.destination}
                            onChange={(e) => updateOrdenVisualRule(rule.id, { destination: e.target.value })}
                            placeholder="~/Documents/Shelfy Backups/"
                            className="min-h-20 w-full resize-y rounded-lg border border-border bg-background px-3 py-2 text-xs leading-5 text-foreground outline-none focus:border-ring focus:ring-2 focus:ring-ring/20"
                          />
                          <div className="mt-2 flex flex-wrap gap-2">
                            <Button
                              onClick={() => handleChooseOrdenDestinations(rule.id)}
                              variant="outline"
                              size="sm"
                            >
                              <FolderOpen size={14} />
                              {t("settings.orden.chooseDestinations")}
                            </Button>
                          </div>
                        </div>
                        {["extract", "compress"].includes(rule.action) && (
                          <>
                            <div>
                              <Label className="mb-1 block text-xs text-muted-foreground">
                                {t("settings.orden.archiveFormat")}
                              </Label>
                              <Select
                                value={rule.archiveFormat || "auto"}
                                onValueChange={(value) => updateOrdenVisualRule(rule.id, { archiveFormat: value })}
                              >
                                <SelectTrigger>
                                  <SelectValue />
                                </SelectTrigger>
                                <SelectContent>
                                  <SelectItem value="auto">{t("settings.orden.archiveFormatAuto")}</SelectItem>
                                  <SelectItem value="zip">ZIP</SelectItem>
                                  <SelectItem value="7z">7z</SelectItem>
                                  <SelectItem value="rar">RAR</SelectItem>
                                </SelectContent>
                              </Select>
                            </div>
                            <div>
                              <Label className="mb-1 block text-xs text-muted-foreground">
                                {rule.action === "extract"
                                  ? t("settings.orden.archivePasswords")
                                  : t("settings.orden.archivePassword")}
                              </Label>
                              <Input
                                value={rule.action === "extract" ? rule.archivePasswords : rule.archivePassword}
                                onChange={(e) =>
                                  updateOrdenVisualRule(
                                    rule.id,
                                    rule.action === "extract"
                                      ? { archivePasswords: e.target.value }
                                      : { archivePassword: e.target.value }
                                  )
                                }
                                placeholder={rule.action === "extract" ? "123456, password" : "optional password"}
                                type="password"
                              />
                            </div>
                            <div>
                              <Label className="mb-1 block text-xs text-muted-foreground">
                                {t("settings.orden.onConflict")}
                              </Label>
                              <Select
                                value={rule.onConflict || "rename_new"}
                                onValueChange={(value) => updateOrdenVisualRule(rule.id, { onConflict: value })}
                              >
                                <SelectTrigger>
                                  <SelectValue />
                                </SelectTrigger>
                                <SelectContent>
                                  <SelectItem value="rename_new">rename_new</SelectItem>
                                  <SelectItem value="skip">skip</SelectItem>
                                  <SelectItem value="overwrite">overwrite</SelectItem>
                                  <SelectItem value="trash">trash</SelectItem>
                                  <SelectItem value="rename_existing">rename_existing</SelectItem>
                                  <SelectItem value="deduplicate">deduplicate</SelectItem>
                                </SelectContent>
                              </Select>
                            </div>
                            <Label className="flex items-center gap-2 self-end text-sm">
                              <Checkbox
                                checked={rule.deleteOriginal || false}
                                onCheckedChange={(checked) =>
                                  updateOrdenVisualRule(rule.id, { deleteOriginal: checked === true })
                                }
                              />
                              {t("settings.orden.deleteOriginal")}
                            </Label>
                          </>
                        )}
                        <div>
                          <Label className="mb-1 block text-xs text-muted-foreground">
                            {t("settings.orden.targets")}
                          </Label>
                          <Select
                            value={rule.targets}
                            onValueChange={(value) => updateOrdenVisualRule(rule.id, { targets: value })}
                          >
                            <SelectTrigger>
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              <SelectItem value="files">{t("settings.orden.targetFiles")}</SelectItem>
                              <SelectItem value="dirs">{t("settings.orden.targetDirs")}</SelectItem>
                            </SelectContent>
                          </Select>
                        </div>
                        <div>
                          <Label className="mb-1 block text-xs text-muted-foreground">
                            {t("settings.orden.tags")}
                          </Label>
                          <Input
                            value={rule.tags}
                            onChange={(e) => updateOrdenVisualRule(rule.id, { tags: e.target.value })}
                            placeholder="backup, docs"
                          />
                        </div>
                      </div>

                      <Label className="flex items-center gap-2 text-sm">
                        <Checkbox
                          checked={rule.subfolders}
                          onCheckedChange={(checked) =>
                            updateOrdenVisualRule(rule.id, { subfolders: checked === true })
                          }
                        />
                        {t("settings.orden.subfolders")}
                      </Label>
                    </Card>
                  ))}
                </div>
              )}
            </div>

              </>
            )}
          </div>
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
    </div>
  );
}
