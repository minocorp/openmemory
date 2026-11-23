import { describe, expect, it } from "vitest";

import { isFolderOpen, revealSelection } from "./folder-state";

describe("folder state", () => {
  it("opens the ancestors of the selected path by default", () => {
    expect(isFolderOpen("customers", {}, "customers/acme")).toBe(true);
    expect(isFolderOpen("decisions", {}, "customers/acme")).toBe(false);
  });

  it("lets an explicit choice win over the selection", () => {
    expect(isFolderOpen("customers", { customers: false }, "customers/acme")).toBe(false);
    expect(isFolderOpen("tax", { tax: true }, null)).toBe(true);
  });

  it("drops choices that would hide a newly selected page", () => {
    expect(revealSelection({ customers: false, tax: true }, "customers/acme")).toEqual({
      tax: true,
    });
  });
});
