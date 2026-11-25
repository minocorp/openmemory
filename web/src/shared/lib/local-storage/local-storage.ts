import type { z } from "@/shared/lib/zod";

export function readStored<T>(key: string, schema: z.ZodType<T>): T | undefined {
  const text = localStorage.getItem(key);
  if (text === null) {
    return undefined;
  }
  const parsed = schema.safeParse(parseJson(text));
  return parsed.success ? parsed.data : undefined;
}

export function writeStored(key: string, value: unknown): void {
  localStorage.setItem(key, JSON.stringify(value));
}

function parseJson(text: string): unknown {
  try {
    return JSON.parse(text);
  } catch {
    return undefined;
  }
}
