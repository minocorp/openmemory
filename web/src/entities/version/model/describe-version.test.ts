import { describe, expect, it } from "vitest";

import { describeOperation } from "./describe-version";

describe("version operation", () => {
  it("names plain operations", () => {
    expect(describeOperation({ op: "edit", reverts_version: null })).toBe("edited");
    expect(describeOperation({ op: "relink", reverts_version: null })).toBe("updated links");
  });

  it("names the version a revert restored", () => {
    expect(describeOperation({ op: "revert", reverts_version: 3 })).toBe("restored v3");
  });
});
