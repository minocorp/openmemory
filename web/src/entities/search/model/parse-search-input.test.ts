import { describe, expect, it } from "vitest";

import { isSearchable, parseSearchInput } from "./parse-search-input";

describe("parseSearchInput", () => {
  it("keeps plain words as text", () => {
    expect(parseSearchInput("  monthly   fee ")).toEqual({ text: "monthly fee", conditions: [] });
  });

  it("splits field conditions from the text", () => {
    expect(parseSearchInput("acme renewal<+90d status!=paused")).toEqual({
      text: "acme",
      conditions: ["renewal<+90d", "status!=paused"],
    });
  });

  it("reads key:exists and key~text as conditions", () => {
    expect(parseSearchInput("owner:exists note~urgent").conditions).toEqual([
      "owner:exists",
      "note~urgent",
    ]);
  });

  it("treats a Korean two-character word as text", () => {
    expect(parseSearchInput("계약")).toEqual({ text: "계약", conditions: [] });
  });

  it("is not searchable when empty", () => {
    expect(isSearchable(parseSearchInput("   "))).toBe(false);
    expect(isSearchable(parseSearchInput("a"))).toBe(true);
    expect(isSearchable(parseSearchInput("status=active"))).toBe(true);
  });
});
