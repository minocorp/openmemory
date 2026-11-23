import { useMutation, useQueryClient } from "@tanstack/react-query";

import { client, describeError, unwrap, unwrapEmpty } from "@/shared/api";
import { confirmWithPasskey } from "@/shared/lib/webauthn";
import { toast } from "@/shared/ui/toaster";

export function useCreateInvite() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async () =>
      unwrap(
        client.POST("/api/invites", {
          body: { confirmation: await confirmWithPasskey("create_invite") },
        }),
      ),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["invites"] }),
    onError: (error) => toast.error(describeError(error)),
  });
}

export function useRevokeInvite() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: number) =>
      unwrapEmpty(client.DELETE("/api/invites/{id}", { params: { path: { id } } })),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["invites"] }),
    onError: (error) => toast.error(describeError(error)),
  });
}

export function useUpdateUserProfile() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (variables: { id: number; username: string; displayName: string }) =>
      unwrap(
        client.PATCH("/api/users/{id}/profile", {
          params: { path: { id: variables.id } },
          body: { username: variables.username, display_name: variables.displayName },
        }),
      ),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["users"] });
      await queryClient.invalidateQueries({ queryKey: ["me"] });
    },
    onError: (error) => toast.error(describeError(error)),
  });
}

export function useSetAdmin() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (variables: { id: number; isAdmin: boolean }) =>
      unwrap(
        client.PATCH("/api/users/{id}", {
          params: { path: { id: variables.id } },
          body: {
            is_admin: variables.isAdmin,
            confirmation: await confirmWithPasskey("set_admin"),
          },
        }),
      ),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["users"] });
      await queryClient.invalidateQueries({ queryKey: ["me"] });
    },
    onError: (error) => toast.error(describeError(error)),
  });
}
