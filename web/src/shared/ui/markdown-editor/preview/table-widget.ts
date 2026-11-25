import { type EditorView, WidgetType } from "@codemirror/view";

import type { TableAlign, TableModel } from "@/shared/lib/markdown";

import type { LinkIndex } from "./context";
import { renderCell } from "./inline-dom";
import { attachTableInput } from "./table-input";

export type TableSpec = {
  table: TableModel;
  changedRows: readonly boolean[];
  isEditable: boolean;
  targets: LinkIndex | null;
};

const rowHeightEstimate = 37;

export class TableWidget extends WidgetType {
  readonly spec: TableSpec;

  constructor(spec: TableSpec) {
    super();
    this.spec = spec;
  }

  override eq(other: TableWidget): boolean {
    const a = this.spec;
    const b = other.spec;
    return (
      a.isEditable === b.isEditable &&
      a.targets === b.targets &&
      sameList(a.changedRows, b.changedRows) &&
      sameList(a.table.aligns, b.table.aligns) &&
      sameList(cellTexts(a.table), cellTexts(b.table))
    );
  }

  override toDOM(view: EditorView): HTMLElement {
    const wrap = document.createElement("div");
    wrap.className = "cm-table-wrap";
    wrap.append(createTable(this.spec));
    attachTableInput(view, wrap);
    return wrap;
  }

  override updateDOM(dom: HTMLElement): boolean {
    const table = dom.querySelector("table");
    if (!table || !hasShape(table, this.spec.table)) {
      return false;
    }
    patchTable(table, this.spec);
    return true;
  }

  override ignoreEvent(event: Event): boolean {
    return event.target instanceof Element && event.target.closest(".cm-table-cell") !== null;
  }

  override get estimatedHeight(): number {
    return this.spec.table.rows.length * rowHeightEstimate;
  }
}

function cellTexts(table: TableModel): string[] {
  return table.rows.flatMap((row) => row.cells.map((cell) => cell?.text ?? "\u0000"));
}

function sameList<T>(a: readonly T[], b: readonly T[]): boolean {
  return a.length === b.length && a.every((value, index) => value === b[index]);
}

function hasShape(table: HTMLTableElement, model: TableModel): boolean {
  return (
    table.rows.length === model.rows.length &&
    [...table.rows].every((row) => row.cells.length === model.aligns.length)
  );
}

function createTable(spec: TableSpec): HTMLTableElement {
  const table = document.createElement("table");
  table.className = "cm-table";
  const head = table.createTHead();
  const body = table.createTBody();
  spec.table.rows.forEach((_row, rowIndex) => {
    const row = (rowIndex === 0 ? head : body).insertRow();
    spec.table.aligns.forEach((_align, column) => {
      const holder = document.createElement(rowIndex === 0 ? "th" : "td");
      holder.dataset["row"] = String(rowIndex);
      holder.dataset["column"] = String(column);
      const cell = document.createElement("div");
      cell.className = "cm-table-cell";
      holder.append(cell);
      row.append(holder);
    });
  });
  patchTable(table, spec);
  return table;
}

function patchTable(table: HTMLTableElement, spec: TableSpec) {
  [...table.rows].forEach((row, rowIndex) => {
    row.classList.toggle("cm-table-changed", spec.changedRows[rowIndex] === true);
    [...row.cells].forEach((holder, column) => {
      setAlign(holder, spec.table.aligns[column] ?? null);
      const cell = holder.firstElementChild;
      if (cell instanceof HTMLElement) {
        const text = spec.table.rows[rowIndex]?.cells[column]?.text ?? "";
        patchCell(cell, text, spec);
      }
    });
  });
}

function patchCell(cell: HTMLElement, text: string, spec: TableSpec) {
  if (spec.isEditable) {
    cell.contentEditable = "plaintext-only";
  } else {
    cell.removeAttribute("contenteditable");
  }
  if (cell !== document.activeElement) {
    renderCell(cell, text, spec.targets);
    return;
  }
  cell.dataset["text"] = text;
  if (cell.textContent !== text) {
    cell.textContent = text;
  }
}

function setAlign(holder: HTMLTableCellElement, align: TableAlign) {
  holder.classList.remove("cm-align-left", "cm-align-center", "cm-align-right");
  if (align) {
    holder.classList.add(`cm-align-${align}`);
  }
}
