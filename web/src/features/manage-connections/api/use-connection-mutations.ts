import { useMutation, useQueryClient } from "@tanstack/react-query";

import type { ConnectionUpdate } from "@/entities/connection";
import { client, describeError, unwrap, unwrapEmpty } from "@/shared/api";
import { toast } from "@/shared/ui/toaster";

export function useUpdateConnection() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (variables: { id: number; update: ConnectionUpdate }) =>
      unwrap(
        client.PATCH("/api/connections/{id}", {
          params: { path: { id: variables.id } },
          body: variables.update,
        }),
      ),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["connections"] }),
    onError: (error) => toast.error(describeError(error)),
  });
}

export function useRevokeConnection() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: number) =>
      unwrapEmpty(client.DELETE("/api/connections/{id}", { params: { path: { id } } })),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["connections"] }),
    onError: (error) => toast.error(describeError(error)),
  });
}
