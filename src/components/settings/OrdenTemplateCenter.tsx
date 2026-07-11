import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Archive,
  ArrowRight,
  Braces,
  Copy,
  FileInput,
  FileText,
  Filter,
  FolderArchive,
  Image,
  Layers3,
  LucideIcon,
  Plus,
  Save,
  Search,
  ShieldCheck,
  Sparkles,
  Trash2,
  WandSparkles,
} from "lucide-react";
import { OrdenTemplate } from "../../store/useAppStore";
import { cn } from "../../lib/utils";
import {
  AlertDialog,
  AlertDialogClose,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogPopup,
  AlertDialogTitle,
} from "../ui/alert-dialog";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { Card } from "../ui/card";
import {
  Dialog,
  DialogClose,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogPanel,
  DialogPopup,
  DialogTitle,
} from "../ui/dialog";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../ui/select";

interface OrdenTemplateCenterProps {
  templates: OrdenTemplate[];
  configNames: string[];
  onUseTemplate: (template: OrdenTemplate) => Promise<void>;
  onLoadConfig: (name: string) => Promise<string>;
  onSaveTemplate: (name: string, yaml: string) => Promise<void>;
  onDeleteTemplate: (template: OrdenTemplate) => Promise<void>;
}

const ICONS: Record<string, LucideIcon> = {
  archive: Archive,
  file: FileText,
  image: Image,
  "folder-archive": FolderArchive,
  layers: Layers3,
  sparkles: Sparkles,
};

function templateLabel(template: OrdenTemplate, t: (key: string) => string): string {
  return template.title_key ? t(template.title_key) : template.name;
}

function templateDescription(template: OrdenTemplate, t: (key: string) => string): string {
  return template.description_key
    ? t(template.description_key)
    : t("settings.orden.templates.customDescription").replace("{{config}}", template.name);
}

function templateCategory(template: OrdenTemplate, t: (key: string) => string): string {
  return template.category_key ? t(template.category_key) : t("settings.orden.templates.categories.custom");
}

type TemplateTone = "organize" | "automation" | "backup" | "maintenance" | "custom";
type SafetyLevel = "keeps" | "changes" | "destructive";

const TEMPLATE_TONES: Record<TemplateTone, { card: string; border: string; icon: string; bar: string }> = {
  organize: { card: "bg-primary/6", border: "border-primary/25", icon: "bg-primary/12 text-primary", bar: "bg-primary/70" },
  automation: { card: "bg-accent/20", border: "border-accent/55", icon: "bg-accent text-accent-foreground", bar: "bg-accent-foreground/55" },
  backup: { card: "bg-secondary/35", border: "border-primary/20", icon: "bg-secondary text-secondary-foreground", bar: "bg-primary/55" },
  maintenance: { card: "bg-muted/45", border: "border-border", icon: "bg-muted text-muted-foreground", bar: "bg-muted-foreground/55" },
  custom: { card: "bg-card", border: "border-border", icon: "bg-secondary text-secondary-foreground", bar: "bg-primary/55" },
};

function templateTone(template: OrdenTemplate): TemplateTone {
  const category = template.tone || template.category_key?.split(".").pop();
  return category && category in TEMPLATE_TONES ? category as TemplateTone : "custom";
}

type TemplateFlowSummary = {
  ruleCount: number;
  sources: string[];
  filters: string[];
  actions: string[];
  safety: SafetyLevel;
};

