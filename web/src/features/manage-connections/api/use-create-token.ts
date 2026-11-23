import { useMutation, useQueryClient } from "@tanstack/react-query";

import { client, describeError, unwrap } from "@/shared/api";
import { confirmWithPasskey } from "@/shared/lib/webauthn";
import { toast } from "@/shared/ui/toaster";

export type NewToken = {
  name: string;
  scopes: string[];
  canWrite: boolean;
  expiresInDays: number;
};

async function createToken(token: NewToken) {
  const confirmation = await confirmWithPasskey("create_token");
  return unwrap(
    client.POST("/api/tokens", {
      body: {
        name: token.name,
        scopes: token.scopes,
        can_write: token.canWrite,
        expires_in_days: token.expiresInDays,
        confirmation,
      },
    }),
  );
}

export function useCreateToken() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: createToken,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["connections"] }),
    onError: (error) => toast.error(describeError(error)),
  });
}
