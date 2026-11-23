import { type Extension, type Range } from "@codemirror/state";
import {
  Decoration,
  type DecorationSet,
  type EditorView,
  ViewPlugin,
  type ViewUpdate,
} from "@codemirror/view";

const marked = Decoration.line({ class: "cm-changed-line" });

export function markedLines(texts: ReadonlySet<string>): Extension {
  if (texts.size === 0) {
    return [];
  }
  return ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;

      constructor(view: EditorView) {
        this.decorations = this.build(view);
      }

      update(update: ViewUpdate) {
        if (update.docChanged || update.viewportChanged) {
          this.decorations = this.build(update.view);
        }
      }

      build(view: EditorView): DecorationSet {
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
    },
    { decorations: (plugin) => plugin.decorations },
  );
}
