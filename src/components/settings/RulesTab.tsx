import { useTranslation } from "react-i18next";
import { Download, FolderOpen, History, Plus, Save, Trash2, Upload, X } from "lucide-react";
import { Rule, WatchedFolder } from "../../store/useAppStore";
import { AnimatedIcon } from "../ui/animated-icon";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { Card } from "../ui/card";
import { Checkbox } from "../ui/checkbox";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../ui/select";
import { Switch } from "../ui/switch";
import { Tooltip, TooltipContent, TooltipTrigger } from "../ui/tooltip";

type Toast = { message: string; type: "success" | "error" } | null;

interface RulesTabProps {
  rules: Rule[];
  folders: WatchedFolder[];
  editingRule: Rule | null;
  setEditingRule: (rule: Rule | null) => void;
  newFolderPath: string;
  setNewFolderPath: (path: string) => void;
  replaceOnImport: boolean;
  setReplaceOnImport: (replace: boolean) => void;
  ruleToast: Toast;
  handleChooseFolder: () => void;
  handleAddFolder: () => void;
  updateFolderMode: (id: number, mode: string) => void;
  removeFolder: (id: number) => void;
  handleExportRules: () => void;
  handleImportRules: () => void;
  handleChooseDestination: () => void;
  handleChooseRuleScopeFolder: () => void;
  handleSaveRule: () => void;
  deleteRule: (id: number) => void;
  handleViewHistory: (ruleLabel: string) => void;
}

