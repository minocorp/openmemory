import { describe, expect, it } from "vitest";

import { filesSearchSchema } from "./search";

describe("filesSearchSchema", () => {
  it("defaults to the root folder and no filter", () => {
    expect(filesSearchSchema.parse({})).toEqual({ folder: "", q: "" });
  });

  it("keeps a folder and a filter", () => {
    expect(filesSearchSchema.parse({ folder: "contracts", q: "acme" })).toEqual({
      folder: "contracts",
      q: "acme",
    });
  });
});
