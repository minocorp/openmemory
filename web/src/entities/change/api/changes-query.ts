import { queryOptions } from "@tanstack/react-query";

import { client, unwrap } from "@/shared/api";

export const changesQuery = () =>
  queryOptions({
    queryKey: ["changes"],
    queryFn: () => unwrap(client.GET("/api/changes", { params: { query: {} } })),
  });
