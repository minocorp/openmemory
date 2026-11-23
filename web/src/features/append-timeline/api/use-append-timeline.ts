import { useMutation, useQueryClient } from "@tanstack/react-query";

import { appendTimeline, applySavedPage, getPage, type Page } from "@/entities/page";
import { describeError, isApiError } from "@/shared/api";
import { toast } from "@/shared/ui/toaster";

type Line = { page: Page; text: string; source: string | null; date: string | null };

async function append({ page, text, source, date }: Line): Promise<Page> {
  try {
    return await appendTimeline({ id: page.id, version: page.version, text, source, date });
  } catch (error) {
    if (!isApiError(error, 412)) {
      throw error;
    }
    const current = await getPage(page.id);
    return appendTimeline({ id: page.id, version: current.version, text, source, date });
  }
}

export function useAppendTimeline() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: append,
    onSuccess: (saved) => applySavedPage(queryClient, saved),
    onError: (error) => toast.error(describeError(error)),
  });
}
