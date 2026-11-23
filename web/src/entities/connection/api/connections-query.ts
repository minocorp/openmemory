import { queryOptions } from "@tanstack/react-query";

import { client, unwrap } from "@/shared/api";

export const connectionsQuery = () =>
  queryOptions({
    queryKey: ["connections"],
    queryFn: () => unwrap(client.GET("/api/connections")),
  });
