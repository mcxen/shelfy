import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  McpClientConfig,
  useAppStore,
  Rule,
  ScheduleSettings,
} from "../store/useAppStore";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { save, open } from "@tauri-apps/plugin-dialog";
import { BrandMark } from "./BrandMark";
import { GeneralTab } from "./settings/GeneralTab";
import { RulesTab } from "./settings/RulesTab";
import { OrdenTab } from "./settings/OrdenTab";
import { TopNavButton } from "./settings/TopNavButton";
import { ordenOperationLabel } from "../lib/ordenI18n";
import {
  defaultMcpDraft,
  defaultSchedule,
  formatDuration,
  getDirectoryFromPath,
  GRACE_STEPS,
  GraceUnit,
  MAX_GRACE_SECONDS,
  McpDraft,
  nearestGraceStep,
  secondsToUnit,
  unitToSeconds,
} from "./settings/utils";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { Card } from "./ui/card";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./ui/select";
import { AnimatedIcon } from "./ui/animated-icon";
import { Tooltip, TooltipContent, TooltipTrigger } from "./ui/tooltip";
import {
  FolderOpen,
  List,
  History,
  Inbox,
  Globe,
  Trash2,
  X,
  Check,
  RotateCcw,
  Code2,
  ShieldAlert,
  LayoutGrid,
  GripHorizontal,
} from "lucide-react";


type Tab = "rules" | "history" | "advanced" | "templates" | "general";
const SETTINGS_TABS: Tab[] = ["rules", "history", "advanced", "templates", "general"];

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
    checkUpdate,
    installUpdate,
    exportRules,
    importRules,
    exportConfig,
    importConfig,
    getMcpClientConfig,
    getMcpHelp,
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

  useEffect(() => {
    if (loadedTabs.current.has(tab)) return;
    loadedTabs.current.add(tab);

    const loadTabData = async () => {
      if (tab === "rules") {
        await Promise.all([loadRules(), loadFolders()]);
      } else if (tab === "history") {
        await loadLogs();
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
  ]);

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
    <div className="relative flex h-full flex-col overflow-hidden rounded-xl bg-background text-foreground">

      <header className="relative z-20 shrink-0 px-4 pt-3">
        <div
          data-tauri-drag-region
          className="absolute left-1/2 top-0.5 z-30 flex h-3 w-20 -translate-x-1/2 cursor-grab items-center justify-center rounded-full text-muted-foreground/55 hover:bg-muted/45 hover:text-muted-foreground active:cursor-grabbing"
          aria-hidden="true"
        >
          <GripHorizontal data-tauri-drag-region size={16} strokeWidth={1.8} />
        </div>
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
      <main className="relative z-10 min-w-0 flex-1 overflow-y-auto overscroll-contain px-4 pb-4 pt-3">
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
          <div className="flex flex-col gap-3">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <h2 className="text-lg font-semibold">{t("settings.history.title")}</h2>
              <div className="flex flex-1 flex-wrap items-center justify-end gap-1.5 sm:flex-none">
                <Select value={historyFilter} onValueChange={setHistoryFilter}>
                  <SelectTrigger className="w-full sm:w-[220px]">
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
                        size="sm"
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
                  size="sm"
                >
                  <Trash2 size={14} />
                  {t("settings.history.clear")}
                </Button>
              </div>
            </div>
            {filteredHistoryLogs.length === 0 ? (
              <div className="flex flex-col items-center justify-center py-8 text-muted-foreground">
                <Inbox size={32} className="mb-2 opacity-50" />
                <span>{t("settings.history.empty")}</span>
              </div>
            ) : (
              <div className="flex flex-col gap-1.5">
                {filteredHistoryLogs.map((log) => (
                  <Card
                    key={log.id}
                    className={`grid gap-2 px-3 py-2.5 md:grid-cols-[minmax(0,1fr)_minmax(15rem,0.85fr)_auto] md:items-center ${
                      log.undone ? "bg-muted opacity-70" : ""
                    }`}
                  >
                    <div className="min-w-0 flex-1">
                      <div className="flex flex-wrap items-center gap-2">
                        <div className="truncate text-sm font-medium">{log.file_name}</div>
                        <Badge variant="outline">{log.engine}</Badge>
                        <Badge variant="secondary">{ordenOperationLabel(t, log.action)}</Badge>
                      </div>
                      <div className="mt-0.5 text-xs text-muted-foreground">
                        {new Date(log.timestamp).toLocaleString()} · {log.rule_label || log.file_type}
                      </div>
                    </div>
                    <div className="min-w-0 text-xs">
                      <div className="truncate text-muted-foreground" title={log.source_path}>
                        {t("settings.history.source")}: {log.source_path}
                      </div>
                      <div className="mt-0.5 truncate" title={log.destination_path || undefined}>
                        {t("settings.history.destination")}: {log.destination_path || "—"}
                      </div>
                    </div>
                    <div className="flex shrink-0 items-center gap-1.5">
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

        <OrdenTab
          activeTab={tab}
          onOpenAdvanced={() => setTab("advanced")}
          onOpenHistory={() => openHistoryFilter("engine:orden")}
          onFolderAccessError={setFolderAccessError}
        />

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
            getMcpHelp={getMcpHelp}
            handleExportConfig={handleExportConfig}
            handleImportConfig={handleImportConfig}
            replaceConfigOnImport={replaceConfigOnImport}
            setReplaceConfigOnImport={setReplaceConfigOnImport}
            configToast={configToast}
            checkUpdate={checkUpdate}
            installUpdate={installUpdate}
          />
        )}


        </div>
      </main>
    </div>
  );
}
