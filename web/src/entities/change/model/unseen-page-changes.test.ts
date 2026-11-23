import { describe, expect, it } from "vitest";

import type { ChangeGroup } from "./group-summary";
import { unseenChangesOfPage } from "./unseen-page-changes";

const view = (
  group: number,
  author: string,
  at: string,
  versions: { page_id: number; version: number; diff: string }[],
): ChangeGroup => ({
  id: group,
  at,
  author,
  author_display_name: author,
  task: null,
  versions: versions.map((version) => ({
    ...version,
    seq: version.version,
    path: "p",
    op: "edit",
  })),
});

describe("unseen page changes", () => {
  it("is empty when the page was not touched", () => {
    const groups = [view(1, "sam", "2026-01-01T00:00:00Z", [])];
    expect(unseenChangesOfPage({ groups, last_seq: 1, since: 0 }, 1)).toBeNull();
  });

  it("keeps each change with its author and counts authors", () => {
    const groups = [
      view(1, "sam", "2026-01-01T00:00:00Z", [{ page_id: 1, version: 2, diff: "+a" }]),
      view(2, "mino/claude-code", "2026-01-02T00:00:00Z", [
        { page_id: 1, version: 3, diff: "+b" },
        { page_id: 2, version: 9, diff: "+x" },
      ]),
    ];
    const unseen = unseenChangesOfPage({ groups, last_seq: 3, since: 0 }, 1);
    expect(unseen).toMatchObject({ author: "mino/claude-code", firstVersion: 2, authorCount: 2 });
    expect(unseen?.changes.map((change) => change.author)).toEqual(["sam", "mino/claude-code"]);
  });
});
