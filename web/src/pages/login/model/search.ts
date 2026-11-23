import { z } from "@/shared/lib/zod";

export const loginSearchSchema = z.object({
  next: z.string().optional().catch(undefined),
});

export function safeNextPath(next: string | undefined): string {
  return next?.startsWith("/") && !next.startsWith("//") ? next : "/";
}
