import { ChangeSet, EditorSelection, Text } from "@codemirror/state";
import { describe, expect, it } from "vitest";

import { textChanges } from "./text-changes";

function apply(from: string, to: string) {
  const changes = ChangeSet.of(textChanges(from, to), from.length);
  return { text: changes.apply(Text.of(from.split("\n"))).toString(), changes };
}

describe("text changes", () => {
  it("turns one text into the other", () => {
    const cases = [
      ["one\ntwo\nthree\n", "one\n2\nthree\nfour\n"],
      ["", "new\n"],
      ["gone\n", ""],
      ["a\nb", "a\nB"],
    ] as const;
    for (const [from, to] of cases) {
      expect(apply(from, to).text).toBe(to);
    }
  });

  it("keeps a cursor in an untouched line where it was", () => {
    const from = "first line\nsecond line\nthird line\n";
    const to = "first line\nsecond line\nthird line, longer\n";
    const cursor = EditorSelection.cursor("first ".length);
    expect(cursor.map(apply(from, to).changes).head).toBe("first ".length);
  });
});
