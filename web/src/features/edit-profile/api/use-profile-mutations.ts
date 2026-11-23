import { useMutation, useQueryClient } from "@tanstack/react-query";

import { client, describeError, unwrap } from "@/shared/api";
import { toast } from "@/shared/ui/toaster";

type ProfileUpdate = { username: string; displayName: string };

export function useUpdateMyProfile() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (update: ProfileUpdate) =>
      unwrap(
        client.PATCH("/api/auth/me", {
          body: { username: update.username, display_name: update.displayName },
        }),
      ),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["me"] });
      toast.success("Profile saved");
    },
    onError: (error) => toast.error(describeError(error)),
  });
}
