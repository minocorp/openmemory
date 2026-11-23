import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";

import { createPage, deletePage, movePage, type Page } from "@/entities/page";
import { describeError, isApiError } from "@/shared/api";
import { toast } from "@/shared/ui/toaster";

function useRefreshPaths() {
  const queryClient = useQueryClient();
  return async () => {
    await queryClient.invalidateQueries({ queryKey: ["target"] });
    await queryClient.invalidateQueries({ queryKey: ["tree"] });
    await queryClient.invalidateQueries({ queryKey: ["changes"] });
  };
}

export function useCreatePage() {
  const navigate = useNavigate();
  const refresh = useRefreshPaths();
  return useMutation({
    mutationFn: (path: string) => createPage({ path, content: "" }),
    onSuccess: async (page) => {
      await refresh();
      await navigate({ to: "/p/$", params: { _splat: page.path } });
    },
    onError: (error) =>
      toast.error(
        isApiError(error, 412) ? "A page already exists at that path." : describeError(error),
      ),
  });
}

export function useMovePage() {
  const navigate = useNavigate();
  const refresh = useRefreshPaths();
  return useMutation({
    mutationFn: (options: { page: Page; to: string }) =>
      movePage({ id: options.page.id, version: options.page.version, to: options.to }),
    onSuccess: async (moved) => {
      await refresh();
      await navigate({ to: "/p/$", params: { _splat: moved.path } });
    },
    onError: (error) => toast.error(describeError(error)),
  });
}

export function useDeletePage() {
  const navigate = useNavigate();
  const refresh = useRefreshPaths();
  return useMutation({
    mutationFn: (page: Page) => deletePage({ id: page.id, version: page.version }),
    onSuccess: async () => {
      await refresh();
      await navigate({ to: "/" });
    },
    onError: (error) => toast.error(describeError(error)),
  });
}
