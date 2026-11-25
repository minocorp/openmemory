import { syntaxTree } from "@codemirror/language";
import type { KeyBinding } from "@codemirror/view";
import type { EditorView } from "@codemirror/view";

import { focusCell } from "./table-focus";
import { readTableNode, tableNodes } from "./table-source";

function enterTable(view: EditorView, forward: boolean): boolean {
  const { main } = view.state.selection;
  if (!main.empty) {
    return false;
  }
  const moved = view.moveVertically(main, forward);
  const table = tableNodes(view.state).find(
    (node) => node.from <= moved.head && moved.head <= node.to,
  );
  if (!table) {
    return false;
  }
  const line = view.state.doc.lineAt(main.head);
  const isAdjacent = forward ? line.to + 1 === table.from : line.from - 1 === table.to;
  if (!isAdjacent) {
    return false;
  }
  const rows = view.state.doc.lineAt(table.to).number - view.state.doc.lineAt(table.from).number;
  focusCell(view, table.from, { row: forward ? 0 : rows - 1, column: 0 }, "end");
  return true;
}

function deleteBlockBackward(view: EditorView): boolean {
  const { state } = view;
  const { main } = state.selection;
  const line = state.doc.lineAt(main.head);
  if (!main.empty || main.head !== line.from || line.number === 1) {
    return false;
  }
  const previous = state.doc.line(line.number - 1);
  const table = tableNodes(state).find((node) => node.to === previous.to);
  if (table) {
    const rows = state.doc.lineAt(table.to).number - state.doc.lineAt(table.from).number;
    const columns = readTableNode(state, table)?.aligns.length ?? 1;
    focusCell(view, table.from, { row: rows - 1, column: Math.max(0, columns - 1) }, "end");
    return true;
  }
  if (syntaxTree(state).resolveInner(previous.from, 1).name !== "HorizontalRule") {
    return false;
  }
  view.dispatch({ changes: { from: previous.from, to: line.from }, userEvent: "delete" });
  return true;
}

export const previewKeymap: readonly KeyBinding[] = [
  { key: "ArrowDown", run: (view) => enterTable(view, true) },
  { key: "ArrowUp", run: (view) => enterTable(view, false) },
  { key: "Backspace", run: deleteBlockBackward },
];
