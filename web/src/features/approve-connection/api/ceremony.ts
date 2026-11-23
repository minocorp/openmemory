import { client, unwrap } from "@/shared/api";
import { assertPasskeyCredential } from "@/shared/lib/webauthn";

export type Approval = {
  reference: string;
  name: string;
  scopes: string[];
  canWrite: boolean;
};

export async function approveWithPasskey(approval: Approval) {
  const params = { path: { reference: approval.reference } };
  const started = await unwrap(
    client.POST("/api/oauth/consents/{reference}/challenge", { params }),
  );
  const credential = await assertPasskeyCredential(started.options);
  return unwrap(
    client.POST("/api/oauth/consents/{reference}/approve", {
      params,
      body: {
        challenge: started.challenge,
        credential,
        name: approval.name,
        scopes: approval.scopes,
        can_write: approval.canWrite,
      },
    }),
  );
}

export function deny(reference: string) {
  return unwrap(
    client.POST("/api/oauth/consents/{reference}/deny", { params: { path: { reference } } }),
  );
}