export function RulesTab({
  rules,
  folders,
  editingRule,
  setEditingRule,
  newFolderPath,
  setNewFolderPath,
  replaceOnImport,
  setReplaceOnImport,
  ruleToast,
  handleChooseFolder,
  handleAddFolder,
  updateFolderMode,
  removeFolder,
  handleExportRules,
  handleImportRules,
  handleChooseDestination,
  handleChooseRuleScopeFolder,
  handleSaveRule,
  deleteRule,
  handleViewHistory,
}: RulesTabProps) {
  const { t } = useTranslation();

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">{t("settings.rules.title")}</h2>
        <Button
          onClick={() =>
            setEditingRule({
              name: "",
              priority: 0,
              enabled: true,
              extensions: [],
              pattern: null,
              destination: "",
              action: "move",
              folder_id: 0,
              folder_path: null,
            })
          }
        >
          <AnimatedIcon icon={Plus} size={14} motion="bounce" />
          {t("settings.rules.add")}
        </Button>
      </div>

      <Card className="space-y-4 p-4">
        <div className="flex items-center justify-between gap-3">
          <div>
            <h3 className="text-sm font-semibold">{t("settings.folders.title")}</h3>
            <p className="text-xs text-muted-foreground">{t("settings.folders.modeDesc")}</p>
          </div>
        </div>
        <div className="flex flex-wrap gap-2">
          <Input
            type="text"
            value={newFolderPath}
            onChange={(e) => setNewFolderPath(e.target.value)}
            placeholder={t("settings.folders.placeholder")}
            className="min-w-[240px] flex-1"
          />
          <Button onClick={handleChooseFolder} variant="outline">
            <AnimatedIcon icon={FolderOpen} size={14} motion="float" />
            {t("settings.folders.choose")}
          </Button>
          <Button onClick={handleAddFolder}>
            <AnimatedIcon icon={Plus} size={14} motion="bounce" />
            {t("settings.folders.add")}
          </Button>
        </div>
        <div className="space-y-2">
          {folders.map((f) => (
            <Card key={f.id} className="flex items-center justify-between gap-3 px-4 py-3">
              <div className="min-w-0 flex-1">
                <div className="truncate text-sm font-medium">{f.path}</div>
                <div className="mt-1.5">
                  <Label className="mb-1 block text-[10px] uppercase text-muted-foreground">
                    {t("settings.folders.mode")}
                  </Label>
                  <Select value={f.mode || "silent"} onValueChange={(value) => f.id && updateFolderMode(f.id, value)}>
                    <SelectTrigger className="h-8 w-full sm:max-w-xs">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="silent">{t("settings.folders.modeSilent")}</SelectItem>
                      <SelectItem value="manual">{t("settings.folders.modeManual")}</SelectItem>
                      <SelectItem value="paused">{t("settings.folders.modePaused")}</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
              <Button
                onClick={() => f.id && removeFolder(f.id)}
                variant="ghost"
                size="icon"
                className="shrink-0 text-destructive hover:bg-destructive/10 hover:text-destructive"
              >
                <Trash2 size={14} />
              </Button>
            </Card>
          ))}
        </div>
      </Card>

      <Card className="space-y-3 p-4">
        <div className="flex flex-wrap items-center gap-2">
          <Button onClick={handleExportRules} variant="outline">
            <AnimatedIcon icon={Download} size={14} motion="float" />
            {t("settings.rules.export")}
          </Button>
          <Button onClick={handleImportRules} variant="outline">
            <AnimatedIcon icon={Upload} size={14} motion="float" />
            {t("settings.rules.import")}
          </Button>
        </div>
        <Label className="flex items-center gap-2 text-sm">
          <Checkbox checked={replaceOnImport} onCheckedChange={(checked) => setReplaceOnImport(checked === true)} />
          {t("settings.rules.replaceOnImport")}
        </Label>
        {ruleToast && (
          <div
            className={`rounded-xl border px-3 py-2 text-xs shadow-sm ${
              ruleToast.type === "success"
                ? "border-emerald-200 bg-emerald-50 text-emerald-700 dark:border-emerald-900 dark:bg-emerald-950 dark:text-emerald-300"
                : "border-destructive/20 bg-destructive/10 text-destructive"
            }`}
          >
            {ruleToast.message}
          </div>
        )}
      </Card>

      {editingRule && (
        <Card className="space-y-4 p-4">
          <div className="grid gap-3 md:grid-cols-2">
            <div>
              <Label className="text-xs text-muted-foreground">{t("settings.rules.name")}</Label>
              <Input value={editingRule.name} onChange={(e) => setEditingRule({ ...editingRule, name: e.target.value })} className="mt-1" />
            </div>
            <div>
              <Label className="text-xs text-muted-foreground">{t("settings.rules.extensions")}</Label>
              <Input
                value={editingRule.extensions.join(", ")}
                onChange={(e) =>
                  setEditingRule({
                    ...editingRule,
                    extensions: e.target.value.split(",").map((s) => s.trim()),
                  })
                }
                placeholder={t("settings.rules.extensions")}
                className="mt-1"
              />
            </div>
            <div>
              <Label className="text-xs text-muted-foreground">{t("settings.rules.pattern")}</Label>
              <Input
                value={editingRule.pattern || ""}
                onChange={(e) =>
                  setEditingRule({
                    ...editingRule,
                    pattern: e.target.value.trim() === "" ? null : e.target.value,
                  })
                }
                placeholder="(?i)report.*\.pdf"
                className="mt-1"
              />
            </div>
            <div>
              <Label className="text-xs text-muted-foreground">{t("settings.rules.destination")}</Label>
              <div className="mt-1 flex gap-2">
                <Input value={editingRule.destination} onChange={(e) => setEditingRule({ ...editingRule, destination: e.target.value })} />
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button type="button" variant="outline" size="icon" onClick={handleChooseDestination} aria-label={t("settings.rules.chooseDestination")}>
                      <FolderOpen size={14} />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>{t("settings.rules.chooseDestination")}</TooltipContent>
                </Tooltip>
              </div>
            </div>
            <div>
              <Label className="text-xs text-muted-foreground">{t("settings.rules.priority")}</Label>
              <Input
                type="number"
                value={editingRule.priority}
                onChange={(e) => setEditingRule({ ...editingRule, priority: parseInt(e.target.value) || 0 })}
                className="mt-1"
              />
            </div>
            <div>
              <Label className="text-xs text-muted-foreground">{t("settings.rules.action")}</Label>
              <Select value={editingRule.action} onValueChange={(value) => setEditingRule({ ...editingRule, action: value })}>
                <SelectTrigger className="mt-1">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="copy">{t("settings.rules.actionCopy")}</SelectItem>
                  <SelectItem value="move">{t("settings.rules.actionMove")}</SelectItem>
                  <SelectItem value="ignore">{t("settings.rules.actionIgnore")}</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="col-span-2">
              <Label className="text-xs text-muted-foreground">{t("settings.rules.folderScope")}</Label>
              <Select
                value={editingRule.folder_path ? "__custom__" : String(editingRule.folder_id)}
                onValueChange={(value) => {
                  if (value === "__custom__") {
                    setEditingRule({ ...editingRule, folder_id: 0, folder_path: editingRule.folder_path || "" });
                    return;
                  }
                  setEditingRule({ ...editingRule, folder_id: parseInt(value, 10) || 0, folder_path: null });
                }}
              >
                <SelectTrigger className="mt-1">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="0">{t("settings.rules.allFolders")}</SelectItem>
                  <SelectItem value="__custom__">{t("settings.rules.customFolder")}</SelectItem>
                  {folders
                    .filter((folder) => folder.id)
                    .map((folder) => (
                      <SelectItem key={folder.id} value={String(folder.id)}>
                        {folder.path}
                      </SelectItem>
                    ))}
                </SelectContent>
              </Select>
              {editingRule.folder_path !== null && editingRule.folder_path !== undefined && (
                <div className="mt-2 flex gap-2">
                  <Input
                    value={editingRule.folder_path}
                    onChange={(e) => setEditingRule({ ...editingRule, folder_id: 0, folder_path: e.target.value })}
                    placeholder={t("settings.rules.customFolderPath")}
                  />
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <Button type="button" variant="outline" size="icon" onClick={handleChooseRuleScopeFolder} aria-label={t("settings.rules.chooseFolderScope")}>
                        <FolderOpen size={14} />
                      </Button>
                    </TooltipTrigger>
                    <TooltipContent>{t("settings.rules.chooseFolderScope")}</TooltipContent>
                  </Tooltip>
                </div>
              )}
            </div>
          </div>
          <div className="flex items-center justify-between rounded-xl border border-border bg-background px-3 py-2 shadow-sm">
            <Label className="flex items-center gap-2 text-sm">{t("settings.rules.enabled")}</Label>
            <Switch checked={editingRule.enabled} onCheckedChange={(checked) => setEditingRule({ ...editingRule, enabled: checked })} />
          </div>
          <div className="flex gap-2">
            <Button onClick={handleSaveRule}>
              <AnimatedIcon icon={Save} size={14} motion="pulse" />
              {t("settings.rules.edit")}
            </Button>
            <Button onClick={() => setEditingRule(null)} variant="outline">
              <X size={14} />
              {t("common.cancel")}
            </Button>
          </div>
        </Card>
      )}

      <div className="space-y-2">
        {rules.map((r) => (
          <Card key={r.id} className="flex items-center justify-between px-4 py-3">
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2">
                <span className="text-sm font-medium">{r.name}</span>
                {!r.enabled && <Badge variant="secondary" className="text-[10px]">{t("common.off")}</Badge>}
              </div>
              <div className="mt-0.5 truncate text-xs text-muted-foreground">
                {r.extensions.join(", ")} → {r.destination}
              </div>
            </div>
            <div className="flex items-center gap-1">
              <Button
                onClick={() => handleViewHistory(r.name)}
                variant="ghost"
                size="icon"
                className="text-muted-foreground"
              >
                <History size={14} />
              </Button>
              <Button onClick={() => setEditingRule({ ...r })} variant="ghost" size="icon" className="text-muted-foreground">
                <Save size={14} />
              </Button>
              <Button
                onClick={() => r.id && deleteRule(r.id)}
                variant="ghost"
                size="icon"
                className="text-destructive hover:bg-destructive/10 hover:text-destructive"
              >
                <Trash2 size={14} />
              </Button>
            </div>
          </Card>
        ))}
      </div>
    </div>
  );
}
