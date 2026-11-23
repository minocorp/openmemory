import { describe, expect, it } from "vitest";

import { flattenResults, toSearchResults } from "./results";

const view = {
  pages: [
    { page_id: 1, path: "a/b", line_number: 3, line: "fee", author: "mino", at: "t" },
    { page_id: 2, path: "c", author: "mino", at: "t" },
  ],
  files: [{ file_id: 9, path: "x.pdf", page: 2, line_number: 1, line: "hi", author: "m", at: "t" }],
};

describe("toSearchResults", () => {
  it("maps page and file hits and keeps the PDF page", () => {
    const results = toSearchResults(view);
    expect(results.pages.map((hit) => [hit.path, hit.line])).toEqual([
      ["a/b", "fee"],
      ["c", null],
    ]);
    expect(results.files[0]).toMatchObject({ path: "x.pdf", filePage: 2, kind: "file" });
  });

  it("lists pages before files with unique keys", () => {
    const flat = flattenResults(toSearchResults(view));
    expect(flat.map((hit) => hit.kind)).toEqual(["page", "page", "file"]);
    expect(new Set(flat.map((hit) => hit.key)).size).toBe(3);
  });
});
