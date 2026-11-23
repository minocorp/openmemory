import { useMutation, useQueryClient } from "@tanstack/react-query";

import { describeError } from "@/shared/api";
import { toast } from "@/shared/ui/toaster";

import { approveWithPasskey, deny } from "./ceremony";

export function useApprove() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: approveWithPasskey,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["connections"] }),
    onError: (error) => toast.error(describeError(error)),
  });
}

export function useDeny() {
  return useMutation({
    mutationFn: deny,
    onError: (error) => toast.error(describeError(error)),
  });
}
