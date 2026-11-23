import { z } from "@/shared/lib/zod";

export const bootstrapSearchSchema = z.object({
  token: z.string().optional().catch(undefined),
});
