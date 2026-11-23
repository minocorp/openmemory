import { type components, client, unwrap } from "@/shared/api";

import { assertPasskeyCredential } from "./webauthn";

export type ConfirmedAction = components["schemas"]["Action"];

export async function confirmWithPasskey(action: ConfirmedAction) {
  const started = await unwrap(client.POST("/api/auth/confirm", { body: { action } }));
  return {
    challenge: started.challenge,
    credential: await assertPasskeyCredential(started.options),
  };
}
