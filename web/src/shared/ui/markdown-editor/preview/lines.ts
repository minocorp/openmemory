import type { Line } from "@codemirror/state";
import { Decoration } from "@codemirror/view";

import { blockPrefix, type PrefixMark } from "./block-prefix";
import { hidden, type PreviewBuild } from "./build";
import { BulletWidget, CheckboxWidget } from "./widgets";

const bullet = Decoration.replace({ widget: new BulletWidget() });

export function decorateLinePrefix(build: PreviewBuild, line: Line) {
  const prefix = blockPrefix(build.state, line);
  if (!prefix || prefix.to === prefix.from) {
    return;
  }
  build.atomic.push(hidden.range(prefix.from, prefix.to));
  const firstMark = prefix.marks[0]?.from ?? prefix.to;
  if (firstMark > prefix.from) {
    build.decorations.push(hidden.range(prefix.from, firstMark));
  }
  prefix.marks.forEach((mark, index) => {
    const decoration = markDecoration(build, mark, prefix.marks[index + 1]);
    build.decorations.push(decoration.range(mark.from, mark.to));
  });
}

function markDecoration(
  build: PreviewBuild,
  mark: PrefixMark,
  next: PrefixMark | undefined,
): Decoration {
  if (mark.name === "TaskMarker") {
    const text = build.state.sliceDoc(mark.from, mark.from + 3);
    const widget = new CheckboxWidget(text.toLowerCase() === "[x]", build.isEditable);
    return Decoration.replace({ widget });
  }
  if (mark.name === "ListMark" && next?.name !== "TaskMarker") {
    return bullet;
  }
  return hidden;
}
