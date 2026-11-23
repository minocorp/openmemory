import { diffLines } from "diff";

export type SideLine = { text: string; isChanged: boolean };

export type SideBySide = { theirs: SideLine[]; mine: SideLine[] };

export function sideBySide(theirs: string, mine: string): SideBySide {
  const result: SideBySide = { theirs: [], mine: [] };
  for (const part of diffLines(theirs, mine)) {
    const lines = part.value.replace(/\n$/, "").split("\n");
    if (!part.added) {
      result.theirs.push(...lines.map((text) => ({ text, isChanged: part.removed })));
    }
    if (!part.removed) {
      result.mine.push(...lines.map((text) => ({ text, isChanged: part.added })));
    }
  }
  return result;
}
