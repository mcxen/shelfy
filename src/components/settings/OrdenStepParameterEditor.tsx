import { useState } from "react";
import { Eye, EyeOff } from "lucide-react";
import { Button } from "../ui/button";
import { Checkbox } from "../ui/checkbox";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../ui/select";
import { TagInput } from "../ui/tag-input";

type Value = string | boolean | number | string[];
type Values = Record<string, Value>;
type FieldType = "text" | "textarea" | "password" | "boolean" | "select" | "tags" | "number";
type Field = { key: string; type: FieldType; options?: string[]; placeholder?: string; wide?: boolean };

const ACTION_FIELDS: Record<string, Field[]> = {
  move: destinationFields(), copy: destinationFields(true), symlink: destinationFields(), hardlink: destinationFields(),
  rename: [{ key: "new_name", type: "text", placeholder: "{name}.organized{extension}", wide: true }, ...conflictFields(false)],
  extract: archiveFields(true), unarchive: archiveFields(true),
  compress: archiveFields(false), archive: archiveFields(false),
  echo: [{ key: "msg", type: "textarea", placeholder: "Matched {path}", wide: true }],
  write: [
    { key: "text", type: "textarea", wide: true }, { key: "outfile", type: "text", wide: true },
    { key: "mode", type: "select", options: ["append", "prepend", "overwrite"] },
    { key: "encoding", type: "text", placeholder: "utf-8" }, { key: "newline", type: "boolean" },
    { key: "clear_before_first_write", type: "boolean" },
  ],
  shell: [
    { key: "cmd", type: "textarea", wide: true }, { key: "run_in_simulation", type: "boolean" },
    { key: "ignore_errors", type: "boolean" }, { key: "simulation_output", type: "text" },
    { key: "simulation_returncode", type: "number" },
  ],
  delete: [], trash: [],
};

const FILTER_FIELDS: Record<string, Field[]> = {
  extension: [{ key: "value", type: "tags", placeholder: "zip, 7z, rar", wide: true }],
  mimetype: [{ key: "value", type: "tags", placeholder: "application/pdf", wide: true }],
  size: [{ key: "value", type: "tags", placeholder: "> 10 MB", wide: true }],
  regex: [{ key: "value", type: "text", wide: true }], filecontent: [{ key: "value", type: "text", wide: true }],
  hash: [{ key: "value", type: "select", options: ["md5", "sha1", "sha256", "sha512"] }],
  duplicate: [{ key: "detect_original_by", type: "select", options: ["first_seen", "last_seen", "created", "lastmodified"] }, { key: "hash_algorithm", type: "select", options: ["sha1", "sha256", "sha512", "md5"] }],
  created: timeFields(), lastmodified: timeFields(),
  name: [{ key: "match", type: "text", wide: true }, { key: "startswith", type: "tags" }, { key: "contains", type: "tags" }, { key: "endswith", type: "tags" }, { key: "case_sensitive", type: "boolean" }],
  empty: [],
};

function conflictFields(includeAutodetect = true): Field[] {
  return [
    { key: "on_conflict", type: "select", options: ["skip", "overwrite", "trash", "rename_new", "rename_existing", "deduplicate"] },
    { key: "rename_template", type: "text", placeholder: "{name} {counter}{extension}" },
    ...(includeAutodetect ? [{ key: "autodetect_folder", type: "boolean" } as Field] : []),
  ];
}

function destinationFields(multiple = false): Field[] {
  return [{ key: "dest", type: multiple ? "tags" : "text", wide: true }, ...conflictFields(), ...(multiple ? [{ key: "continue_with", type: "select", options: ["copy", "original"] } as Field] : [])];
}

function archiveFields(extract: boolean): Field[] {
  return [
    { key: "dest", type: "text", wide: true }, { key: "format", type: "select", options: extract ? ["auto", "zip", "7z", "rar"] : ["zip", "7z", "rar"] },
    { key: extract ? "passwords" : "password", type: extract ? "tags" : "password", wide: true },
    { key: "delete_original", type: "boolean" }, ...conflictFields(),
  ];
}

function timeFields(): Field[] {
  return [...["years", "months", "weeks", "days", "hours", "minutes", "seconds"].map((key): Field => ({ key, type: "number" })), { key: "mode", type: "select", options: ["older", "newer"] }];
}

const PRIMARY_KEY: Record<string, string> = { move: "dest", copy: "dest", symlink: "dest", hardlink: "dest", rename: "new_name", echo: "msg", regex: "value", filecontent: "value", hash: "value", extension: "value", mimetype: "value", size: "value" };

