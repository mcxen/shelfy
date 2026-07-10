import { useTranslation } from "react-i18next";
import { ArrowDown, ArrowUp, FolderOpen, Plus, Trash2 } from "lucide-react";
import { OrdenVisualStep } from "../../store/useAppStore";
import { Button } from "../ui/button";
import { Checkbox } from "../ui/checkbox";
import { Label } from "../ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../ui/select";

const FILTER_TYPES = [
  "extension",
  "name",
  "regex",
  "size",
  "empty",
  "mimetype",
  "hash",
  "duplicate",
  "created",
  "lastmodified",
  "filecontent",
  "exif",
];

const ACTION_TYPES = [
  "copy",
  "move",
  "rename",
  "delete",
  "trash",
  "echo",
  "write",
  "symlink",
  "hardlink",
  "shell",
  "extract",
  "compress",
  "archive",
  "unarchive",
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
  const types = mode === "filter" ? FILTER_TYPES : ACTION_TYPES;

  const update = (id: string, patch: Partial<OrdenVisualStep>) => {
    onChange(steps.map((step) => (step.id === id ? { ...step, ...patch } : step)));
  };

  const add = () => {
    const kind = mode === "filter" ? "extension" : "copy";
    onChange([
      ...steps,
      {
        id: `${mode}-${Date.now()}-${steps.length}`,
        kind,
        value: PRESETS[kind],
        inverted: false,
      },
    ]);
  };

  const move = (index: number, offset: number) => {
    const target = index + offset;
    if (target < 0 || target >= steps.length) return;
    const next = [...steps];
    [next[index], next[target]] = [next[target], next[index]];
    onChange(next);
  };

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between gap-2">
        <div>
          <Label className="text-xs text-muted-foreground">
            {mode === "filter" ? t("settings.orden.filterPipeline") : t("settings.orden.actionPipeline")}
          </Label>
          <p className="text-[11px] text-muted-foreground">{t("settings.orden.pipelineHelp")}</p>
        </div>
        <Button type="button" onClick={add} variant="outline" size="sm">
          <Plus size={13} />
          {mode === "filter" ? t("settings.orden.addFilter") : t("settings.orden.addAction")}
        </Button>
      </div>

      {steps.length === 0 && (
        <div className="rounded-lg border border-dashed border-border px-3 py-5 text-center text-xs text-muted-foreground">
          {mode === "filter" ? t("settings.orden.noFiltersConfigured") : t("settings.orden.noActionsConfigured")}
        </div>
      )}

      {steps.map((step, index) => (
        <div key={step.id} className="rounded-xl border border-border bg-muted/20 p-3">
          <div className="grid items-center gap-2 sm:grid-cols-[2rem_minmax(8rem,0.8fr)_minmax(0,1.4fr)_auto]">
            <span className="text-center text-xs font-medium text-muted-foreground">{index + 1}</span>
            <Select
              value={step.kind}
              onValueChange={(kind) => update(step.id, { kind, value: PRESETS[kind] ?? "" })}
            >
              <SelectTrigger aria-label={t("settings.orden.pipelineType")}><SelectValue /></SelectTrigger>
              <SelectContent>
                {types.map((type) => <SelectItem key={type} value={type}>{type}</SelectItem>)}
              </SelectContent>
            </Select>
            <textarea
              value={step.value}
              onChange={(event) => update(step.id, { value: event.target.value })}
              placeholder={t("settings.orden.pipelineValuePlaceholder")}
              className="min-h-10 w-full resize-y rounded-lg border border-border bg-background px-3 py-2 font-mono text-xs leading-5 text-foreground outline-none focus:border-ring focus:ring-2 focus:ring-ring/20"
              aria-label={t("settings.orden.pipelineValue")}
            />
            <div className="flex items-center justify-end gap-0.5">
              {mode === "action" && DESTINATION_ACTIONS.has(step.kind) && onChooseDestination && (
                <Button type="button" onClick={() => onChooseDestination(step.id)} variant="ghost" size="icon" aria-label={t("settings.orden.chooseDestinations")}>
                  <FolderOpen size={13} />
                </Button>
              )}
              <Button type="button" onClick={() => move(index, -1)} disabled={index === 0} variant="ghost" size="icon" aria-label={t("settings.orden.moveUp")}>
                <ArrowUp size={13} />
              </Button>
              <Button type="button" onClick={() => move(index, 1)} disabled={index === steps.length - 1} variant="ghost" size="icon" aria-label={t("settings.orden.moveDown")}>
                <ArrowDown size={13} />
              </Button>
              <Button type="button" onClick={() => onChange(steps.filter((item) => item.id !== step.id))} variant="ghost" size="icon" className="text-destructive hover:bg-destructive/10 hover:text-destructive" aria-label={t("settings.orden.removePipelineStep")}>
                <Trash2 size={13} />
              </Button>
            </div>
          </div>
          {mode === "filter" && (
            <Label className="mt-2 flex items-center gap-2 text-xs text-muted-foreground">
              <Checkbox checked={step.inverted} onCheckedChange={(checked) => update(step.id, { inverted: checked === true })} />
              {t("settings.orden.invertFilter")}
            </Label>
          )}
        </div>
      ))}
    </div>
  );
}
