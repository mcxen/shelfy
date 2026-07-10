import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Archive,
  ArrowDownToLine,
  FileText,
  FolderArchive,
  Image,
  Layers3,
  LucideIcon,
  Plus,
  Save,
  Search,
  Sparkles,
  Trash2,
} from "lucide-react";
import { OrdenTemplate } from "../../store/useAppStore";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { Card } from "../ui/card";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../ui/select";
import { cn } from "../../lib/utils";

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

const TEMPLATE_TONES: Record<TemplateTone, { card: string; border: string; icon: string; bar: string }> = {
  organize: {
    card: "bg-primary/8",
    border: "border-primary/30",
    icon: "bg-primary/12 text-primary",
    bar: "bg-primary/70",
  },
  automation: {
    card: "bg-accent/20",
    border: "border-accent/50",
    icon: "bg-accent text-accent-foreground",
    bar: "bg-accent-foreground/60",
  },
  backup: {
    card: "bg-secondary/35",
    border: "border-primary/20",
    icon: "bg-secondary text-secondary-foreground",
    bar: "bg-primary/55",
  },
  maintenance: {
    card: "bg-muted/45",
    border: "border-border",
    icon: "bg-muted text-muted-foreground",
    bar: "bg-muted-foreground/55",
  },
  custom: {
    card: "bg-card",
    border: "border-border",
    icon: "bg-secondary text-secondary-foreground",
    bar: "bg-primary/55",
  },
};

