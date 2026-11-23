import { describe, expect, it } from "vitest";

import { sideBySide } from "./side-by-side";

describe("side by side", () => {
  it("marks only the lines that differ on each side", () => {
    const result = sideBySide("one\ntwo\nthree\n", "one\n2\nthree\n");
    expect(result.theirs).toEqual([
      { text: "one", isChanged: false },
      { text: "two", isChanged: true },
      { text: "three", isChanged: false },
    ]);
    expect(result.mine).toEqual([
      { text: "one", isChanged: false },
      { text: "2", isChanged: true },
      { text: "three", isChanged: false },
    ]);
  });

  it("shows an addition on one side only", () => {
    const result = sideBySide("one\n", "one\ntwo\n");
    expect(result.theirs).toHaveLength(1);
    expect(result.mine.at(-1)).toEqual({ text: "two", isChanged: true });
  });
});
