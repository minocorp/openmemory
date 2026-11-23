import { useMutation, useQueryClient } from "@tanstack/react-query";

import { restorePageVersion } from "@/entities/page";
import { describeError } from "@/shared/api";
import { toast } from "@/shared/ui/toaster";

export function useRestoreVersion() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: restorePageVersion,
    onSuccess: async (_restored, options) => {
      await queryClient.invalidateQueries({ queryKey: ["page", options.id] });
      await queryClient.invalidateQueries({ queryKey: ["changes"] });
      await queryClient.invalidateQueries({ queryKey: ["tree"] });
      await queryClient.invalidateQueries({ queryKey: ["target"] });
      toast.success(`Restored version ${options.version}`);
    },
    onError: (error) => toast.error(describeError(error)),
  });
}
