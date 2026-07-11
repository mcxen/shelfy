import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Archive,
  ArrowDown,
  ArrowUp,
  Braces,
  ChevronLeft,
  ChevronRight,
  Clock3,
  Copy,
  FileCheck2,
  FileOutput,
  FileSearch,
  FileType2,
  Fingerprint,
  FolderInput,
  FolderOpen,
  Hash,
  Link2,
  LucideIcon,
  MessageSquareText,
  PackageOpen,
  Plus,
  Regex,
  Ruler,
  SearchCheck,
  Shell,
  Trash2,
  Type,
} from "lucide-react";
import { OrdenVisualStep } from "../../store/useAppStore";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { Checkbox } from "../ui/checkbox";
import { Label } from "../ui/label";
import {
  Menu,
  MenuGroup,
  MenuGroupLabel,
  MenuItem,
  MenuPopup,
  MenuSeparator,
  MenuTrigger,
} from "../ui/menu";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../ui/select";

type StepDefinition = {
  kind: string;
  label: string;
  hint: string;
  icon: LucideIcon;
  common?: boolean;
  destructive?: boolean;
};

const FILTER_DEFINITIONS: StepDefinition[] = [
  { kind: "extension", label: "File type", hint: "Match extensions such as PDF or PNG", icon: FileType2, common: true },
  { kind: "name", label: "File name", hint: "Match a name or wildcard pattern", icon: Type, common: true },
  { kind: "regex", label: "Pattern", hint: "Use a regular expression", icon: Regex, common: true },
  { kind: "size", label: "File size", hint: "Compare file or folder size", icon: Ruler, common: true },
  { kind: "created", label: "Created time", hint: "Filter by age or creation time", icon: Clock3 },
  { kind: "lastmodified", label: "Modified time", hint: "Filter by last modification", icon: Clock3 },
  { kind: "filecontent", label: "File content", hint: "Search text inside supported files", icon: FileSearch },
  { kind: "mimetype", label: "MIME type", hint: "Match a media or document type", icon: FileCheck2 },
  { kind: "empty", label: "Empty", hint: "Find empty files or folders", icon: SearchCheck },
  { kind: "duplicate", label: "Duplicate", hint: "Detect duplicate file contents", icon: Copy },
  { kind: "hash", label: "File hash", hint: "Calculate and expose a file hash", icon: Hash },
  { kind: "exif", label: "EXIF metadata", hint: "Read image metadata fields", icon: Fingerprint },
];

const ACTION_DEFINITIONS: StepDefinition[] = [
  { kind: "copy", label: "Copy", hint: "Keep the original and make a copy", icon: Copy, common: true },
  { kind: "move", label: "Move", hint: "Move the matched item to a folder", icon: FolderInput, common: true },
  { kind: "rename", label: "Rename", hint: "Rename with Orden template variables", icon: Type, common: true },
  { kind: "trash", label: "Move to trash", hint: "Send the matched item to system trash", icon: Trash2, common: true, destructive: true },
  { kind: "extract", label: "Extract archive", hint: "Unpack an archive into a folder", icon: PackageOpen, common: true },
  { kind: "compress", label: "Create archive", hint: "Compress matched items into a ZIP", icon: Archive, common: true },
  { kind: "echo", label: "Write to log", hint: "Add a message to the run log", icon: MessageSquareText },
  { kind: "write", label: "Write file", hint: "Append, prepend, or overwrite text", icon: FileOutput },
  { kind: "symlink", label: "Symbolic link", hint: "Create a symbolic link", icon: Link2 },
  { kind: "hardlink", label: "Hard link", hint: "Create a hard link", icon: Link2 },
  { kind: "shell", label: "Shell command", hint: "Run a local shell command", icon: Shell, destructive: true },
  { kind: "delete", label: "Delete permanently", hint: "Permanently remove the matched item", icon: Trash2, destructive: true },
  { kind: "archive", label: "Create archive", hint: "Alias for creating an archive", icon: Archive },
  { kind: "unarchive", label: "Extract archive", hint: "Alias for extracting an archive", icon: PackageOpen },
];

const DESTINATION_ACTIONS = new Set([
  "copy",
  "move",
  "symlink",
  "hardlink",
  "extract",
  "compress",
  "archive",
  "unarchive",
]);

