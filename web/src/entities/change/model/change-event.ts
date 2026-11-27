import type { components } from "@/shared/api";
import { z } from "@/shared/lib/zod";

export type ChangeEvent = components["schemas"]["ChangeEvent"];
export type StreamReady = components["schemas"]["StreamReady"];

const operationSchema = z.enum(["create", "edit", "move", "relink", "delete", "revert"]);

export const changeEventSchema = z.discriminatedUnion("kind", [
  z.object({
    kind: z.literal("page"),
    id: z.number(),
    version: z.number(),
    op: operationSchema,
    path: z.string().nullable(),
    previous_path: z.string().nullable(),
  }),
  z.object({
    kind: z.literal("file"),
    id: z.number(),
    op: z.enum(["upload", "delete"]),
    path: z.string(),
  }),
  z.object({
    kind: z.literal("purge"),
    subject: z.enum(["page", "file"]),
    id: z.number(),
    path: z.string(),
  }),
]) satisfies z.ZodType<ChangeEvent>;

export const streamReadySchema = z.object({
  resumed: z.boolean(),
}) satisfies z.ZodType<StreamReady>;
