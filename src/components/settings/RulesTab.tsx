import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Download, FolderOpen, History, MoreHorizontal, Pencil, Plus, Save, Trash2, Upload, X } from "lucide-react";
import { Rule, WatchedFolder } from "../../store/useAppStore";
import { AnimatedIcon } from "../ui/animated-icon";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../ui/card";
import { Checkbox } from "../ui/checkbox";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import { Menu, MenuGroup, MenuGroupLabel, MenuItem, MenuPopup, MenuSeparator, MenuTrigger } from "../ui/menu";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../ui/select";
import { Switch } from "../ui/switch";
import { Table, TableBody, TableCell, TableFooter, TableHead, TableHeader, TableRow } from "../ui/table";
import { Tooltip, TooltipContent, TooltipTrigger } from "../ui/tooltip";
import { AlertDialog, AlertDialogClose, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogPopup, AlertDialogTitle } from "../ui/alert-dialog";
import { Dialog, DialogDescription, DialogFooter, DialogHeader, DialogPanel, DialogPopup, DialogTitle } from "../ui/dialog";

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
  updateRule: (rule: Rule) => Promise<void>;
  deleteRule: (id: number) => Promise<void>;
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
  updateRule,
  deleteRule,
  handleViewHistory,
}: RulesTabProps) {
  const { t } = useTranslation();
  const [pendingDeleteRule, setPendingDeleteRule] = useState<Rule | null>(null);
  const [deletingRule, setDeletingRule] = useState(false);
  const [deleteRuleError, setDeleteRuleError] = useState<string | null>(null);

  const confirmDeleteRule = async () => {
    if (!pendingDeleteRule?.id) return;
    setDeletingRule(true);
    setDeleteRuleError(null);
    try {
      await deleteRule(pendingDeleteRule.id);
      setPendingDeleteRule(null);
    } catch (error) {
      setDeleteRuleError(String(error || t("settings.rules.deleteError")));
    } finally {
      setDeletingRule(false);
    }
  };

  const newRule = (): Rule => ({
    name: "",
    priority: rules.length > 0 ? Math.max(...rules.map((rule) => rule.priority)) + 1 : 1,
    enabled: true,
    extensions: [],
    pattern: null,
    destination: "",
    action: "move",
    folder_id: 0,
    folder_path: null,
  });

  const ruleScope = (rule: Rule) => {
    if (rule.folder_path) return rule.folder_path;
    if (rule.folder_id) return folders.find((folder) => folder.id === rule.folder_id)?.path || t("settings.rules.customFolder");
    return t("settings.rules.allFolders");
  };

  const ruleDescription = (rule: Rule) => {
    const extensions = rule.extensions.filter(Boolean);
    const matches = extensions.includes("*") || extensions.length === 0
      ? t("settings.rules.allFileTypes")
      : extensions.map((extension) => `.${extension}`).join(", ");
    const pattern = rule.pattern ? t("settings.rules.withPattern", { pattern: rule.pattern }) : "";
    const action = rule.action === "copy"
      ? t("settings.rules.actionCopy")
      : rule.action === "ignore"
        ? t("settings.rules.actionIgnore")
        : t("settings.rules.actionMove");
    return [matches, pattern, action, rule.action === "ignore" ? null : rule.destination || "—"].filter(Boolean).join(" · ");
  };

  const ruleActions = (rule: Rule) => (
    <div className="flex justify-end gap-0.5">
      <Tooltip>
        <TooltipTrigger asChild>
          <Button type="button" onClick={() => setEditingRule({ ...rule })} variant="ghost" size="icon-sm" aria-label={t("settings.rules.edit")}>
            <Pencil size={13} />
          </Button>
        </TooltipTrigger>
        <TooltipContent>{t("settings.rules.edit")}</TooltipContent>
      </Tooltip>
      <Menu>
        <MenuTrigger render={<Button type="button" variant="ghost" size="icon-sm" aria-label={t("settings.rules.moreActions")} />}><MoreHorizontal size={15} /></MenuTrigger>
        <MenuPopup>
          <MenuGroup>
            <MenuGroupLabel>{t("settings.rules.ruleManagement")}</MenuGroupLabel>
            <MenuItem onClick={() => handleViewHistory(rule.name)}><History />{t("settings.rules.viewHistory")}</MenuItem>
            <MenuItem onClick={() => setEditingRule({ ...rule })}><Pencil />{t("settings.rules.edit")}</MenuItem>
          </MenuGroup>
          <MenuSeparator />
          <MenuItem variant="destructive" onClick={() => setPendingDeleteRule(rule)}>
            <Trash2 />{t("settings.rules.delete")}
          </MenuItem>
        </MenuPopup>
      </Menu>
    </div>
  );

  const canSaveRule = Boolean(
    editingRule?.name.trim()
      && editingRule.extensions.some((extension) => extension.trim())
      && (editingRule.action === "ignore" || editingRule.destination.trim())
  );

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="min-w-0">
          <h2 className="text-lg font-semibold">{t("settings.rules.title")}</h2>
          <p className="text-xs text-muted-foreground">{t("settings.rules.description")}</p>
        </div>
        <Button
          type="button"
          size="sm"
          className="shrink-0"
          onClick={() => setEditingRule(newRule())}
        >
          <AnimatedIcon icon={Plus} size={14} motion="bounce" />
          {t("settings.rules.add")}
        </Button>
      </div>

      <Card className="order-3 space-y-2.5 p-3">
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
            <div key={f.id} className="flex items-center justify-between gap-3 rounded-lg border border-border/70 bg-muted/20 px-3 py-2">
              <div className="min-w-0 flex-1">
                <div className="truncate text-sm font-medium">{f.path}</div>
                <div className="mt-1">
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
              <Tooltip>
                <TooltipTrigger asChild><Button onClick={() => f.id && removeFolder(f.id)} variant="ghost" size="icon-sm" className="shrink-0 text-destructive hover:bg-destructive/10 hover:text-destructive" aria-label={t("settings.folders.remove")}><Trash2 size={14} /></Button></TooltipTrigger>
                <TooltipContent>{t("settings.folders.remove")}</TooltipContent>
              </Tooltip>
            </div>
          ))}
        </div>
      </Card>

      <Card className="order-4 space-y-2.5 p-3">
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
            className={`rounded-lg border px-3 py-2 text-xs ${
              ruleToast.type === "success"
                ? "border-primary/25 bg-primary/8 text-primary"
                : "border-destructive/20 bg-destructive/10 text-destructive"
            }`}
          >
            {ruleToast.message}
          </div>
        )}
      </Card>

      <Dialog open={editingRule != null} onOpenChange={(open) => { if (!open) setEditingRule(null); }}>
        {editingRule && (
        <DialogPopup className="max-h-[calc(100vh-2rem)] max-w-3xl">
          <DialogHeader className="border-b border-border bg-muted/20">
            <DialogTitle>{editingRule.id ? t("settings.rules.updateTitle") : t("settings.rules.createTitle")}</DialogTitle>
            <DialogDescription>{t("settings.rules.editorDescription")}</DialogDescription>
          </DialogHeader>
          <DialogPanel className="space-y-3 pt-4">
            <section className="border-b border-border/70 pb-3">
              <CardHeader className="px-0 py-0 pb-2">
                <CardTitle className="text-sm">1. {t("settings.rules.basicSettings")}</CardTitle>
                <CardDescription className="text-xs">{t("settings.rules.basicSettingsDesc")}</CardDescription>
              </CardHeader>
              <CardContent className="grid gap-3 p-0 md:grid-cols-[minmax(0,1fr)_8rem_auto]">
                <div>
                  <Label htmlFor="rule-name" className="mb-1 block text-xs text-muted-foreground">{t("settings.rules.name")}</Label>
                  <Input id="rule-name" type="text" value={editingRule.name} onChange={(event) => setEditingRule({ ...editingRule, name: event.target.value })} />
                </div>
                <div>
                  <Label htmlFor="rule-priority" className="mb-1 block text-xs text-muted-foreground">{t("settings.rules.priority")}</Label>
                  <Input id="rule-priority" type="number" value={editingRule.priority} onChange={(event) => setEditingRule({ ...editingRule, priority: parseInt(event.target.value, 10) || 0 })} />
                </div>
                <Label className="flex items-end gap-2 pb-2 text-sm">
                  <Switch checked={editingRule.enabled} onCheckedChange={(checked) => setEditingRule({ ...editingRule, enabled: checked })} />
                  {t("settings.rules.enabled")}
                </Label>
              </CardContent>
            </section>

            <section className="border-b border-border/70 pb-3">
              <CardHeader className="px-0 py-0 pb-2">
                <CardTitle className="text-sm">2. {t("settings.rules.matchSettings")}</CardTitle>
                <CardDescription className="text-xs">{t("settings.rules.matchSettingsDesc")}</CardDescription>
              </CardHeader>
              <CardContent className="grid gap-3 p-0 md:grid-cols-2">
                <div className="md:col-span-2">
                  <Label htmlFor="rule-scope" className="mb-1 block text-xs text-muted-foreground">{t("settings.rules.folderScope")}</Label>
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
                    <SelectTrigger id="rule-scope"><SelectValue /></SelectTrigger>
                    <SelectContent>
                      <SelectItem value="0">{t("settings.rules.allFolders")}</SelectItem>
                      <SelectItem value="__custom__">{t("settings.rules.customFolder")}</SelectItem>
                      {folders.filter((folder) => folder.id).map((folder) => (
                        <SelectItem key={folder.id} value={String(folder.id)}>{folder.path}</SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  {editingRule.folder_path !== null && editingRule.folder_path !== undefined && (
                    <div className="mt-2 flex gap-2">
                      <Input type="text" value={editingRule.folder_path} onChange={(event) => setEditingRule({ ...editingRule, folder_id: 0, folder_path: event.target.value })} placeholder={t("settings.rules.customFolderPath")} />
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <Button type="button" variant="outline" size="icon" onClick={handleChooseRuleScopeFolder} aria-label={t("settings.rules.chooseFolderScope")}><FolderOpen size={14} /></Button>
                        </TooltipTrigger>
                        <TooltipContent>{t("settings.rules.chooseFolderScope")}</TooltipContent>
                      </Tooltip>
                    </div>
                  )}
                </div>
                <div>
                  <Label htmlFor="rule-extensions" className="mb-1 block text-xs text-muted-foreground">{t("settings.rules.extensions")}</Label>
                  <Input
                    id="rule-extensions"
                    type="text"
                    value={editingRule.extensions.join(", ")}
                    onChange={(event) => setEditingRule({ ...editingRule, extensions: event.target.value.split(",").map((extension) => extension.trim()) })}
                    placeholder="pdf, docx, xlsx"
                  />
                  <p className="mt-1 text-[11px] text-muted-foreground">{t("settings.rules.extensionsHint")}</p>
                </div>
                <div>
                  <Label htmlFor="rule-pattern" className="mb-1 block text-xs text-muted-foreground">{t("settings.rules.pattern")}</Label>
                  <Input
                    id="rule-pattern"
                    type="text"
                    value={editingRule.pattern || ""}
                    onChange={(event) => setEditingRule({ ...editingRule, pattern: event.target.value.trim() === "" ? null : event.target.value })}
                    placeholder="(?i)report.*\\.pdf"
                  />
                  <p className="mt-1 text-[11px] text-muted-foreground">{t("settings.rules.patternHint")}</p>
                </div>
              </CardContent>
            </section>

            <section>
              <CardHeader className="px-0 py-0 pb-2">
                <CardTitle className="text-sm">3. {t("settings.rules.actionSettings")}</CardTitle>
                <CardDescription className="text-xs">{t("settings.rules.actionSettingsDesc")}</CardDescription>
              </CardHeader>
              <CardContent className="grid gap-3 p-0 md:grid-cols-[12rem_minmax(0,1fr)]">
                <div>
                  <Label htmlFor="rule-action" className="mb-1 block text-xs text-muted-foreground">{t("settings.rules.action")}</Label>
                  <Select value={editingRule.action} onValueChange={(value) => setEditingRule({ ...editingRule, action: value })}>
                    <SelectTrigger id="rule-action"><SelectValue /></SelectTrigger>
                    <SelectContent>
                      <SelectItem value="copy">{t("settings.rules.actionCopy")}</SelectItem>
                      <SelectItem value="move">{t("settings.rules.actionMove")}</SelectItem>
                      <SelectItem value="ignore">{t("settings.rules.actionIgnore")}</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                {editingRule.action !== "ignore" && (
                  <div>
                    <Label htmlFor="rule-destination" className="mb-1 block text-xs text-muted-foreground">{t("settings.rules.destination")}</Label>
                    <div className="flex gap-2">
                      <Input id="rule-destination" type="text" value={editingRule.destination} onChange={(event) => setEditingRule({ ...editingRule, destination: event.target.value })} />
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <Button type="button" variant="outline" size="icon" onClick={handleChooseDestination} aria-label={t("settings.rules.chooseDestination")}><FolderOpen size={14} /></Button>
                        </TooltipTrigger>
                        <TooltipContent>{t("settings.rules.chooseDestination")}</TooltipContent>
                      </Tooltip>
                    </div>
                    <p className="mt-1 text-[11px] text-muted-foreground">{t("settings.rules.destinationHint")}</p>
                  </div>
                )}
              </CardContent>
            </section>

            {!canSaveRule && <p className="text-xs text-destructive">{t("settings.rules.requiredFields")}</p>}
          </DialogPanel>
          <DialogFooter>
            <Button type="button" onClick={() => setEditingRule(null)} variant="outline">
              <X size={14} />
              {t("common.cancel")}
            </Button>
            <Button type="button" onClick={handleSaveRule} disabled={!canSaveRule}>
              <AnimatedIcon icon={Save} size={14} motion="pulse" />
              {editingRule.id ? t("settings.rules.update") : t("settings.rules.save")}
            </Button>
          </DialogFooter>
        </DialogPopup>
        )}
      </Dialog>

      <Card className="order-2 overflow-hidden">
        <CardHeader className="flex-row items-center justify-between gap-3 border-b border-border">
          <div>
            <CardTitle className="text-sm">{t("settings.rules.ruleList")}</CardTitle>
            <CardDescription className="text-xs">{t("settings.rules.ruleListDesc")}</CardDescription>
          </div>
          <Badge variant="secondary">{t("settings.rules.ruleCount", { count: rules.length })}</Badge>
        </CardHeader>
        <CardContent className="p-0">
          <div className="hidden min-[900px]:block"><Table className="table-fixed">
            <TableHeader>
              <TableRow>
                <TableHead className="w-[38%]">{t("settings.rules.rule")}</TableHead>
                <TableHead className="w-[22%]">{t("settings.rules.folderScope")}</TableHead>
                <TableHead className="w-[10%]">{t("settings.rules.action")}</TableHead>
                <TableHead className="w-[8%]">{t("settings.rules.priority")}</TableHead>
                <TableHead className="w-[12%]">{t("settings.rules.status")}</TableHead>
                <TableHead className="w-[10%] text-right">{t("settings.rules.actions")}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {rules.map((rule) => (
                <TableRow key={rule.id || rule.name}>
                  <TableCell className="min-w-0">
                    <div className="truncate font-medium" title={rule.name}>{rule.name}</div>
                    <div className="mt-0.5 truncate text-xs text-muted-foreground" title={ruleDescription(rule)}>{ruleDescription(rule)}</div>
                  </TableCell>
                  <TableCell className="max-w-48 truncate text-xs text-muted-foreground" title={ruleScope(rule)}>{ruleScope(rule)}</TableCell>
                  <TableCell><Badge variant="outline">{rule.action === "copy" ? t("settings.rules.actionCopy") : rule.action === "ignore" ? t("settings.rules.actionIgnore") : t("settings.rules.actionMove")}</Badge></TableCell>
                  <TableCell className="tabular-nums">{rule.priority}</TableCell>
                  <TableCell>
                    <div className="flex items-center">
                      <Switch
                        checked={rule.enabled}
                        onCheckedChange={(enabled) => updateRule({ ...rule, enabled })}
                        aria-label={t("settings.rules.toggleRule", { name: rule.name })}
                      />
                    </div>
                  </TableCell>
                  <TableCell>{ruleActions(rule)}</TableCell>
                </TableRow>
              ))}
              {rules.length === 0 && (
                <TableRow><TableCell colSpan={6} className="py-5 text-center text-muted-foreground">{t("settings.rules.noRules")}</TableCell></TableRow>
              )}
            </TableBody>
            <TableFooter><TableRow><TableCell colSpan={5}>{t("settings.rules.totalRules")}</TableCell><TableCell className="text-right">{rules.length}</TableCell></TableRow></TableFooter>
          </Table></div>
          <div className="divide-y divide-border min-[900px]:hidden">
            {rules.map((rule) => (
              <div key={rule.id || rule.name} className="space-y-2 px-3 py-3">
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0"><div className="truncate text-sm font-medium">{rule.name}</div><div className="mt-0.5 line-clamp-2 text-xs text-muted-foreground">{ruleDescription(rule)}</div></div>
                  {ruleActions(rule)}
                </div>
                <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                  <span className="max-w-[16rem] truncate" title={ruleScope(rule)}>{ruleScope(rule)}</span>
                  <Badge variant="outline">{rule.action === "copy" ? t("settings.rules.actionCopy") : rule.action === "ignore" ? t("settings.rules.actionIgnore") : t("settings.rules.actionMove")}</Badge>
                  <span>{t("settings.rules.priority")}: {rule.priority}</span>
                  <Switch checked={rule.enabled} onCheckedChange={(enabled) => updateRule({ ...rule, enabled })} aria-label={t("settings.rules.toggleRule", { name: rule.name })} />
                </div>
              </div>
            ))}
            {rules.length === 0 && <div className="px-3 py-5 text-center text-sm text-muted-foreground">{t("settings.rules.noRules")}</div>}
          </div>
        </CardContent>
      </Card>

      <AlertDialog open={Boolean(pendingDeleteRule)} onOpenChange={(open) => { if (!open && !deletingRule) { setPendingDeleteRule(null); setDeleteRuleError(null); } }}>
        <AlertDialogPopup>
          <AlertDialogHeader><AlertDialogTitle>{t("settings.rules.delete")}</AlertDialogTitle><AlertDialogDescription>{pendingDeleteRule ? t("settings.rules.deleteConfirm", { name: pendingDeleteRule.name }) : ""}</AlertDialogDescription></AlertDialogHeader>
          {deleteRuleError && <div className="px-4 text-xs text-destructive">{deleteRuleError}</div>}
          <AlertDialogFooter>
            <AlertDialogClose render={<Button type="button" variant="outline" size="sm" disabled={deletingRule} />}>{t("common.cancel")}</AlertDialogClose>
            <Button type="button" variant="destructive" size="sm" onClick={() => void confirmDeleteRule()} disabled={deletingRule}>{deletingRule ? t("settings.rules.deleting") : t("settings.rules.delete")}</Button>
          </AlertDialogFooter>
        </AlertDialogPopup>
      </AlertDialog>
    </div>
  );
}
