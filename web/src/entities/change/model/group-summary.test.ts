import { describe, expect, it } from "vitest";

import {
  describeOperation,
  newestFirst,
  pageChanges,
  pageCountLabel,
  type ChangeGroup,
} from "./group-summary";

const version = (seq: number, pageId: number, op: ChangeGroup["versions"][number]["op"]) => ({
  seq,
  page_id: pageId,
  path: `p${pageId}`,
  version: seq,
  op,
  diff: `d${seq}`,
});

const group = (id: number, at: string, versions: ChangeGroup["versions"]): ChangeGroup => ({
  id,
  at,
  author: "mino/claude-code",
  author_display_name: "Mino",
  task: null,
  versions,
});

describe("group summary", () => {
  it("lists the newest group first", () => {
    const ids = newestFirst([
      group(1, "2026-01-01T00:00:00Z", []),
      group(2, "2026-02-01T00:00:00Z", []),
    ]);
    expect(ids.map((item) => item.id)).toEqual([2, 1]);
  });

  it("merges several versions of one page and keeps a create", () => {
    const changes = pageChanges(
      group(1, "2026-01-01T00:00:00Z", [
        version(2, 1, "edit"),
        version(1, 1, "create"),
        version(3, 2, "edit"),
      ]),
    );
    expect(changes).toHaveLength(2);
    expect(changes[0]).toMatchObject({ pageId: 1, op: "create", diff: "d1\nd2" });
  });

  it("names operations and page counts", () => {
    expect(describeOperation("relink")).toBe("relinked");
    expect(pageCountLabel(1)).toBe("1 page");
    expect(pageCountLabel(3)).toBe("3 pages");
  });
});
