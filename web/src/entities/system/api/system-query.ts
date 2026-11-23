import { queryOptions } from "@tanstack/react-query";

import { client, unwrap } from "@/shared/api";

export const systemQuery = () =>
  queryOptions({
    queryKey: ["system"],
    queryFn: () => unwrap(client.GET("/api/system")),
  });
