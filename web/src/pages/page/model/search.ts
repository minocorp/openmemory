import { z } from "@/shared/lib/zod";

export const pageSearchSchema = z.object({
  file: z.string().optional().catch(undefined),
  fpage: z.number().int().positive().optional().catch(undefined),
  quote: z.string().optional().catch(undefined),
  changes: z.boolean().optional().catch(undefined),
  history: z.boolean().optional().catch(undefined),
});

export type PageSearch = z.infer<typeof pageSearchSchema>;
