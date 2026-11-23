import { useMutation, useQueryClient } from "@tanstack/react-query";

import { client, describeError, unwrap, unwrapEmpty } from "@/shared/api";
import { assertPasskeyCredential } from "@/shared/lib/webauthn";
import { toast } from "@/shared/ui/toaster";

export type PurgeTarget = { kind: "page" | "file"; id: number };

async function purge({ kind, id }: PurgeTarget) {
  const challenge = await unwrap(client.POST("/api/purge/challenge"));
  const body = {
    challenge: challenge.challenge,
    credential: await assertPasskeyCredential(challenge.options),
  };
  await unwrapEmpty(
    kind === "page"
      ? client.POST("/api/pages/{id}/purge", { params: { path: { id } }, body })
      : client.POST("/api/files/{id}/purge", { params: { path: { id } }, body }),
  );
}

export function usePurge() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: purge,
    onSuccess: () => queryClient.invalidateQueries(),
    onError: (error) => toast.error(describeError(error)),
  });
}
