import { useMutation, useQueryClient } from "@tanstack/react-query";

import { client, describeError, unwrap, unwrapEmpty } from "@/shared/api";
import { confirmWithPasskey, createPasskeyCredential } from "@/shared/lib/webauthn";
import { toast } from "@/shared/ui/toaster";

async function addPasskey(name: string) {
  const confirmation = await confirmWithPasskey("add_passkey");
  const started = await unwrap(
    client.POST("/api/auth/passkeys/start", { body: { name, confirmation } }),
  );
  const credential = await createPasskeyCredential(started.options);
  await unwrapEmpty(
    client.POST("/api/auth/passkeys/finish", {
      body: { challenge: started.challenge, credential },
    }),
  );
}

export function useAddPasskey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: addPasskey,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["passkeys"] }),
    onError: (error) => toast.error(describeError(error)),
  });
}

export function useRemovePasskey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (id: number) =>
      unwrapEmpty(
        client.DELETE("/api/auth/passkeys/{id}", {
          params: { path: { id } },
          body: { confirmation: await confirmWithPasskey("remove_passkey") },
        }),
      ),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["passkeys"] }),
    onError: (error) => toast.error(describeError(error)),
  });
}
