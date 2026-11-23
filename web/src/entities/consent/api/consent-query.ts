import { queryOptions } from "@tanstack/react-query";

import { client, unwrap } from "@/shared/api";

export const consentQuery = (reference: string) =>
  queryOptions({
    queryKey: ["consent", reference],
    queryFn: () =>
      unwrap(client.GET("/api/oauth/consents/{reference}", { params: { path: { reference } } })),
    retry: false,
  });
