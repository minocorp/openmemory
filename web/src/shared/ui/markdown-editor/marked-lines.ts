import type { Range } from "@codemirror/state";
import {
  Decoration,
  type DecorationSet,
  type EditorView,
  ViewPlugin,
  type ViewUpdate,
} from "@codemirror/view";

import { changedLinesFacet } from "./preview/context";

const marked = Decoration.line({ class: "cm-changed-line" });

function buildMarks(view: EditorView): DecorationSet {
  const texts = view.state.facet(changedLinesFacet);
  if (texts.size === 0) {
    return Decoration.none;
  }
  const found: Range<Decoration>[] = [];
  for (const { from, to } of view.visibleRanges) {
    for (let position = from; position <= to;) {
      const line = view.state.doc.lineAt(position);
      if (line.text.trim() !== "" && texts.has(line.text)) {
        found.push(marked.range(line.from));
      }
      position = line.to + 1;
    }
  }
  return Decoration.set(found);
}

export const markedLines = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;

    constructor(view: EditorView) {
      this.decorations = buildMarks(view);
    }

    update(update: ViewUpdate) {
      const changedTexts =
        update.startState.facet(changedLinesFacet) !== update.state.facet(changedLinesFacet);
      if (update.docChanged || update.viewportChanged || changedTexts) {
        this.decorations = buildMarks(update.view);
      }
    }
  },
  { decorations: (plugin) => plugin.decorations },
);
