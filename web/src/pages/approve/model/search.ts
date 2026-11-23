import { z } from "@/shared/lib/zod";

export const approveSearchSchema = z.object({ consent: z.string().optional() });

export const deviceSearchSchema = z.object({ user_code: z.string().optional() });

export const normalizeUserCode = (input: string) => input.trim().toUpperCase();
