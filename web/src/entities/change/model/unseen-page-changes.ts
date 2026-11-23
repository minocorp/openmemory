import type { components } from "@/shared/api";
import { addedLines } from "@/shared/lib/unified-diff";

type Changes = components["schemas"]["Changes"];

export type UnseenChange = {
  version: number;
  author: string;
  at: string;
  diff: string;
};

export type UnseenPageChanges = {
  author: string;
  at: string;
  firstVersion: number;
  authorCount: number;
  changes: UnseenChange[];
  addedLines: ReadonlySet<string>;
};

export function unseenChangesOfPage(changes: Changes, pageId: number): UnseenPageChanges | null {
  const touched = changes.groups
    .flatMap((group) =>
      group.versions
        .filter((version) => version.page_id === pageId)
        .map((version) => ({ group, version })),
    )
    .toSorted((a, b) => a.version.version - b.version.version);
  const first = touched.at(0);
  const last = touched.at(-1);
  if (!first || !last) {
    return null;
  }
  return {
    author: last.group.author,
    at: last.group.at,
    firstVersion: first.version.version,
    authorCount: new Set(touched.map(({ group }) => group.author)).size,
    changes: touched.map(({ group, version }) => ({
      version: version.version,
      author: group.author,
      at: group.at,
      diff: version.diff,
    })),
    addedLines: new Set(touched.flatMap(({ version }) => addedLines(version.diff))),
  };
}
