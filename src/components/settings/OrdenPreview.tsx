import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { ChevronLeft } from "lucide-react";
import { OrdenRunResult } from "../../store/useAppStore";
import { buildOrdenPreviewRows } from "./utils";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { Card } from "../ui/card";
import { ordenLevelLabel, ordenOperationLabel } from "../../lib/ordenI18n";

interface OrdenPreviewProps {
  ordenResult: OrdenRunResult | null;
  ordenPreviewError: string | null;
  onBack: () => void;
}

export function OrdenPreview({ ordenResult, ordenPreviewError, onBack }: OrdenPreviewProps) {
  const { t } = useTranslation();
  const rows = useMemo(() => buildOrdenPreviewRows(ordenResult), [ordenResult]);

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between gap-3">
        <div>
          <h2 className="text-lg font-semibold">{t("settings.orden.title")}</h2>
          <p className="text-xs text-muted-foreground">{t("settings.orden.desc")}</p>
        </div>
        <Button onClick={onBack} variant="outline">
          <ChevronLeft size={14} />
          {t("settings.orden.backToEditor")}
        </Button>
      </div>

      <Card className="space-y-2 p-3">
        <div className="flex flex-wrap items-center gap-2">
          <Badge variant={ordenPreviewError || ordenResult?.errors ? "destructive" : "default"}>
            {t("settings.orden.successCount", { count: ordenResult?.success || 0 })}
          </Badge>
          <Badge variant={ordenPreviewError || ordenResult?.errors ? "destructive" : "secondary"}>
            {t("settings.orden.errorCount", { count: ordenResult?.errors || (ordenPreviewError ? 1 : 0) })}
          </Badge>
          <Badge variant="secondary">
            {ordenResult?.simulate !== false ? t("settings.orden.simulated") : t("settings.orden.applied")}
          </Badge>
        </div>
        {ordenPreviewError && (
          <div className="rounded-lg border border-destructive/20 bg-destructive/10 px-3 py-2 text-sm text-destructive">
            {ordenPreviewError}
          </div>
        )}
      </Card>

      <div className="grid gap-3 md:grid-cols-2">
        <Card className="space-y-2 p-3">
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-semibold">{t("settings.orden.previewSource")}</h3>
            <Badge variant="secondary">{rows.length}</Badge>
          </div>
          <div className="max-h-[420px] overflow-auto border-t border-border">
            {rows.length === 0 ? (
              <div className="px-3 py-2 text-xs text-muted-foreground">{t("settings.orden.noLogs")}</div>
            ) : (
              <div className="divide-y divide-border">
                {rows.map((row) => (
                  <div key={row.id} className="px-3 py-2 text-xs">
                    <div className="font-medium text-foreground">{row.source}</div>
                    <div className="mt-1 flex items-center gap-2 text-muted-foreground">
                      <Badge variant={row.level === "error" ? "destructive" : "secondary"}>{ordenOperationLabel(t, row.action)}</Badge>
                      <span>{ordenLevelLabel(t, row.level)}</span>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </Card>

        <Card className="space-y-2 p-3">
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-semibold">{t("settings.orden.previewResult")}</h3>
            <Badge variant="secondary">
              {ordenResult?.simulate !== false ? t("settings.orden.simulated") : t("settings.orden.applied")}
            </Badge>
          </div>
          <div className="max-h-[420px] overflow-auto border-t border-border">
            {rows.length === 0 ? (
              <div className="px-3 py-2 text-xs text-muted-foreground">
                {ordenPreviewError || t("settings.orden.noLogs")}
              </div>
            ) : (
              <div className="divide-y divide-border">
                {rows.map((row) => (
                  <div key={`${row.id}-result`} className="px-3 py-2 text-xs">
                    <div className="font-medium text-foreground">{row.destination || row.message}</div>
                    <div className="mt-1 text-muted-foreground">{row.message}</div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </Card>
      </div>

      {ordenResult && (
        <Card className="space-y-2 p-3">
          <h3 className="text-sm font-semibold">{t("settings.orden.previewLogs")}</h3>
          <div className="max-h-56 overflow-auto border-t border-border">
            {ordenResult.logs.length === 0 ? (
              <div className="px-3 py-2 text-xs text-muted-foreground">{t("settings.orden.noLogs")}</div>
            ) : (
              <div className="divide-y divide-border">
                {ordenResult.logs.map((log, idx) => (
                  <div key={idx} className="grid gap-1 px-3 py-2 text-xs md:grid-cols-[2.5rem_5rem_7rem_minmax(0,1fr)]">
                    <span className="text-muted-foreground">{idx + 1}</span>
                    <span className={log.level === "error" ? "text-destructive" : "text-muted-foreground"}>{ordenLevelLabel(t, log.level)}</span>
                    <span className="text-muted-foreground">{ordenOperationLabel(t, log.sender)} · #{log.rule_nr + 1}</span>
                    <div className="min-w-0">
                      <div className="break-all font-mono text-[11px] text-muted-foreground">{log.path}</div>
                      <div className="mt-1 break-words text-foreground">{log.msg}</div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </Card>
      )}
    </div>
  );
}
