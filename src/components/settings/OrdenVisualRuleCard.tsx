import { useTranslation } from "react-i18next";
import { FileCheck2, FolderOpen, Trash2 } from "lucide-react";
import { OrdenVisualRule } from "../../store/useAppStore";
import { Button } from "../ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../ui/card";
import { Checkbox } from "../ui/checkbox";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../ui/select";
import { Switch } from "../ui/switch";

interface OrdenVisualRuleCardProps {
  rule: OrdenVisualRule;
  index: number;
  onUpdate: (id: string, patch: Partial<OrdenVisualRule>) => void;
  onRemove: (id: string) => void;
  onChooseLocations: (id: string, directory: boolean) => void;
  onChooseDestinations: (id: string) => void;
}

export function OrdenVisualRuleCard({
  rule,
  index,
  onUpdate,
  onRemove,
  onChooseLocations,
  onChooseDestinations,
}: OrdenVisualRuleCardProps) {
  const { t } = useTranslation();
  const fieldId = (name: string) => `orden-${rule.id}-${name}`;

  return (
    <Card className="overflow-hidden">
      <CardHeader className="flex-row items-center justify-between gap-3 border-b border-border bg-muted/20">
        <div className="min-w-0">
          <CardTitle>{t("settings.orden.ruleName", { number: index + 1 })}</CardTitle>
          <CardDescription className="truncate">
            {rule.name || t("settings.orden.untitledRule")}
          </CardDescription>
        </div>
        <div className="flex shrink-0 items-center gap-3">
          <Label className="flex items-center gap-2 text-xs text-muted-foreground">
            <Switch
              checked={rule.enabled}
              onCheckedChange={(checked) => onUpdate(rule.id, { enabled: checked })}
              aria-label={t("settings.orden.toggleRule", { name: rule.name })}
            />
            {rule.enabled ? t("settings.orden.enabled") : t("settings.orden.stopped")}
          </Label>
          <Button
            type="button"
            onClick={() => onRemove(rule.id)}
            variant="ghost"
            size="icon"
            aria-label={t("settings.orden.deleteRule", { name: rule.name })}
            className="text-destructive hover:bg-destructive/10 hover:text-destructive"
          >
            <Trash2 size={14} />
          </Button>
        </div>
      </CardHeader>

      <CardContent className="space-y-3 pt-4">
        <Card className="shadow-none">
          <CardHeader>
            <CardTitle className="text-sm">1. {t("settings.orden.basicSettings")}</CardTitle>
            <CardDescription className="text-xs">{t("settings.orden.basicSettingsDesc")}</CardDescription>
          </CardHeader>
          <CardContent className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
            <div className="md:col-span-2 xl:col-span-1">
              <Label htmlFor={fieldId("name")} className="mb-1 block text-xs text-muted-foreground">
                {t("settings.orden.name")}
              </Label>
              <Input
                id={fieldId("name")}
                type="text"
                value={rule.name}
                onChange={(event) => onUpdate(rule.id, { name: event.target.value })}
              />
            </div>
            <div>
              <Label htmlFor={fieldId("targets")} className="mb-1 block text-xs text-muted-foreground">
                {t("settings.orden.targets")}
              </Label>
              <Select value={rule.targets} onValueChange={(value) => onUpdate(rule.id, { targets: value })}>
                <SelectTrigger id={fieldId("targets")}><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="files">{t("settings.orden.targetFiles")}</SelectItem>
                  <SelectItem value="dirs">{t("settings.orden.targetDirs")}</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div>
              <Label htmlFor={fieldId("tags")} className="mb-1 block text-xs text-muted-foreground">
                {t("settings.orden.tags")}
              </Label>
              <Input
                id={fieldId("tags")}
                type="text"
                value={rule.tags}
                onChange={(event) => onUpdate(rule.id, { tags: event.target.value })}
                placeholder="backup, docs"
              />
            </div>
            <Label className="flex items-center gap-2 text-sm md:col-span-2 xl:col-span-3">
              <Checkbox
                checked={rule.subfolders}
                onCheckedChange={(checked) => onUpdate(rule.id, { subfolders: checked === true })}
              />
              {t("settings.orden.subfolders")}
            </Label>
          </CardContent>
        </Card>

        <div className="grid gap-3 xl:grid-cols-2">
          <Card className="shadow-none">
            <CardHeader>
              <CardTitle className="text-sm">2. {t("settings.orden.sourceAndFilters")}</CardTitle>
              <CardDescription className="text-xs">{t("settings.orden.sourceAndFiltersDesc")}</CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              <div>
                <Label htmlFor={fieldId("locations")} className="mb-1 block text-xs text-muted-foreground">
                  {t("settings.orden.locations")}
                </Label>
                <textarea
                  id={fieldId("locations")}
                  value={rule.location}
                  onChange={(event) => onUpdate(rule.id, { location: event.target.value })}
                  placeholder="~/Downloads"
                  className="min-h-24 w-full resize-y rounded-lg border border-border bg-background px-3 py-2 text-xs leading-5 text-foreground outline-none focus:border-ring focus:ring-2 focus:ring-ring/20"
                />
                <div className="mt-2 flex flex-wrap gap-2">
                  <Button type="button" onClick={() => onChooseLocations(rule.id, false)} variant="outline" size="sm">
                    <FileCheck2 size={14} />
                    {t("settings.orden.chooseFiles")}
                  </Button>
                  <Button type="button" onClick={() => onChooseLocations(rule.id, true)} variant="outline" size="sm">
                    <FolderOpen size={14} />
                    {t("settings.orden.chooseFolders")}
                  </Button>
                </div>
              </div>
              <div className="grid gap-3 sm:grid-cols-[minmax(0,1fr)_10rem]">
                <div>
                  <Label htmlFor={fieldId("extensions")} className="mb-1 block text-xs text-muted-foreground">
                    {t("settings.orden.extensions")}
                  </Label>
                  <Input
                    id={fieldId("extensions")}
                    type="text"
                    value={rule.extensions}
                    onChange={(event) => onUpdate(rule.id, { extensions: event.target.value })}
                    placeholder="pdf, docx, xlsx"
                  />
                </div>
                <div>
                  <Label htmlFor={fieldId("filter-mode")} className="mb-1 block text-xs text-muted-foreground">
                    {t("settings.orden.filterMode")}
                  </Label>
                  <Select value={rule.filterMode || "all"} onValueChange={(value) => onUpdate(rule.id, { filterMode: value })}>
                    <SelectTrigger id={fieldId("filter-mode")}><SelectValue /></SelectTrigger>
                    <SelectContent>
                      <SelectItem value="all">{t("settings.orden.filterAll")}</SelectItem>
                      <SelectItem value="any">{t("settings.orden.filterAny")}</SelectItem>
                      <SelectItem value="none">{t("settings.orden.filterNone")}</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
            </CardContent>
          </Card>

          <Card className="shadow-none">
            <CardHeader>
              <CardTitle className="text-sm">3. {t("settings.orden.actionSettings")}</CardTitle>
              <CardDescription className="text-xs">{t("settings.orden.actionSettingsDesc")}</CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              <div>
                <Label htmlFor={fieldId("action")} className="mb-1 block text-xs text-muted-foreground">
                  {t("settings.orden.action")}
                </Label>
                <Select value={rule.action} onValueChange={(value) => onUpdate(rule.id, { action: value })}>
                  <SelectTrigger id={fieldId("action")}><SelectValue /></SelectTrigger>
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
                <Label htmlFor={fieldId("destinations")} className="mb-1 block text-xs text-muted-foreground">
                  {t("settings.orden.destinations")}
                </Label>
                <textarea
                  id={fieldId("destinations")}
                  value={rule.destination}
                  onChange={(event) => onUpdate(rule.id, { destination: event.target.value })}
                  placeholder="~/Documents/Shelfy Backups/"
                  className="min-h-24 w-full resize-y rounded-lg border border-border bg-background px-3 py-2 text-xs leading-5 text-foreground outline-none focus:border-ring focus:ring-2 focus:ring-ring/20"
                />
                <Button type="button" onClick={() => onChooseDestinations(rule.id)} variant="outline" size="sm" className="mt-2">
                  <FolderOpen size={14} />
                  {t("settings.orden.chooseDestinations")}
                </Button>
              </div>

              {["extract", "compress"].includes(rule.action) && (
                <div className="grid gap-3 border-t border-border pt-3 sm:grid-cols-2">
                  <div>
                    <Label htmlFor={fieldId("archive-format")} className="mb-1 block text-xs text-muted-foreground">
                      {t("settings.orden.archiveFormat")}
                    </Label>
                    <Select value={rule.archiveFormat || "auto"} onValueChange={(value) => onUpdate(rule.id, { archiveFormat: value })}>
                      <SelectTrigger id={fieldId("archive-format")}><SelectValue /></SelectTrigger>
                      <SelectContent>
                        <SelectItem value="auto">{t("settings.orden.archiveFormatAuto")}</SelectItem>
                        <SelectItem value="zip">ZIP</SelectItem>
                        <SelectItem value="7z">7z</SelectItem>
                        <SelectItem value="rar">RAR</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                  <div>
                    <Label htmlFor={fieldId("archive-password")} className="mb-1 block text-xs text-muted-foreground">
                      {rule.action === "extract" ? t("settings.orden.archivePasswords") : t("settings.orden.archivePassword")}
                    </Label>
                    <Input
                      id={fieldId("archive-password")}
                      type="password"
                      value={rule.action === "extract" ? rule.archivePasswords : rule.archivePassword}
                      onChange={(event) => onUpdate(rule.id, rule.action === "extract" ? { archivePasswords: event.target.value } : { archivePassword: event.target.value })}
                      placeholder={rule.action === "extract" ? "123456, password" : "optional password"}
                    />
                  </div>
                  <div>
                    <Label htmlFor={fieldId("conflict")} className="mb-1 block text-xs text-muted-foreground">
                      {t("settings.orden.onConflict")}
                    </Label>
                    <Select value={rule.onConflict || "rename_new"} onValueChange={(value) => onUpdate(rule.id, { onConflict: value })}>
                      <SelectTrigger id={fieldId("conflict")}><SelectValue /></SelectTrigger>
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
                      onCheckedChange={(checked) => onUpdate(rule.id, { deleteOriginal: checked === true })}
                    />
                    {t("settings.orden.deleteOriginal")}
                  </Label>
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      </CardContent>
    </Card>
  );
}
