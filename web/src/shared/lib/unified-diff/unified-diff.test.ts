import { describe, expect, it } from "vitest";

import { addedLines, parseUnifiedDiff } from "./unified-diff";

const diff = [
  "--- a@v1",
  "+++ a@v2",
  "@@ -1,3 +1,3 @@",
  " Design partner.",
  "-450,000 per month.",
  "+500,000 per month.",
  " ",
  "\\ No newline at end of file",
].join("\n");

describe("unified diff", () => {
  it("skips file headers and keeps hunks, additions, removals and context", () => {
    expect(parseUnifiedDiff(diff).map((line) => line.kind)).toEqual([
      "hunk",
      "context",
      "remove",
      "add",
      "context",
    ]);
  });

  it("drops the marker column from line text", () => {
    expect(parseUnifiedDiff(diff)[2]).toEqual({ kind: "remove", text: "450,000 per month." });
  });

  it("keeps a body line that starts with dashes out of the header skip", () => {
    const withDashes = ["--- a", "+++ b", "@@ -1 +1 @@", "-old", "+--- new"].join("\n");
    expect(addedLines(withDashes)).toEqual(["--- new"]);
  });

  it("lists the added lines", () => {
    expect(addedLines(diff)).toEqual(["500,000 per month."]);
  });
});
