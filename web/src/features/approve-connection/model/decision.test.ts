import { describe, expect, it } from "vitest";

import { connectionNameProblem, hasScopeSelection, outcomeOf } from "./decision";

describe("outcomeOf", () => {
  it("redirects when the server names a target", () => {
    expect(outcomeOf({ redirect_uri: "http://127.0.0.1:9/callback?code=x" })).toEqual({
      kind: "redirect",
      target: "http://127.0.0.1:9/callback?code=x",
    });
  });

  it("is done for a device approval", () => {
    expect(outcomeOf({ redirect_uri: null })).toEqual({ kind: "done" });
    expect(outcomeOf({})).toEqual({ kind: "done" });
  });
});

describe("hasScopeSelection", () => {
  it("needs at least one folder or all", () => {
    expect(hasScopeSelection([])).toBe(false);
    expect(hasScopeSelection(["*"])).toBe(true);
    expect(hasScopeSelection(["customers"])).toBe(true);
  });
});

describe("connectionNameProblem", () => {
  it("accepts a plain name", () => {
    expect(connectionNameProblem(" claude-code ")).toBeNull();
  });

  it("rejects empty, long and slashed names", () => {
    expect(connectionNameProblem("  ")).toBe("empty");
    expect(connectionNameProblem("a".repeat(65))).toBe("long");
    expect(connectionNameProblem("a/b")).toBe("slash");
  });
});
