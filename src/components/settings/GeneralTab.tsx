import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Activity, BookOpen, Bot, Copy, Download, FileCheck2, RefreshCw, Rocket, Save, ServerCog, SlidersHorizontal, Upload, X } from "lucide-react";
import { AppSettings, McpClientConfig, ScheduleSettings, SchedulerLog, UpdateInfo } from "../../store/useAppStore";
import { AnimatedIcon } from "../ui/animated-icon";
import { Button } from "../ui/button";
import { Card } from "../ui/card";
import { Checkbox } from "../ui/checkbox";
import { Dialog, DialogDescription, DialogFooter, DialogHeader, DialogPanel, DialogPopup, DialogTitle } from "../ui/dialog";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../ui/select";
import { Slider } from "../ui/slider";
import { Switch } from "../ui/switch";

type GraceUnit = "seconds" | "minutes" | "hours";
type Toast = { message: string; type: "success" | "error" } | null;
type McpDraft = Pick<
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

interface GeneralTabProps {
  settings: AppSettings | null;
  saveSettings: (settings: AppSettings) => Promise<void>;
  setAutostart: (enabled: boolean) => Promise<void>;
  handleChangeLanguage: (lang: string) => Promise<void>;
  graceValue: number;
  graceUnit: GraceUnit;
  graceError: string | null;
  currentGraceSeconds: number;
  graceSteps: number[];
  sliderIndex: number;
  formatDuration: (seconds: number) => string;
  handleGraceSliderChange: (index: number) => void;
  handleGraceNumberChange: (value: number, unit: GraceUnit) => void;
  localSchedule: ScheduleSettings;
  handleScheduleChange: (patch: Partial<ScheduleSettings>) => void;
  handleValidateCron: () => void;
  systemKeepaliveSupported: boolean;
  handleInstallSystemKeepalive: () => void;
  handleUninstallSystemKeepalive: () => void;
  scheduleToast: Toast;
  handleSaveSchedule: () => void;
  schedulerLogs: SchedulerLog[];
  loadSchedulerLogs: () => Promise<void>;
  clearSchedulerLogs: () => Promise<void>;
  localMcp: McpDraft;
  setLocalMcp: (draft: McpDraft) => void;
  handleSaveMcp: () => void;
  handleCopyMcpConfig: () => void;
  mcpClientConfig: McpClientConfig | null;
  mcpToast: Toast;
  getMcpHelp: (language: string) => Promise<string>;
  handleExportConfig: () => void;
  handleImportConfig: () => void;
  replaceConfigOnImport: boolean;
  setReplaceConfigOnImport: (replace: boolean) => void;
  configToast: Toast;
  checkUpdate: () => Promise<UpdateInfo>;
  installUpdate: (info: UpdateInfo) => Promise<void>;
}

