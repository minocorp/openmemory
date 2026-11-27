import type { InvalidateQueryFilters, QueryClient } from "@tanstack/react-query";

import { changesQuery, type ChangeEvent, type StreamReady } from "@/entities/change";
import { fileByPathQuery, fileQuery } from "@/entities/file";
import { pageQuery, versionsQuery } from "@/entities/page";
import { treeQuery } from "@/entities/tree";
import { pathTargetQuery } from "@/pages/page";

const flushDelayMs = 100;

const memoryRoots: readonly unknown[] = ["page", "tree", "changes", "target", "file", "search"];

type Filter = InvalidateQueryFilters & Required<Pick<InvalidateQueryFilters, "queryKey">>;

function sharedFilters(paths: readonly (string | null | undefined)[]): Filter[] {
  const targets = paths
    .filter((path) => typeof path === "string")
    .map((path) => ({ queryKey: pathTargetQuery(path).queryKey }));
  return [
    { queryKey: treeQuery().queryKey },
    { queryKey: changesQuery().queryKey },
    { queryKey: ["search"] },
    ...targets,
  ];
}

function pageFilters(queryClient: QueryClient, id: number, version: number): Filter[] {
  const cached = queryClient.getQueryData(pageQuery(id).queryKey);
  const isBehind = cached === undefined || cached.version < version;
  return [
    ...(isBehind ? [{ queryKey: pageQuery(id).queryKey, exact: true }] : []),
    { queryKey: versionsQuery(id).queryKey },
  ];
}

export function filtersFor(queryClient: QueryClient, change: ChangeEvent): Filter[] {
  if (change.kind === "page") {
    return [
      ...pageFilters(queryClient, change.id, change.version),
      ...sharedFilters([change.path, change.previous_path]),
    ];
  }
  if (change.kind === "file" || change.subject === "file") {
    return [
      { queryKey: fileQuery(change.id).queryKey, exact: true },
      { queryKey: fileByPathQuery(change.path).queryKey },
      ...sharedFilters([change.path]),
    ];
  }
  return [{ queryKey: pageQuery(change.id).queryKey }, ...sharedFilters([change.path])];
}

export function createLiveCache(queryClient: QueryClient) {
  let pending = new Map<string, Filter>();
  let timer: ReturnType<typeof setTimeout> | null = null;

  const flush = () => {
    const filters = [...pending.values()];
    pending = new Map();
    timer = null;
    for (const filter of filters) {
      void queryClient.invalidateQueries(filter);
    }
  };

  return {
    onReady: (ready: StreamReady) => {
      if (!ready.resumed) {
        void queryClient.invalidateQueries({
          predicate: (query) => memoryRoots.includes(query.queryKey[0]),
        });
      }
    },
    onChange: (change: ChangeEvent) => {
      for (const filter of filtersFor(queryClient, change)) {
        pending.set(JSON.stringify([filter.queryKey, filter.exact ?? false]), filter);
      }
      timer ??= setTimeout(flush, flushDelayMs);
    },
    stop: () => {
      if (timer !== null) {
        clearTimeout(timer);
      }
    },
  };
}