function unquote(value: string): string {
  const text = value.trim();
  if ((text.startsWith('"') && text.endsWith('"')) || (text.startsWith("'") && text.endsWith("'"))) return text.slice(1, -1).replace(/\\"/g, '"');
  return text;
}

function parseValue(raw: string): Value {
  const text = raw.trim();
  if (text === "true" || text === "false") return text === "true";
  if (/^-?\d+$/.test(text)) return Number(text);
  if (text.startsWith("[") && text.endsWith("]")) return text.slice(1, -1).split(",").map(unquote).map((item) => item.trim()).filter(Boolean);
  return unquote(text);
}

function parseStepValue(kind: string, raw: string): Values {
  const text = raw.trim();
  if (!text) return {};
  if (!text.includes(":")) return { [PRIMARY_KEY[kind] || "value"]: parseValue(text) };
  const values: Values = {};
  let listKey = "";
  text.split(/\r?\n/).forEach((line) => {
    const list = line.match(/^\s*-\s*(.+)$/);
    if (list && listKey) {
      const current = Array.isArray(values[listKey]) ? values[listKey] as string[] : [];
      values[listKey] = [...current, unquote(list[1])];
      return;
    }
    const pair = line.match(/^\s*([\w-]+)\s*:\s*(.*)$/);
    if (!pair) return;
    listKey = pair[2].trim() ? "" : pair[1];
    values[pair[1]] = pair[2].trim() ? parseValue(pair[2]) : [];
  });
  return values;
}

function quote(value: string): string {
  if (!value || /[:#\[\]{},&*!|>'"%@`\n]|^[-?]|\s$/.test(value)) return JSON.stringify(value);
  return value;
}

function serialize(values: Values): string {
  return Object.entries(values).filter(([, value]) => value !== "" && (!Array.isArray(value) || value.length > 0)).flatMap(([key, value]) => {
    if (Array.isArray(value)) return value.length < 4 ? [`${key}: [${value.map(quote).join(", ")}]`] : [`${key}:`, ...value.map((item) => `  - ${quote(item)}`)];
    return [`${key}: ${typeof value === "string" ? quote(value) : value}`];
  }).join("\n");
}

interface Props { mode: "filter" | "action"; kind: string; value: string; onChange: (value: string) => void; label: (key: string) => string; }

export const supportsStepParameterEditor = (mode: "filter" | "action", kind: string) =>
  Boolean((mode === "action" ? ACTION_FIELDS : FILTER_FIELDS)[kind]);

export function OrdenStepParameterEditor({ mode, kind, value, onChange, label }: Props) {
  const fields = (mode === "action" ? ACTION_FIELDS : FILTER_FIELDS)[kind];
  const [showPassword, setShowPassword] = useState(false);
  if (!fields) return null;
  const values = parseStepValue(kind, value);
  const set = (key: string, next: Value) => onChange(serialize({ ...values, [key]: next }));

  if (fields.length === 0) return <div className="rounded-md border border-dashed border-border bg-muted/15 px-3 py-4 text-center text-xs text-muted-foreground">{label("no_parameters")}</div>;

  return <div className="grid gap-3 md:grid-cols-2">
    {fields.map((field) => {
      const id = `orden-step-${kind}-${field.key}`;
      const current = values[field.key];
      if (field.type === "boolean") return <Label key={field.key} className="flex min-h-9 items-center gap-2 rounded-md border border-border bg-card px-3 text-xs"><Checkbox checked={current === true} onCheckedChange={(checked) => set(field.key, checked === true)} />{label(field.key)}</Label>;
      return <div key={field.key} className={field.wide ? "md:col-span-2" : undefined}>
        <Label htmlFor={id} className="mb-1 block text-xs text-muted-foreground">{label(field.key)}</Label>
        {field.type === "select" ? <Select value={String(current ?? field.options?.[0] ?? "")} onValueChange={(next) => set(field.key, next)}><SelectTrigger id={id}><SelectValue /></SelectTrigger><SelectContent>{field.options?.map((option) => <SelectItem key={option} value={option}>{label(option)}</SelectItem>)}</SelectContent></Select>
          : field.type === "tags" ? <TagInput value={Array.isArray(current) ? current : current ? [String(current)] : []} onChange={(next) => set(field.key, next)} placeholder={field.placeholder} ariaLabel={label(field.key)} maskValues={field.key === "passwords"} />
          : field.type === "textarea" ? <textarea id={id} value={String(current ?? "")} onChange={(event) => set(field.key, event.target.value)} placeholder={field.placeholder} className="min-h-20 w-full resize-y rounded-md border border-input bg-card px-2.5 py-2 text-xs leading-5 outline-none focus:border-ring focus:ring-2 focus:ring-ring/20" />
          : <div className="relative"><Input id={id} type={field.type === "number" ? "number" : field.type === "password" && !showPassword ? "password" : "text"} value={String(current ?? "")} onChange={(event) => set(field.key, field.type === "number" ? Number(event.target.value) : event.target.value)} placeholder={field.placeholder} className={field.type === "password" ? "pr-9" : undefined} />{field.type === "password" && <Button type="button" variant="ghost" size="icon-sm" onClick={() => setShowPassword((shown) => !shown)} className="absolute right-1 top-1/2 -translate-y-1/2" aria-label={showPassword ? label("hide_password") : label("show_password")}>{showPassword ? <EyeOff /> : <Eye />}</Button>}</div>}
      </div>;
    })}
  </div>;
}
