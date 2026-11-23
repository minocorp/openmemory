import { describe, expect, it } from "vitest";

import { nextIndex } from "./next-index";

describe("nextIndex", () => {
  it("wraps around in both directions", () => {
    expect(nextIndex("ArrowDown", 2, 3)).toBe(0);
    expect(nextIndex("ArrowUp", 0, 3)).toBe(2);
  });

  it("ignores other keys and empty lists", () => {
    expect(nextIndex("a", 0, 3)).toBeNull();
    expect(nextIndex("ArrowDown", 0, 0)).toBeNull();
  });
});
