import { queryOptions } from "@tanstack/react-query";

import { client, unwrap } from "@/shared/api";

export const meQuery = () =>
  queryOptions({
    queryKey: ["me"],
    queryFn: () => unwrap(client.GET("/api/auth/me")),
    retry: false,
  });
