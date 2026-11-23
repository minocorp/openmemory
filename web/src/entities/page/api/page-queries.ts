import { infiniteQueryOptions, queryOptions, type QueryClient } from "@tanstack/react-query";

import type { Page } from "../model/types";
import { getPage, getPageVersion, listVersions } from "./page-api";

export const pageQuery = (id: number) =>
  queryOptions({ queryKey: ["page", id], queryFn: () => getPage(id) });

export const pageVersionQuery = (id: number, version: number) =>
  queryOptions({
    queryKey: ["page", id, "version", version],
    queryFn: () => getPageVersion(id, version),
    staleTime: Infinity,
  });

export const versionsQuery = (id: number) =>
  infiniteQueryOptions({
    queryKey: ["page", id, "versions"],
    queryFn: ({ pageParam }) => listVersions({ id, cursor: pageParam }),
    initialPageParam: undefined as string | undefined,
    getNextPageParam: (last) => last.next ?? undefined,
  });

export function applySavedPage(queryClient: QueryClient, page: Page) {
  queryClient.setQueryData(pageQuery(page.id).queryKey, page);
  void queryClient.invalidateQueries({ queryKey: ["page", page.id, "versions"] });
  void queryClient.invalidateQueries({ queryKey: ["changes"] });
  void queryClient.invalidateQueries({ queryKey: ["tree"] });
}
