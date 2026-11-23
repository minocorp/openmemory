import { queryOptions } from "@tanstack/react-query";

import { client, unwrap } from "@/shared/api";

export const passkeysQuery = () =>
  queryOptions({
    queryKey: ["passkeys"],
    queryFn: () => unwrap(client.GET("/api/auth/passkeys")),
  });
