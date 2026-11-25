import { type EditorView, runScopeHandlers } from "@codemirror/view";

import { type CellPosition, editCell, type TableModel } from "@/shared/lib/markdown";

import { handlersFacet, linkTargetsFacet } from "./context";
import { renderCell } from "./inline-dom";
import { openLinkTarget } from "./link-click";
import { type Caret, editTable, exitTable, focusCell, placeCaret } from "./table-focus";
import { tableAt } from "./table-source";

type CellTarget = { element: HTMLElement; position: CellPosition };

const editorShortcutKeys = new Set(["s", "z", "y"]);

export function attachTableInput(view: EditorView, wrap: HTMLElement) {
  wrap.addEventListener("mousedown", (event) => handleMouseDown(view, event));
  wrap.addEventListener("focusin", (event) => handleFocusIn(view, wrap, event));
  wrap.addEventListener("focusout", (event) => handleFocusOut(view, wrap, event));
  wrap.addEventListener("input", (event) => {
    if (!(event instanceof InputEvent && event.isComposing)) {
      commitCell(view, wrap, event.target);
    }
  });
  wrap.addEventListener("compositionend", (event) => commitCell(view, wrap, event.target));
  wrap.addEventListener("paste", (event) => handlePaste(view, wrap, event));
  wrap.addEventListener("keydown", (event) => handleKeyDown(view, wrap, event));
}

function cellTarget(target: EventTarget | null): CellTarget | null {
  const element = target instanceof Element ? target.closest<HTMLElement>(".cm-table-cell") : null;
  const holder = element?.parentElement;
  if (!element || !holder) {
    return null;
  }
  return {
    element,
    position: { row: Number(holder.dataset["row"]), column: Number(holder.dataset["column"]) },
  };
}

function currentTable(view: EditorView, wrap: HTMLElement): TableModel | null {
  return tableAt(view.state, view.posAtDOM(wrap));
}

function handleMouseDown(view: EditorView, event: MouseEvent) {
  const cell = cellTarget(event.target);
  const isEditing = cell !== null && cell.element === document.activeElement;
  if (event.button !== 0 || isEditing || !(event.target instanceof Element)) {
    return;
  }
  if (openLinkTarget(view, event.target)) {
    event.preventDefault();
  }
}

function handleFocusIn(view: EditorView, wrap: HTMLElement, event: FocusEvent) {
  const cell = cellTarget(event.target);
  const table = currentTable(view, wrap);
  if (!cell || !table || view.state.readOnly) {
    return;
  }
  const text = cell.element.dataset["text"] ?? "";
  if (cell.element.textContent !== text) {
    cell.element.textContent = text;
    placeCaret(cell.element, "end");
  }
  cell.element.parentElement?.classList.add("cm-table-active");
  reportFocus(view, table, cell);
}

function handleFocusOut(view: EditorView, wrap: HTMLElement, event: FocusEvent) {
  const cell = cellTarget(event.target);
  if (!cell) {
    return;
  }
  cell.element.parentElement?.classList.remove("cm-table-active");
  renderCell(cell.element, cell.element.dataset["text"] ?? "", view.state.facet(linkTargetsFacet));
  const next = event.relatedTarget;
  const staysInTable =
    next instanceof Element && (wrap.contains(next) || next.closest("[data-table-tools]"));
  if (!staysInTable) {
    view.state.facet(handlersFacet).focusTable(null);
  }
}

function reportFocus(view: EditorView, table: TableModel, cell: CellTarget) {
  view.state.facet(handlersFacet).focusTable({
    from: table.from,
    row: cell.position.row,
    column: cell.position.column,
    rowCount: table.rows.length,
    columnCount: table.aligns.length,
    align: table.aligns[cell.position.column] ?? null,
    element: cell.element,
  });
}

function commitCell(view: EditorView, wrap: HTMLElement, target: EventTarget | null) {
  const cell = cellTarget(target);
  const table = currentTable(view, wrap);
  if (!cell || !table) {
    return;
  }
  const text = cell.element.textContent ?? "";
  const change = editCell(table, cell.position, text);
  if (!change || view.state.sliceDoc(change.from, change.to) === change.insert) {
    return;
  }
  cell.element.dataset["text"] = text;
  view.dispatch({ changes: change, userEvent: "input.type" });
}

function handlePaste(view: EditorView, wrap: HTMLElement, event: ClipboardEvent) {
  const selection = window.getSelection();
  const pasted = event.clipboardData?.getData("text/plain");
  if (!selection || selection.rangeCount === 0 || pasted === undefined) {
    return;
  }
  event.preventDefault();
  const range = selection.getRangeAt(0);
  range.deleteContents();
  const text = document.createTextNode(pasted.replace(/\s*\r?\n\s*/g, " "));
  range.insertNode(text);
  range.setStartAfter(text);
  range.collapse(true);
  selection.removeAllRanges();
  selection.addRange(range);
  commitCell(view, wrap, event.target);
}

