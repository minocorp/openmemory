import { useMutation, useQueryClient } from "@tanstack/react-query";

import { describeError } from "@/shared/api";
import { toast } from "@/shared/ui/toaster";

import { downloadExport, importArchive } from "./transfer";

export function useExport() {
  return useMutation({
    mutationFn: downloadExport,
    onError: (error) => toast.error(describeError(error)),
  });
}

export function useImport() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: importArchive,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["tree"] });
      void queryClient.invalidateQueries({ queryKey: ["changes"] });
    },
    onError: (error) => toast.error(describeError(error)),
  });
}
