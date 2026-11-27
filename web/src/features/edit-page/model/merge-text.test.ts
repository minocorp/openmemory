import { describe, expect, it } from "vitest";

import { mergeText } from "./merge-text";

const base = "one\ntwo\nthree\nfour\nfive\n";

describe("merge text", () => {
  it("keeps both sides when they changed lines far apart", () => {
    expect(mergeText(base, "ONE\ntwo\nthree\nfour\nfive\n", "one\ntwo\nthree\nfour\nFIVE\n")).toBe(
      "ONE\ntwo\nthree\nfour\nFIVE\n",
    );
  });

  it("keeps an insertion on one side and a deletion on the other", () => {
    expect(mergeText(base, "zero\none\ntwo\nthree\nfour\nfive\n", "one\ntwo\nthree\nfive\n")).toBe(
      "zero\none\ntwo\nthree\nfive\n",
    );
  });

  it("refuses when both sides changed the same line differently", () => {
    expect(
      mergeText(base, "one\n2\nthree\nfour\nfive\n", "one\nzwei\nthree\nfour\nfive\n"),
    ).toBeNull();
  });

  it("refuses when the changes touch neighbouring lines", () => {
    expect(mergeText(base, "one\n2\nthree\nfour\nfive\n", "one\ntwo\n3\nfour\nfive\n")).toBeNull();
  });

  it("applies an identical change once", () => {
    const both = "one\ntwo\n3\nfour\nfive\n";
    expect(mergeText(base, both, both)).toBe(both);
    expect(mergeText(base, "one\ntwo\n3\nfour\nFIVE\n", "one\ntwo\n3\nfour\nfive\n")).toBe(
      "one\ntwo\n3\nfour\nFIVE\n",
    );
  });

  it("takes the other side whole when one side made no change", () => {
    expect(mergeText(base, base, "new\n")).toBe("new\n");
    expect(mergeText(base, "mine\n", base)).toBe("mine\n");
  });

  it("merges into text without a final newline", () => {
    expect(mergeText("a\nb\nc\nd", "A\nb\nc\nd", "a\nb\nc\nD")).toBe("A\nb\nc\nD");
  });
});
