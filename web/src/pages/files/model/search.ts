import { z } from "@/shared/lib/zod";

export const filesSearchSchema = z.object({
  folder: z.string().default(""),
  q: z.string().default(""),
});

export type FilesSearch = z.infer<typeof filesSearchSchema>;
