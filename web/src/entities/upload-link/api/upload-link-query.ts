import { queryOptions } from "@tanstack/react-query";

import { client, unwrap } from "@/shared/api";

export const uploadLinkQuery = (token: string) =>
  queryOptions({
    queryKey: ["upload-link", token],
    queryFn: () => unwrap(client.GET("/api/upload-links/{token}", { params: { path: { token } } })),
    retry: false,
    staleTime: Infinity,
  });