function scalar(value: string): string {
  return value.trim().replace(/^['"]|['"]$/g, "").replace(/,$/, "");
}

function sectionLines(yaml: string, section: string): string[] {
  const lines = yaml.split(/\r?\n/);
  const collected: string[] = [];
  lines.forEach((line, index) => {
    if (line.trim() !== `${section}:`) return;
    const indent = line.search(/\S/);
    for (let cursor = index + 1; cursor < lines.length; cursor += 1) {
      const next = lines[cursor];
      if (!next.trim()) continue;
      const nextIndent = next.search(/\S/);
      if (nextIndent <= indent) break;
      collected.push(next.trim());
    }
  });
  return collected;
}

function parseTemplateFlow(yaml: string): TemplateFlowSummary {
  const sourceLines = sectionLines(yaml, "locations");
  const sources = sourceLines.flatMap((line) => {
    const item = line.replace(/^\-\s*/, "");
    if (item.startsWith("path:")) return [scalar(item.slice(5))];
    if (!item.includes(":")) return [scalar(item)];
    return [];
  }).filter(Boolean);

  const stepKinds = (section: "filters" | "actions") => sectionLines(yaml, section).flatMap((line) => {
    const item = line.replace(/^\-\s*/, "").replace(/^not\s+/, "");
    if (!line.startsWith("- ")) return [];
    const kind = item.split(":", 1)[0].trim().split(/\s+/, 1)[0];
    return kind ? [kind] : [];
  });
  const filters = Array.from(new Set(stepKinds("filters")));
  const actions = Array.from(new Set(stepKinds("actions")));
  const destructive = actions.some((action) => ["delete", "trash", "shell"].includes(action));
  const changes = actions.some((action) => ["move", "rename", "write", "extract", "compress", "archive", "unarchive"].includes(action));

  return {
    ruleCount: Math.max((yaml.match(/^\s*-\s+name:/gm) || []).length, 1),
    sources: sources.length > 0 ? sources : ["—"],
    filters,
    actions,
    safety: destructive ? "destructive" : changes ? "changes" : "keeps",
  };
}

function compactList(values: string[], emptyLabel: string): string {
  if (values.length === 0) return emptyLabel;
  if (values.length <= 2) return values.join(" · ");
  return `${values.slice(0, 2).join(" · ")} +${values.length - 2}`;
}

export function OrdenTemplateCenter({
  templates,
  configNames,
  onUseTemplate,
  onLoadConfig,
  onSaveTemplate,
  onDeleteTemplate,
}: OrdenTemplateCenterProps) {
  const { t } = useTranslation();
  const [selectedId, setSelectedId] = useState<string | null>(templates[0]?.id || null);
  const [query, setQuery] = useState("");
  const [section, setSection] = useState<"all" | "system" | "custom">("all");
  const [category, setCategory] = useState<"all" | TemplateTone>("all");
  const [selectedConfig, setSelectedConfig] = useState(configNames[0] || "");
  const [busyId, setBusyId] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [draftYaml, setDraftYaml] = useState("");
  const [draftName, setDraftName] = useState("");
  const [createDialogOpen, setCreateDialogOpen] = useState(false);
  const [createName, setCreateName] = useState("");
  const [deleteTarget, setDeleteTarget] = useState<OrdenTemplate | null>(null);
  const [pendingSelection, setPendingSelection] = useState<string | null>(null);

  useEffect(() => {
    if (!selectedConfig || !configNames.includes(selectedConfig)) setSelectedConfig(configNames[0] || "");
  }, [configNames, selectedConfig]);

  const filteredTemplates = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    return templates.filter((template) => {
      const matchesSection = section === "all" || (section === "custom" ? !template.is_system : template.is_system);
      const matchesCategory = category === "all" || templateTone(template) === category;
      if (!matchesSection || !matchesCategory) return false;
      if (!normalized) return true;
      return `${templateLabel(template, t)} ${templateDescription(template, t)} ${templateCategory(template, t)}`.toLowerCase().includes(normalized);
    });
  }, [category, query, section, t, templates]);

  const selectedTemplate = templates.find((template) => template.id === selectedId) || null;
  const selectedSummary = useMemo(() => selectedTemplate ? parseTemplateFlow(draftYaml || selectedTemplate.yaml) : null, [draftYaml, selectedTemplate]);
  const selectedTone = selectedTemplate ? TEMPLATE_TONES[templateTone(selectedTemplate)] : null;
  const isDraftDirty = Boolean(selectedTemplate) && (
    draftYaml !== selectedTemplate!.yaml
    || Boolean(selectedTemplate!.is_system && draftName.trim())
  );

  useEffect(() => {
    if (templates.length === 0) {
      if (selectedId !== null) setSelectedId(null);
      return;
    }
    if (!templates.some((template) => template.id === selectedId)) setSelectedId(templates[0].id);
  }, [selectedId, templates]);

  useEffect(() => {
    if (isDraftDirty || filteredTemplates.length === 0) return;
    if (!filteredTemplates.some((template) => template.id === selectedId)) setSelectedId(filteredTemplates[0].id);
  }, [filteredTemplates, isDraftDirty, selectedId]);

  useEffect(() => {
    if (!selectedTemplate) {
      setDraftYaml("");
      setDraftName("");
      return;
    }
    setDraftYaml(selectedTemplate.yaml);
    setDraftName(selectedTemplate.is_system ? "" : selectedTemplate.name);
  }, [selectedTemplate]);

  const renderIcon = (template: OrdenTemplate, className = "size-6") => {
    const Icon = ICONS[template.icon] || Sparkles;
    return <Icon className={className} />;
  };

  const selectTemplate = (id: string) => {
    if (id === selectedId) return;
    if (isDraftDirty) {
      setPendingSelection(id);
      return;
    }
    setSelectedId(id);
  };

  const handleUse = async (template: OrdenTemplate) => {
    setBusyId(template.id);
    setNotice(null);
    try {
      const yaml = template.id === selectedId ? (draftYaml || template.yaml) : template.yaml;
      await onUseTemplate({ ...template, yaml });
      setNotice(t("settings.orden.templates.addedNotice"));
    } catch (error) {
      setNotice(String(error || t("settings.orden.templates.addError")));
    } finally {
      setBusyId(null);
    }
  };

  const handleSave = async () => {
    if (!selectedTemplate) return;
    const name = selectedTemplate.is_system ? draftName.trim() : selectedTemplate.name;
    if (!name || !draftYaml.trim()) {
      setNotice(t("settings.orden.templates.saveError"));
      return;
    }
    setBusyId(selectedTemplate.id);
    try {
      await onSaveTemplate(name, draftYaml);
      if (selectedTemplate.is_system) {
        const cleanName = name.replace(/\.ya?ml$/i, "");
        setQuery("");
        setCategory("custom");
        setSection("custom");
        setSelectedId(`custom-${cleanName}`);
      }
      setNotice(t("settings.orden.templates.savedNotice"));
    } catch (error) {
      setNotice(String(error || t("settings.orden.templates.saveError")));
    } finally {
      setBusyId(null);
    }
  };

  const openCreateDialog = () => {
    if (!selectedConfig) return;
    setCreateName(selectedConfig);
    setCreateDialogOpen(true);
  };

  const handleCreateFromConfig = async () => {
    const name = createName.trim();
    if (!selectedConfig || !name) return;
    setBusyId("create-from-config");
    setNotice(null);
    try {
      const yaml = await onLoadConfig(selectedConfig);
      await onSaveTemplate(name, yaml);
      const cleanName = name.replace(/\.ya?ml$/i, "");
      setQuery("");
      setCategory("custom");
      setSection("custom");
      setSelectedId(`custom-${cleanName}`);
      setCreateDialogOpen(false);
      setNotice(t("settings.orden.templates.createdNotice"));
    } catch (error) {
      setNotice(String(error || t("settings.orden.templates.createError")));
    } finally {
      setBusyId(null);
    }
  };

  const handleDelete = async () => {
    if (!deleteTarget) return;
    const target = deleteTarget;
    setBusyId(target.id);
    setNotice(null);
    try {
      await onDeleteTemplate(target);
      setDeleteTarget(null);
      setNotice(t("settings.orden.templates.deletedNotice"));
    } catch (error) {
      setNotice(String(error || t("settings.orden.templates.deleteError")));
    } finally {
      setBusyId(null);
    }
  };

  const safetyLabel = (safety: SafetyLevel) => t(`settings.orden.templates.safety.${safety}`, {
    defaultValue: safety === "keeps" ? "Keeps originals" : safety === "changes" ? "Changes files" : "Review before running",
  });

  const categories: Array<"all" | TemplateTone> = ["all", "organize", "automation", "backup", "maintenance", "custom"];

  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 className="text-lg font-semibold">{t("settings.orden.templates.title")}</h2>
          <p className="mt-1 text-xs text-muted-foreground">{t("settings.orden.templates.description")}</p>
        </div>
        {configNames.length > 0 && (
          <div className="flex flex-wrap items-center gap-2">
            <Select value={selectedConfig} onValueChange={setSelectedConfig} disabled={busyId === "create-from-config"}>
              <SelectTrigger className="h-8 w-44 text-xs"><SelectValue placeholder={t("settings.orden.templates.chooseConfig")} /></SelectTrigger>
              <SelectContent>{configNames.map((name) => <SelectItem key={name} value={name}>{name}</SelectItem>)}</SelectContent>
            </Select>
            <Button type="button" variant="outline" size="sm" onClick={openCreateDialog} disabled={busyId === "create-from-config"}><Save />{t("settings.orden.templates.saveAsTemplate")}</Button>
          </div>
        )}
      </div>

      <Card className="overflow-hidden border-primary/20">
        <div className="grid gap-0 min-[900px]:grid-cols-[15rem_minmax(0,1fr)]">
          <div className="border-b border-border bg-primary/8 p-4 min-[900px]:border-b-0 min-[900px]:border-r">
            <Badge variant="secondary">{t("settings.orden.templates.badge")}</Badge>
            <h3 className="mt-3 text-base font-semibold">{t("settings.orden.templates.goalTitle", { defaultValue: "What do you want Shelfy to do?" })}</h3>
            <p className="mt-1 text-xs text-muted-foreground">{t("settings.orden.templates.goalDescription", { defaultValue: "Start with an outcome. You can adjust every card before running." })}</p>
          </div>
          <div className="no-scrollbar flex gap-2 overflow-x-auto p-3">
            {categories.map((value) => {
              const count = value === "all" ? templates.length : templates.filter((template) => templateTone(template) === value).length;
              return (
                <button key={value} type="button" onClick={() => setCategory(value)} aria-pressed={category === value} className={cn("min-w-28 rounded-lg border px-3 py-2.5 text-left transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring", category === value ? "border-primary/50 bg-primary/10 text-primary" : "border-border bg-card hover:border-primary/25 hover:bg-muted/20")}>
                  <div className="text-xs font-semibold">{value === "all" ? t("settings.orden.templates.sections.all") : t(`settings.orden.templates.categories.${value}`)}</div>
                  <div className="mt-1 text-[10px] text-muted-foreground">{t("settings.orden.templates.recipeCount", { defaultValue: "{{count}} recipes", count })}</div>
                </button>
              );
            })}
          </div>
        </div>
      </Card>

      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex items-center gap-1 rounded-lg border border-border bg-muted/30 p-0.5">
          {(["all", "system", "custom"] as const).map((value) => <Button key={value} type="button" size="sm" variant={section === value ? "secondary" : "ghost"} aria-pressed={section === value} onClick={() => setSection(value)}>{t(`settings.orden.templates.sections.${value}`)}</Button>)}
        </div>
        <div className="relative w-full sm:w-64"><Search className="absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" /><Input type="search" aria-label={t("settings.orden.templates.search")} value={query} onChange={(event) => setQuery(event.target.value)} placeholder={t("settings.orden.templates.search")} className="h-8 pl-8" /></div>
      </div>

      {notice && <div role="status" aria-live="polite" className="rounded-lg border border-primary/25 bg-primary/10 px-3 py-2 text-xs text-primary">{notice}</div>}

      <div className="grid items-start gap-3 xl:grid-cols-[minmax(0,1fr)_22rem]">
        <div className="order-2 xl:order-1">
          {filteredTemplates.length > 0 ? (
            <div className="grid gap-2.5 sm:grid-cols-2 min-[1100px]:grid-cols-3 xl:grid-cols-2">
              {filteredTemplates.map((template) => {
                const tone = TEMPLATE_TONES[templateTone(template)];
                const label = templateLabel(template, t);
                const summary = parseTemplateFlow(template.yaml);
                return (
                  <article key={template.id} className={cn("group relative min-h-44 overflow-hidden rounded-lg border text-left transition hover:-translate-y-0.5 hover:shadow-md", tone.card, tone.border, selectedId === template.id && "ring-1 ring-primary/45")}>
                    <button type="button" className="absolute inset-0 z-0 rounded-lg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring" onClick={() => selectTemplate(template.id)} aria-label={t("settings.orden.templates.viewTemplate", { name: label })} />
                    <div className={cn("h-1", tone.bar)} />
                    <div className="pointer-events-none relative z-[1] flex min-h-43 flex-col p-3 text-foreground">
                      <div className="flex items-start justify-between gap-3">
                        <div className={cn("flex size-9 items-center justify-center rounded-lg", tone.icon)}>{renderIcon(template, "size-4")}</div>
                        <Button type="button" size="icon-sm" variant="ghost" className="pointer-events-auto relative z-10 bg-background/70 text-foreground hover:bg-background" onClick={() => void handleUse(template)} disabled={busyId === template.id} aria-label={t("settings.orden.templates.addToOrden")}><Plus /></Button>
                      </div>
                      <div className="mt-3"><div className="text-[10px] font-medium uppercase tracking-[0.12em] text-muted-foreground">{templateCategory(template, t)}</div><h3 className="mt-0.5 text-sm font-semibold leading-tight">{label}</h3><p className="mt-1 line-clamp-2 text-xs text-muted-foreground">{templateDescription(template, t)}</p></div>
                      <div className="mt-auto flex items-center gap-1.5 pt-3 text-[10px] text-muted-foreground">
                        <span className="max-w-20 truncate">{compactList(summary.sources, "—")}</span><ArrowRight className="size-3 shrink-0" /><span className="max-w-16 truncate">{compactList(summary.filters, t("settings.orden.noFilter"))}</span><ArrowRight className="size-3 shrink-0" /><span className="max-w-16 truncate">{compactList(summary.actions, "—")}</span>
                      </div>
                    </div>
                  </article>
                );
              })}
            </div>
          ) : (
            <Card className="py-12 text-center text-sm text-muted-foreground">{section === "custom" && !query.trim() ? t("settings.orden.templates.emptyCustom") : t("settings.orden.templates.empty")}</Card>
          )}
        </div>

        <aside className="order-1 xl:order-2 xl:sticky xl:top-0">
          {selectedTemplate && selectedTone && selectedSummary ? (
            <Card className={cn("overflow-hidden", selectedTone.border)}>
              <div className={cn("h-1.5", selectedTone.bar)} />
              <div className="p-3.5">
                <div className="flex items-start gap-3">
                  <div className={cn("flex size-10 shrink-0 items-center justify-center rounded-lg", selectedTone.icon)}>{renderIcon(selectedTemplate, "size-5")}</div>
                  <div className="min-w-0"><div className="flex flex-wrap items-center gap-1.5"><Badge variant="outline">{templateCategory(selectedTemplate, t)}</Badge><Badge variant={selectedSummary.safety === "destructive" ? "destructive" : "secondary"}>{safetyLabel(selectedSummary.safety)}</Badge></div><h3 className="mt-2 text-base font-semibold">{templateLabel(selectedTemplate, t)}</h3><p className="mt-1 text-xs text-muted-foreground">{templateDescription(selectedTemplate, t)}</p></div>
                </div>

                <div className="mt-3 space-y-1.5">
                  <div className="rounded-lg border border-border bg-background p-2.5"><div className="flex items-center gap-2 text-xs font-semibold"><FileInput className="size-3.5 text-primary" />{t("settings.orden.workflow.source", { defaultValue: "Source" })}</div><div className="mt-1 truncate font-mono text-[10px] text-muted-foreground" title={selectedSummary.sources.join("\n")}>{compactList(selectedSummary.sources, "—")}</div></div>
                  <div className="ml-5 h-2 border-l border-dashed border-border" />
                  <div className="rounded-lg border border-border bg-background p-2.5"><div className="flex items-center gap-2 text-xs font-semibold"><Filter className="size-3.5 text-primary" />{t("settings.orden.workflow.conditions", { defaultValue: "Conditions" })}</div><div className="mt-1 text-[10px] text-muted-foreground">{compactList(selectedSummary.filters, t("settings.orden.workflow.matchEverything", { defaultValue: "Match everything" }))}</div></div>
                  <div className="ml-5 h-2 border-l border-dashed border-border" />
                  <div className="rounded-lg border border-border bg-background p-2.5"><div className="flex items-center gap-2 text-xs font-semibold"><WandSparkles className="size-3.5 text-primary" />{t("settings.orden.workflow.actions", { defaultValue: "Actions" })}</div><div className="mt-1 text-[10px] text-muted-foreground">{compactList(selectedSummary.actions, "—")}</div></div>
                </div>

                <div className="mt-3 flex items-start gap-2 rounded-lg border border-primary/20 bg-primary/8 p-2.5 text-xs text-muted-foreground">
                  {selectedSummary.safety === "keeps" ? <ShieldCheck className="mt-0.5 size-3.5 shrink-0 text-primary" /> : <Braces className="mt-0.5 size-3.5 shrink-0 text-primary" />}
                  <span>{t("settings.orden.templates.summary", { defaultValue: "{{rules}} rule(s), {{sources}} source(s), {{steps}} total steps.", rules: selectedSummary.ruleCount, sources: selectedSummary.sources.length, steps: selectedSummary.filters.length + selectedSummary.actions.length })}</span>
                </div>

                <Button type="button" className="mt-3 w-full" onClick={() => void handleUse(selectedTemplate)} disabled={busyId === selectedTemplate.id}><Plus />{t("settings.orden.templates.useAndCustomize", { defaultValue: "Use and customize" })}</Button>

                <details className="mt-3 rounded-lg border border-border bg-muted/15">
                  <summary className="cursor-pointer select-none px-3 py-2 text-xs font-medium text-muted-foreground">{t("settings.orden.templates.advanced", { defaultValue: "Advanced template options" })}</summary>
                  <div className="space-y-2.5 border-t border-border p-3">
                    {selectedTemplate.is_system && <div><Label htmlFor="orden-template-name" className="mb-1 block text-xs">{t("settings.orden.templates.customNameLabel")}</Label><Input id="orden-template-name" value={draftName} onChange={(event) => setDraftName(event.target.value)} placeholder={t("settings.orden.templates.customName")} /></div>}
                    <div><Label htmlFor="orden-template-yaml" className="mb-1 block text-xs">{t("settings.orden.templates.yamlLabel")}</Label><textarea id="orden-template-yaml" value={draftYaml} onChange={(event) => setDraftYaml(event.target.value)} spellCheck={false} className="min-h-36 w-full resize-y rounded-md border border-input bg-background p-2.5 font-mono text-[10px] leading-5 text-foreground outline-none focus:border-ring focus:ring-2 focus:ring-ring/20" /></div>
                    <div className="flex flex-wrap gap-1.5"><Button type="button" variant="outline" size="sm" onClick={() => void handleSave()} disabled={busyId === selectedTemplate.id || (selectedTemplate.is_system && !draftName.trim())}><Copy />{selectedTemplate.is_system ? t("settings.orden.templates.saveAsCustom") : t("settings.orden.templates.saveChanges")}</Button>{!selectedTemplate.is_system && <Button type="button" variant="ghost" size="sm" className="text-destructive hover:bg-destructive/10 hover:text-destructive" onClick={() => setDeleteTarget(selectedTemplate)} disabled={busyId === selectedTemplate.id}><Trash2 />{t("settings.orden.templates.delete")}</Button>}</div>
                  </div>
                </details>
              </div>
            </Card>
          ) : <Card className="p-6 text-center text-sm text-muted-foreground">{t("settings.orden.templates.empty")}</Card>}
        </aside>
      </div>

      <Dialog open={createDialogOpen} onOpenChange={setCreateDialogOpen}>
        <DialogPopup>
          <DialogHeader><DialogTitle>{t("settings.orden.templates.saveAsTemplate")}</DialogTitle><DialogDescription>{t("settings.orden.templates.createDescription", { defaultValue: "Save this workflow as a reusable starting point." })}</DialogDescription></DialogHeader>
          <DialogPanel className="space-y-3"><div><Label htmlFor="orden-create-template-name" className="mb-1 block text-xs">{t("settings.orden.templates.customNameLabel")}</Label><Input id="orden-create-template-name" autoFocus value={createName} onChange={(event) => setCreateName(event.target.value)} /></div><div className="rounded-lg border border-border bg-muted/20 px-3 py-2 text-xs text-muted-foreground">{selectedConfig}</div></DialogPanel>
          <DialogFooter><DialogClose render={<Button type="button" variant="outline" size="sm" />}>{t("common.cancel")}</DialogClose><Button type="button" size="sm" onClick={() => void handleCreateFromConfig()} disabled={!createName.trim() || busyId === "create-from-config"}><Save />{t("settings.orden.templates.saveAsTemplate")}</Button></DialogFooter>
        </DialogPopup>
      </Dialog>

      <AlertDialog open={Boolean(deleteTarget)} onOpenChange={(open) => { if (!open && busyId !== deleteTarget?.id) setDeleteTarget(null); }}>
        <AlertDialogPopup><AlertDialogHeader><AlertDialogTitle>{t("settings.orden.templates.delete")}</AlertDialogTitle><AlertDialogDescription>{deleteTarget ? t("settings.orden.templates.deleteConfirm", { name: deleteTarget.name }) : ""}</AlertDialogDescription></AlertDialogHeader><AlertDialogFooter><AlertDialogClose render={<Button type="button" variant="outline" size="sm" />}>{t("common.cancel")}</AlertDialogClose><Button type="button" variant="destructive" size="sm" onClick={() => void handleDelete()} disabled={Boolean(deleteTarget && busyId === deleteTarget.id)}><Trash2 />{t("settings.orden.templates.delete")}</Button></AlertDialogFooter></AlertDialogPopup>
      </AlertDialog>

      <AlertDialog open={Boolean(pendingSelection)} onOpenChange={(open) => { if (!open) setPendingSelection(null); }}>
        <AlertDialogPopup><AlertDialogHeader><AlertDialogTitle>{t("settings.orden.templates.unsavedTitle", { defaultValue: "Discard template changes?" })}</AlertDialogTitle><AlertDialogDescription>{t("settings.orden.templates.unsavedConfirm")}</AlertDialogDescription></AlertDialogHeader><AlertDialogFooter><AlertDialogClose render={<Button type="button" variant="outline" size="sm" />}>{t("common.cancel")}</AlertDialogClose><Button type="button" variant="destructive" size="sm" onClick={() => { if (pendingSelection) setSelectedId(pendingSelection); setPendingSelection(null); }}>{t("settings.orden.templates.discard", { defaultValue: "Discard changes" })}</Button></AlertDialogFooter></AlertDialogPopup>
      </AlertDialog>
    </div>
  );
}
