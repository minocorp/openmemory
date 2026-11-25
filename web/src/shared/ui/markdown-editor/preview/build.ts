import type { EditorState, Range } from "@codemirror/state";
import { Decoration } from "@codemirror/view";

export type ListLine = { className: string; depth: number };

export type PreviewBuild = {
  state: EditorState;
  isEditable: boolean;
  decorations: Range<Decoration>[];
  atomic: Range<Decoration>[];
  listLines: Map<number, ListLine>;
};

export const hidden = Decoration.replace({});

export function createBuild(state: EditorState): PreviewBuild {
  return {
    state,
    isEditable: !state.readOnly,
    decorations: [],
    atomic: [],
    listLines: new Map(),
  };
}

export function hideAtomic(build: PreviewBuild, from: number, to: number) {
  build.decorations.push(hidden.range(from, to));
  build.atomic.push(hidden.range(from, to));
}

export function addLineClass(build: PreviewBuild, position: number, className: string) {
  build.decorations.push(
    Decoration.line({ class: className }).range(build.state.doc.lineAt(position).from),
  );
}

export function flushListLines(build: PreviewBuild) {
  for (const [from, line] of build.listLines) {
    build.decorations.push(
      Decoration.line({
        class: line.className,
        attributes: { style: `--list-depth: ${line.depth}` },
      }).range(from),
    );
  }
}
