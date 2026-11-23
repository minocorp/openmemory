import { z } from "@/shared/lib/zod";

export const registerSearchSchema = z.object({
  token: z.string().optional().catch(undefined),
});
