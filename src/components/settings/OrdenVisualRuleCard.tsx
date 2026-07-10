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
import { OrdenPipelineEditor } from "./OrdenPipelineEditor";

interface OrdenVisualRuleCardProps {
  rule: OrdenVisualRule;
  index: number;
  onUpdate: (id: string, patch: Partial<OrdenVisualRule>) => void;
  onRemove: (id: string) => void;
  onChooseLocations: (id: string, directory: boolean) => void;
  onChooseDestinations: (id: string, stepId: string) => void;
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
              <OrdenPipelineEditor
                mode="filter"
                steps={rule.filterSteps || []}
                onChange={(filterSteps) => onUpdate(rule.id, { filterSteps })}
              />
            </CardContent>
          </Card>

          <Card className="shadow-none">
            <CardHeader>
              <CardTitle className="text-sm">3. {t("settings.orden.actionSettings")}</CardTitle>
              <CardDescription className="text-xs">{t("settings.orden.actionSettingsDesc")}</CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              <OrdenPipelineEditor
                mode="action"
                steps={rule.actionSteps || []}
                onChange={(actionSteps) => onUpdate(rule.id, { actionSteps })}
                onChooseDestination={(stepId) => onChooseDestinations(rule.id, stepId)}
              />
            </CardContent>
          </Card>
        </div>
      </CardContent>
    </Card>
  );
}
