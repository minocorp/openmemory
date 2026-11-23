import { describe, expect, it } from "vitest";

import { findQuote } from "./pdf-quote";

describe("quote search in text pieces", () => {
  it("finds a quote inside one piece", () => {
    expect(findQuote(["Article 3 monthly fee"], "monthly fee")).toEqual({
      start: { piece: 0, offset: 10 },
      end: { piece: 0, offset: 21 },
    });
  });

  it("finds a quote that spans pieces and ignores spacing differences", () => {
    const pieces = ["monthly ", "fee of ", "500,000"];
    expect(findQuote(pieces, "monthly fee   of 500,000")).toEqual({
      start: { piece: 0, offset: 0 },
      end: { piece: 2, offset: 7 },
    });
  });

  it("returns null when the quote is absent or blank", () => {
    expect(findQuote(["abc"], "xyz")).toBeNull();
    expect(findQuote(["abc"], "  ")).toBeNull();
  });
});
