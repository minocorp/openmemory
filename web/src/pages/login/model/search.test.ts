import { describe, expect, it } from "vitest";

import { safeNextPath } from "./search";

describe("next path after sign-in", () => {
  it("keeps a path inside the app", () => {
    expect(safeNextPath("/p/customers/Acme?history=true")).toBe("/p/customers/Acme?history=true");
  });

  it("falls back to home for anything else", () => {
    expect(safeNextPath(undefined)).toBe("/");
    expect(safeNextPath("//evil.example")).toBe("/");
    expect(safeNextPath("https://evil.example")).toBe("/");
  });
});
