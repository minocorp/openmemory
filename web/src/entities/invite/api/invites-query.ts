import { queryOptions } from "@tanstack/react-query";

import { client, unwrap } from "@/shared/api";

export const invitesQuery = () =>
  queryOptions({
    queryKey: ["invites"],
    queryFn: () => unwrap(client.GET("/api/invites")),
  });