function templateTone(template: OrdenTemplate): TemplateTone {
  const category = template.tone || template.category_key?.split(".").pop();
  return category && category in TEMPLATE_TONES ? category as TemplateTone : "custom";
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
  const [selectedConfig, setSelectedConfig] = useState(configNames[0] || "");
  const [busyId, setBusyId] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [draftYaml, setDraftYaml] = useState("");
  const [draftName, setDraftName] = useState("");

  useEffect(() => {
    if (!selectedConfig || !configNames.includes(selectedConfig)) setSelectedConfig(configNames[0] || "");
  }, [configNames, selectedConfig]);

  const filteredTemplates = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    return templates.filter((template) => {
      const matchesSection = section === "all" || (section === "custom" ? !template.is_system : template.is_system);
      if (!matchesSection) return false;
      if (!normalized) return true;
      return `${templateLabel(template, t)} ${templateDescription(template, t)} ${templateCategory(template, t)}`.toLowerCase().includes(normalized);
    });
  }, [query, section, t, templates]);
  const selectedTemplate = templates.find((template) => template.id === selectedId) || null;
  const selectedTone = selectedTemplate ? TEMPLATE_TONES[templateTone(selectedTemplate)] : null;
  const isDraftDirty = Boolean(selectedTemplate) && draftYaml !== selectedTemplate!.yaml;

  // Keep selection pointing at a valid template without silently switching when
  // the active one is filtered out by section/search — that would reset the
  // draft and lose unsaved edits.
  useEffect(() => {
    if (templates.length === 0) {
      if (selectedId !== null) setSelectedId(null);
      return;
    }
    if (!templates.some((template) => template.id === selectedId)) {
      setSelectedId(templates[0].id);
    }
  }, [selectedId, templates]);

  const handleSelectTemplate = (id: string) => {
    if (id === selectedId) return;
    if (isDraftDirty && !window.confirm(t("settings.orden.templates.unsavedConfirm"))) return;
    setSelectedId(id);
  };

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

  const handleUse = async (template: OrdenTemplate) => {
    setBusyId(template.id);
    setNotice(null);
    try {
      await onUseTemplate(template);
      setNotice(t("settings.orden.templates.addedNotice"));
    } catch (error) {
      setNotice(String(error || t("settings.orden.templates.addError")));
    } finally {
      setBusyId(null);
    }
  };

  const handleSave = async () => {
    if (!selectedTemplate) return;
    const isSystemTemplate = selectedTemplate.is_system;
    const name = isSystemTemplate ? draftName.trim() : selectedTemplate.name;
    if (!name || !draftYaml.trim()) {
      setNotice(t("settings.orden.templates.saveError"));
      return;
    }
    setBusyId(selectedTemplate.id);
    try {
      await onSaveTemplate(name, draftYaml);
      if (isSystemTemplate) {
        const cleanName = name.replace(/\.ya?ml$/i, "");
        setQuery("");
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

  const handleCreateFromConfig = async () => {
    if (!selectedConfig) return;
    const name = window.prompt(t("settings.orden.templates.createPrompt"), selectedConfig);
    if (!name?.trim()) return;
    setBusyId("create-from-config");
    setNotice(null);
    try {
      const yaml = await onLoadConfig(selectedConfig);
      await onSaveTemplate(name.trim(), yaml);
      const cleanName = name.trim().replace(/\.ya?ml$/i, "");
      setQuery("");
      setSection("custom");
      setSelectedId(`custom-${cleanName}`);
      setNotice(t("settings.orden.templates.createdNotice"));
    } catch (error) {
      setNotice(String(error || t("settings.orden.templates.createError")));
    } finally {
      setBusyId(null);
    }
  };

  const handleDelete = async (template: OrdenTemplate) => {
    if (template.is_system) return;
    if (!window.confirm(t("settings.orden.templates.deleteConfirm", { name: template.name }))) return;
    setBusyId(template.id);
    setNotice(null);
    try {
      await onDeleteTemplate(template);
      setSelectedId(null);
      setNotice(t("settings.orden.templates.deletedNotice"));
    } catch (error) {
      setNotice(String(error || t("settings.orden.templates.deleteError")));
    } finally {
      setBusyId(null);
    }
  };

  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 className="text-lg font-semibold">{t("settings.orden.templates.title")}</h2>
          <p className="mt-1 text-xs text-muted-foreground">{t("settings.orden.templates.description")}</p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          {configNames.length > 0 && <>
            <Select value={selectedConfig} onValueChange={setSelectedConfig} disabled={busyId === "create-from-config"}>
              <SelectTrigger className="h-8 w-44 text-xs"><SelectValue placeholder={t("settings.orden.templates.chooseConfig")} /></SelectTrigger>
              <SelectContent>{configNames.map((name) => <SelectItem key={name} value={name}>{name}</SelectItem>)}</SelectContent>
            </Select>
            <Button type="button" variant="outline" size="sm" onClick={handleCreateFromConfig} disabled={busyId === "create-from-config"}><Layers3 size={14} />{t("settings.orden.templates.saveAsTemplate")}</Button>
          </>}
        </div>
      </div>

      <Card className="relative overflow-hidden border-primary/20 bg-card p-4">
        <div className="pointer-events-none absolute -right-8 -top-12 size-36 rounded-full bg-primary/8 blur-3xl" />
        <div className="relative flex flex-wrap items-center justify-between gap-4">
          <div className="max-w-2xl">
            <Badge variant="secondary" className="mb-2">{t("settings.orden.templates.badge")}</Badge>
            <h3 className="text-xl font-semibold tracking-tight">{t("settings.orden.templates.heroTitle")}</h3>
            <p className="mt-1 max-w-xl text-sm text-muted-foreground">{t("settings.orden.templates.heroDescription")}</p>
          </div>
          <div className="flex size-14 items-center justify-center rounded-lg bg-primary/10 text-primary ring-1 ring-primary/25"><ArrowDownToLine className="size-7 rotate-180" /></div>
        </div>
      </Card>

      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-1 rounded-lg border border-border bg-muted/30 p-0.5">
          {(["all", "system", "custom"] as const).map((value) => <Button key={value} type="button" size="sm" variant={section === value ? "secondary" : "ghost"} aria-pressed={section === value} onClick={() => setSection(value)}>{t(`settings.orden.templates.sections.${value}`)}</Button>)}
        </div>
        <div className="relative w-full sm:w-64"><Search className="absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" /><Input type="search" aria-label={t("settings.orden.templates.search")} value={query} onChange={(event) => setQuery(event.target.value)} placeholder={t("settings.orden.templates.search")} className="h-8 pl-8" /></div>
      </div>

      {selectedTemplate && selectedTone && <Card className={cn("overflow-hidden bg-card", selectedTone.border)}>
        <div className={cn("h-1.5", selectedTone.bar)} />
        <div className="grid gap-3 p-3 lg:grid-cols-[minmax(0,1fr)_minmax(18rem,0.9fr)]">
          <div>
            <div className="flex items-start gap-3">
              <div className={cn("flex size-10 shrink-0 items-center justify-center rounded-lg shadow-sm", selectedTone.icon)}>{renderIcon(selectedTemplate, "size-5")}</div>
              <div className="min-w-0"><div className="flex flex-wrap items-center gap-2"><h3 className="font-semibold">{templateLabel(selectedTemplate, t)}</h3><Badge variant="outline">{templateCategory(selectedTemplate, t)}</Badge></div><p className="mt-1 text-sm text-muted-foreground">{templateDescription(selectedTemplate, t)}</p></div>
            </div>
            {selectedTemplate.is_system && <div className="mt-3 max-w-sm space-y-1.5">
              <Label htmlFor="orden-template-name">{t("settings.orden.templates.customNameLabel")}</Label>
              <Input id="orden-template-name" type="text" value={draftName} onChange={(event) => setDraftName(event.target.value)} placeholder={t("settings.orden.templates.customName")} aria-describedby="orden-template-name-description" />
              <p id="orden-template-name-description" className="text-xs text-muted-foreground">{t("settings.orden.templates.customNameDescription")}</p>
            </div>}
            <div className="mt-3 flex flex-wrap items-center gap-2">
              <Button type="button" onClick={() => void handleUse(selectedTemplate)} disabled={busyId === selectedTemplate.id}><Plus size={14} />{t("settings.orden.templates.addToOrden")}</Button>
              <Button type="button" variant="outline" onClick={() => void handleSave()} disabled={busyId === selectedTemplate.id || (selectedTemplate.is_system && !draftName.trim())}><Save size={14} />{selectedTemplate.is_system ? t("settings.orden.templates.saveAsCustom") : t("settings.orden.templates.saveChanges")}</Button>
              {!selectedTemplate.is_system && <Button type="button" variant="ghost" className="text-destructive hover:bg-destructive/10 hover:text-destructive" onClick={() => void handleDelete(selectedTemplate)} disabled={busyId === selectedTemplate.id}><Trash2 size={14} />{t("settings.orden.templates.delete")}</Button>}
            </div>
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="orden-template-yaml">{t("settings.orden.templates.yamlLabel")}</Label>
            <textarea id="orden-template-yaml" value={draftYaml} onChange={(event) => setDraftYaml(event.target.value)} spellCheck={false} className="min-h-36 w-full resize-y rounded-lg border border-border bg-background/70 p-3 font-mono text-[11px] leading-5 text-foreground outline-none focus:border-ring focus:ring-2 focus:ring-ring/20" />
          </div>
        </div>
      </Card>}

      {notice && <div role="status" aria-live="polite" className="rounded-lg border border-primary/25 bg-primary/10 px-3 py-2 text-xs text-primary">{notice}</div>}

      {filteredTemplates.length > 0 ? <div className="grid gap-2.5 sm:grid-cols-2 xl:grid-cols-3">
        {filteredTemplates.map((template) => { const tone = TEMPLATE_TONES[templateTone(template)]; const label = templateLabel(template, t); return <article key={template.id} className={cn("group relative min-h-32 overflow-hidden rounded-lg border text-left transition hover:-translate-y-0.5 hover:shadow-md", tone.card, tone.border, selectedId === template.id && "ring-1 ring-primary/40")}>
          <button type="button" className="absolute inset-0 z-0 rounded-lg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring" onClick={() => handleSelectTemplate(template.id)} aria-label={t("settings.orden.templates.viewTemplate", { name: label })} />
          <div className="pointer-events-none relative z-[1] flex h-full min-h-32 flex-col justify-between p-3 text-foreground"><div className="flex items-start justify-between gap-3"><div className={cn("flex size-9 items-center justify-center rounded-lg", tone.icon)}>{renderIcon(template, "size-4")}</div><Button type="button" size="icon-sm" variant="ghost" className="pointer-events-auto relative z-10 bg-background/70 text-foreground hover:bg-background" onClick={() => void handleUse(template)} disabled={busyId === template.id} aria-label={t("settings.orden.templates.addToOrden")}><Plus size={15} /></Button></div><div><div className="mb-0.5 text-[10px] font-medium uppercase tracking-[0.12em] text-muted-foreground">{templateCategory(template, t)}</div><h3 className="text-sm font-semibold leading-tight">{label}</h3><p className="mt-1 line-clamp-2 text-xs text-muted-foreground">{templateDescription(template, t)}</p></div></div>
        </article>; })}
      </div> : <Card className="py-12 text-center text-sm text-muted-foreground">{section === "custom" && !query.trim() ? t("settings.orden.templates.emptyCustom") : t("settings.orden.templates.empty")}</Card>}
    </div>
  );
}