const PRESETS: Record<string, string> = {
  extension: "[pdf, docx]",
  name: '"*.pdf"',
  regex: '"(?i).*invoice.*\\.pdf$"',
  size: '"> 10 MB"',
  empty: "",
  mimetype: '"application/pdf"',
  hash: "sha256",
  duplicate: "detect_original_by: first_seen\nhash_algorithm: sha256",
  created: "days: 30\nmode: older",
  lastmodified: "days: 30\nmode: older",
  filecontent: '"(?i)invoice"',
  exif: "tags: [DateTimeOriginal]\nlowercase: true",
  copy: "~/Documents/Shelfy Backups/",
  move: "~/Documents/Organized/",
  rename: '"{name}.organized{extension}"',
  delete: "",
  trash: "",
  echo: '"Matched {path}"',
  write: 'text: "{path}"\noutfile: ~/Documents/shelfy-matches.txt\nmode: append',
  symlink: "~/Documents/Shelfy Links/",
  hardlink: "~/Documents/Shelfy Links/",
  shell: 'cmd: "echo {path}"\nrun_in_simulation: false\nignore_errors: false',
  extract: "dest: ~/Documents/Extracted/\nformat: zip\ndelete_original: false",
  unarchive: "dest: ~/Documents/Extracted/\nformat: zip\ndelete_original: false",
  compress: "dest: ~/Documents/Archives/{name}.zip\nformat: zip\ndelete_original: false",
  archive: "dest: ~/Documents/Archives/{name}.zip\nformat: zip\ndelete_original: false",
};

interface OrdenPipelineEditorProps {
  mode: "filter" | "action";
  steps: OrdenVisualStep[];
  onChange: (steps: OrdenVisualStep[]) => void;
  onChooseDestination?: (stepId: string) => void;
}

