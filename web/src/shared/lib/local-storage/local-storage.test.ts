import { beforeEach, describe, expect, it, vi } from "vitest";

import { z } from "@/shared/lib/zod";

import { readStored, writeStored } from "./local-storage";

const key = "test.value";
const schema = z.object({ count: z.number() });

describe("local storage", () => {
  beforeEach(() => {
    const values = new Map<string, string>();
    vi.stubGlobal("localStorage", {
      getItem: (name: string) => values.get(name) ?? null,
      setItem: (name: string, value: string) => values.set(name, value),
    });
  });

  it("reads back what it wrote", () => {
    writeStored(key, { count: 3 });
    expect(readStored(key, schema)).toEqual({ count: 3 });
  });

  it("reads nothing when the key is missing", () => {
    expect(readStored(key, schema)).toBeUndefined();
  });

  it("ignores values that are not json or do not match the schema", () => {
    localStorage.setItem(key, "{broken");
    expect(readStored(key, schema)).toBeUndefined();
    writeStored(key, { count: "three" });
    expect(readStored(key, schema)).toBeUndefined();
  });
});
