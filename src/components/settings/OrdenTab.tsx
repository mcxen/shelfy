import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { DEFAULT_ORDEN_EXAMPLE, OrdenJob, OrdenRunResult, OrdenTemplate, OrdenVisualConfig, OrdenVisualRule, useAppStore } from "../../store/useAppStore";
import { defaultOrdenVisualConfig, mergePathText, normalizeDialogSelection, OrdenEditorMode, OrdenView, visualToOrdenYaml, yamlQuote } from "./utils";
import { OrdenPreview } from "./OrdenPreview";
import { OrdenRunHistoryTable } from "./OrdenRunHistoryTable";
import { OrdenVisualRuleCard } from "./OrdenVisualRuleCard";
import { OrdenTemplateCenter } from "./OrdenTemplateCenter";
import { AlertDialog, AlertDialogClose, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogPopup, AlertDialogTitle } from "../ui/alert-dialog";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { Card } from "../ui/card";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import { Menu, MenuGroup, MenuGroupLabel, MenuItem, MenuPopup, MenuSeparator, MenuTrigger } from "../ui/menu";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../ui/select";
import { Switch } from "../ui/switch";
import { Table, TableBody, TableCell, TableFooter, TableHead, TableHeader, TableRow } from "../ui/table";
import { TagInput } from "../ui/tag-input";
import { Tooltip, TooltipContent, TooltipTrigger } from "../ui/tooltip";
import { Braces, ChevronDown, ChevronLeft, ChevronRight, Copy, Eye, FileCheck2, History, MoreHorizontal, Pause, Pencil, Play, Plus, Save, ScanSearch, Search, StickyNote, Trash2, X } from "lucide-react";
import { ordenOperationLabel } from "../../lib/ordenI18n";

type OrdenTabProps = { activeTab: "advanced" | "templates" | string; onOpenAdvanced: () => void; onOpenHistory: () => void; onFolderAccessError: (error: { path: string; error: string; permission_denied: boolean } | null) => void; };

