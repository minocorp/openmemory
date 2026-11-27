import type { ChangeSpec } from "@codemirror/state";
import { diffLines } from "diff";

export function textChanges(from: string, to: string): ChangeSpec[] {
  const changes: ChangeSpec[] = [];
  let position = 0;
  for (const part of diffLines(from, to)) {
    if (part.added) {
      changes.push({ from: position, insert: part.value });
    } else if (part.removed) {
      changes.push({ from: position, to: position + part.value.length });
      position += part.value.length;
    } else {
      position += part.value.length;
    }
  }
  return changes;
}
