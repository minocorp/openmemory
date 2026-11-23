import { client, unwrap, unwrapEmpty } from "@/shared/api";
import { z } from "@/shared/lib/zod";

const blockedSchema = z.object({
  blocked: z.array(z.object({ path: z.string(), reason: z.string() })),
});

export type BlockedPage = z.infer<typeof blockedSchema>["blocked"][number];

export class RevertBlockedError extends Error {
  readonly blocked: BlockedPage[];

  constructor(blocked: BlockedPage[]) {
    super("revert_blocked");
    this.name = "RevertBlockedError";
    this.blocked = blocked;
  }
}

export async function revertGroup(group: number) {
  const result = await client.POST("/api/changes/{group}/revert", {
    params: { path: { group } },
  });
  if (result.error === undefined) {
    return result.data;
  }
  const parsed = blockedSchema.safeParse(result.error);
  if (result.response.status === 409 && parsed.success) {
    throw new RevertBlockedError(parsed.data.blocked);
  }
  return unwrap(Promise.resolve(result));
}

export function markSeen(seq: number) {
  return unwrapEmpty(client.POST("/api/changes/seen", { body: { seq } }));
}
