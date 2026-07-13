import { KeyboardEvent, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronLeft, ChevronRight, RefreshCw, Search, Trash2 } from "lucide-react";
import { OrdenLog, OrdenRunHistory } from "../../store/useAppStore";
import { ordenLevelLabel, ordenOperationLabel, ordenRunTriggerLabel } from "../../lib/ordenI18n";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
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
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "../ui/table";
import { Tooltip, TooltipContent, TooltipTrigger } from "../ui/tooltip";

interface OrdenRunHistoryTableProps {
  rows: OrdenRunHistory[];
  onRefresh?: () => void | Promise<void>;
  onDelete?: (id: number) => void | Promise<void>;
  onClear?: () => void | Promise<void>;
}

const LOG_PAGE_SIZE = 50;

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

function matchesLog(log: OrdenLog, query: string): boolean {
  if (!query) return true;
  return [log.path, log.msg, log.sender, log.level, String(log.rule_nr + 1)]
    .some((value) => value.toLocaleLowerCase().includes(query));
}

export function OrdenRunHistoryTable({
  rows,
  onRefresh,
  onDelete,
  onClear,
}: OrdenRunHistoryTableProps) {
  const { t } = useTranslation();
  const [selectedRow, setSelectedRow] = useState<OrdenRunHistory | null>(null);
  const [query, setQuery] = useState("");
  const [page, setPage] = useState(0);
  const logCounts = useMemo(
    () => Object.fromEntries(rows.map((row) => [rowKey(row), parseLogs(row.logs_json).length])),
    [rows]
  );
  const selectedLogs = useMemo(
    () => (selectedRow ? parseLogs(selectedRow.logs_json) : []),
    [selectedRow]
  );
  const normalizedQuery = query.trim().toLocaleLowerCase();
  const filteredLogs = useMemo(
    () => selectedLogs
      .map((log, originalIndex) => ({ log, originalIndex }))
      .filter(({ log }) => matchesLog(log, normalizedQuery)),
    [selectedLogs, normalizedQuery]
  );
  const pageCount = Math.max(1, Math.ceil(filteredLogs.length / LOG_PAGE_SIZE));
  const pageLogs = filteredLogs.slice(page * LOG_PAGE_SIZE, (page + 1) * LOG_PAGE_SIZE);
  const pageStart = filteredLogs.length === 0 ? 0 : page * LOG_PAGE_SIZE + 1;
  const pageEnd = Math.min((page + 1) * LOG_PAGE_SIZE, filteredLogs.length);

  useEffect(() => {
    setPage(0);
  }, [selectedRow, normalizedQuery]);

  useEffect(() => {
    if (page >= pageCount) setPage(pageCount - 1);
  }, [page, pageCount]);

  const openDetails = (row: OrdenRunHistory) => {
    setQuery("");
    setPage(0);
    setSelectedRow(row);
  };

  const handleRowKeyDown = (event: KeyboardEvent, row: OrdenRunHistory) => {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      openDetails(row);
    }
  };

  const deleteButton = (row: OrdenRunHistory) => row.id != null && onDelete && (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          type="button"
          onClick={(event) => {
            event.stopPropagation();
            if (window.confirm(t("settings.orden.deleteHistoryConfirm"))) void onDelete(row.id!);
          }}
          variant="ghost"
          size="icon-sm"
          className="text-destructive hover:bg-destructive/10 hover:text-destructive"
          aria-label={t("settings.orden.deleteHistory")}
        >
          <Trash2 size={13} />
        </Button>
      </TooltipTrigger>
      <TooltipContent>{t("settings.orden.deleteHistory")}</TooltipContent>
    </Tooltip>
  );

  return (
    <div className="space-y-2">
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
        <div className="rounded-lg border border-dashed border-border px-3 py-4 text-center text-xs text-muted-foreground">
          {t("settings.orden.noHistory")}
        </div>
      ) : (
        <>
          <div className="hidden overflow-hidden rounded-lg border border-border min-[900px]:block">
            <Table>
              <TableHeader>
                <TableRow>
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
                  return (
                    <TableRow
                      key={key}
                      tabIndex={0}
                      role="button"
                      onClick={() => openDetails(row)}
                      onKeyDown={(event) => handleRowKeyDown(event, row)}
                      className="cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring"
                      aria-label={t("settings.orden.openHistoryDetails", { time: new Date(row.timestamp).toLocaleString() })}
                    >
                      <TableCell className="whitespace-nowrap text-xs">{new Date(row.timestamp).toLocaleString()}</TableCell>
                      <TableCell className="text-xs text-muted-foreground">{ordenRunTriggerLabel(t, row.trigger)}</TableCell>
                      <TableCell><Badge variant="outline">{row.simulate ? t("settings.orden.simulated") : t("settings.orden.applied")}</Badge></TableCell>
                      <TableCell className="text-xs text-muted-foreground">{t("settings.orden.processSteps", { count: logCounts[key] || 0 })}</TableCell>
                      <TableCell><Badge variant={row.errors > 0 ? "destructive" : "secondary"}>{row.success} / {row.errors}</Badge></TableCell>
                      <TableCell className="text-right">{deleteButton(row)}</TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          </div>

          <div className="space-y-2 min-[900px]:hidden">
            {rows.map((row) => {
              const key = rowKey(row);
              return (
                <div
                  key={key}
                  tabIndex={0}
                  role="button"
                  onClick={() => openDetails(row)}
                  onKeyDown={(event) => handleRowKeyDown(event, row)}
                  className="cursor-pointer rounded-lg border border-border p-3 transition hover:bg-muted/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  aria-label={t("settings.orden.openHistoryDetails", { time: new Date(row.timestamp).toLocaleString() })}
                >
                  <div className="flex items-start justify-between gap-2">
                    <div className="min-w-0">
                      <div className="text-sm font-medium">{new Date(row.timestamp).toLocaleString()}</div>
                      <div className="mt-1 flex flex-wrap items-center gap-1.5 text-xs text-muted-foreground">
                        <span>{ordenRunTriggerLabel(t, row.trigger)}</span>
                        <Badge variant="outline">{row.simulate ? t("settings.orden.simulated") : t("settings.orden.applied")}</Badge>
                        <Badge variant={row.errors > 0 ? "destructive" : "secondary"}>{row.success} / {row.errors}</Badge>
                      </div>
                    </div>
                    <div className="shrink-0">{deleteButton(row)}</div>
                  </div>
                  <div className="mt-2 text-xs text-muted-foreground">{t("settings.orden.processSteps", { count: logCounts[key] || 0 })}</div>
                </div>
              );
            })}
          </div>
        </>
      )}

      <Dialog open={selectedRow != null} onOpenChange={(open) => { if (!open) setSelectedRow(null); }}>
        <DialogPopup className="max-h-[calc(100vh-2rem)] max-w-5xl">
          <DialogHeader className="border-b border-border">
            <DialogTitle>{t("settings.orden.historyDetails")}</DialogTitle>
            <DialogDescription>
              {selectedRow && t("settings.orden.historyDetailsDesc", {
                config: selectedRow.config_name,
                time: new Date(selectedRow.timestamp).toLocaleString(),
                count: selectedLogs.length,
              })}
            </DialogDescription>
          </DialogHeader>
          <DialogPanel className="space-y-3 pt-4">
            <div className="relative">
              <Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder={t("settings.orden.searchHistoryLogs")}
                className="pl-9"
                autoFocus
              />
            </div>
            {pageLogs.length === 0 ? (
              <div className="rounded-lg border border-dashed border-border px-4 py-8 text-center text-sm text-muted-foreground">
                {normalizedQuery ? t("settings.orden.noMatchingLogs") : t("settings.orden.noLogs")}
              </div>
            ) : (
              <div className="divide-y divide-border overflow-hidden rounded-lg border border-border">
                {pageLogs.map(({ log, originalIndex }) => {
                  return (
                    <div key={`${rowKey(selectedRow!)}-${originalIndex}`} className="grid gap-2 px-3 py-3 text-xs md:grid-cols-[3rem_6rem_7rem_minmax(0,1fr)]">
                      <span className={log.level === "error" ? "text-destructive" : "text-muted-foreground"}>#{originalIndex + 1}</span>
                      <Badge variant={log.level === "error" ? "destructive" : "outline"} className="w-fit">{ordenLevelLabel(t, log.level)}</Badge>
                      <span className="text-muted-foreground">{ordenOperationLabel(t, log.sender)} · #{log.rule_nr + 1}</span>
                      <div className="min-w-0">
                        <div className="break-all font-mono text-[11px] text-muted-foreground">{log.path}</div>
                        <div className="mt-1 break-words text-foreground">{log.msg}</div>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </DialogPanel>
          <DialogFooter className="flex-wrap items-center justify-between">
            <div className="text-xs text-muted-foreground">
              {t("settings.orden.historyPageSummary", { start: pageStart, end: pageEnd, total: filteredLogs.length })}
            </div>
            <div className="flex items-center gap-2">
              <Button type="button" variant="outline" size="sm" onClick={() => setPage((value) => Math.max(0, value - 1))} disabled={page === 0}>
                <ChevronLeft size={14} />{t("settings.orden.previousPage")}
              </Button>
              <span className="min-w-16 text-center text-xs text-muted-foreground">{t("settings.orden.pageStatus", { page: page + 1, pages: pageCount })}</span>
              <Button type="button" variant="outline" size="sm" onClick={() => setPage((value) => Math.min(pageCount - 1, value + 1))} disabled={page + 1 >= pageCount}>
                {t("settings.orden.nextPage")}<ChevronRight size={14} />
              </Button>
              <DialogClose render={<Button type="button" variant="ghost" size="sm" />}>{t("settings.orden.closeDetails")}</DialogClose>
            </div>
          </DialogFooter>
        </DialogPopup>
      </Dialog>
    </div>
  );
}
