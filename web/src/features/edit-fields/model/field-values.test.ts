import { describe, expect, it } from "vitest";

import { isValidFieldKey, parseFieldInput, sameValues } from "./field-values";

describe("field values", () => {
  it("keeps a scalar as typed, commas included", () => {
    expect(parseFieldInput(" 450,000 ", false)).toEqual(["450,000"]);
  });

  it("splits a list field on commas", () => {
    expect(parseFieldInput("a, b ,c", true)).toEqual(["a", "b", "c"]);
  });

  it("treats blank input as no values", () => {
    expect(parseFieldInput("   ", false)).toEqual([]);
  });

  it("compares value lists", () => {
    expect(sameValues(["a"], ["a"])).toBe(true);
    expect(sameValues(["a"], ["a", "b"])).toBe(false);
    expect(sameValues(null, null)).toBe(true);
    expect(sameValues(null, ["a"])).toBe(false);
  });

  it("accepts English keys only", () => {
    expect(isValidFieldKey("renewal")).toBe(true);
    expect(isValidFieldKey("due_date")).toBe(true);
    expect(isValidFieldKey("갱신")).toBe(false);
    expect(isValidFieldKey("two words")).toBe(false);
  });
});
