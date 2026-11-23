import { useMutation, useQueryClient } from "@tanstack/react-query";

import { client, describeError, unwrapEmpty } from "@/shared/api";
import { toast } from "@/shared/ui/toaster";

export function useDeleteFile() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: number) =>
      unwrapEmpty(client.DELETE("/api/files/{id}", { params: { path: { id } } })),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["tree"] }),
    onError: (error) => toast.error(describeError(error)),
  });
}
