import { describe, expect, it } from "vitest";

import { isSearchable, parseSearchInput, withFolder } from "./parse-search-input";

describe("parseSearchInput", () => {
  it("keeps plain words as text", () => {
    expect(parseSearchInput("  monthly   fee ")).toEqual({
      text: "monthly fee",
      conditions: [],
      folder: null,
    });
  });

  it("splits field conditions from the text", () => {
    expect(parseSearchInput("acme renewal<+90d status!=paused")).toEqual({
      text: "acme",
      conditions: ["renewal<+90d", "status!=paused"],
      folder: null,
    });
  });

  it("reads key:exists and key~text as conditions", () => {
    expect(parseSearchInput("owner:exists note~urgent").conditions).toEqual([
      "owner:exists",
      "note~urgent",
    ]);
  });

  it("reads in:folder as the folder to search", () => {
    expect(parseSearchInput("정산 in:career/toss")).toEqual({
      text: "정산",
      conditions: [],
      folder: "career/toss",
    });
    expect(parseSearchInput("in:exists").conditions).toEqual(["in:exists"]);
    expect(parseSearchInput("in:").text).toBe("in:");
  });

  it("treats a Korean two-character word as text", () => {
    expect(parseSearchInput("계약")).toEqual({ text: "계약", conditions: [], folder: null });
  });

  it("is not searchable when empty", () => {
    expect(isSearchable(parseSearchInput("   "))).toBe(false);
    expect(isSearchable(parseSearchInput("a"))).toBe(true);
    expect(isSearchable(parseSearchInput("status=active"))).toBe(true);
    expect(isSearchable(parseSearchInput("in:clients"))).toBe(true);
  });
});

describe("withFolder", () => {
  it("adds the folder or replaces the one already given", () => {
    expect(withFolder("정산 ", "career")).toBe("정산 in:career");
    expect(withFolder("정산 in:career", "career/toss")).toBe("정산 in:career/toss");
  });
});
