import { queryOptions } from "@tanstack/react-query";

import { ApiError, client, unwrap } from "@/shared/api";

export function getFileByPath(path: string) {
  return unwrap(client.GET("/api/files", { params: { query: { path } } }));
}

export const fileQuery = (id: number) =>
  queryOptions({
    queryKey: ["file", id],
    queryFn: () => unwrap(client.GET("/api/files/{id}", { params: { path: { id } } })),
  });

export const fileByPathQuery = (path: string) =>
  queryOptions({ queryKey: ["file", "path", path], queryFn: () => getFileByPath(path) });

export function fileContentUrl(id: number, options: { download?: boolean } = {}): string {
  return `/api/files/${id}/content${options.download ? "?download=true" : ""}`;
}

export const fileTextQuery = (id: number) =>
  queryOptions({
    queryKey: ["file", id, "text"],
    queryFn: async () => {
      const response = await fetch(fileContentUrl(id));
      if (!response.ok) {
        throw new ApiError(response.status, null);
      }
      return response.text();
    },
    staleTime: Infinity,
  });
