import { describe, expect, it } from "vitest";

import { parseAuthor } from "./author";

describe("author", () => {
  it("treats a plain name as a person", () => {
    expect(parseAuthor("mino")).toEqual({ kind: "person", name: "mino", owner: null });
  });

  it("splits an agent into owner and connection name", () => {
    expect(parseAuthor("mino/claude-code")).toEqual({
      kind: "agent",
      name: "claude-code",
      owner: "mino",
    });
  });

  it("keeps the link update marker on the author", () => {
    expect(parseAuthor("mino/claude-code (link update)").name).toBe("claude-code (link update)");
    expect(parseAuthor("mino (link update)").kind).toBe("person");
  });
});
