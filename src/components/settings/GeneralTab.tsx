import { ReactNode, useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { Activity, BookOpen, Bot, Copy, Download, FileCheck2, RefreshCw, Rocket, Save, ServerCog, SlidersHorizontal, Upload, X } from "lucide-react";
import { AppSettings, McpClientConfig, ScheduleSettings, SchedulerLog, UpdateInfo } from "../../store/useAppStore";
import { cn } from "../../lib/utils";
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

function SettingRow({ id, label, description, children, className }: { id?: string; label: ReactNode; description?: ReactNode; children: ReactNode; className?: string }) {
  return (
    <div className={cn("grid min-h-12 gap-2 px-3 py-2 min-[900px]:grid-cols-[minmax(14rem,1fr)_minmax(28rem,1.7fr)] min-[900px]:items-center", className)}>
      <div className="min-w-0">
        <Label htmlFor={id} className="text-sm font-medium">{label}</Label>
        {description && <p className="mt-0.5 text-xs text-muted-foreground">{description}</p>}
      </div>
      <div className="flex min-w-0 flex-wrap items-center justify-end gap-2">{children}</div>
    </div>
  );
}

function SettingGroup({ children }: { children: ReactNode }) {
  return <div className="divide-y divide-border/60 overflow-hidden rounded-lg border border-border/70 bg-muted/10">{children}</div>;
}

function SectionHeading({ icon, title, description, actions }: { icon?: ReactNode; title: ReactNode; description: ReactNode; actions?: ReactNode }) {
  return (
    <div className="flex min-h-9 items-center justify-between gap-3">
      <div className="min-w-0">
        <h3 className="flex items-center gap-2 text-sm font-semibold">{icon}{title}</h3>
        <p className="text-xs text-muted-foreground">{description}</p>
      </div>
      {actions && <div className="flex shrink-0 items-center gap-2">{actions}</div>}
    </div>
  );
}
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
  installUpdate: () => Promise<void>;
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
  const autoUpdateId = useId();
  const autostartId = useId();
  const fileLockId = useId();
  const scheduleEnabledId = useId();
  const cronEnabledId = useId();
  const keepaliveEnabledId = useId();
  const mcpEnabledId = useId();
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [updateActivity, setUpdateActivity] = useState<"idle" | "checking" | "installing">("idle");
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
    setUpdateActivity("checking");
    setUpdateError(null);
    try {
      setUpdateInfo(await checkUpdate());
    } catch (error) {
      setUpdateError(String(error));
    } finally {
      setUpdateActivity("idle");
    }
  };

  const handleInstallUpdate = async () => {
    if (!updateInfo) return;
    setUpdateActivity("installing");
    setUpdateError(null);
    try {
      await installUpdate();
    } catch (error) {
      setUpdateError(String(error));
      setUpdateActivity("idle");
    }
  };

  return (
    <div className="flex w-full flex-col gap-3">
      <Card className="space-y-3 p-3">
        <h2 className="text-base font-semibold">{t("settings.general.title")}</h2>

        <section className="space-y-2">
          <SectionHeading
            title={t("settings.general.preferences", { defaultValue: "Preferences" })}
            description={t("settings.general.preferencesDesc", { defaultValue: "Language, appearance, and startup behavior." })}
          />
          <div className="grid gap-px overflow-hidden rounded-lg border border-border/70 bg-border/60 min-[900px]:grid-cols-3">
            <div className="flex min-h-12 items-center justify-between gap-3 bg-card px-3 py-2">
              <Label className="shrink-0 text-sm font-medium">{t("settings.general.language")}</Label>
              <Select value={settings?.language || "en"} onValueChange={handleChangeLanguage}>
                <SelectTrigger className="h-8 w-36"><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="en">English</SelectItem><SelectItem value="pl">Polski</SelectItem><SelectItem value="it">Italiano</SelectItem><SelectItem value="de">Deutsch</SelectItem><SelectItem value="fr">Français</SelectItem><SelectItem value="ru">Русский</SelectItem><SelectItem value="ja">日本語</SelectItem><SelectItem value="zh">中文</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="flex min-h-12 items-center justify-between gap-3 bg-card px-3 py-2">
              <Label className="shrink-0 text-sm font-medium">{t("settings.general.theme")}</Label>
              <Select value={settings?.theme || "system"} onValueChange={(value) => settings && saveSettings({ ...settings, theme: value })}>
                <SelectTrigger className="h-8 w-36"><SelectValue /></SelectTrigger>
                <SelectContent><SelectItem value="system">{t("settings.general.themeSystem")}</SelectItem><SelectItem value="light">{t("settings.general.themeLight")}</SelectItem><SelectItem value="dark">{t("settings.general.themeDark")}</SelectItem></SelectContent>
              </Select>
            </div>
            <div className="flex min-h-12 items-center justify-between gap-3 bg-card px-3 py-2">
              <Label htmlFor={autostartId} className="text-sm font-medium">{t("settings.general.startWithSystem")}</Label>
              <Switch id={autostartId} checked={settings?.autostart || false} onCheckedChange={setAutostart} />
            </div>
          </div>
        </section>

        <section className="space-y-2 border-t border-border/70 pt-3">
          <SectionHeading
            title={t("settings.general.fileHandling", { defaultValue: "File handling" })}
            description={t("settings.general.fileHandlingDesc", { defaultValue: "Control when files are processed and how busy files are handled." })}
          />
          <SettingGroup>
            <SettingRow label={t("settings.general.gracePeriod")} description={graceError || t("settings.general.gracePeriodDesc")}>
              <Slider className="min-w-36 flex-1" min={0} max={graceSteps.length - 1} step={1} value={[sliderIndex]} onValueChange={([value]) => handleGraceSliderChange(value)} />
              <span className={cn("w-10 text-right text-xs tabular-nums", graceError ? "text-destructive" : "text-muted-foreground")}>{formatDuration(currentGraceSeconds)}</span>
              <Input type="number" min={0} value={graceValue} onChange={(e) => handleGraceNumberChange(parseInt(e.target.value, 10) || 0, graceUnit)} className="h-8 w-20 tabular-nums" />
              <Select value={graceUnit} onValueChange={(value) => handleGraceNumberChange(graceValue, value as GraceUnit)}>
                <SelectTrigger className="h-8 w-28"><SelectValue /></SelectTrigger>
                <SelectContent><SelectItem value="seconds">{t("settings.general.gracePeriodSeconds")}</SelectItem><SelectItem value="minutes">{t("settings.general.gracePeriodMinutes")}</SelectItem><SelectItem value="hours">{t("settings.general.gracePeriodHours")}</SelectItem></SelectContent>
              </Select>
            </SettingRow>
            <SettingRow id={fileLockId} label={t("settings.general.checkFileLock")} description={t("settings.general.checkFileLockDesc")}>
              <Switch id={fileLockId} checked={settings?.lock_check_enabled || false} onCheckedChange={(checked) => settings && saveSettings({ ...settings, lock_check_enabled: checked })} />
            </SettingRow>
          </SettingGroup>
        </section>
      </Card>

      <Card className="order-5 space-y-2 p-3">
        <SectionHeading
          icon={<Rocket size={15} className="text-primary" />}
          title={t("settings.general.updates", { defaultValue: "Application updates" })}
          description={t("settings.general.updatesDesc", { defaultValue: "Check for new releases and install compatible updates." })}
          actions={<Button type="button" onClick={handleCheckUpdate} variant="outline" size="sm" disabled={updateActivity !== "idle"}><RefreshCw className={updateActivity === "checking" ? "animate-spin" : ""} />{updateActivity === "checking" ? t("settings.general.checkingUpdates", { defaultValue: "Checking…" }) : t("settings.general.checkUpdates", { defaultValue: "Check for updates" })}</Button>}
        />
        <SettingGroup>
          <SettingRow id={autoUpdateId} label={t("settings.general.autoUpdate", { defaultValue: "Install updates automatically" })} description={t("settings.general.autoUpdateDesc", { defaultValue: "Check in the background and restart Shelfy after a verified update is ready." })}>
            <Switch id={autoUpdateId} checked={settings?.auto_update || false} onCheckedChange={(checked) => settings && saveSettings({ ...settings, auto_update: checked })} />
          </SettingRow>
          <SettingRow label={updateInfo?.release_name || t("settings.general.latestRelease", { defaultValue: "Latest release" })} description={updateInfo ? (!updateInfo.available ? t("settings.general.upToDate", { defaultValue: "Shelfy is up to date." }) : updateInfo.can_install ? t("settings.general.updateReady", { defaultValue: "Verified update package is ready to download." }) : t("settings.general.manualUpdateRequired", { defaultValue: "This release must be installed manually." })) : t("settings.general.notChecked", { defaultValue: "Not checked" })}>
            <span className="text-xs text-muted-foreground">{t("settings.general.currentVersion", { defaultValue: "Current version" })}</span>
            <span className="min-w-14 text-sm font-medium tabular-nums">{updateInfo?.current_version || "—"}</span>
            <span className="text-xs text-muted-foreground">{t("settings.general.latestRelease", { defaultValue: "Latest release" })}</span>
            <span className="min-w-14 text-sm font-medium tabular-nums">{updateInfo?.latest_version || "—"}</span>
            {updateInfo?.available && updateInfo.can_install && <Button type="button" onClick={handleInstallUpdate} size="sm" disabled={updateActivity !== "idle"}><Download />{updateActivity === "installing" ? t("settings.general.preparingUpdate", { defaultValue: "Downloading and preparing…" }) : t("settings.general.installUpdate", { defaultValue: "Download, install & restart" })}</Button>}
          </SettingRow>
        </SettingGroup>
        {updateInfo?.release_notes && <p className="max-h-20 overflow-auto whitespace-pre-wrap px-1 text-xs text-muted-foreground">{updateInfo.release_notes}</p>}
        {updateError && <div className="rounded-lg border border-destructive/20 bg-destructive/10 px-3 py-2 text-xs text-destructive">{updateError}</div>}
      </Card>

      <Card className="order-2 space-y-2 p-3">
        <SectionHeading
          icon={<SlidersHorizontal size={15} className="text-primary" />}
          title={t("settings.scheduler.title")}
          description={t("settings.scheduler.desc")}
          actions={<Button type="button" onClick={handleSaveSchedule} size="sm"><AnimatedIcon icon={Save} motion="pulse" />{t("settings.scheduler.save")}</Button>}
        />

        <SettingGroup>
          <SettingRow id={scheduleEnabledId} label={t("settings.scheduler.enable")} description={t("settings.scheduler.desc")}>
            <Switch id={scheduleEnabledId} checked={localSchedule.schedule_enabled} onCheckedChange={(checked) => handleScheduleChange({ schedule_enabled: checked })} />
          </SettingRow>
          <SettingRow label={t("settings.scheduler.timesPerDay")} description={t("settings.scheduler.fixedTimeDesc", { defaultValue: "Choose how often and at what times Shelfy runs each day." })}>
            <Select value={String(localSchedule.schedule_times_per_day)} onValueChange={(value) => handleScheduleChange({ schedule_times_per_day: parseInt(value, 10) })}>
              <SelectTrigger className="h-8 w-32"><SelectValue /></SelectTrigger>
              <SelectContent><SelectItem value="1">{t("settings.scheduler.once")}</SelectItem><SelectItem value="2">{t("settings.scheduler.twice")}</SelectItem><SelectItem value="3">{t("settings.scheduler.thrice")}</SelectItem><SelectItem value="4">{t("settings.scheduler.fourTimes")}</SelectItem></SelectContent>
            </Select>
            {Array.from({ length: localSchedule.schedule_times_per_day }).map((_, idx) => {
              const key = `schedule_time_${idx + 1}` as keyof ScheduleSettings;
              return <Input key={key} aria-label={t("settings.scheduler.time", { number: idx + 1 })} type="time" value={(localSchedule[key] as string | null) || ""} onChange={(event) => handleScheduleChange({ [key]: event.target.value || null } as Partial<ScheduleSettings>)} className="h-8 w-28" />;
            })}
          </SettingRow>
          <SettingRow id={cronEnabledId} label={t("settings.scheduler.cronEnable")} description={t("settings.scheduler.cronDesc")}>
            <Input aria-label={t("settings.scheduler.cronExpression")} value={localSchedule.schedule_cron_expr || ""} onChange={(event) => handleScheduleChange({ schedule_cron_expr: event.target.value })} placeholder="*/30 * * * *" className="h-8 min-w-48 flex-1 font-mono" />
            <Button type="button" onClick={handleValidateCron} variant="outline" size="sm"><FileCheck2 />{t("settings.scheduler.validateCron")}</Button>
            <Switch id={cronEnabledId} checked={localSchedule.schedule_cron_enabled} onCheckedChange={(checked) => handleScheduleChange({ schedule_cron_enabled: checked })} />
          </SettingRow>
          <SettingRow id={keepaliveEnabledId} label={<span className="inline-flex items-center gap-1.5"><Activity className="size-3.5 text-primary" />{t("settings.scheduler.keepalive")}</span>} description={systemKeepaliveSupported ? t("settings.scheduler.keepaliveDesc") : t("settings.scheduler.keepaliveUnsupported")}>
            <Input aria-label={t("settings.scheduler.keepaliveInterval")} type="number" min={1} max={1440} value={localSchedule.keepalive_interval_minutes} onChange={(event) => handleScheduleChange({ keepalive_interval_minutes: parseInt(event.target.value, 10) || 15 })} className="h-8 w-20" />
            <span className="text-xs text-muted-foreground">{t("settings.scheduler.minutesShort", { defaultValue: "min" })}</span>
            <Button type="button" onClick={handleInstallSystemKeepalive} variant="outline" size="sm" disabled={!systemKeepaliveSupported}><ServerCog />{t("settings.scheduler.installKeepalive")}</Button>
            <Button type="button" onClick={handleUninstallSystemKeepalive} variant="ghost" size="sm" disabled={!systemKeepaliveSupported}><X />{t("settings.scheduler.uninstallKeepalive")}</Button>
            <Switch id={keepaliveEnabledId} checked={localSchedule.keepalive_enabled} onCheckedChange={(checked) => handleScheduleChange({ keepalive_enabled: checked })} />
          </SettingRow>
          <SettingRow label={t("settings.scheduler.logs")} description={t("settings.scheduler.noLogs")}>
            <Button type="button" onClick={loadSchedulerLogs} variant="outline" size="sm"><RefreshCw />{t("settings.scheduler.refreshLogs")}</Button>
            <Button type="button" onClick={clearSchedulerLogs} variant="ghost" size="sm">{t("settings.scheduler.clearLogs")}</Button>
          </SettingRow>
        </SettingGroup>

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

        <div className="space-y-1.5">
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
        <SectionHeading
          icon={<Bot size={15} className="text-primary" />}
          title={t("settings.mcp.title")}
          description={t("settings.mcp.desc")}
          actions={<><Button type="button" onClick={handleCopyMcpConfig} variant="outline" size="sm" disabled={!mcpClientConfig}><Copy />{t("settings.mcp.copyConfig")}</Button><Button type="button" onClick={() => void handleOpenMcpHelp()} variant="outline" size="sm"><BookOpen />{t("settings.mcp.help")}</Button><Button type="button" onClick={handleSaveMcp} size="sm"><AnimatedIcon icon={Save} motion="pulse" />{t("settings.mcp.save")}</Button></>}
        />

        <SettingGroup>
          <SettingRow id={mcpEnabledId} label={t("settings.mcp.enable")} description={t("settings.mcp.enableDesc")}>
            <Switch id={mcpEnabledId} checked={localMcp.mcp_enabled} onCheckedChange={(checked) => setLocalMcp({ ...localMcp, mcp_enabled: checked })} />
          </SettingRow>
          <SettingRow label={t("settings.mcp.serverName")} description={t("settings.mcp.transport")}>
            <Input aria-label={t("settings.mcp.serverName")} value={localMcp.mcp_server_name} onChange={(event) => setLocalMcp({ ...localMcp, mcp_server_name: event.target.value })} placeholder="shelfy" className="h-8 min-w-44 flex-1" />
            <Select value={localMcp.mcp_transport} onValueChange={(value) => setLocalMcp({ ...localMcp, mcp_transport: value })}>
              <SelectTrigger aria-label={t("settings.mcp.transport")} className="h-8 w-36"><SelectValue /></SelectTrigger>
              <SelectContent><SelectItem value="stdio">{t("settings.mcp.transportStdio")}</SelectItem><SelectItem value="http">{t("settings.mcp.transportHttp")}</SelectItem></SelectContent>
            </Select>
          </SettingRow>
          {localMcp.mcp_transport === "http" ? (
            <SettingRow label={t("settings.mcp.httpUrl")} description={t("settings.mcp.token")}>
              <Input aria-label={t("settings.mcp.httpUrl")} value={localMcp.mcp_http_url || ""} onChange={(event) => setLocalMcp({ ...localMcp, mcp_http_url: event.target.value })} placeholder="http://127.0.0.1:8765/mcp" className="h-8 min-w-52 flex-1" />
              <Input aria-label={t("settings.mcp.token")} value={localMcp.mcp_token || ""} onChange={(event) => setLocalMcp({ ...localMcp, mcp_token: event.target.value })} placeholder="optional" className="h-8 min-w-36 flex-1" />
            </SettingRow>
          ) : (
            <SettingRow label={t("settings.mcp.command")} description={t("settings.mcp.args")}>
              <Input aria-label={t("settings.mcp.command")} value={localMcp.mcp_command || ""} onChange={(event) => setLocalMcp({ ...localMcp, mcp_command: event.target.value })} placeholder={t("settings.mcp.commandPlaceholder")} className="h-8 min-w-52 flex-1" />
              <Input aria-label={t("settings.mcp.args")} value={localMcp.mcp_args || ""} onChange={(event) => setLocalMcp({ ...localMcp, mcp_args: event.target.value })} placeholder="--mcp" className="h-8 min-w-36 flex-1" />
            </SettingRow>
          )}
          <SettingRow label={t("settings.mcp.allowWrite")} description={t("settings.mcp.allowWriteDesc", { defaultValue: "Allow AI clients to change Shelfy data and run file operations." })}>
            <Label className="flex items-center gap-2 text-xs text-muted-foreground"><Checkbox checked={localMcp.mcp_allow_write} onCheckedChange={(checked) => setLocalMcp({ ...localMcp, mcp_allow_write: checked === true })} />{t("settings.mcp.allowWrite")}</Label>
          </SettingRow>
        </SettingGroup>

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
        <SectionHeading
          title={t("settings.config.title")}
          description={t("settings.config.desc")}
          actions={<><Button type="button" onClick={handleExportConfig} variant="outline" size="sm"><AnimatedIcon icon={Download} motion="float" />{t("settings.config.export")}</Button><Button type="button" onClick={handleImportConfig} variant="outline" size="sm"><AnimatedIcon icon={Upload} motion="float" />{t("settings.config.import")}</Button></>}
        />
        <SettingGroup>
          <SettingRow label={t("settings.config.replaceOnImport")} description={t("settings.config.desc")}>
            <Label className="flex items-center gap-2 text-xs text-muted-foreground"><Checkbox checked={replaceConfigOnImport} onCheckedChange={(checked) => setReplaceConfigOnImport(checked === true)} />{t("settings.config.replaceOnImport")}</Label>
          </SettingRow>
        </SettingGroup>
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