export function OrdenTab({ activeTab, onOpenAdvanced, onOpenHistory, onFolderAccessError }: OrdenTabProps) {
  const { t } = useTranslation();
  const loadedTabs = useRef(new Set<string>());
  const {
    ordenList, ordenLoad, ordenSave, ordenRename, ordenDuplicate, ordenDelete, ordenTemplateList, ordenTemplateSave,
    ordenTemplateDelete, ordenCheck, ordenRun, ordenVisualFromYaml, ordenHistory,
    ordenDeleteHistory, ordenClearHistory, ordenJobs, ordenSaveJob, ordenDeleteJob,
    ordenRunJob, loadLogs, loadStats, validateFolderAccess,
  } = useAppStore();

  const [ordenConfigs, setOrdenConfigs] = useState<string[]>([]);
  const [ordenTemplates, setOrdenTemplates] = useState<OrdenTemplate[]>([]);
  const [ordenName, setOrdenName] = useState("main");
  const [editingConfigName, setEditingConfigName] = useState<string | null>(null);
  const [ordenDirty, setOrdenDirty] = useState(false);
  const [discardEditorOpen, setDiscardEditorOpen] = useState(false);
  const [previewReturnView, setPreviewReturnView] = useState<OrdenView>("list");
  const [ordenYaml, setOrdenYaml] = useState(DEFAULT_ORDEN_EXAMPLE);
  const [ordenEditorMode, setOrdenEditorMode] = useState<OrdenEditorMode>("visual");
  const [ordenSourceExpanded, setOrdenSourceExpanded] = useState(false);
  const [ordenView, setOrdenView] = useState<OrdenView>("list");
  const [ordenPreviewError, setOrdenPreviewError] = useState<string | null>(null);
  const [ordenVisual, setOrdenVisual] = useState<OrdenVisualConfig>(defaultOrdenVisualConfig());
  const [ordenTags, setOrdenTags] = useState("");
  const [ordenSkipTags, setOrdenSkipTags] = useState("");
  const [ordenResult, setOrdenResult] = useState<OrdenRunResult | null>(null);
  const [ordenHistoryRows, setOrdenHistoryRows] = useState<import("../../store/useAppStore").OrdenRunHistory[]>([]);
  const [ordenHistoryByConfig, setOrdenHistoryByConfig] = useState<Record<string, import("../../store/useAppStore").OrdenRunHistory[]>>({});
  const [ordenSearch, setOrdenSearch] = useState("");
  const [ordenNotes, setOrdenNotes] = useState<Record<string, string>>(() => {
    try { return JSON.parse(localStorage.getItem("shelfy.orden.notes") || "{}"); } catch { return {}; }
  });
  const [ordenDetailName, setOrdenDetailName] = useState<string | null>(null);
  const [ordenJobsRows, setOrdenJobsRows] = useState<OrdenJob[]>([]);
  const [editingOrdenJob, setEditingOrdenJob] = useState<OrdenJob | null>(null);
  const [ordenBusy, setOrdenBusy] = useState(false);
  const [ordenToast, setOrdenToast] = useState<{ message: string; type: "success" | "error" } | null>(null);
  const reportFolderAccessError = (error: { path: string; error: string; permission_denied: boolean } | null) => {
    onFolderAccessError(error);
  };

  const loadOrdenConfigs = async (preferredName?: string) => {
    const names = await ordenList();
    setOrdenConfigs(names);
    Promise.all(names.map(async (name) => [name, await ordenHistory(name, 1)] as const))
      .then((rows) => setOrdenHistoryByConfig(Object.fromEntries(rows)))
      .catch(console.error);
    const nameToLoad = preferredName || (ordenName && names.includes(ordenName) ? ordenName : names[0]);
    if (nameToLoad) setOrdenName(nameToLoad);
    else {
      setOrdenName("main");
      setOrdenYaml(DEFAULT_ORDEN_EXAMPLE);
      setOrdenHistoryRows([]);
    }
  };

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
    setOrdenDirty(true);
    setOrdenVisual((prev) => {
      const next = {
        rules: prev.rules.map((rule) => (rule.id === id ? { ...rule, ...patch } : rule)),
      };
      setOrdenYaml(visualToOrdenYaml(next));
      return next;
    });
  };

  const handleAddOrdenVisualRule = () => {
    setOrdenDirty(true);
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
    setOrdenDirty(true);
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
          reportFolderAccessError({
            path: denied.path,
            error: denied.error || t("settings.permissions.folderDenied"),
            permission_denied: denied.permission_denied,
          });
          return;
        }
      }
      reportFolderAccessError(null);
      const rule = ordenVisual.rules.find((item) => item.id === id);
      updateOrdenVisualRule(id, {
        location: mergePathText(rule?.location || "", paths),
      });
    } catch (error) {
      reportFolderAccessError({ path: "", error: String(error), permission_denied: true });
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
        reportFolderAccessError({
          path: denied.path,
          error: denied.error || t("settings.permissions.folderDenied"),
          permission_denied: denied.permission_denied,
        });
        return;
      }
      reportFolderAccessError(null);
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
      reportFolderAccessError({ path: "", error: String(error), permission_denied: true });
    }
  };

  const handleOrdenSelect = async (name: string) => {
    try {
      const [yaml, history] = await Promise.all([ordenLoad(name), ordenHistory(name, 100)]);
      setOrdenName(name);
      setEditingConfigName(name);
      setOrdenYaml(yaml);
      setOrdenHistoryRows(history);
      setOrdenHistoryByConfig((previous) => ({ ...previous, [name]: history }));
      if (ordenEditorMode === "visual") {
        await parseOrdenVisual(yaml);
      }
      setOrdenDirty(false);
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
      const cleanName = name.replace(/\.ya?ml$/i, "");
      const previousName = editingConfigName;
      if (previousName && previousName !== cleanName) {
        await ordenRename(previousName, cleanName, yaml);
        setOrdenNotes((previous) => {
          const next = { ...previous, [cleanName]: previous[previousName] || "" };
          delete next[previousName];
          localStorage.setItem("shelfy.orden.notes", JSON.stringify(next));
          return next;
        });
        setOrdenHistoryByConfig((previous) => {
          const next = { ...previous, [cleanName]: previous[previousName] || [] };
          delete next[previousName];
          return next;
        });
        setOrdenJobsRows(await ordenJobs());
      } else {
        await ordenSave(cleanName, yaml);
      }
      setOrdenName(cleanName);
      setEditingConfigName(cleanName);
      setOrdenDirty(false);
      setOrdenYaml(yaml);
      await loadOrdenConfigs(cleanName);
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
    if (template.automation) {
      const preset = newOrdenJob(configName);
      await ordenSaveJob({
        ...preset,
        name: `${configName}-schedule`,
        mode: template.automation.mode,
        cron_expr: template.automation.cron_expr,
        fixed_time: template.automation.fixed_time,
        interval_minutes: template.automation.interval_minutes,
        watch_paths: template.automation.watch_paths,
        path_exists: template.automation.path_exists,
      });
      setOrdenJobsRows(await ordenJobs());
    }
    const visual = await ordenVisualFromYaml(template.yaml);
    setOrdenName(configName);
    setEditingConfigName(configName);
    setOrdenYaml(template.yaml);
    setOrdenVisual(visual.rules.length > 0 ? visual : defaultOrdenVisualConfig());
    setOrdenEditorMode("visual");
    setOrdenDirty(false);
    setOrdenView("editor");
    await loadOrdenConfigs(configName);
    onOpenAdvanced();
    setOrdenToast({
      message: t(template.automation
        ? "settings.orden.templates.addedWithScheduleNotice"
        : "settings.orden.templates.addedNotice"),
      type: "success",
    });
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
    if (!editingConfigName) return;
    if (!window.confirm(t("settings.orden.deleteConfirm", { name: editingConfigName }))) return;
    try {
      await ordenDelete(editingConfigName);
      await loadOrdenConfigs();
      setEditingConfigName(null);
      setOrdenDirty(false);
      setOrdenResult(null);
      setOrdenPreviewError(null);
      setOrdenView("list");
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
    setEditingConfigName(null);
    setOrdenYaml(DEFAULT_ORDEN_EXAMPLE);
    setOrdenVisual(defaultOrdenVisualConfig());
    setOrdenEditorMode("visual");
    setOrdenResult(null);
    setOrdenPreviewError(null);
    setOrdenDirty(false);
    setOrdenView("editor");
  };

  const requestEditorExit = () => {
    if (ordenDirty) {
      setDiscardEditorOpen(true);
    } else {
      setOrdenView("list");
    }
  };

  const discardEditorChanges = () => {
    setOrdenDirty(false);
    setDiscardEditorOpen(false);
    setOrdenView("list");
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
      setOrdenPreviewError(null);
      setPreviewReturnView("list");
      setOrdenView("preview");
      setOrdenJobsRows(await ordenJobs());
      await refreshOrdenHistory(job.config_name);
    } catch (error) {
      setOrdenResult(null);
      setOrdenPreviewError(String(error || t("settings.orden.runError")));
      setPreviewReturnView("list");
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

  const runOrdenConfigByName = async (name: string, simulate: boolean, returnView: OrdenView = "list") => {
    setOrdenBusy(true);
    try {
      const yaml = await ordenLoad(name);
      setOrdenName(name);
      setOrdenYaml(yaml);
      setOrdenPreviewError(null);
      const result = await ordenRun(yaml, simulate, parseTagList(ordenTags), parseTagList(ordenSkipTags));
      setOrdenResult(result);
      const history = await ordenHistory(name, 20);
      setOrdenHistoryRows(history);
      setOrdenHistoryByConfig((previous) => ({ ...previous, [name]: history }));
      setPreviewReturnView(returnView);
      setOrdenView("preview");
      if (!simulate) await Promise.all([loadLogs(), loadStats()]);
    } catch (error) {
      setOrdenResult(null);
      setOrdenPreviewError(String(error || t("settings.orden.runError")));
      setPreviewReturnView(returnView);
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

  const duplicateOrdenConfigByName = async (name: string, yaml?: string) => {
    setOrdenBusy(true);
    try {
      const copyName = await ordenDuplicate(name, yaml);
      await loadOrdenConfigs(copyName);
      await handleOrdenSelect(copyName);
      showOrdenToast(t("settings.orden.duplicateSuccess", { name: copyName }), "success");
    } catch (error) {
      showOrdenToast(String(error || t("settings.orden.duplicateError")), "error");
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
      setPreviewReturnView("editor");
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
      setPreviewReturnView("editor");
      setOrdenView("preview");
      await refreshOrdenHistory(ordenName).catch(console.error);
      showOrdenToast(String(e || t("settings.orden.runError")), "error");
    } finally {
      setOrdenBusy(false);
    }
  };

  useEffect(() => {
    if (loadedTabs.current.has(activeTab)) return;
    loadedTabs.current.add(activeTab);
    if (activeTab === "advanced") {
      Promise.all([loadOrdenConfigs(), ordenJobs()]).then(([, jobs]) => setOrdenJobsRows(jobs)).catch(console.error);
    } else if (activeTab === "templates") {
      Promise.all([ordenList(), ordenTemplateList()]).then(([names, templates]) => { setOrdenConfigs(names); setOrdenTemplates(templates); }).catch(console.error);
    }
  }, [activeTab]);

  return (
    <>
        {activeTab === "advanced" && (
          <div className="space-y-3">
            {ordenView === "preview" ? (
              <OrdenPreview
                ordenResult={ordenResult}
                ordenPreviewError={ordenPreviewError}
                onBack={() => setOrdenView(previewReturnView)}
              />
            ) : ordenView === "detail" && ordenDetailName ? (() => {
              const history = ordenHistoryByConfig[ordenDetailName] || [];
              const jobs = configJobs(ordenDetailName);
              return <div className="space-y-3">
                <div className="flex items-center justify-between gap-3">
                  <div className="flex items-center gap-3">
                    <Button type="button" onClick={() => setOrdenView("list")} variant="ghost" size="icon" aria-label={t("settings.orden.backToList")}><ChevronLeft size={17} /></Button>
                    <div><h2 className="text-lg font-semibold">{ordenDetailName}</h2><p className="text-xs text-muted-foreground">{t("settings.orden.detailDesc")}</p></div>
                  </div>
                  <div className="flex items-center gap-2">
                    <Button type="button" onClick={() => runOrdenConfigByName(ordenDetailName, true, "detail")} variant="outline" disabled={ordenBusy}><ScanSearch size={14} />{t("settings.orden.tryRun")}</Button>
                    <Button type="button" onClick={() => void duplicateOrdenConfigByName(ordenDetailName)} variant="outline" disabled={ordenBusy}><Copy size={14} />{t("settings.orden.duplicate")}</Button>
                    <Button type="button" onClick={() => handleOrdenSelect(ordenDetailName)}><Pencil size={14} />{t("settings.orden.edit")}</Button>
                  </div>
                </div>
                <div className="grid gap-3 md:grid-cols-3">
                  <Card className="p-3"><div className="text-xs text-muted-foreground">{t("settings.orden.runs")}</div><div className="mt-0.5 text-xl font-semibold">{history.length}</div></Card>
                  <Card className="p-3"><div className="text-xs text-muted-foreground">{t("settings.orden.schedules")}</div><div className="mt-0.5 text-xl font-semibold">{jobs.length}</div></Card>
                  <Card className="p-3"><div className="text-xs text-muted-foreground">{t("settings.orden.lastResult")}</div><div className="mt-0.5 text-sm font-medium">{history[0] ? `${history[0].success} / ${history[0].errors}` : "—"}</div></Card>
                </div>
                <Card className="space-y-2 p-3">
                  <div><h3 className="font-medium">{t("settings.orden.previewRules")}</h3><p className="text-xs text-muted-foreground">{t("settings.orden.previewRulesDesc")}</p></div>
                  <Table>
                    <TableHeader><TableRow><TableHead>{t("settings.orden.rule")}</TableHead><TableHead>{t("settings.orden.locations")}</TableHead><TableHead>{t("settings.orden.filter")}</TableHead><TableHead>{t("settings.orden.action")}</TableHead><TableHead>{t("settings.orden.destination")}</TableHead></TableRow></TableHeader>
                    <TableBody>
                      {ordenVisual.rules.map((rule) => <TableRow key={rule.id}>
                        <TableCell><div className="font-medium">{rule.name}</div><div className="text-xs text-muted-foreground">{rule.enabled ? t("settings.orden.enabled") : t("settings.orden.stopped")}</div></TableCell>
                        <TableCell className="max-w-48 truncate text-xs text-muted-foreground" title={rule.location}>{rule.location || "—"}</TableCell>
                        <TableCell className="text-xs">{rule.filterSteps?.map((step) => `${step.inverted ? "not " : ""}${step.kind}`).join(", ") || rule.extensions || t("settings.orden.noFilter")} · {rule.filterMode || "all"}</TableCell>
                        <TableCell><div className="flex flex-wrap gap-1">{(rule.actionSteps?.length ? rule.actionSteps : [{ kind: rule.action }]).map((step, index) => <Badge key={`${step.kind}-${index}`} variant="outline">{ordenOperationLabel(t, step.kind)}</Badge>)}</div></TableCell>
                        <TableCell className="max-w-48 truncate text-xs text-muted-foreground" title={rule.actionSteps?.map((step) => step.value).join("\n") || rule.destination}>{rule.actionSteps?.[0]?.value || rule.destination || "—"}</TableCell>
                      </TableRow>)}
                    </TableBody>
                  </Table>
                </Card>
                <Card className="space-y-2 p-3"><Label>{t("settings.orden.note")}</Label><textarea value={ordenNotes[ordenDetailName] || ""} onChange={(event) => updateOrdenNote(ordenDetailName, event.target.value)} placeholder={t("settings.orden.notePlaceholder")} className="min-h-20 w-full resize-y rounded-lg border border-border bg-background px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-ring/20" /></Card>
                <OrdenRunHistoryTable
                  rows={history}
                  onRefresh={() => refreshOrdenHistory(ordenDetailName)}
                  onDelete={(id) => handleDeleteOrdenHistory(ordenDetailName, id)}
                  onClear={() => handleClearOrdenHistory(ordenDetailName)}
                />
              </div>;
            })() : (
              <>

            <div className="flex flex-wrap items-center justify-between gap-3">
              <div className="flex min-w-0 items-center gap-3">
                {ordenView === "editor" && (
                  <Button onClick={requestEditorExit} variant="ghost" size="icon" aria-label={t("settings.orden.backToList")}>
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
              <div className="flex flex-wrap items-center gap-2">
                {ordenView === "list" ? (
                  <>
                    <Button onClick={() => onOpenHistory()} variant="outline">
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

            <div className="space-y-3">
              {ordenView === "editor" && (
                <>
              <div className="space-y-3 rounded-lg border border-border/80 bg-card/70 p-3 shadow-sm">
              <div className="grid gap-2 min-[900px]:grid-cols-[minmax(0,1fr)_auto_auto_auto] min-[900px]:items-end">
                <div>
                  <Label className="mb-1 block text-xs text-muted-foreground">
                    {t("settings.orden.configName")}
                  </Label>
                  <Input
                    value={ordenName}
                    onChange={(e) => { setOrdenName(e.target.value); setOrdenDirty(true); }}
                    placeholder="main"
                  />
                  {editingConfigName && ordenName !== editingConfigName && <p className="mt-1 text-xs text-muted-foreground">{t("settings.orden.renameOnSave", { oldName: editingConfigName, newName: ordenName || "—" })}</p>}
                </div>
                <Button onClick={handleOrdenSave} variant="outline" disabled={ordenBusy}>
                  <Save size={14} />
                  {t("settings.orden.save")}
                </Button>
                {editingConfigName && (
                  <Button onClick={() => void duplicateOrdenConfigByName(editingConfigName, currentOrdenYaml())} variant="outline" disabled={ordenBusy}>
                    <Copy size={14} />
                    {t("settings.orden.duplicate")}
                  </Button>
                )}
                {editingConfigName && (
                  <Button
                    onClick={handleOrdenDelete}
                    variant="ghost"
                    disabled={ordenBusy}
                    className="text-destructive hover:bg-destructive/10 hover:text-destructive"
                  >
                    <Trash2 size={14} />
                    {t("settings.orden.delete")}
                  </Button>
                )}
              </div>

              <div className="grid gap-2 md:grid-cols-2">
                <div>
                  <Label className="mb-1 block text-xs text-muted-foreground">
                    {t("settings.orden.tags")}
                  </Label>
                  <TagInput
                    value={parseTagList(ordenTags)}
                    onChange={(tags) => setOrdenTags(tags.join(", "))}
                    placeholder="work, invoices"
                    ariaLabel={t("settings.orden.tags")}
                  />
                </div>
                <div>
                  <Label className="mb-1 block text-xs text-muted-foreground">
                    {t("settings.orden.skipTags")}
                  </Label>
                  <TagInput
                    value={parseTagList(ordenSkipTags)}
                    onChange={(tags) => setOrdenSkipTags(tags.join(", "))}
                    placeholder="never"
                    ariaLabel={t("settings.orden.skipTags")}
                  />
                </div>
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
                <div className="hidden overflow-visible rounded-lg border border-border min-[900px]:block">
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
                                  <MenuTrigger render={<Button type="button" variant="ghost" size="icon-sm" aria-label={t("settings.orden.moreActions")} />}>
                                    <MoreHorizontal size={15} />
                                  </MenuTrigger>
                                  <MenuPopup>
                                    <MenuGroup>
                                      <MenuGroupLabel>{t("settings.orden.configManagement")}</MenuGroupLabel>
                                      <MenuItem onClick={() => handleOrdenSelect(name)}><Pencil />{t("settings.orden.edit")}</MenuItem>
                                      <MenuItem onClick={() => void duplicateOrdenConfigByName(name)}><Copy />{t("settings.orden.duplicate")}</MenuItem>
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
                    {filteredOrdenConfigs.length === 0 && <TableBody><TableRow><TableCell colSpan={5} className="py-5 text-center text-muted-foreground">{t("settings.orden.noConfigs")}</TableCell></TableRow></TableBody>}
                    <TableFooter><TableRow><TableCell colSpan={4}>{t("settings.orden.totalConfigs")}</TableCell><TableCell className="text-right">{filteredOrdenConfigs.length}</TableCell></TableRow></TableFooter>
                  </Table>
                </div>
                <div className="space-y-2 min-[900px]:hidden">
                  {filteredOrdenConfigs.map((name) => {
                    const history = ordenHistoryByConfig[name] || [];
                    const last = history[0];
                    const jobs = configJobs(name);
                    const scheduled = jobs.some((job) => job.enabled);
                    const status = !last ? "ready" : last.errors > 0 ? "failed" : "success";
                    return <div key={name} className="rounded-lg border border-border p-3" data-state={name === ordenName ? "selected" : undefined}>
                      <div className="flex items-start justify-between gap-2">
                        <button type="button" className="min-w-0 text-left" onClick={() => handleOrdenPreview(name)}><div className="truncate text-sm font-medium">{name}</div><div className="mt-0.5 line-clamp-2 text-xs text-muted-foreground">{ordenNotes[name] || t("settings.orden.noNote")}</div></button>
                        <div className="flex shrink-0 items-center gap-1">
                          <Tooltip><TooltipTrigger asChild><Button type="button" onClick={() => runOrdenConfigByName(name, true)} variant="ghost" size="icon-sm" disabled={ordenBusy} aria-label={t("settings.orden.tryRun")}><ScanSearch size={14} /></Button></TooltipTrigger><TooltipContent>{t("settings.orden.tryRun")}</TooltipContent></Tooltip>
                          <Tooltip><TooltipTrigger asChild><Button type="button" onClick={() => handleOrdenPreview(name)} variant="ghost" size="icon-sm" aria-label={t("settings.orden.preview")}><Eye size={14} /></Button></TooltipTrigger><TooltipContent>{t("settings.orden.preview")}</TooltipContent></Tooltip>
                          <Menu>
                            <MenuTrigger render={<Button type="button" variant="ghost" size="icon-sm" aria-label={t("settings.orden.moreActions")} />}><MoreHorizontal size={15} /></MenuTrigger>
                            <MenuPopup><MenuGroup><MenuGroupLabel>{t("settings.orden.configManagement")}</MenuGroupLabel><MenuItem onClick={() => handleOrdenSelect(name)}><Pencil />{t("settings.orden.edit")}</MenuItem><MenuItem onClick={() => void duplicateOrdenConfigByName(name)}><Copy />{t("settings.orden.duplicate")}</MenuItem><MenuItem onClick={() => runOrdenConfigByName(name, false)}><Play />{t("settings.orden.run")}</MenuItem><MenuItem onClick={() => handleNewOrdenJob(name)}><Plus />{t("settings.orden.newTask")}</MenuItem><MenuItem onClick={() => { const note = window.prompt(t("settings.orden.notePrompt"), ordenNotes[name] || ""); if (note !== null) updateOrdenNote(name, note); }}><StickyNote />{t("settings.orden.addNote")}</MenuItem></MenuGroup><MenuSeparator /><MenuGroup><MenuGroupLabel>{t("settings.orden.taskManagement")}</MenuGroupLabel><MenuItem disabled={jobs.length === 0 || !scheduled} onClick={() => setConfigJobsEnabled(name, false)}><Pause />{t("settings.orden.stopSchedules")}</MenuItem><MenuItem disabled={jobs.length === 0 || scheduled} onClick={() => setConfigJobsEnabled(name, true)}><Play />{t("settings.orden.startSchedules")}</MenuItem></MenuGroup><MenuSeparator /><MenuItem variant="destructive" onClick={() => deleteOrdenConfigByName(name)}><Trash2 />{t("settings.orden.delete")}</MenuItem></MenuPopup>
                          </Menu>
                        </div>
                      </div>
                      <div className="mt-3 flex flex-wrap items-center gap-1.5 text-xs text-muted-foreground"><Badge variant="outline" className="gap-1.5"><span aria-hidden="true" className={`size-1.5 rounded-full ${status === "success" ? "bg-primary" : status === "failed" ? "bg-destructive" : "bg-muted-foreground/64"}`} />{t(`settings.orden.status_${status}`)}</Badge><Badge variant="outline" className="gap-1.5"><span aria-hidden="true" className={`size-1.5 rounded-full ${scheduled ? "bg-primary" : "bg-muted-foreground/64"}`} />{scheduled ? t("settings.orden.running") : t("settings.orden.stopped")}</Badge><span>{last ? new Date(last.timestamp).toLocaleString() : "—"}</span></div>
                    </div>;
                  })}
                  {filteredOrdenConfigs.length === 0 && <div className="rounded-lg border border-dashed border-border px-3 py-4 text-center text-sm text-muted-foreground">{t("settings.orden.noConfigs")}</div>}
                </div>
              </div>
              )}
            </div>

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

            {ordenView === "list" && <section className="space-y-3">
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
                <div className="grid gap-3 rounded-lg border border-border bg-muted/30 p-3 md:grid-cols-2">
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
                    <TagInput value={parseTagList(editingOrdenJob.tags)} onChange={(tags) => setEditingOrdenJob({ ...editingOrdenJob, tags: tags.join(", ") })} ariaLabel={t("settings.orden.tags")} />
                  </div>
                  <div>
                    <Label className="mb-1 block text-xs text-muted-foreground">{t("settings.orden.skipTags")}</Label>
                    <TagInput value={parseTagList(editingOrdenJob.skip_tags)} onChange={(tags) => setEditingOrdenJob({ ...editingOrdenJob, skip_tags: tags.join(", ") })} ariaLabel={t("settings.orden.skipTags")} />
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
              <div className="hidden overflow-hidden rounded-lg border border-border min-[900px]:block">
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
                        <TableCell><div className="flex justify-end gap-1"><Button onClick={() => handleRunOrdenJob(job)} disabled={ordenBusy} variant="ghost" size="icon-sm" aria-label={t("settings.orden.runTask", { name: job.name })}><Play size={14} /></Button><Button onClick={() => setEditingOrdenJob(job)} variant="ghost" size="icon-sm" aria-label={t("settings.orden.editTask", { name: job.name })}><Pencil size={14} /></Button><Button onClick={() => handleDeleteOrdenJob(job)} variant="ghost" size="icon-sm" aria-label={t("settings.orden.deleteTask", { name: job.name })} className="text-destructive hover:bg-destructive/10 hover:text-destructive"><Trash2 size={14} /></Button></div></TableCell>
                      </TableRow>
                    ))}
                    {ordenJobsRows.length === 0 && <TableRow><TableCell colSpan={6} className="py-5 text-center text-muted-foreground">{t("settings.orden.noTasks")}</TableCell></TableRow>}
                  </TableBody>
                </Table>
              </div>
              <div className="space-y-2 min-[900px]:hidden">
                {ordenJobsRows.map((job) => <div key={job.id || job.name} className="rounded-lg border border-border p-3">
                  <div className="flex items-start justify-between gap-2"><div className="min-w-0"><div className="truncate text-sm font-medium">{job.name}</div><div className="mt-1 flex flex-wrap items-center gap-1.5 text-xs text-muted-foreground"><span>{job.config_name}</span><Badge variant="outline">{job.mode}</Badge>{job.simulate && <span>{t("settings.orden.simulate")}</span>}</div></div><div className="flex shrink-0 items-center gap-1"><Tooltip><TooltipTrigger asChild><Button onClick={() => handleRunOrdenJob(job)} disabled={ordenBusy} variant="ghost" size="icon-sm" aria-label={t("settings.orden.runTask", { name: job.name })}><Play size={14} /></Button></TooltipTrigger><TooltipContent>{t("settings.orden.runTask", { name: job.name })}</TooltipContent></Tooltip><Tooltip><TooltipTrigger asChild><Button onClick={() => setEditingOrdenJob(job)} variant="ghost" size="icon-sm" aria-label={t("settings.orden.editTask", { name: job.name })}><Pencil size={14} /></Button></TooltipTrigger><TooltipContent>{t("settings.orden.editTask", { name: job.name })}</TooltipContent></Tooltip><Tooltip><TooltipTrigger asChild><Button onClick={() => handleDeleteOrdenJob(job)} variant="ghost" size="icon-sm" aria-label={t("settings.orden.deleteTask", { name: job.name })} className="text-destructive hover:bg-destructive/10 hover:text-destructive"><Trash2 size={14} /></Button></TooltipTrigger><TooltipContent>{t("settings.orden.deleteTask", { name: job.name })}</TooltipContent></Tooltip></div></div>
                  <div className="mt-3 flex flex-wrap items-center gap-2 text-xs text-muted-foreground"><Switch checked={job.enabled} onCheckedChange={(enabled) => ordenSaveJob({ ...job, enabled }).then(() => ordenJobs()).then(setOrdenJobsRows)} aria-label={t("settings.orden.toggleTask", { name: job.name })} /><span>{job.enabled ? t("settings.orden.running") : t("settings.orden.stopped")}</span><span>{job.last_run_at ? new Date(job.last_run_at).toLocaleString() : "—"}</span></div>
                </div>)}
                {ordenJobsRows.length === 0 && <div className="rounded-lg border border-dashed border-border px-3 py-4 text-center text-sm text-muted-foreground">{t("settings.orden.noTasks")}</div>}
              </div>
            </section>}

            {ordenView === "editor" && <>
            <OrdenRunHistoryTable
              rows={ordenHistoryRows}
              onRefresh={() => refreshOrdenHistory(ordenName)}
              onDelete={(id) => handleDeleteOrdenHistory(ordenName, id)}
              onClear={() => handleClearOrdenHistory(ordenName)}
            />

            <div className="space-y-3">
              <div className="flex items-center justify-between gap-2">
                <Label className="text-xs text-muted-foreground">{t("settings.orden.yaml")}</Label>
                <div className="flex rounded-lg border border-border bg-muted/30 p-0.5">
                  <Button
                    onClick={() => handleOrdenEditorModeChange("visual")}
                    variant={ordenEditorMode === "visual" ? "secondary" : "ghost"}
                    size="sm"
                  >
                    <Eye size={14} />
                    {t("settings.orden.visual")}
                  </Button>
                  <Button
                    onClick={() => handleOrdenEditorModeChange("source")}
                    variant={ordenEditorMode === "source" ? "secondary" : "ghost"}
                    size="sm"
                  >
                    <Braces size={14} />
                    {t("settings.orden.source")}
                  </Button>
                </div>
              </div>

              {ordenEditorMode === "source" ? (
                <div className="overflow-hidden rounded-lg border border-border bg-card">
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
                        onChange={(e) => { setOrdenYaml(e.target.value); setOrdenDirty(true); }}
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

        {activeTab === "templates" && (
          <OrdenTemplateCenter
            templates={ordenTemplates}
            configNames={ordenConfigs}
            onUseTemplate={handleUseOrdenTemplate}
            onLoadConfig={ordenLoad}
            onSaveTemplate={handleSaveOrdenTemplate}
            onDeleteTemplate={handleDeleteOrdenTemplate}
          />
        )}

        <AlertDialog open={discardEditorOpen} onOpenChange={setDiscardEditorOpen}>
          <AlertDialogPopup>
            <AlertDialogHeader>
              <AlertDialogTitle>{t("settings.orden.discardTitle")}</AlertDialogTitle>
              <AlertDialogDescription>{t("settings.orden.discardConfirm", { name: editingConfigName || ordenName })}</AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogClose render={<Button type="button" variant="outline" />}>{t("common.cancel")}</AlertDialogClose>
              <Button type="button" variant="destructive" onClick={discardEditorChanges}>{t("settings.orden.discard")}</Button>
            </AlertDialogFooter>
          </AlertDialogPopup>
        </AlertDialog>

    </>
  );
}
