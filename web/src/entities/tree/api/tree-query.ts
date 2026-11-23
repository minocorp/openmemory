import { queryOptions } from "@tanstack/react-query";

import { client, unwrap } from "@/shared/api";

export const treeQuery = () =>
  queryOptions({
    queryKey: ["tree"],
    queryFn: () => unwrap(client.GET("/api/tree")),
  });
