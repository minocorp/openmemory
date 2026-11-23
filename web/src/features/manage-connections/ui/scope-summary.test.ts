import { describe, expect, it } from "vitest";

import { scopeSummary } from "./scope-summary";

describe("scopeSummary", () => {
  it("says all folders for the wildcard", () => {
    expect(scopeSummary(["*"])).toBe("All folders");
  });

  it("lists the chosen folders", () => {
    expect(scopeSummary(["customers", "notes/ideas"])).toBe("customers, notes/ideas");
  });

  it("says no folders for an empty scope", () => {
    expect(scopeSummary([])).toBe("No folders");
  });
});
