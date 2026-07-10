import { Fragment, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronDown, ChevronRight, RefreshCw, Trash2 } from "lucide-react";
import { OrdenLog, OrdenRunHistory } from "../../store/useAppStore";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "../ui/table";

interface OrdenRunHistoryTableProps {
  rows: OrdenRunHistory[];
  onRefresh?: () => void | Promise<void>;
  onDelete?: (id: number) => void | Promise<void>;
  onClear?: () => void | Promise<void>;
}

function parseLogs(raw: string): OrdenLog[] {
  try {
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

function rowKey(row: OrdenRunHistory): string {
  return String(row.id ?? `${row.config_name}-${row.timestamp}`);
}

export function OrdenRunHistoryTable({
  rows,
  onRefresh,
  onDelete,
  onClear,
}: OrdenRunHistoryTableProps) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const logCounts = useMemo(
    () => Object.fromEntries(rows.map((row) => [rowKey(row), parseLogs(row.logs_json).length])),
    [rows]
  );

  const toggle = (key: string) => {
    setExpanded((current) => {
      const next = new Set(current);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between gap-2">
        <div>
          <h3 className="font-medium">{t("settings.orden.history")}</h3>
          <p className="text-xs text-muted-foreground">{t("settings.orden.historyDesc")}</p>
        </div>
        <div className="flex items-center gap-1">
          {onRefresh && (
            <Button type="button" onClick={onRefresh} variant="ghost" size="sm">
              <RefreshCw size={13} />
              {t("settings.scheduler.refreshLogs")}
            </Button>
          )}
          {onClear && (
            <Button
              type="button"
              onClick={() => {
                if (window.confirm(t("settings.orden.clearHistoryConfirm"))) void onClear();
              }}
              variant="ghost"
              size="sm"
              className="text-destructive hover:bg-destructive/10 hover:text-destructive"
              disabled={rows.length === 0}
            >
              <Trash2 size={13} />
              {t("settings.orden.clearHistory")}
            </Button>
          )}
        </div>
      </div>

      {rows.length === 0 ? (
        <div className="rounded-xl border border-dashed border-border px-3 py-6 text-center text-xs text-muted-foreground">
          {t("settings.orden.noHistory")}
        </div>
      ) : (
        <div className="overflow-hidden rounded-xl border border-border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-10" />
                <TableHead>{t("settings.orden.time")}</TableHead>
                <TableHead>{t("settings.orden.trigger")}</TableHead>
                <TableHead>{t("settings.orden.mode")}</TableHead>
                <TableHead>{t("settings.orden.process")}</TableHead>
                <TableHead>{t("settings.orden.result")}</TableHead>
                <TableHead className="w-12 text-right">{t("settings.orden.actions")}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {rows.map((row) => {
                const key = rowKey(row);
                const isExpanded = expanded.has(key);
                const logs = isExpanded ? parseLogs(row.logs_json) : [];
                return (
                  <Fragment key={key}>
                    <TableRow>
                      <TableCell>
                        <Button
                          type="button"
                          onClick={() => toggle(key)}
                          variant="ghost"
                          size="icon"
                          className="size-7"
                          aria-label={isExpanded ? t("settings.orden.collapseProcess") : t("settings.orden.expandProcess")}
                        >
                          {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                        </Button>
                      </TableCell>
                      <TableCell className="whitespace-nowrap text-xs">
                        {new Date(row.timestamp).toLocaleString()}
                      </TableCell>
                      <TableCell className="text-xs text-muted-foreground">{row.trigger}</TableCell>
                      <TableCell>
                        <Badge variant="outline">
                          {row.simulate ? t("settings.orden.simulated") : t("settings.orden.applied")}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-xs text-muted-foreground">
                        {t("settings.orden.processSteps", { count: logCounts[key] || 0 })}
                      </TableCell>
                      <TableCell>
                        <Badge variant={row.errors > 0 ? "destructive" : "secondary"}>
                          {row.success} / {row.errors}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-right">
                        {row.id != null && onDelete && (
                          <Button
                            type="button"
                            onClick={() => {
                              if (window.confirm(t("settings.orden.deleteHistoryConfirm"))) void onDelete(row.id!);
                            }}
                            variant="ghost"
                            size="icon"
                            className="size-7 text-destructive hover:bg-destructive/10 hover:text-destructive"
                            aria-label={t("settings.orden.deleteHistory")}
                          >
                            <Trash2 size={13} />
                          </Button>
                        )}
                      </TableCell>
                    </TableRow>
                    {isExpanded && (
                      <TableRow className="hover:bg-transparent">
                        <TableCell colSpan={7} className="bg-muted/20 p-0">
                          {logs.length === 0 ? (
                            <div className="px-4 py-4 text-xs text-muted-foreground">
                              {t("settings.orden.noLogs")}
                            </div>
                          ) : (
                            <div className="divide-y divide-border">
                              {logs.map((log, index) => (
                                <div
                                  key={`${key}-${index}`}
                                  className="grid gap-1 px-4 py-3 text-xs md:grid-cols-[3rem_6rem_5rem_minmax(0,1fr)]"
                                >
                                  <span className={log.level === "error" ? "text-destructive" : "text-muted-foreground"}>
                                    {index + 1}
                                  </span>
                                  <Badge variant={log.level === "error" ? "destructive" : "outline"} className="w-fit">
                                    {log.level}
                                  </Badge>
                                  <span className="text-muted-foreground">
                                    {log.sender} · #{log.rule_nr + 1}
                                  </span>
                                  <div className="min-w-0">
                                    <div className="break-all font-mono text-[11px] text-muted-foreground">{log.path}</div>
                                    <div className="mt-1 break-words text-foreground">{log.msg}</div>
                                  </div>
                                </div>
                              ))}
                            </div>
                          )}
                        </TableCell>
                      </TableRow>
                    )}
                  </Fragment>
                );
              })}
            </TableBody>
          </Table>
        </div>
      )}
    </div>
  );
}