export function OrdenPipelineEditor({
  mode,
  steps,
  onChange,
  onChooseDestination,
}: OrdenPipelineEditorProps) {
  const { t } = useTranslation();
  const definitions = mode === "filter" ? FILTER_DEFINITIONS : ACTION_DEFINITIONS;
  const railRef = useRef<HTMLDivElement>(null);
  const [selectedId, setSelectedId] = useState<string | null>(steps[0]?.id || null);

  useEffect(() => {
    if (steps.length === 0) {
      setSelectedId(null);
      return;
    }
    if (!steps.some((step) => step.id === selectedId)) setSelectedId(steps[0].id);
  }, [selectedId, steps]);

  const definitionFor = (kind: string) =>
    definitions.find((definition) => definition.kind === kind) || {
      kind,
      label: kind,
      hint: kind,
      icon: Braces,
    };
  const translated = (definition: StepDefinition, field: "label" | "hint") =>
    t(`settings.orden.workflow.steps.${definition.kind}.${field}`, {
      defaultValue: definition[field],
    });

  const update = (id: string, patch: Partial<OrdenVisualStep>) => {
    onChange(steps.map((step) => (step.id === id ? { ...step, ...patch } : step)));
  };

  const add = (kind: string) => {
    const id = `${mode}-${Date.now()}-${steps.length}`;
    onChange([
      ...steps,
      {
        id,
        kind,
        value: PRESETS[kind] ?? "",
        inverted: false,
      },
    ]);
    setSelectedId(id);
    window.setTimeout(() => railRef.current?.scrollTo({ left: railRef.current.scrollWidth, behavior: "smooth" }), 0);
  };

  const move = (index: number, offset: number) => {
    const target = index + offset;
    if (target < 0 || target >= steps.length) return;
    const next = [...steps];
    [next[index], next[target]] = [next[target], next[index]];
    onChange(next);
  };

  const scrollRail = (direction: -1 | 1) => {
    railRef.current?.scrollBy({ left: direction * 190, behavior: "smooth" });
  };

  const renderLibraryGroup = (common: boolean) => {
    const items = definitions.filter((definition) => Boolean(definition.common) === common);
    if (items.length === 0) return null;
    return (
      <MenuGroup>
        <MenuGroupLabel>
          {common
            ? t("settings.orden.workflow.commonSteps", { defaultValue: "Quick steps" })
            : t("settings.orden.workflow.moreSteps", { defaultValue: "More steps" })}
        </MenuGroupLabel>
        {items.map((definition) => {
          const Icon = definition.icon;
          return (
            <MenuItem key={definition.kind} onClick={() => add(definition.kind)} className="items-start py-2">
              <Icon className="mt-0.5" />
              <span className="min-w-0">
                <span className="block font-medium">{translated(definition, "label")}</span>
                <span className="block text-xs text-muted-foreground">{translated(definition, "hint")}</span>
              </span>
            </MenuItem>
          );
        })}
      </MenuGroup>
    );
  };

  const selectedIndex = steps.findIndex((step) => step.id === selectedId);
  const selectedStep = selectedIndex >= 0 ? steps[selectedIndex] : null;
  const selectedDefinition = selectedStep ? definitionFor(selectedStep.kind) : null;
  const SelectedIcon = selectedDefinition?.icon || Braces;

  return (
    <div className="space-y-2.5">
      <div className="flex items-center justify-between gap-3">
        <div className="min-w-0">
          <Label className="text-sm font-medium">
            {mode === "filter" ? t("settings.orden.filterPipeline") : t("settings.orden.actionPipeline")}
          </Label>
          <p className="mt-0.5 text-xs text-muted-foreground">
            {mode === "filter"
              ? t("settings.orden.workflow.filterHelp", { defaultValue: "Add only the conditions this rule needs." })
              : t("settings.orden.workflow.actionHelp", { defaultValue: "Actions run from top to bottom." })}
          </p>
        </div>
        <div className="flex items-center gap-1">
          <Button type="button" variant="ghost" size="icon-sm" onClick={() => scrollRail(-1)} disabled={steps.length < 2} aria-label={t("settings.orden.workflow.previousCard", { defaultValue: "Previous card" })}>
            <ChevronLeft />
          </Button>
          <Button type="button" variant="ghost" size="icon-sm" onClick={() => scrollRail(1)} disabled={steps.length < 2} aria-label={t("settings.orden.workflow.nextCard", { defaultValue: "Next card" })}>
            <ChevronRight />
          </Button>
          <Menu>
            <MenuTrigger
              render={
                <Button type="button" variant="outline" size="sm">
                  <Plus />
                  {mode === "filter" ? t("settings.orden.addFilter") : t("settings.orden.addAction")}
                </Button>
              }
            />
            <MenuPopup className="w-72" align="end">
              {renderLibraryGroup(true)}
              <MenuSeparator />
              {renderLibraryGroup(false)}
            </MenuPopup>
          </Menu>
        </div>
      </div>

      {steps.length === 0 && (
        <div className="rounded-lg border border-dashed border-border bg-muted/15 px-4 py-7 text-center">
          <div className="mx-auto mb-2 flex size-8 items-center justify-center rounded-lg bg-muted text-muted-foreground">
            <Plus className="size-4" />
          </div>
          <p className="text-sm font-medium">
            {mode === "filter"
              ? t("settings.orden.workflow.matchEverything", { defaultValue: "No conditions — match everything" })
              : t("settings.orden.noActionsConfigured")}
          </p>
          <p className="mt-1 text-xs text-muted-foreground">
            {mode === "filter"
              ? t("settings.orden.workflow.matchEverythingHelp", { defaultValue: "Add a condition to narrow down the files entering this flow." })
              : t("settings.orden.workflow.noActionHelp", { defaultValue: "Add at least one action to complete this flow." })}
          </p>
        </div>
      )}

      {steps.length > 0 && (
        <>
          <div ref={railRef} className="no-scrollbar flex snap-x gap-2 overflow-x-auto pb-1" role="list" aria-label={mode === "filter" ? t("settings.orden.filterPipeline") : t("settings.orden.actionPipeline")}>
            {steps.map((step, index) => {
              const definition = definitionFor(step.kind);
              const Icon = definition.icon;
              const selected = step.id === selectedId;
              return (
                <button
                  key={step.id}
                  type="button"
                  role="listitem"
                  onClick={() => setSelectedId(step.id)}
                  className={`group min-h-28 w-40 shrink-0 snap-start rounded-lg border p-3 text-left shadow-sm transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${
                    selected
                      ? "border-primary/55 bg-primary/8 ring-1 ring-primary/20"
                      : "border-border bg-card hover:border-primary/30 hover:bg-muted/20"
                  }`}
                  aria-pressed={selected}
                >
                  <div className="flex items-start justify-between gap-2">
                    <div className={`flex size-8 items-center justify-center rounded-lg ${selected ? "bg-primary text-primary-foreground" : "bg-muted text-muted-foreground"}`}>
                      <Icon className="size-4" />
                    </div>
                    <span className="text-[10px] font-semibold uppercase tracking-[0.12em] text-muted-foreground">
                      {mode === "filter" ? "IF" : `${index + 1}`}
                    </span>
                  </div>
                  <div className="mt-3 text-sm font-semibold leading-tight">{translated(definition, "label")}</div>
                  <div className="mt-1 truncate font-mono text-[10px] text-muted-foreground">
                    {step.value.trim() || t("settings.orden.workflow.noParameters", { defaultValue: "No parameters" })}
                  </div>
                  {step.inverted && mode === "filter" && (
                    <Badge variant="outline" className="mt-2 h-5 px-1.5 text-[10px]">NOT</Badge>
                  )}
                </button>
              );
            })}
            <Menu>
              <MenuTrigger
                render={
                  <button type="button" className="flex min-h-28 w-32 shrink-0 snap-start flex-col items-center justify-center gap-2 rounded-lg border border-dashed border-border bg-muted/10 p-3 text-xs font-medium text-muted-foreground transition hover:border-primary/40 hover:bg-primary/5 hover:text-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">
                    <span className="flex size-8 items-center justify-center rounded-lg border border-current/20 bg-background"><Plus className="size-4" /></span>
                    {mode === "filter" ? t("settings.orden.addFilter") : t("settings.orden.addAction")}
                  </button>
                }
              />
              <MenuPopup className="w-72" align="start">
                {renderLibraryGroup(true)}
                <MenuSeparator />
                {renderLibraryGroup(false)}
              </MenuPopup>
            </Menu>
          </div>

          {selectedStep && selectedDefinition && (
            <div className="rounded-lg border border-border/90 bg-background shadow-sm">
              <div className="flex flex-wrap items-center gap-2 border-b border-border/70 bg-muted/20 px-2.5 py-2">
                <div className="flex size-7 shrink-0 items-center justify-center rounded-md bg-primary/10 text-primary">
                  <SelectedIcon className="size-3.5" />
                </div>
                <Select value={selectedStep.kind} onValueChange={(kind) => update(selectedStep.id, { kind, value: PRESETS[kind] ?? "" })}>
                  <SelectTrigger className="h-7 min-w-40 flex-1 border-0 bg-transparent px-1 shadow-none focus:ring-0" aria-label={t("settings.orden.pipelineType")}>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {definitions.map((item) => <SelectItem key={item.kind} value={item.kind}>{translated(item, "label")}</SelectItem>)}
                  </SelectContent>
                </Select>
                {selectedDefinition.destructive && <Badge variant="destructive" className="h-5 px-1.5 text-[10px]">{t("settings.orden.workflow.changesFiles", { defaultValue: "Changes files" })}</Badge>}
                <div className="ml-auto flex items-center gap-0.5">
                  {mode === "action" && DESTINATION_ACTIONS.has(selectedStep.kind) && onChooseDestination && <Button type="button" onClick={() => onChooseDestination(selectedStep.id)} variant="ghost" size="icon-sm" aria-label={t("settings.orden.chooseDestinations")}><FolderOpen /></Button>}
                  <Button type="button" onClick={() => move(selectedIndex, -1)} disabled={selectedIndex === 0} variant="ghost" size="icon-sm" aria-label={t("settings.orden.moveUp")}><ArrowUp /></Button>
                  <Button type="button" onClick={() => move(selectedIndex, 1)} disabled={selectedIndex === steps.length - 1} variant="ghost" size="icon-sm" aria-label={t("settings.orden.moveDown")}><ArrowDown /></Button>
                  <Button type="button" onClick={() => onChange(steps.filter((item) => item.id !== selectedStep.id))} variant="ghost" size="icon-sm" className="text-destructive hover:bg-destructive/10 hover:text-destructive" aria-label={t("settings.orden.removePipelineStep")}><Trash2 /></Button>
                </div>
              </div>
              <div className="space-y-2 px-2.5 py-2.5">
                <p className="text-xs text-muted-foreground">{translated(selectedDefinition, "hint")}</p>
                <textarea value={selectedStep.value} onChange={(event) => update(selectedStep.id, { value: event.target.value })} placeholder={t("settings.orden.pipelineValuePlaceholder")} className="min-h-16 w-full resize-y rounded-md border border-input bg-card px-2.5 py-2 font-mono text-xs leading-5 text-foreground outline-none focus:border-ring focus:ring-2 focus:ring-ring/20" aria-label={`${translated(selectedDefinition, "label")} · ${t("settings.orden.pipelineValue")}`} />
                {mode === "filter" && <Label className="flex w-fit items-center gap-2 text-xs text-muted-foreground"><Checkbox checked={selectedStep.inverted} onCheckedChange={(checked) => update(selectedStep.id, { inverted: checked === true })} />{t("settings.orden.invertFilter")}</Label>}
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}
