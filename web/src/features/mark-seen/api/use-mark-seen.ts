import { useMutation, useQueryClient } from "@tanstack/react-query";

import { markSeen } from "@/entities/change";
import { describeError } from "@/shared/api";
import { toast } from "@/shared/ui/toaster";

export function useMarkSeen() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: markSeen,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["changes"] }),
    onError: (error) => toast.error(describeError(error)),
  });
}
