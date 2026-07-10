import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { OrdenQuickTask, useAppStore } from "../store/useAppStore";
import { BrandMark } from "./BrandMark";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { Card } from "./ui/card";
import { AnimatedIcon } from "./ui/animated-icon";
import { Tooltip, TooltipContent, TooltipTrigger } from "./ui/tooltip";
import {
  FolderOpen,
  Play,
  RotateCcw,
  X,
  FileText,
  Image,
  Music,
  Video,
  Archive,
  Package,
  File,
  ExternalLink,
  Inbox,
  ScanSearch,
  ListChecks,
  SlidersHorizontal,
  Wand2,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

function getIconForType(typeName: string) {
  const lower = typeName.toLowerCase();
  if (lower.includes("image")) return <Image size={14} />;
  if (lower.includes("music")) return <Music size={14} />;
  if (lower.includes("video")) return <Video size={14} />;
  if (lower.includes("archive")) return <Archive size={14} />;
  if (lower.includes("install")) return <Package size={14} />;
  if (lower.includes("document")) return <FileText size={14} />;
  return <File size={14} />;
}

function getFolderFromPath(filePath: string | null | undefined): string | null {
  if (!filePath) return null;
  const lastSlash = Math.max(filePath.lastIndexOf("/"), filePath.lastIndexOf("\\"));
  if (lastSlash === -1) return filePath;
  return filePath.slice(0, lastSlash) || filePath;
}

export default function Popup() {
  const { t } = useTranslation();
  const {
    logs,
    stats,
    isLoading,
    loadLogs,
    loadStats,
    loadFolders,
    undoAction,
    folders,
    pendingFiles,
    getPendingFiles,
    getOrdenQuickTasks,
    runOrdenQuickTask,
  } = useAppStore();
  const [ordenTasks, setOrdenTasks] = useState<OrdenQuickTask[]>([]);
  const [taskBusy, setTaskBusy] = useState<string | null>(null);
  const [taskResult, setTaskResult] = useState<{ name: string; count: number; simulated: boolean } | null>(null);
  const [toast, setToast] = useState<{
    file: string;
    rule: string;
    destination: string;
    destination_folder: string;
  } | null>(null);

  useEffect(() => {
    const refreshTasks = () => {
      if (document.visibilityState !== "visible" || !document.hasFocus()) return;
      getOrdenQuickTasks()
        .then(setOrdenTasks)
        .catch((e) => console.error("Failed to load Orden quick tasks:", e));
    };
    const refreshPending = () => {
      if (document.visibilityState !== "visible" || !document.hasFocus()) return;
      getPendingFiles().catch((e) => console.error("Failed to load pending files:", e));
    };

    Promise.all([loadLogs(), loadStats(), loadFolders(), getPendingFiles()]).catch(console.error);
    refreshTasks();

    // Listen for file-organized events from Rust watcher — show in-app toast
    const unlisten = listen("file-organized", (event: any) => {
      if (!document.hasFocus()) return;
      const payload = event.payload;
      if (payload?.success) {
        const destFolder: string = payload.destination_folder || payload.destination;
        // Show in-app toast (popup must be open/visible for this to appear)
        setToast({
          file: payload.file,
          rule: payload.rule,
          destination: payload.destination,
          destination_folder: destFolder,
        });
        setTimeout(() => setToast(null), 30000);
        loadLogs();
        loadStats();
        getPendingFiles();
      }
    });

    const pendingInterval = window.setInterval(refreshPending, 15000);
    const taskInterval = window.setInterval(refreshTasks, 60000);
    const handleFocus = () => {
      loadLogs();
      loadStats();
      refreshPending();
      refreshTasks();
    };
    window.addEventListener("focus", handleFocus);

    return () => {
      unlisten.then((f) => f());
      window.clearInterval(pendingInterval);
      window.clearInterval(taskInterval);
      window.removeEventListener("focus", handleFocus);
    };
  }, [loadLogs, loadStats, loadFolders, getPendingFiles, getOrdenQuickTasks]);

  const handleRunOrdenTask = async (task: OrdenQuickTask, simulate: boolean) => {
    setTaskBusy(`${task.ruleId}:${simulate ? "sim" : "run"}`);
    try {
      const result = await runOrdenQuickTask(task.yaml, simulate);
      setTaskResult({ name: task.ruleName, count: result.success, simulated: simulate });
      setTimeout(() => setTaskResult(null), 12000);
      if (!simulate) {
        await Promise.all([loadLogs(), loadStats(), getPendingFiles()]);
      }
      await invoke("show_notification", {
        title: t("app.name"),
        body: simulate
          ? t("popup.taskSimulated", { name: task.ruleName, count: result.success })
          : t("popup.taskRan", { name: task.ruleName, count: result.success }),
      });
    } catch (e) {
      console.error("Failed to run Orden quick task:", e);
      setTaskResult({ name: String(e || task.ruleName), count: 0, simulated: simulate });
    } finally {
      setTaskBusy(null);
    }
  };

  const handleOpenOrdenSettings = async () => {
    await invoke("show_settings_cmd", { section: "advanced" });
  };

  const handleOpenDownloads = async () => {
    const downloads = folders[0]?.path || (await invoke<string>("get_downloads_folder"));
    await invoke("open_folder_cmd", { path: downloads });
  };

  const handleOpenActionFolder = async (filePath: string | null | undefined) => {
    const folderPath = getFolderFromPath(filePath);
    if (!folderPath) return;
    try {
      await invoke("open_folder_cmd", { path: folderPath });
    } catch {
      console.error("Failed to open folder");
    }
  };

  const handleQuit = () => {
    invoke("close_popup");
  };

  const totalStats = useMemo(() => stats.reduce((sum, stat) => sum + stat.count, 0), [stats]);
  const visibleStats = useMemo(
    () => [...stats].sort((left, right) => right.count - left.count).slice(0, 4),
    [stats]
  );

  return (
    <div className="glass-panel flex h-full flex-col overflow-hidden rounded-xl text-foreground">
      {/* Header */}
      <div data-tauri-drag-region className="flex items-center justify-between border-b border-border/60 bg-card/45 px-3 py-2.5">
        <BrandMark showLabel />
        <Tooltip>
          <TooltipTrigger asChild>
            <Button onClick={handleQuit} variant="ghost" size="icon" aria-label={t("popup.quit")}>
              <X size={14} />
            </Button>
          </TooltipTrigger>
          <TooltipContent>{t("popup.quit")}</TooltipContent>
        </Tooltip>
      </div>

      {/* Quick tasks */}
      <div className="flex flex-col gap-2 px-3 py-2.5">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 text-xs font-semibold uppercase text-muted-foreground">
            <ListChecks size={14} />
            {t("popup.quickTasks")}
          </div>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button onClick={handleOpenOrdenSettings} variant="ghost" size="icon" className="size-7">
                <SlidersHorizontal size={13} />
              </Button>
            </TooltipTrigger>
            <TooltipContent>{t("popup.manageTasks")}</TooltipContent>
          </Tooltip>
        </div>

        {ordenTasks.length === 0 ? (
          <Card className="px-3 py-3 text-xs text-muted-foreground">
            <div className="mb-2 flex items-center gap-2 font-medium text-foreground">
              <Wand2 size={14} className="text-primary" />
              {t("popup.noQuickTasks")}
            </div>
            <Button onClick={handleOpenOrdenSettings} variant="outline" size="sm" className="w-full">
              <SlidersHorizontal size={13} />
              {t("popup.createTask")}
            </Button>
          </Card>
        ) : (
          <div className="flex flex-col gap-1.5">
            {ordenTasks.slice(0, 3).map((task) => {
              const busyRun = taskBusy === `${task.ruleId}:run`;
              const busySim = taskBusy === `${task.ruleId}:sim`;
              return (
                <Card key={task.ruleId} className="px-2.5 py-2">
                  <div className="flex items-start gap-2">
                    <div className="mt-0.5 rounded-lg bg-primary/10 p-1 text-primary">
                      <ListChecks size={13} />
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-1.5">
                        <span className="truncate text-xs font-medium">{task.ruleName}</span>
                        {!task.enabled && <Badge variant="secondary">{t("popup.disabled")}</Badge>}
                      </div>
                      <div className="mt-0.5 truncate text-[10px] text-muted-foreground">
                        {task.action} · {task.location || task.configName}
                      </div>
                    </div>
                    <Button
                      onClick={() => handleRunOrdenTask(task, true)}
                      disabled={!!taskBusy || !task.enabled}
                      variant="ghost"
                      size="icon"
                      className="size-7 text-muted-foreground"
                      aria-label={t("popup.simulateTask")}
                    >
                      {busySim ? "..." : <ScanSearch size={13} />}
                    </Button>
                    <Button
                      onClick={() => handleRunOrdenTask(task, false)}
                      disabled={!!taskBusy || !task.enabled}
                      size="icon"
                      className="size-7"
                      aria-label={t("popup.runTask")}
                    >
                      {busyRun || isLoading ? "..." : <Play size={13} />}
                    </Button>
                  </div>
                </Card>
              );
            })}
            {ordenTasks.length > 3 && (
              <Button onClick={handleOpenOrdenSettings} variant="outline" size="sm" className="w-full">
                {t("popup.moreTasks", { count: ordenTasks.length - 3 })}
              </Button>
            )}
          </div>
        )}

        {pendingFiles.length > 0 && (
          <div className="flex justify-center">
            <Badge variant="secondary">
              {t("popup.pendingFiles", { count: pendingFiles.length })}
            </Badge>
          </div>
        )}
        <Button onClick={handleOpenDownloads} variant="outline" className="w-full">
          <AnimatedIcon icon={FolderOpen} size={15} motion="float" />
          {t("popup.openDownloads")}
        </Button>
      </div>

      {/* Recent */}
      <div className="flex-1 overflow-auto border-t border-border/60 px-3 pt-3">
        <div className="mb-2 text-xs font-semibold uppercase text-muted-foreground">
          {t("popup.recentActions")}
        </div>
        {logs.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-6 text-sm text-muted-foreground">
            <Inbox size={28} className="mb-2 opacity-60" />
            {t("popup.noActions")}
          </div>
        ) : (
          <div className="flex flex-col gap-1.5">
            {logs.slice(0, 5).map((log) => (
              <div
                key={log.id}
                className="flex min-h-10 items-center gap-2 rounded-lg border border-border/80 bg-card px-2.5 py-2 text-xs shadow-sm"
              >
                <span className="shrink-0 text-muted-foreground">
                  {getIconForType(log.file_type)}
                </span>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <span className="min-w-0 flex-1 truncate font-medium">
                      {log.file_name}
                    </span>
                  </TooltipTrigger>
                  <TooltipContent>{log.file_name}</TooltipContent>
                </Tooltip>
                <span className="max-w-[80px] truncate text-muted-foreground">
                  {log.file_type}
                </span>
                {!log.undone && log.id && (
                  <>
                    <Tooltip>
                      <TooltipTrigger asChild>
                        <Button
                          onClick={() => handleOpenActionFolder(log.destination_path)}
                          variant="ghost"
                          size="icon"
                          className="size-6 text-muted-foreground"
                          aria-label={t("popup.openActionFolder")}
                        >
                          <FolderOpen size={12} />
                        </Button>
                      </TooltipTrigger>
                      <TooltipContent>{t("popup.openFolder", { folder: getFolderFromPath(log.destination_path) || "" })}</TooltipContent>
                    </Tooltip>
                    <Tooltip>
                      <TooltipTrigger asChild>
                        <Button
                          onClick={() => undoAction(log.id!)}
                          variant="ghost"
                          size="icon"
                          className="size-6 text-muted-foreground"
                          aria-label={t("popup.undo")}
                        >
                          <RotateCcw size={12} />
                        </Button>
                      </TooltipTrigger>
                      <TooltipContent>{t("popup.undo")}</TooltipContent>
                    </Tooltip>
                  </>
                )}
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Stats */}
      {stats.length > 0 && (
        <div className="border-t border-border/60 p-3">
          <div className="mb-2 text-xs font-semibold uppercase text-muted-foreground">
            {t("popup.weeklyStats")}
          </div>
          <div className="flex flex-col gap-1.5">
            {visibleStats.map((s) => {
              const pct = totalStats > 0 ? Math.round((s.count / totalStats) * 100) : 0;
              return (
                <div key={s.file_type} className="flex items-center gap-2 text-xs">
                  <span className="w-16 truncate text-muted-foreground">{s.file_type}</span>
                  <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-secondary">
                    <div
                      className="h-full rounded-full bg-primary transition-all"
                      style={{ width: `${pct}%` }}
                    />
                  </div>
                  <span className="w-8 text-right text-muted-foreground">{pct}%</span>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Clickable toast for auto-organized files */}
      {toast && (
        <div className="px-3 pb-2">
          <Card
            onPointerDown={async () => {
              try {
                await invoke("open_folder_cmd", { path: toast.destination_folder });
              } catch {
                console.error("Failed to open folder");
              }
              setToast(null);
            }}
            className="w-full cursor-pointer px-3 py-2 text-left text-xs transition-colors hover:bg-accent"
            role="button"
          >
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-1.5 text-primary">
                <ExternalLink size={12} />
                <span className="font-medium">{t("popup.organized", { file: toast.file })}</span>
              </div>
              <Button
                onClick={(e) => {
                  e.stopPropagation();
                  setToast(null);
                }}
                variant="ghost"
                size="icon"
                className="size-5 text-primary"
              >
                <X size={10} />
              </Button>
            </div>
            <div className="mt-0.5 truncate text-muted-foreground">
              {t("popup.openFolder", { folder: toast.destination_folder })}
            </div>
          </Card>
        </div>
      )}

      {/* Quick task result toast */}
      {taskResult && (
        <div className="px-3 pb-3">
          <div className="rounded-xl border border-primary/25 bg-primary/8 px-3 py-2 text-xs text-primary shadow-sm">
            {taskResult.simulated
              ? t("popup.taskSimulated", { name: taskResult.name, count: taskResult.count })
              : t("popup.taskRan", { name: taskResult.name, count: taskResult.count })}
          </div>
        </div>
      )}
    </div>
  );
}
