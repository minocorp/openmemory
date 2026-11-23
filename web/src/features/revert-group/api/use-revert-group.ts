import { useMutation, useQueryClient } from "@tanstack/react-query";

import { RevertBlockedError, revertGroup } from "@/entities/change";
import { describeError } from "@/shared/api";
import { toast } from "@/shared/ui/toaster";

export function useRevertGroup() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: revertGroup,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["changes"] });
      await queryClient.invalidateQueries({ queryKey: ["tree"] });
      await queryClient.invalidateQueries({ queryKey: ["page"] });
      await queryClient.invalidateQueries({ queryKey: ["target"] });
      toast.success("Group reverted");
    },
    onError: (error) => {
      if (!(error instanceof RevertBlockedError)) {
        toast.error(describeError(error));
      }
    },
  });
}
