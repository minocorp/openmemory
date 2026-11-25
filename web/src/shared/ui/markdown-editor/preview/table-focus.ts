import type { EditorView } from "@codemirror/view";

import {
  applyTableEdit,
  type CellPosition,
  type TableEdit,
  type TableModel,
} from "@/shared/lib/markdown";

import { tableAt } from "./table-source";

export type Caret = "start" | "end";

export function tableElementAt(view: EditorView, from: number): HTMLElement | null {
  const wraps = view.contentDOM.querySelectorAll<HTMLElement>(".cm-table-wrap");
  return [...wraps].find((wrap) => view.posAtDOM(wrap) === from) ?? null;
}

export function focusCell(view: EditorView, from: number, position: CellPosition, caret: Caret) {
  const wrap = tableElementAt(view, from);
  const cell = wrap?.querySelector<HTMLElement>(
    `[data-row="${position.row}"][data-column="${position.column}"] > .cm-table-cell`,
  );
  if (!cell) {
    return;
  }
  cell.focus({ preventScroll: true });
  placeCaret(cell, caret);
  cell.scrollIntoView({ block: "nearest", inline: "nearest" });
}

export function placeCaret(element: HTMLElement, caret: Caret) {
  const selection = window.getSelection();
  if (!selection) {
    return;
  }
  const range = document.createRange();
  range.selectNodeContents(element);
  range.collapse(caret === "start");
  selection.removeAllRanges();
  selection.addRange(range);
}

export function exitTable(view: EditorView, table: TableModel, side: "before" | "after") {
  const { length } = view.state.doc;
  if (side === "before") {
    view.dispatch(
      table.from > 0
        ? { selection: { anchor: table.from - 1 }, scrollIntoView: true }
        : { changes: { from: 0, insert: "\n" }, selection: { anchor: 0 }, scrollIntoView: true },
    );
  } else {
    view.dispatch(
      table.to < length
        ? { selection: { anchor: table.to + 1 }, scrollIntoView: true }
        : {
            changes: { from: length, insert: "\n" },
            selection: { anchor: length + 1 },
            scrollIntoView: true,
          },
    );
  }
  view.focus();
}

export function editTable(view: EditorView, from: number, edit: TableEdit, next: CellPosition) {
  const table = tableAt(view.state, from);
  if (!table) {
    return;
  }
  view.dispatch({ changes: applyTableEdit(table, edit), userEvent: "input.table" });
  focusCell(view, from, next, "end");
}

export function deleteTable(view: EditorView, from: number) {
  const table = tableAt(view.state, from);
  if (!table) {
    return;
  }
  const end = Math.min(table.to + 1, view.state.doc.length);
  view.dispatch({
    changes: { from: table.from, to: end },
    selection: { anchor: table.from },
    userEvent: "delete.table",
  });
  view.focus();
}
