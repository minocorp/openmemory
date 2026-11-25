import { describe, expect, it } from "vitest";

import { parseInline } from "./inline";

function wikiLinks(text: string): string[] {
  return parseInline(text).flatMap((node) => (node.kind === "wikilink" ? [node.path] : []));
}

describe("wiki links", () => {
  it.each([
    ["see [[clients/acme]] and [[tax]]", ["clients/acme", "tax"]],
    ["[[a]][[b]]", ["a", "b"]],
    ["[[x [[y]]", ["y"]],
    ["[[a\nb]] [[[c]]", []],
    ["[[]]", []],
    ["open [[x", []],
    ["\\[[a]]", []],
    ["`[[a]]` and [[b]]", ["b"]],
    ["[[한글/페이지]]", ["한글/페이지"]],
  ])("finds the targets in %j", (text, expected) => {
    expect(wikiLinks(text)).toEqual(expected);
  });
});
