import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { useAppStore } from "../../store/useAppStore";
import { AnimatedIcon } from "../ui/animated-icon";
import { Button } from "../ui/button";
import { Card } from "../ui/card";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../ui/select";
import { Tooltip, TooltipContent, TooltipTrigger } from "../ui/tooltip";
import { Save, X } from "lucide-react";

export function IgnoreTab() {
  const { t } = useTranslation();
  const { folders } = useAppStore();
  const [selectedFolder, setSelectedFolder] = useState("");
  const [patterns, setPatterns] = useState<string[]>([]);
  const [newPattern, setNewPattern] = useState("");
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    if (selectedFolder) {
      invoke<string[]>("load_shelfyignore_cmd", { folderPath: selectedFolder })
        .then(setPatterns)
        .catch(() => setPatterns([]));
    } else {
      setPatterns([]);
    }
  }, [selectedFolder]);

  const handleAdd = () => {
    const trimmed = newPattern.trim();
    if (!trimmed || patterns.includes(trimmed)) return;
    setPatterns([...patterns, trimmed]);
    setNewPattern("");
    setSaved(false);
  };

  const handleRemove = (idx: number) => {
    setPatterns(patterns.filter((_, i) => i !== idx));
    setSaved(false);
  };

  const handleSave = async () => {
    if (!selectedFolder) return;
    try {
      await invoke("save_shelfyignore_cmd", {
        folderPath: selectedFolder,
        patterns,
      });
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      console.error("save_shelfyignore failed:", e);
    }
  };

  return (
    <div className="w-full space-y-4">
      <div>
        <h2 className="text-lg font-semibold">{t("settings.ignore.rulesTitle")}</h2>
        <p className="text-xs text-muted-foreground">{t("settings.ignore.description")}</p>
      </div>

      <div>
        <Label className="mb-2 block text-sm text-muted-foreground">{t("settings.ignore.folder")}</Label>
        <Select value={selectedFolder} onValueChange={setSelectedFolder}>
          <SelectTrigger>
            <SelectValue placeholder={t("settings.ignore.selectFolder")} />
          </SelectTrigger>
          <SelectContent>
            {folders.map((f) => (
              <SelectItem key={f.id} value={f.path}>
                {f.path}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {selectedFolder && (
        <>
          <div className="space-y-2">
            <Label className="block text-sm text-muted-foreground">{t("settings.ignore.patterns")}</Label>
            {patterns.length === 0 && (
              <p className="text-sm italic text-muted-foreground">{t("settings.ignore.noRules")}</p>
            )}
            {patterns.map((p, i) => (
              <Card key={i} className="flex items-center justify-between px-3 py-2">
                <code className="text-sm text-primary">{p}</code>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      onClick={() => handleRemove(i)}
                      variant="ghost"
                      size="icon"
                      className="h-7 w-7 text-muted-foreground hover:text-destructive"
                      aria-label={t("settings.ignore.remove")}
                    >
                      <X size={14} />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>{t("settings.ignore.remove")}</TooltipContent>
                </Tooltip>
              </Card>
            ))}
          </div>

          <div className="flex gap-2">
            <Input
              type="text"
              value={newPattern}
              onChange={(e) => setNewPattern(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleAdd()}
              placeholder={t("settings.ignore.placeholder")}
              className="flex-1"
            />
            <Button onClick={handleAdd} disabled={!newPattern.trim()}>
              {t("settings.ignore.add")}
            </Button>
          </div>

          <Card className="p-3">
            <p className="mb-1 text-xs text-muted-foreground">
              <strong className="text-foreground">{t("settings.ignore.tips")}</strong>
            </p>
            <ul className="list-disc space-y-1 pl-4 text-xs text-muted-foreground">
              <li><code>*.tmp</code> - ignore all .tmp files</li>
              <li><code>node_modules/</code> - ignore the folder</li>
              <li><code>~$*</code> - ignore Office temp files</li>
              <li><code>.DS_Store</code> - ignore exact file name</li>
            </ul>
          </Card>

          <Button onClick={handleSave}>
            <AnimatedIcon icon={Save} size={16} motion="pulse" />
            {saved ? t("settings.ignore.saved") : t("settings.ignore.save")}
          </Button>
        </>
      )}
    </div>
  );
}
