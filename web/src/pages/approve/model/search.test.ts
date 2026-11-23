import { describe, expect, it } from "vitest";

import { approveSearchSchema, deviceSearchSchema, normalizeUserCode } from "./search";

describe("approve search", () => {
  it("reads the consent token", () => {
    expect(approveSearchSchema.parse({ consent: "abc" })).toEqual({ consent: "abc" });
    expect(approveSearchSchema.parse({})).toEqual({});
  });

  it("reads the device user code", () => {
    expect(deviceSearchSchema.parse({ user_code: "BCDF-GHJK" })).toEqual({
      user_code: "BCDF-GHJK",
    });
  });
});

describe("normalizeUserCode", () => {
  it("trims and upper-cases what a person typed", () => {
    expect(normalizeUserCode("  bcdf-ghjk ")).toBe("BCDF-GHJK");
  });
});
