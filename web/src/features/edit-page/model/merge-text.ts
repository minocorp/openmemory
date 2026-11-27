import { diffArrays } from "diff";

type Hunk = { start: number; end: number; lines: string[] };

function linesOf(text: string): string[] {
  return text === "" ? [] : text.split(/(?<=\n)/);
}

function hunksOf(base: string[], other: string[]): Hunk[] {
  const hunks: Hunk[] = [];
  let index = 0;
  let open: Hunk | null = null;
  for (const part of diffArrays(base, other)) {
    if (part.added || part.removed) {
      open ??= { start: index, end: index, lines: [] };
      if (part.added) {
        open.lines.push(...part.value);
      } else {
        index += part.count;
        open.end = index;
      }
    } else {
      if (open) {
        hunks.push(open);
        open = null;
      }
      index += part.count;
    }
  }
  return open ? [...hunks, open] : hunks;
}

function touches(a: Hunk, b: Hunk): boolean {
  return a.start <= b.end && b.start <= a.end;
}

function isSame(a: Hunk, b: Hunk): boolean {
  return a.start === b.start && a.end === b.end && a.lines.join("") === b.lines.join("");
}

export function mergeText(base: string, mine: string, theirs: string): string | null {
  if (mine === theirs || theirs === base) {
    return mine;
  }
  if (mine === base) {
    return theirs;
  }
  const baseLines = linesOf(base);
  const ours = hunksOf(baseLines, linesOf(mine));
  const others = hunksOf(baseLines, linesOf(theirs));
  const merged: string[] = [];
  let index = 0;
  const apply = (hunk: Hunk) => {
    merged.push(...baseLines.slice(index, hunk.start), ...hunk.lines);
    index = hunk.end;
  };
  let mineAt = 0;
  let theirsAt = 0;
  while (mineAt < ours.length || theirsAt < others.length) {
    const a = ours[mineAt];
    const b = others[theirsAt];
    if (a && b && touches(a, b)) {
      if (!isSame(a, b)) {
        return null;
      }
      apply(a);
      mineAt += 1;
      theirsAt += 1;
    } else if (a && (!b || a.start < b.start)) {
      apply(a);
      mineAt += 1;
    } else if (b) {
      apply(b);
      theirsAt += 1;
    }
  }
  merged.push(...baseLines.slice(index));
  return merged.join("");
}
