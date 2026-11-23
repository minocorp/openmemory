export type DiffLine =
  | { kind: "add" | "remove" | "context"; text: string }
  | { kind: "hunk"; text: string };

export function parseUnifiedDiff(diff: string): DiffLine[] {
  const lines: DiffLine[] = [];
  let hasHunk = false;
  for (const line of diff.split("\n")) {
    if (line.startsWith("@@")) {
      hasHunk = true;
      lines.push({ kind: "hunk", text: line });
    } else if (hasHunk && line.startsWith("+")) {
      lines.push({ kind: "add", text: line.slice(1) });
    } else if (hasHunk && line.startsWith("-")) {
      lines.push({ kind: "remove", text: line.slice(1) });
    } else if (hasHunk && line.startsWith(" ")) {
      lines.push({ kind: "context", text: line.slice(1) });
    }
  }
  return lines;
}

export function addedLines(diff: string): string[] {
  return parseUnifiedDiff(diff).flatMap((line) => (line.kind === "add" ? [line.text] : []));
}
