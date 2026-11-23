import { queryOptions } from "@tanstack/react-query";

import { client, unwrap } from "@/shared/api";

export const usersQuery = () =>
  queryOptions({
    queryKey: ["users"],
    queryFn: () => unwrap(client.GET("/api/users")),
  });
