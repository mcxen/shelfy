import { KeyboardEvent, useState } from "react";
import { X } from "lucide-react";
import { cn } from "../../lib/utils";

interface TagInputProps {
  value: string[];
  onChange: (value: string[]) => void;
  placeholder?: string;
  ariaLabel: string;
  className?: string;
  maskValues?: boolean;
}

export function TagInput({ value, onChange, placeholder, ariaLabel, className, maskValues = false }: TagInputProps) {
  const [draft, setDraft] = useState("");

  const addDraft = (raw = draft) => {
    const additions = raw.split(/[\n,，]+/).map((item) => item.trim()).filter(Boolean);
    if (additions.length > 0) onChange(Array.from(new Set([...value, ...additions])));
    setDraft("");
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter" || event.key === "," || event.key === "，") {
      event.preventDefault();
      addDraft();
    } else if (event.key === "Backspace" && !draft && value.length > 0) {
      onChange(value.slice(0, -1));
    }
  };

  return (
    <div className={cn("flex min-h-9 w-full flex-wrap items-center gap-1.5 rounded-lg border border-input bg-background px-2 py-1 shadow-sm transition-colors focus-within:border-ring focus-within:ring-2 focus-within:ring-ring/20", className)}>
      {value.map((tag) => (
        <span key={tag} className="inline-flex h-6 max-w-full items-center gap-1 rounded-md bg-secondary px-2 text-xs font-medium text-secondary-foreground">
          <span className="truncate">{maskValues ? "•".repeat(Math.min(Math.max(tag.length, 4), 12)) : tag}</span>
          <button type="button" onClick={() => onChange(value.filter((item) => item !== tag))} className="-mr-1 rounded p-0.5 text-muted-foreground transition-colors hover:bg-background/70 hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" aria-label={maskValues ? ariaLabel : `${ariaLabel}: ${tag}`}>
            <X className="size-3" />
          </button>
        </span>
      ))}
      <input type="text" value={draft} onChange={(event) => setDraft(event.target.value)} onKeyDown={handleKeyDown} onBlur={() => addDraft()} onPaste={(event) => { const text = event.clipboardData.getData("text"); if (/[\n,，]/.test(text)) { event.preventDefault(); addDraft(text); } }} placeholder={value.length === 0 ? placeholder : undefined} aria-label={ariaLabel} className="h-6 min-w-24 flex-1 bg-transparent px-1 text-sm outline-none placeholder:text-muted-foreground" />
    </div>
  );
}
