import type { EditorView } from "@codemirror/view";

export type InlineFormat = "bold" | "italic" | "strike" | "code";

const markers: Record<InlineFormat, string> = {
  bold: "**",
  italic: "*",
  strike: "~~",
  code: "`",
};

export function toggleInline(view: EditorView, format: InlineFormat) {
  const marker = markers[format];
  const { from, to } = view.state.selection.main;
  const before = view.state.doc.sliceString(Math.max(0, from - marker.length), from);
  const after = view.state.doc.sliceString(to, to + marker.length);
  if (before === marker && after === marker) {
    view.dispatch({
      changes: [
        { from: from - marker.length, to: from },
        { from: to, to: to + marker.length },
      ],
    });
  } else {
    view.dispatch({
      changes: [
        { from, insert: marker },
        { from: to, insert: marker },
      ],
      selection: { anchor: from + marker.length, head: to + marker.length },
    });
  }
  view.focus();
}

export function toggleHeading(view: EditorView) {
  const line = view.state.doc.lineAt(view.state.selection.main.from);
  const existing = /^#{1,6} /.exec(line.text);
  const next = existing ? "" : "## ";
  view.dispatch({
    changes: { from: line.from, to: line.from + (existing?.[0].length ?? 0), insert: next },
  });
  view.focus();
}

export function makeLink(view: EditorView) {
  const { from, to } = view.state.selection.main;
  const label = view.state.doc.sliceString(from, to);
  const urlStart = from + label.length + 3;
  view.dispatch({
    changes: { from, to, insert: `[${label}](url)` },
    selection: { anchor: urlStart, head: urlStart + 3 },
  });
  view.focus();
}
