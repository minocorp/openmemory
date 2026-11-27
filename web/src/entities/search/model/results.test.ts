import { describe, expect, it } from "vitest";

import { flattenResults, toSearchResults } from "./results";

const hit = { author: "mino", at: "t" };

const view = {
  hits: [
    { ...hit, kind: "page" as const, id: 1, path: "a/b", line_number: 3, line: "fee" },
    { ...hit, kind: "file" as const, id: 9, path: "a/x.pdf", page: 2, line_number: 1, line: "hi" },
  ],
  folders: [
    {
      path: "a/c",
      matches: 12,
      hits: [{ ...hit, kind: "page" as const, id: 2, path: "a/c/d", line: "fee" }],
    },
  ],
};

describe("toSearchResults", () => {
  it("keeps the order of hits and the PDF page of a file", () => {
    const results = toSearchResults(view);
    expect(results.hits.map((result) => [result.kind, result.path, result.filePage])).toEqual([
      ["page", "a/b", null],
      ["file", "a/x.pdf", 2],
    ]);
  });

  it("ends each folder with an entry that searches the whole folder", () => {
    const [folder] = toSearchResults(view).folders;
    expect(folder?.results.map((result) => [result.kind, result.path, result.matches])).toEqual([
      ["page", "a/c/d", null],
      ["folder", "a/c", 12],
    ]);
  });

  it("flattens hits before folders with unique keys", () => {
    const flat = flattenResults(toSearchResults(view));
    expect(flat.map((result) => result.path)).toEqual(["a/b", "a/x.pdf", "a/c/d", "a/c"]);
    expect(new Set(flat.map((result) => result.key)).size).toBe(4);
  });
});
