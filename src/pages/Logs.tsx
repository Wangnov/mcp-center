import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  useInfiniteQuery,
  useQuery,
} from "@tanstack/react-query";
import {
  listServerLogs,
  getLogEntries,
  listMcpServers,
  openLogStream,
} from "@/lib/api";
import type {
  LogEntriesResponse,
  LogEntry,
  LogListResponse,
} from "@/lib/api-types.generated";
import type { McpServer } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import type { BadgeVariant } from "@/components/ui/badge-variants";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";

import { cn } from "@/lib/utils";

const PAGE_SIZE = 200;
const LIVE_BUFFER_LIMIT = 200;

type ServerSummary = LogListResponse["servers"][number];

export function LogsPage() {
  const { t } = useTranslation();
  const [selectedServer, setSelectedServer] = useState<string | null>(null);
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const [tailing, setTailing] = useState(false);
  const [autoScroll, setAutoScroll] = useState(true);
  const [liveEntries, setLiveEntries] = useState<LogEntry[]>([]);
  const [tailError, setTailError] = useState<string | null>(null);
  const listRef = useRef<HTMLDivElement | null>(null);
  const eventSourceRef = useRef<EventSource | null>(null);

  const {
    data: overview,
    isLoading: isLoadingOverview,
    isError: overviewError,
    refetch: refetchOverview,
  } = useQuery<LogListResponse, Error>({
    queryKey: ["logs", "servers"],
    queryFn: () => listServerLogs(),
  });

  const { data: mcpServers } = useQuery<McpServer[], Error>({
    queryKey: ["servers"],
    queryFn: listMcpServers,
  });

  const serverNameMap = useMemo(() => {
    const map = new Map<string, string>();
    mcpServers?.forEach((server) => {
      if (server.id) {
        const name = server.name?.trim() || server.id;
        map.set(server.id, name);
      }
    });
    return map;
  }, [mcpServers]);

  const formatServerLabel = (id: string) => {
    const name = serverNameMap.get(id);
    if (!name || name === id) return id;
    return `${name} (${id})`;
  };

  const servers = overview?.servers ?? [];

  // Ensure we always have a valid selected server when data changes
  useEffect(() => {
    if (!servers.length) {
      setSelectedServer(null);
      setSelectedFile(null);
      return;
    }

    if (selectedServer) {
      const existing = servers.find((s) => s.serverId === selectedServer);
      if (!existing) {
        setSelectedServer(servers[0].serverId);
        return;
      }
    } else {
      const serverWithLogs =
        servers.find((server) => server.files.length > 0) ?? servers[0];
      setSelectedServer(serverWithLogs.serverId);
    }
  }, [servers, selectedServer]);

  const selectedServerSummary = useMemo<ServerSummary | undefined>(() => {
    if (!selectedServer) return undefined;
    return servers.find((server) => server.serverId === selectedServer);
  }, [servers, selectedServer]);

  // Update selected file whenever server or available files change
  useEffect(() => {
    if (!selectedServerSummary) {
      setSelectedFile(null);
      return;
    }

    if (selectedServerSummary.files.length === 0) {
      setSelectedFile(null);
      return;
    }

    if (
      !selectedFile ||
      !selectedServerSummary.files.some((file) => file.file === selectedFile)
    ) {
      const latestFile =
        selectedServerSummary.files[selectedServerSummary.files.length - 1];
      setSelectedFile(latestFile.file);
    }
  }, [selectedServerSummary, selectedFile]);

  // Reset live buffer when the focus changes
  useEffect(() => {
    setLiveEntries([]);
    setTailError(null);
  }, [selectedServer, selectedFile]);

  const queryEnabled = Boolean(selectedServer && selectedFile);

  const {
    data: entryPages,
    isLoading: isLoadingEntries,
    isFetching: isFetchingEntries,
    error: entriesError,
    fetchNextPage,
    hasNextPage,
    isFetchingNextPage,
    refetch: refetchEntries,
  } = useInfiniteQuery<LogEntriesResponse, Error>({
    queryKey: ["logs", "entries", selectedServer, selectedFile],
    enabled: queryEnabled,
    initialPageParam: 0,
    queryFn: ({ pageParam }) =>
      getLogEntries({
        serverId: selectedServer!,
        file: selectedFile ?? undefined,
        cursor: typeof pageParam === "number" && pageParam > 0 ? pageParam : undefined,
        limit: PAGE_SIZE,
      }),
    getNextPageParam: (page) => page.nextCursor ?? undefined,
  });

  const baseEntries = useMemo<LogEntry[]>(() => {
    if (!entryPages) return [];
    return entryPages.pages.flatMap((page) => page.entries);
  }, [entryPages]);

  const combinedEntries = useMemo<LogEntry[]>(() => {
    if (!liveEntries.length) return baseEntries;
    return [...baseEntries, ...liveEntries];
  }, [baseEntries, liveEntries]);

  // Auto-scroll to the bottom when new entries arrive in live mode
  useEffect(() => {
    if (!autoScroll) return;
    const container = listRef.current;
    if (!container) return;
    container.scrollTop = container.scrollHeight;
  }, [combinedEntries, autoScroll]);

  // Manage SSE lifecycle when tailing is toggled
  useEffect(() => {
    if (!tailing || !selectedServer) {
      if (eventSourceRef.current) {
        eventSourceRef.current.close();
        eventSourceRef.current = null;
      }
      return;
    }

    try {
      const source = openLogStream(selectedServer);
      eventSourceRef.current = source;
      setTailError(null);

      source.onmessage = (event) => {
        try {
          const payload = JSON.parse(event.data) as LogEntry;
          setLiveEntries((prev) => {
            const next = [...prev, payload];
            if (next.length > LIVE_BUFFER_LIMIT) {
              next.splice(0, next.length - LIVE_BUFFER_LIMIT);
            }
            return next;
          });
        } catch (err) {
          console.error("Failed to parse log event", err);
        }
      };

      source.onerror = () => {
        source.close();
        eventSourceRef.current = null;
        setTailError(t("logs_tail_error"));
        setTailing(false);
      };
    } catch (err) {
      console.error("Failed to open log stream", err);
      setTailError(t("logs_tail_error"));
      setTailing(false);
    }

    return () => {
      if (eventSourceRef.current) {
        eventSourceRef.current.close();
        eventSourceRef.current = null;
      }
    };
  }, [tailing, selectedServer, t]);

  const isEmptyState =
    !isLoadingOverview && (!servers.length || selectedServerSummary?.files.length === 0);

  const handleRefresh = () => {
    refetchOverview();
    if (queryEnabled) {
      refetchEntries();
    }
  };

  const toggleTail = () => {
    if (!selectedServer) return;
    setTailing((prev) => !prev);
    setLiveEntries([]);
  };

  const selectedFileMeta = selectedServerSummary?.files.find(
    (file) => file.file === selectedFile,
  );

  return (
    <div className="flex flex-col gap-6 p-6">
      <div className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">
            {t("logs_page_title")}
          </h1>
          <p className="text-sm text-muted-foreground">
            {t("logs_page_subtitle")}
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Button
            variant="outline"
            onClick={handleRefresh}
            disabled={isLoadingOverview || isFetchingEntries}
          >
            {t("logs_refresh")}
          </Button>
          <Button
            variant={tailing ? "destructive" : "default"}
            onClick={toggleTail}
            disabled={!selectedServerSummary}
          >
            {tailing ? t("logs_stop_tail") : t("logs_start_tail")}
          </Button>
        </div>
      </div>

      <div className="rounded-xl border border-border bg-background text-foreground shadow-sm">
        <div className="flex flex-col gap-4 border-b px-6 py-4">
          <h2 className="text-base font-medium">
            {t("logs_filters_title")}
          </h2>
          <div className="flex flex-col gap-4 lg:flex-row lg:items-center">
            <div className="flex flex-col gap-2">
              <span className="text-sm font-medium text-muted-foreground">
                {t("logs_server_label")}
              </span>
              <Select
                value={selectedServer ?? ""}
                onValueChange={(value) => {
                  setSelectedServer(value);
                  setSelectedFile(null);
                }}
                disabled={isLoadingOverview || servers.length === 0}
              >
                <SelectTrigger className="w-64">
                  <SelectValue placeholder={t("logs_select_server_placeholder", { defaultValue: "" })} />
                </SelectTrigger>
                <SelectContent>
                  {servers.map((server) => (
                <SelectItem key={server.serverId} value={server.serverId}>
                  {formatServerLabel(server.serverId)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

            <div className="flex flex-col gap-2">
              <span className="text-sm font-medium text-muted-foreground">
                {t("logs_file_label")}
              </span>
              <Select
                value={selectedFile ?? ""}
                onValueChange={(value) => setSelectedFile(value)}
                disabled={!selectedServerSummary || selectedServerSummary.files.length === 0}
              >
                <SelectTrigger className="w-64">
                  <SelectValue placeholder={t("logs_select_file_placeholder", { defaultValue: "" })} />
                </SelectTrigger>
                <SelectContent>
                  {selectedServerSummary?.files.map((file) => (
                    <SelectItem key={file.file} value={file.file}>
                      {file.file}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="flex items-center gap-2">
              <Switch
                id="logs-auto-scroll"
                checked={autoScroll}
                onCheckedChange={setAutoScroll}
                disabled={!tailing}
              />
              <label
                htmlFor="logs-auto-scroll"
                className={cn(
                  "text-sm",
                  tailing ? "text-foreground" : "text-muted-foreground",
                )}
              >
                {t("logs_auto_scroll")}
              </label>
            </div>

            {tailing && (
              <Badge variant="secondary" className="uppercase tracking-wide">
                {t("logs_live_badge")}
              </Badge>
            )}
          </div>
          {selectedFileMeta && (
            <div className="text-xs text-muted-foreground">
              {t("logs_stats_lines", { count: selectedFileMeta.lineCount })} · {t(
                "logs_stats_size",
                { size: formatBytes(selectedFileMeta.sizeBytes) },
              )}
              {" "}·{" "}
              {selectedFileMeta.from && selectedFileMeta.to
                ? t("logs_stats_range", {
                    from: selectedFileMeta.from,
                    to: selectedFileMeta.to,
                  })
                : t("logs_stats_range_unknown")}
            </div>
          )}
          {tailError && (
            <div className="rounded-md bg-destructive/10 px-3 py-2 text-sm text-destructive">
              {tailError}
            </div>
          )}
        </div>
        <div className="px-6 py-4">
          {overviewError && (
            <div className="rounded-md bg-destructive/10 px-3 py-2 text-sm text-destructive">
              {overviewError.message}
            </div>
          )}
          {entriesError && (
            <div className="rounded-md bg-destructive/10 px-3 py-2 text-sm text-destructive">
              {entriesError.message}
            </div>
          )}

          {isEmptyState ? (
            <EmptyState message={t("logs_empty_state", { defaultValue: "" })} />
          ) : (
            <div className="flex flex-col gap-4">
              <div
                ref={listRef}
                className="max-h-[65vh] min-h-[300px] overflow-y-auto rounded-lg border bg-muted/10"
              >
                {isLoadingOverview || isLoadingEntries ? (
                  <LoadingState message={t("loading", { defaultValue: "Loading..." })} />
                ) : combinedEntries.length === 0 ? (
                  <EmptyState message={t("logs_no_entries", { defaultValue: "" })} />
                ) : (
                  combinedEntries.map((entry, index) => (
                    <LogEntryItem key={`${entry.timestamp}-${index}`} entry={entry} />
                  ))
                )}
              </div>

              {hasNextPage && (
                <Button
                  variant="outline"
                  onClick={() => fetchNextPage()}
                  disabled={isFetchingNextPage}
                  className="self-start"
                >
                  {isFetchingNextPage ? t("loading") : t("logs_load_more")}
                </Button>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function LogEntryItem({ entry }: { entry: LogEntry }) {
  const infoParts: string[] = [];

  if (entry.server?.id) {
    const label = entry.server.name
      ? `${entry.server.name} (${entry.server.id})`
      : entry.server.id;
    infoParts.push(`mcpServer=${label}`);
  }
  if (entry.tool?.name) {
    infoParts.push(`tool=${entry.tool.name}`);
  }
  if (entry.tool?.callId) {
    infoParts.push(`call=${truncate(entry.tool.callId)}`);
  }
  if (typeof entry.durationMs === "number") {
    infoParts.push(`duration=${formatDuration(entry.durationMs)}`);
  }

  const levelVariant: Record<string, BadgeVariant> = {
    trace: "outline",
    debug: "outline",
    info: "secondary",
    warn: "default",
    error: "destructive",
  };

  const levelLabel = entry.level?.toUpperCase?.() ?? "INFO";
  const levelStyle = levelVariant[entry.level as keyof typeof levelVariant] ?? "secondary";
  const categoryLabel = formatCategory(entry.category ?? "");

  return (
    <div className="border-b border-border/60 px-4 py-3 text-sm last:border-b-0">
      <div className="flex flex-col gap-1">
        <div className="flex flex-wrap items-center gap-2">
          <Badge variant={levelStyle}>{levelLabel}</Badge>
          {categoryLabel && categoryLabel !== "" && (
            <Badge variant="outline" className="capitalize">
              {categoryLabel}
            </Badge>
          )}
          <span className="font-mono text-xs text-muted-foreground">
            {formatTimestamp(entry.timestamp)}
          </span>
        </div>
        <div className="font-medium text-foreground">{entry.message}</div>
        {infoParts.length > 0 && (
          <div className="text-xs text-muted-foreground">
            {infoParts.join(" · ")}
          </div>
        )}
        {entry.details && !isEmptyValue(entry.details) && (
          <pre className="mt-2 max-h-60 overflow-auto rounded-md bg-muted/50 p-3 text-xs text-foreground">
            {formatDetails(entry.details)}
          </pre>
        )}
      </div>
    </div>
  );
}

function LoadingState({ message }: { message: string }) {
  return (
    <div className="flex h-full items-center justify-center py-16 text-muted-foreground">
      {message}
    </div>
  );
}

function EmptyState({ message }: { message: string }) {
  return (
    <div className="flex h-full items-center justify-center py-16 text-muted-foreground">
      {message}
    </div>
  );
}

function truncate(value: string, length = 8) {
  if (value.length <= length) return value;
  return `${value.slice(0, length)}…`;
}

function formatCategory(value: string) {
  if (!value) return "";
  return value
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/_/g, " ")
    .toLowerCase();
}

function formatTimestamp(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return date.toLocaleString();
}

function formatDuration(ms: number | bigint) {
  const value = Number(ms);
  if (Number.isNaN(value)) {
    return `${ms}`;
  }
  if (value >= 1000) {
    return `${(value / 1000).toFixed(2)}s`;
  }
  return `${value}ms`;
}

function formatBytes(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(2)} KB`;
  if (bytes < 1024 * 1024 * 1024) {
    return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
  }
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

function formatDetails(details: unknown) {
  if (typeof details === "string") return details;
  try {
    return JSON.stringify(details, null, 2);
  } catch (err) {
    console.error("Failed to stringify log details", err);
    return String(details);
  }
}

function isEmptyValue(value: unknown) {
  if (value === null || value === undefined) return true;
  if (typeof value === "string") return value.trim().length === 0;
  if (Array.isArray(value)) return value.length === 0;
  if (typeof value === "object") return Object.keys(value as Record<string, unknown>).length === 0;
  return false;
}