export function GeneralTab({
  settings,
  saveSettings,
  setAutostart,
  handleChangeLanguage,
  graceValue,
  graceUnit,
  graceError,
  currentGraceSeconds,
  graceSteps,
  sliderIndex,
  formatDuration,
  handleGraceSliderChange,
  handleGraceNumberChange,
  localSchedule,
  handleScheduleChange,
  handleValidateCron,
  systemKeepaliveSupported,
  handleInstallSystemKeepalive,
  handleUninstallSystemKeepalive,
  scheduleToast,
  handleSaveSchedule,
  schedulerLogs,
  loadSchedulerLogs,
  clearSchedulerLogs,
  localMcp,
  setLocalMcp,
  handleSaveMcp,
  handleCopyMcpConfig,
  mcpClientConfig,
  mcpToast,
  getMcpHelp,
  handleExportConfig,
  handleImportConfig,
  replaceConfigOnImport,
  setReplaceConfigOnImport,
  configToast,
  checkUpdate,
  installUpdate,
}: GeneralTabProps) {
  const { t, i18n } = useTranslation();
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [updateBusy, setUpdateBusy] = useState(false);
  const [updateError, setUpdateError] = useState<string | null>(null);
  const [mcpHelpOpen, setMcpHelpOpen] = useState(false);
  const [mcpHelp, setMcpHelp] = useState("");
  const [mcpHelpBusy, setMcpHelpBusy] = useState(false);

  const handleOpenMcpHelp = async () => {
    setMcpHelpOpen(true);
    if (mcpHelp) return;
    setMcpHelpBusy(true);
    try {
      setMcpHelp(await getMcpHelp(i18n.resolvedLanguage || i18n.language || "en"));
    } catch (error) {
      setMcpHelp(String(error));
    } finally {
      setMcpHelpBusy(false);
    }
  };

  const handleCheckUpdate = async () => {
    setUpdateBusy(true);
    setUpdateError(null);
    try {
      setUpdateInfo(await checkUpdate());
    } catch (error) {
      setUpdateError(String(error));
    } finally {
      setUpdateBusy(false);
    }
  };

  const handleInstallUpdate = async () => {
    if (!updateInfo) return;
    setUpdateBusy(true);
    setUpdateError(null);
    try {
      await installUpdate(updateInfo);
    } catch (error) {
      setUpdateError(String(error));
      setUpdateBusy(false);
    }
  };

  return (
    <div className="flex w-full flex-col gap-3">
      <Card className="space-y-4 p-3">
        <h2 className="text-lg font-semibold">{t("settings.general.title")}</h2>

        <section className="space-y-2">
          <div>
            <h3 className="text-sm font-semibold">{t("settings.general.preferences", { defaultValue: "Preferences" })}</h3>
            <p className="text-xs text-muted-foreground">{t("settings.general.preferencesDesc", { defaultValue: "Language, appearance, and startup behavior." })}</p>
          </div>
          <div className="grid gap-2 md:grid-cols-3">
        <div className="min-w-0">
          <Label className="mb-1 block text-xs text-muted-foreground">{t("settings.general.language")}</Label>
          <Select value={settings?.language || "en"} onValueChange={handleChangeLanguage}>
            <SelectTrigger className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="en">English</SelectItem>
              <SelectItem value="pl">Polski</SelectItem>
              <SelectItem value="it">Italiano</SelectItem>
              <SelectItem value="de">Deutsch</SelectItem>
              <SelectItem value="fr">Français</SelectItem>
              <SelectItem value="ru">Русский</SelectItem>
              <SelectItem value="ja">日本語</SelectItem>
              <SelectItem value="zh">中文</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div className="min-w-0">
          <Label className="mb-1 block text-xs text-muted-foreground">{t("settings.general.theme")}</Label>
          <Select value={settings?.theme || "system"} onValueChange={(value) => settings && saveSettings({ ...settings, theme: value })}>
            <SelectTrigger className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="system">{t("settings.general.themeSystem")}</SelectItem>
              <SelectItem value="light">{t("settings.general.themeLight")}</SelectItem>
              <SelectItem value="dark">{t("settings.general.themeDark")}</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div className="flex min-h-9 items-center justify-between rounded-lg border border-border/60 bg-muted/20 px-3">
          <Label className="text-sm">{t("settings.general.startWithSystem")}</Label>
          <Switch checked={settings?.autostart || false} onCheckedChange={(checked) => setAutostart(checked)} />
        </div>
          </div>
        </section>

        <section className="space-y-2 border-t border-border/70 pt-3">
          <div>
            <h3 className="text-sm font-semibold">{t("settings.general.fileHandling", { defaultValue: "File handling" })}</h3>
            <p className="text-xs text-muted-foreground">{t("settings.general.fileHandlingDesc", { defaultValue: "Control when files are processed and how busy files are handled." })}</p>
          </div>
          <div className="grid gap-2 lg:grid-cols-[minmax(0,1.35fr)_minmax(18rem,.65fr)]">
        <div className="space-y-2 rounded-lg border border-border/60 bg-muted/15 p-2.5">
          <div className="flex items-center justify-between">
            <Label className="text-sm font-medium">{t("settings.general.gracePeriod")}</Label>
            <span className="text-xs text-muted-foreground">{formatDuration(currentGraceSeconds)}</span>
          </div>
          <Slider min={0} max={graceSteps.length - 1} step={1} value={[sliderIndex]} onValueChange={([value]) => handleGraceSliderChange(value)} />
          <div className="grid grid-cols-[minmax(0,1fr)_7.5rem] gap-2">
            <Input
              type="number"
              min={0}
              value={graceValue}
              onChange={(e) => handleGraceNumberChange(parseInt(e.target.value, 10) || 0, graceUnit)}
              className="w-full tabular-nums"
            />
            <Select value={graceUnit} onValueChange={(value) => handleGraceNumberChange(graceValue, value as GraceUnit)}>
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="seconds">{t("settings.general.gracePeriodSeconds")}</SelectItem>
                <SelectItem value="minutes">{t("settings.general.gracePeriodMinutes")}</SelectItem>
                <SelectItem value="hours">{t("settings.general.gracePeriodHours")}</SelectItem>
              </SelectContent>
            </Select>
          </div>
          {graceError && <p className="text-xs text-destructive">{graceError}</p>}
          <p className="text-xs text-muted-foreground">{t("settings.general.gracePeriodDesc")}</p>
        </div>

        <div className="flex items-center justify-between gap-3 rounded-lg border border-border/60 bg-muted/15 p-2.5">
          <div>
            <Label className="text-sm font-medium">{t("settings.general.checkFileLock")}</Label>
            <p className="text-xs text-muted-foreground">{t("settings.general.checkFileLockDesc")}</p>
          </div>
          <Switch
            checked={settings?.lock_check_enabled || false}
            onCheckedChange={(checked) => {
              if (!settings) return;
              saveSettings({ ...settings, lock_check_enabled: checked });
            }}
          />
        </div>
          </div>
        </section>
      </Card>

      <Card className="order-5 space-y-3 p-3">
        <div className="flex items-start justify-between gap-3">
          <div>
            <h3 className="flex items-center gap-2 text-base font-semibold">
              <Rocket size={16} className="text-primary" />
              {t("settings.general.updates", { defaultValue: "Application updates" })}
            </h3>
            <p className="text-xs text-muted-foreground">{t("settings.general.updatesDesc", { defaultValue: "Check for new releases and install compatible updates." })}</p>
          </div>
          <Button onClick={handleCheckUpdate} variant="outline" size="sm" disabled={updateBusy}>
            <RefreshCw size={14} className={updateBusy ? "animate-spin" : ""} />
            {updateBusy ? t("settings.general.checkingUpdates", { defaultValue: "Checking…" }) : t("settings.general.checkUpdates", { defaultValue: "Check for updates" })}
          </Button>
        </div>
        <div className="grid gap-2 text-xs sm:grid-cols-2">
          <div className="rounded-lg border border-border/60 bg-muted/25 px-3 py-2">
            <span className="text-muted-foreground">{t("settings.general.currentVersion", { defaultValue: "Current version" })}</span>
            <div className="mt-0.5 font-medium">{updateInfo?.current_version || "0.2.2"}</div>
          </div>
          <div className="rounded-lg border border-border/60 bg-muted/25 px-3 py-2">
            <span className="text-muted-foreground">{t("settings.general.latestRelease", { defaultValue: "Latest release" })}</span>
            <div className="mt-0.5 font-medium">{updateInfo?.latest_version || t("settings.general.notChecked", { defaultValue: "Not checked" })}</div>
          </div>
        </div>
        {updateInfo && (
          <div className="space-y-2 rounded-xl border border-border bg-card p-3 text-sm">
            <div className="flex items-center justify-between gap-2">
              <div>
                <div className="font-medium">{updateInfo.release_name}</div>
                <div className="text-xs text-muted-foreground">{updateInfo.available ? `Compatible asset: ${updateInfo.asset_name || "not found"}` : "Shelfy is up to date."}</div>
              </div>
              {updateInfo.available && updateInfo.asset_url && (
                <Button onClick={handleInstallUpdate} disabled={updateBusy}>
                  <Download size={14} />
                  {t("settings.general.installUpdate", { defaultValue: "Download, install & restart" })}
                </Button>
              )}
            </div>
            {updateInfo.release_notes && <p className="max-h-24 overflow-auto whitespace-pre-wrap text-xs text-muted-foreground">{updateInfo.release_notes}</p>}
          </div>
        )}
        {updateError && <div className="rounded-lg border border-destructive/20 bg-destructive/10 px-3 py-2 text-xs text-destructive">{updateError}</div>}
      </Card>

      <Card className="order-2 space-y-2.5 p-3">
        <div className="flex items-center justify-between gap-3">
          <div>
          <h3 className="flex items-center gap-2 text-base font-semibold">
            <SlidersHorizontal size={16} className="text-primary" />
            {t("settings.scheduler.title")}
          </h3>
          <p className="text-xs text-muted-foreground">{t("settings.scheduler.desc")}</p>
          </div>
          <Label className="flex shrink-0 items-center gap-2 text-sm text-muted-foreground">
          <span>{t("settings.scheduler.enable")}</span>
          <Switch checked={localSchedule.schedule_enabled} onCheckedChange={(checked) => handleScheduleChange({ schedule_enabled: checked })} />
          </Label>
        </div>

        <div className="grid gap-3 rounded-lg border border-border/70 bg-muted/10 p-2.5 lg:grid-cols-[12rem_minmax(0,1fr)] lg:items-end">
        <div>
          <Label className="mb-1 block text-xs text-muted-foreground">{t("settings.scheduler.timesPerDay")}</Label>
          <Select value={String(localSchedule.schedule_times_per_day)} onValueChange={(value) => handleScheduleChange({ schedule_times_per_day: parseInt(value, 10) })}>
            <SelectTrigger className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="1">{t("settings.scheduler.once")}</SelectItem>
              <SelectItem value="2">{t("settings.scheduler.twice")}</SelectItem>
              <SelectItem value="3">{t("settings.scheduler.thrice")}</SelectItem>
              <SelectItem value="4">{t("settings.scheduler.fourTimes")}</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <div className="grid gap-2 sm:grid-cols-2 xl:grid-cols-4">
          {Array.from({ length: localSchedule.schedule_times_per_day }).map((_, idx) => {
            const key = `schedule_time_${idx + 1}` as keyof ScheduleSettings;
            return (
              <div key={idx}>
                <Label className="mb-1 block text-xs text-muted-foreground">{t("settings.scheduler.time", { number: idx + 1 })}</Label>
                <Input
                  type="time"
                  value={(localSchedule[key] as string | null) || ""}
                  onChange={(e) => handleScheduleChange({ [key]: e.target.value || null } as Partial<ScheduleSettings>)}
                />
              </div>
            );
          })}
        </div>
        </div>

        <div className="grid gap-3 xl:grid-cols-2">
        <section className="space-y-2.5 rounded-lg border border-border/70 bg-muted/15 p-2.5">
          <div className="flex items-center justify-between gap-3">
            <div>
              <Label className="text-sm text-muted-foreground">{t("settings.scheduler.cronEnable")}</Label>
              <p className="text-xs text-muted-foreground">{t("settings.scheduler.cronDesc")}</p>
            </div>
            <Switch checked={localSchedule.schedule_cron_enabled} onCheckedChange={(checked) => handleScheduleChange({ schedule_cron_enabled: checked })} />
          </div>
          <div className="grid gap-2 md:grid-cols-[minmax(0,1fr)_auto] md:items-end">
            <div className="flex-1">
              <Label className="mb-1 block text-xs text-muted-foreground">{t("settings.scheduler.cronExpression")}</Label>
              <Input
                value={localSchedule.schedule_cron_expr || ""}
                onChange={(e) => handleScheduleChange({ schedule_cron_expr: e.target.value })}
                placeholder="*/30 * * * *"
                className="font-mono"
              />
            </div>
            <Button onClick={handleValidateCron} variant="outline">
              <FileCheck2 size={14} />
              {t("settings.scheduler.validateCron")}
            </Button>
          </div>
        </section>

        <section className="space-y-2.5 rounded-lg border border-border/70 bg-muted/15 p-2.5">
          <div className="flex items-center justify-between gap-3">
            <div>
              <Label className="flex items-center gap-2 text-sm text-muted-foreground">
                <Activity size={14} />
                {t("settings.scheduler.keepalive")}
              </Label>
              <p className="text-xs text-muted-foreground">{t("settings.scheduler.keepaliveDesc")}</p>
            </div>
            <Switch checked={localSchedule.keepalive_enabled} onCheckedChange={(checked) => handleScheduleChange({ keepalive_enabled: checked })} />
          </div>
          <div className="grid gap-2 xl:grid-cols-[minmax(0,180px)_auto_auto] xl:items-end">
            <div className="w-full">
              <Label className="mb-1 block text-xs text-muted-foreground">{t("settings.scheduler.keepaliveInterval")}</Label>
              <Input
                type="number"
                min={1}
                max={1440}
                value={localSchedule.keepalive_interval_minutes}
                onChange={(e) => handleScheduleChange({ keepalive_interval_minutes: parseInt(e.target.value, 10) || 15 })}
              />
            </div>
            <Button onClick={handleInstallSystemKeepalive} variant="outline" disabled={!systemKeepaliveSupported}>
              <ServerCog size={14} />
              {t("settings.scheduler.installKeepalive")}
            </Button>
            <Button onClick={handleUninstallSystemKeepalive} variant="ghost" disabled={!systemKeepaliveSupported}>
              <X size={14} />
              {t("settings.scheduler.uninstallKeepalive")}
            </Button>
          </div>
          {!systemKeepaliveSupported && <p className="text-xs text-muted-foreground">{t("settings.scheduler.keepaliveUnsupported")}</p>}
        </section>
        </div>

        {scheduleToast && (
          <div
            className={`rounded-lg border px-3 py-2 text-xs ${
              scheduleToast.type === "success"
                ? "border-primary/25 bg-primary/8 text-primary"
                : "border-destructive/20 bg-destructive/10 text-destructive"
            }`}
          >
            {scheduleToast.message}
          </div>
        )}

        <Button onClick={handleSaveSchedule}>
          <AnimatedIcon icon={Save} size={14} motion="pulse" />
          {t("settings.scheduler.save")}
        </Button>

        <div className="space-y-1.5">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <Label className="text-sm text-muted-foreground">{t("settings.scheduler.logs")}</Label>
            <div className="flex flex-wrap gap-1.5">
              <Button onClick={loadSchedulerLogs} variant="outline" size="sm">{t("settings.scheduler.refreshLogs")}</Button>
              <Button onClick={clearSchedulerLogs} variant="ghost" size="sm">{t("settings.scheduler.clearLogs")}</Button>
            </div>
          </div>
          <div className="max-h-40 overflow-auto rounded-lg border border-border bg-muted/30">
            {schedulerLogs.length === 0 ? (
              <div className="px-3 py-2 text-xs text-muted-foreground">{t("settings.scheduler.noLogs")}</div>
            ) : (
              <div className="divide-y divide-border">
                {schedulerLogs.map((log) => (
                  <div key={log.id} className="grid grid-cols-[70px_130px_minmax(0,1fr)] gap-2 px-3 py-1.5 text-xs">
                    <span className={log.level === "error" ? "text-destructive" : "text-muted-foreground"}>{log.level}</span>
                    <span className="text-muted-foreground">{log.event}</span>
                    <span className="min-w-0 truncate" title={log.details || log.message}>{log.message}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </Card>

      <Card className="order-3 space-y-2.5 p-3">
        <div>
          <h3 className="flex items-center gap-2 text-base font-semibold">
            <Bot size={16} className="text-primary" />
            {t("settings.mcp.title")}
          </h3>
          <p className="text-xs text-muted-foreground">{t("settings.mcp.desc")}</p>
        </div>

        <div className="flex max-w-4xl items-center justify-between rounded-lg border border-border/70 bg-muted/20 p-3">
          <div>
            <Label className="text-sm text-muted-foreground">{t("settings.mcp.enable")}</Label>
            <p className="text-xs text-muted-foreground">{t("settings.mcp.enableDesc")}</p>
          </div>
          <Switch checked={localMcp.mcp_enabled} onCheckedChange={(checked) => setLocalMcp({ ...localMcp, mcp_enabled: checked })} />
        </div>

        <div className="grid max-w-4xl gap-3 md:grid-cols-2">
          <div>
            <Label className="mb-1 block text-xs text-muted-foreground">{t("settings.mcp.serverName")}</Label>
            <Input value={localMcp.mcp_server_name} onChange={(e) => setLocalMcp({ ...localMcp, mcp_server_name: e.target.value })} placeholder="shelfy" />
          </div>
          <div>
            <Label className="mb-1 block text-xs text-muted-foreground">{t("settings.mcp.transport")}</Label>
            <Select value={localMcp.mcp_transport} onValueChange={(value) => setLocalMcp({ ...localMcp, mcp_transport: value })}>
              <SelectTrigger className="w-full sm:w-72">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="stdio">{t("settings.mcp.transportStdio")}</SelectItem>
                <SelectItem value="http">{t("settings.mcp.transportHttp")}</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>

        {localMcp.mcp_transport === "http" ? (
          <div className="grid max-w-4xl gap-3 md:grid-cols-2">
            <div>
              <Label className="mb-1 block text-xs text-muted-foreground">{t("settings.mcp.httpUrl")}</Label>
              <Input value={localMcp.mcp_http_url || ""} onChange={(e) => setLocalMcp({ ...localMcp, mcp_http_url: e.target.value })} placeholder="http://127.0.0.1:8765/mcp" />
            </div>
            <div>
              <Label className="mb-1 block text-xs text-muted-foreground">{t("settings.mcp.token")}</Label>
              <Input value={localMcp.mcp_token || ""} onChange={(e) => setLocalMcp({ ...localMcp, mcp_token: e.target.value })} placeholder="optional" />
            </div>
          </div>
        ) : (
          <div className="grid max-w-4xl gap-3 md:grid-cols-2">
            <div>
              <Label className="mb-1 block text-xs text-muted-foreground">{t("settings.mcp.command")}</Label>
              <Input value={localMcp.mcp_command || ""} onChange={(e) => setLocalMcp({ ...localMcp, mcp_command: e.target.value })} placeholder={t("settings.mcp.commandPlaceholder")} />
            </div>
            <div>
              <Label className="mb-1 block text-xs text-muted-foreground">{t("settings.mcp.args")}</Label>
              <Input value={localMcp.mcp_args || ""} onChange={(e) => setLocalMcp({ ...localMcp, mcp_args: e.target.value })} placeholder="--mcp" />
            </div>
          </div>
        )}

        <Label className="flex items-center gap-2 text-sm">
          <Checkbox checked={localMcp.mcp_allow_write} onCheckedChange={(checked) => setLocalMcp({ ...localMcp, mcp_allow_write: checked === true })} />
          {t("settings.mcp.allowWrite")}
        </Label>

        <div className="flex flex-wrap items-center gap-2">
          <Button onClick={handleSaveMcp}>
            <AnimatedIcon icon={Save} size={14} motion="pulse" />
            {t("settings.mcp.save")}
          </Button>
          <Button onClick={handleCopyMcpConfig} variant="outline" disabled={!mcpClientConfig}>
            <Copy size={14} />
            {t("settings.mcp.copyConfig")}
          </Button>
          <Button onClick={() => void handleOpenMcpHelp()} variant="outline">
            <BookOpen size={14} />
            {t("settings.mcp.help")}
          </Button>
        </div>

        {mcpToast && (
          <div
            className={`rounded-lg border px-3 py-2 text-xs ${
              mcpToast.type === "success"
                ? "border-primary/25 bg-primary/8 text-primary"
                : "border-destructive/20 bg-destructive/10 text-destructive"
            }`}
          >
            {mcpToast.message}
          </div>
        )}

        <div className="space-y-2">
          <Label className="text-xs text-muted-foreground">{t("settings.mcp.clientConfig")}</Label>
          <textarea
            value={mcpClientConfig?.config_json || ""}
            readOnly
            spellCheck={false}
            className="min-h-36 w-full resize-y rounded-lg border border-border bg-muted/30 px-3 py-2 font-mono text-xs leading-5 text-foreground outline-none"
          />
        </div>
      </Card>

      <div className="order-4 px-0.5 pt-1">
        <h2 className="text-sm font-semibold">{t("settings.general.maintenance", { defaultValue: "Maintenance" })}</h2>
        <p className="text-xs text-muted-foreground">{t("settings.general.maintenanceDesc", { defaultValue: "Updates, backup, and configuration migration." })}</p>
      </div>

      <Card className="order-5 space-y-2.5 p-3">
        <div>
          <h3 className="text-base font-semibold">{t("settings.config.title")}</h3>
          <p className="text-xs text-muted-foreground">{t("settings.config.desc")}</p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Button onClick={handleExportConfig} variant="outline">
            <AnimatedIcon icon={Download} size={14} motion="float" />
            {t("settings.config.export")}
          </Button>
          <Button onClick={handleImportConfig} variant="outline">
            <AnimatedIcon icon={Upload} size={14} motion="float" />
            {t("settings.config.import")}
          </Button>
        </div>
        <Label className="flex items-center gap-2 text-sm">
          <Checkbox checked={replaceConfigOnImport} onCheckedChange={(checked) => setReplaceConfigOnImport(checked === true)} />
          {t("settings.config.replaceOnImport")}
        </Label>
        {configToast && (
          <div
            className={`rounded-lg border px-3 py-2 text-xs ${
              configToast.type === "success"
                ? "border-primary/25 bg-primary/8 text-primary"
                : "border-destructive/20 bg-destructive/10 text-destructive"
            }`}
          >
            {configToast.message}
          </div>
        )}
      </Card>

      <Dialog open={mcpHelpOpen} onOpenChange={setMcpHelpOpen}>
        <DialogPopup className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>{t("settings.mcp.helpTitle")}</DialogTitle>
            <DialogDescription>{t("settings.mcp.helpDesc")}</DialogDescription>
          </DialogHeader>
          <DialogPanel>
            <pre className="whitespace-pre-wrap rounded-lg border border-border bg-muted/30 p-3 font-mono text-xs leading-5 text-foreground">
              {mcpHelpBusy ? t("settings.mcp.helpLoading") : mcpHelp}
            </pre>
          </DialogPanel>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => setMcpHelpOpen(false)}>{t("settings.mcp.helpClose")}</Button>
          </DialogFooter>
        </DialogPopup>
      </Dialog>
    </div>
  );
}
