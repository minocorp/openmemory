import { useMutation, useQueryClient } from "@tanstack/react-query";

import { applySavedPage, overwritePage, withBody, type Page } from "@/entities/page";

export function useSaveBody() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (options: { page: Page; body: string }) =>
      overwritePage({
        id: options.page.id,
        version: options.page.version,
        content: withBody(options.page, options.body),
      }),
    onSuccess: (saved) => applySavedPage(queryClient, saved),
  });
}
