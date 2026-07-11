import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  ChevronLeft,
  ChevronRight,
  FileCheck2,
  Files,
  Filter,
  FolderOpen,
  Route,
  Trash2,
  WandSparkles,
} from "lucide-react";
import { OrdenVisualRule } from "../../store/useAppStore";
import { Badge } from "../ui/badge";
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
  const sources = rule.location.split(/\r?\n/).map((path) => path.trim()).filter(Boolean);
  const visibleSources = sources.length > 0 ? sources : [t("settings.orden.workflow.sourcePlaceholder", { defaultValue: "Choose a file or folder" })];
  const [sourceIndex, setSourceIndex] = useState(0);
  const [sourceEditing, setSourceEditing] = useState(false);

  useEffect(() => {
    if (sourceIndex >= visibleSources.length) setSourceIndex(Math.max(visibleSources.length - 1, 0));
  }, [sourceIndex, visibleSources.length]);

  const filterCount = rule.filterSteps?.length || 0;
  const actionCount = rule.actionSteps?.length || 0;

  return (
    <Card className="overflow-hidden">
      <CardHeader className="flex-row items-center justify-between gap-3 border-b border-border bg-muted/20">
        <div className="flex min-w-0 items-center gap-3">
          <div className="flex size-9 shrink-0 items-center justify-center rounded-lg border border-primary/20 bg-primary/10 text-sm font-semibold text-primary">
            {index + 1}
          </div>
          <div className="min-w-0">
            <CardTitle className="truncate">{rule.name || t("settings.orden.untitledRule")}</CardTitle>
            <CardDescription className="mt-0.5 flex flex-wrap items-center gap-1.5 text-xs">
              <span>{t("settings.orden.workflow.cardSummary", { defaultValue: "{{sources}} sources · {{filters}} conditions · {{actions}} actions", sources: sources.length || 1, filters: filterCount, actions: actionCount })}</span>
            </CardDescription>
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <Label className="flex items-center gap-2 text-xs text-muted-foreground">
            <Switch checked={rule.enabled} onCheckedChange={(checked) => onUpdate(rule.id, { enabled: checked })} aria-label={t("settings.orden.toggleRule", { name: rule.name })} />
            <span className="hidden sm:inline">{rule.enabled ? t("settings.orden.enabled") : t("settings.orden.stopped")}</span>
          </Label>
          <Button type="button" onClick={() => onRemove(rule.id)} variant="ghost" size="icon-sm" aria-label={t("settings.orden.deleteRule", { name: rule.name })} className="text-destructive hover:bg-destructive/10 hover:text-destructive">
            <Trash2 />
          </Button>
        </div>
      </CardHeader>

      <CardContent className="space-y-3 pt-3.5">
        <div className="grid gap-2.5 rounded-lg border border-border/80 bg-muted/15 p-3 md:grid-cols-2 xl:grid-cols-[minmax(0,1.2fr)_minmax(9rem,.55fr)_minmax(0,.8fr)]">
          <div>
            <Label htmlFor={fieldId("name")} className="mb-1 block text-xs text-muted-foreground">{t("settings.orden.name")}</Label>
            <Input id={fieldId("name")} value={rule.name} onChange={(event) => onUpdate(rule.id, { name: event.target.value })} />
          </div>
          <div>
            <Label htmlFor={fieldId("targets")} className="mb-1 block text-xs text-muted-foreground">{t("settings.orden.targets")}</Label>
            <Select value={rule.targets} onValueChange={(value) => onUpdate(rule.id, { targets: value })}>
              <SelectTrigger id={fieldId("targets")}><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem value="files">{t("settings.orden.targetFiles")}</SelectItem>
                <SelectItem value="dirs">{t("settings.orden.targetDirs")}</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div className="md:col-span-2 xl:col-span-1">
            <Label htmlFor={fieldId("tags")} className="mb-1 block text-xs text-muted-foreground">{t("settings.orden.tags")}</Label>
            <Input id={fieldId("tags")} value={rule.tags} onChange={(event) => onUpdate(rule.id, { tags: event.target.value })} placeholder="backup, docs" />
          </div>
        </div>

        <div className="flex items-center gap-2 px-1">
          <Route className="size-4 text-primary" />
          <div>
            <div className="text-sm font-semibold">{t("settings.orden.workflow.flowTitle", { defaultValue: "Workflow cards" })}</div>
            <div className="text-xs text-muted-foreground">{t("settings.orden.workflow.flowDescription", { defaultValue: "Choose a card to inspect it. Cards run from source to action." })}</div>
          </div>
        </div>

        <div className="grid gap-3 min-[1120px]:grid-cols-[minmax(15rem,.72fr)_minmax(20rem,1fr)_minmax(20rem,1fr)]">
          <section className="min-w-0 rounded-lg border border-border bg-card p-3 shadow-sm">
            <div className="mb-2.5 flex items-center justify-between gap-2">
              <div className="flex items-center gap-2">
                <div className="flex size-7 items-center justify-center rounded-md bg-primary/10 text-primary"><Files className="size-3.5" /></div>
                <div>
                  <h4 className="text-sm font-semibold">{t("settings.orden.workflow.source", { defaultValue: "Source" })}</h4>
                  <p className="text-[11px] text-muted-foreground">{t("settings.orden.workflow.sourceHelp", { defaultValue: "Files entering this flow" })}</p>
                </div>
              </div>
              <div className="flex items-center gap-0.5">
                <Button type="button" variant="ghost" size="icon-sm" onClick={() => setSourceIndex((current) => Math.max(0, current - 1))} disabled={sourceIndex === 0} aria-label={t("settings.orden.workflow.previousCard", { defaultValue: "Previous card" })}><ChevronLeft /></Button>
                <span className="min-w-9 text-center text-[10px] tabular-nums text-muted-foreground">{sourceIndex + 1}/{visibleSources.length}</span>
                <Button type="button" variant="ghost" size="icon-sm" onClick={() => setSourceIndex((current) => Math.min(visibleSources.length - 1, current + 1))} disabled={sourceIndex >= visibleSources.length - 1} aria-label={t("settings.orden.workflow.nextCard", { defaultValue: "Next card" })}><ChevronRight /></Button>
              </div>
            </div>

            <button type="button" onClick={() => setSourceEditing((current) => !current)} className={`w-full rounded-lg border p-3 text-left shadow-sm transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${sourceEditing ? "border-primary/55 bg-primary/8 ring-1 ring-primary/20" : "border-border bg-background hover:border-primary/30"}`} aria-expanded={sourceEditing}>
              <div className="flex items-start justify-between gap-2">
                <div className="flex size-8 items-center justify-center rounded-lg bg-primary text-primary-foreground"><FolderOpen className="size-4" /></div>
                <Badge variant="outline" className="text-[10px]">{rule.targets === "dirs" ? t("settings.orden.targetDirs") : t("settings.orden.targetFiles")}</Badge>
              </div>
              <div className="mt-3 break-all text-sm font-semibold leading-tight">{visibleSources[sourceIndex]}</div>
              <div className="mt-1 text-xs text-muted-foreground">{rule.subfolders ? t("settings.orden.subfolders") : t("settings.orden.workflow.currentFolderOnly", { defaultValue: "Current folder only" })}</div>
            </button>

            {sourceEditing && (
              <div className="mt-2.5 space-y-2 rounded-lg border border-border/80 bg-muted/15 p-2.5">
                <Label htmlFor={fieldId("locations")} className="text-xs text-muted-foreground">{t("settings.orden.locations")}</Label>
                <textarea id={fieldId("locations")} value={rule.location} onChange={(event) => onUpdate(rule.id, { location: event.target.value })} placeholder="~/Downloads" className="min-h-20 w-full resize-y rounded-md border border-input bg-background px-2.5 py-2 text-xs leading-5 text-foreground outline-none focus:border-ring focus:ring-2 focus:ring-ring/20" />
                <Label className="flex items-center gap-2 text-xs text-muted-foreground"><Checkbox checked={rule.subfolders} onCheckedChange={(checked) => onUpdate(rule.id, { subfolders: checked === true })} />{t("settings.orden.subfolders")}</Label>
                <div className="flex flex-wrap gap-1.5">
                  <Button type="button" onClick={() => onChooseLocations(rule.id, false)} variant="outline" size="sm"><FileCheck2 />{t("settings.orden.chooseFiles")}</Button>
                  <Button type="button" onClick={() => onChooseLocations(rule.id, true)} variant="outline" size="sm"><FolderOpen />{t("settings.orden.chooseFolders")}</Button>
                </div>
              </div>
            )}
          </section>

          <section className="min-w-0 rounded-lg border border-border bg-card p-3 shadow-sm">
            <div className="mb-2.5 flex flex-wrap items-center justify-between gap-2">
              <div className="flex items-center gap-2">
                <div className="flex size-7 items-center justify-center rounded-md bg-primary/10 text-primary"><Filter className="size-3.5" /></div>
                <div>
                  <h4 className="text-sm font-semibold">{t("settings.orden.workflow.conditions", { defaultValue: "Conditions" })}</h4>
                  <p className="text-[11px] text-muted-foreground">{t("settings.orden.workflow.conditionsHelp", { defaultValue: "Which items may continue" })}</p>
                </div>
              </div>
              <div className="flex rounded-lg border border-border bg-muted/20 p-0.5" aria-label={t("settings.orden.filterMode")}>
                {(["all", "any", "none"] as const).map((mode) => (
                  <Button key={mode} type="button" size="xs" variant={(rule.filterMode || "all") === mode ? "secondary" : "ghost"} aria-pressed={(rule.filterMode || "all") === mode} onClick={() => onUpdate(rule.id, { filterMode: mode })} className="h-6 px-2 text-[10px]">
                    {t(`settings.orden.workflow.mode_${mode}`, { defaultValue: mode.toUpperCase() })}
                  </Button>
                ))}
              </div>
            </div>
            <OrdenPipelineEditor mode="filter" steps={rule.filterSteps || []} onChange={(filterSteps) => onUpdate(rule.id, { filterSteps })} />
          </section>

          <section className="min-w-0 rounded-lg border border-border bg-card p-3 shadow-sm">
            <div className="mb-2.5 flex items-center gap-2">
              <div className="flex size-7 items-center justify-center rounded-md bg-primary/10 text-primary"><WandSparkles className="size-3.5" /></div>
              <div>
                <h4 className="text-sm font-semibold">{t("settings.orden.workflow.actions", { defaultValue: "Actions" })}</h4>
                <p className="text-[11px] text-muted-foreground">{t("settings.orden.workflow.actionsHelp", { defaultValue: "What happens, in order" })}</p>
              </div>
            </div>
            <OrdenPipelineEditor mode="action" steps={rule.actionSteps || []} onChange={(actionSteps) => onUpdate(rule.id, { actionSteps })} onChooseDestination={(stepId) => onChooseDestinations(rule.id, stepId)} />
          </section>
        </div>
      </CardContent>
    </Card>
  );
}
