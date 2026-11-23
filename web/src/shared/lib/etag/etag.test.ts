import { describe, expect, it } from "vitest";

import { etagOf, versionOfEtag } from "./etag";

describe("etag", () => {
  it("quotes the version as a strong tag", () => {
    expect(etagOf(7)).toBe('"7"');
  });

  it("reads the version back from a strong tag", () => {
    expect(versionOfEtag(etagOf(12))).toBe(12);
  });

  it("refuses weak and malformed tags", () => {
    expect(versionOfEtag('W/"3"')).toBeNull();
    expect(versionOfEtag("3")).toBeNull();
    expect(versionOfEtag(null)).toBeNull();
  });
});