function handleKeyDown(view: EditorView, wrap: HTMLElement, event: KeyboardEvent) {
  const cell = cellTarget(event.target);
  const table = currentTable(view, wrap);
  if (event.isComposing || !cell || !table) {
    return;
  }
  const isShortcut = (event.metaKey || event.ctrlKey) && !event.altKey;
  if (isShortcut && event.key.toLowerCase() === "a") {
    event.preventDefault();
    selectContents(cell.element);
    return;
  }
  if (isShortcut && editorShortcutKeys.has(event.key.toLowerCase())) {
    if (runScopeHandlers(view, event, "editor")) {
      event.preventDefault();
      restoreFocus(view, table.from, cell.position);
    }
    return;
  }
  if (moveFromCell({ view, table, position: cell.position, element: cell.element, event })) {
    event.preventDefault();
  }
}

function selectContents(element: HTMLElement) {
  const range = document.createRange();
  range.selectNodeContents(element);
  window.getSelection()?.removeAllRanges();
  window.getSelection()?.addRange(range);
}

function restoreFocus(view: EditorView, from: number, position: CellPosition) {
  const table = tableAt(view.state, from);
  if (!table || document.activeElement?.closest(".cm-table-cell")) {
    return;
  }
  focusCell(
    view,
    table.from,
    {
      row: Math.min(position.row, table.rows.length - 1),
      column: Math.min(position.column, table.aligns.length - 1),
    },
    "end",
  );
}

type KeyContext = {
  view: EditorView;
  table: TableModel;
  position: CellPosition;
  element: HTMLElement;
  event: KeyboardEvent;
};

const cellKeys: Record<string, (context: KeyContext) => boolean> = {
  Tab: (context) =>
    context.event.shiftKey
      ? moveOrStay(context, previousCell(context.table, context.position), "end")
      : moveToNextCell(context),
  Enter: (context) => moveDown(context, true),
  ArrowDown: (context) => moveDown(context, false),
  ArrowUp: ({ view, table, position }) =>
    position.row === 0
      ? leave(view, table, "before")
      : moveOrStay({ view, table }, { row: position.row - 1, column: position.column }, "end"),
  ArrowLeft: (context) =>
    !context.event.shiftKey &&
    caretOffset(context.element) === 0 &&
    moveOrLeave(context, previousCell(context.table, context.position), "end", "before"),
  ArrowRight: (context) =>
    !context.event.shiftKey &&
    caretOffset(context.element) === (context.element.textContent ?? "").length &&
    moveOrLeave(context, nextCell(context.table, context.position), "start", "after"),
  Escape: ({ view, table }) => leave(view, table, "after"),
};

function moveFromCell(context: KeyContext): boolean {
  return cellKeys[context.event.key]?.(context) ?? false;
}

function moveDown(context: KeyContext, canAddRow: boolean): boolean {
  const { view, table, position } = context;
  const below = { row: position.row + 1, column: position.column };
  if (position.row < table.rows.length - 1) {
    return moveOrStay(context, below, "end");
  }
  if (!canAddRow) {
    return leave(view, table, "after");
  }
  editTable(view, table.from, { kind: "insertRow", at: below.row }, below);
  return true;
}

function moveToNextCell(context: KeyContext): boolean {
  const { view, table } = context;
  const next = nextCell(table, context.position);
  if (next) {
    return moveOrStay(context, next, "end");
  }
  const at = table.rows.length;
  editTable(view, table.from, { kind: "insertRow", at }, { row: at, column: 0 });
  return true;
}

function previousCell(table: TableModel, position: CellPosition): CellPosition | null {
  if (position.column > 0) {
    return { row: position.row, column: position.column - 1 };
  }
  return position.row > 0 ? { row: position.row - 1, column: table.aligns.length - 1 } : null;
}

function nextCell(table: TableModel, position: CellPosition): CellPosition | null {
  if (position.column < table.aligns.length - 1) {
    return { row: position.row, column: position.column + 1 };
  }
  return position.row < table.rows.length - 1 ? { row: position.row + 1, column: 0 } : null;
}

function moveOrStay(
  context: Pick<KeyContext, "view" | "table">,
  position: CellPosition | null,
  caret: Caret,
): boolean {
  if (position) {
    focusCell(context.view, context.table.from, position, caret);
  }
  return true;
}

function moveOrLeave(
  context: KeyContext,
  position: CellPosition | null,
  caret: Caret,
  side: "before" | "after",
): boolean {
  return position ? moveOrStay(context, position, caret) : leave(context.view, context.table, side);
}

function leave(view: EditorView, table: TableModel, side: "before" | "after"): boolean {
  exitTable(view, table, side);
  return true;
}

function caretOffset(element: HTMLElement): number | null {
  const selection = window.getSelection();
  if (!selection || selection.rangeCount === 0 || !selection.isCollapsed) {
    return null;
  }
  const range = selection.getRangeAt(0);
  const before = document.createRange();
  before.selectNodeContents(element);
  before.setEnd(range.startContainer, range.startOffset);
  return before.toString().length;
}
