export type TableAlign = "left" | "center" | "right" | null;

export type TableCell = { text: string; from: number; to: number; pipe: number | null };

export type TableRow = { from: number; to: number; cells: readonly (TableCell | null)[] };

export type TableModel = {
  from: number;
  to: number;
  aligns: readonly TableAlign[];
  rows: readonly TableRow[];
};

export type SourceLine = { text: string; from: number };

export type TextChange = { from: number; to: number; insert: string };

type Slot = { from: number; to: number };

const pipe = 124;
const backslash = 92;
const minimumDashes = 3;

export function readTable(lines: readonly SourceLine[]): TableModel | null {
  const [header, delimiter, ...body] = lines;
  if (!header || !delimiter) {
    return null;
  }
  const aligns = splitSlots(delimiter.text).map((slot) =>
    readAlign(delimiter.text.slice(slot.from, slot.to)),
  );
  const rows = [header, ...body].map((line) => readRow(line, aligns.length));
  const last = lines.at(-1) ?? delimiter;
  return { from: header.from, to: last.from + last.text.length, aligns, rows };
}

export function editCell(
  table: TableModel,
  position: CellPosition,
  text: string,
): TextChange | null {
  const row = table.rows[position.row];
  if (!row) {
    return null;
  }
  const cell = row.cells[position.column];
  if (cell) {
    return replaceCell(cell, escapeCell(text));
  }
  const texts = row.cells.map((existing, index) =>
    index === position.column ? text : (existing?.text ?? ""),
  );
  return { from: row.from, to: row.to, insert: formatRow(texts, texts.map(displayWidth)) };
}

function replaceCell(cell: TableCell, insert: string): TextChange {
  if (cell.pipe === null) {
    return { from: cell.from, to: cell.to, insert };
  }
  const padding = cell.pipe - cell.to;
  const width = displayWidth(escapeCell(cell.text));
  const spaces = Math.max(Math.min(1, padding), padding + width - displayWidth(insert));
  return { from: cell.from, to: cell.pipe, insert: insert + " ".repeat(spaces) };
}

export type CellPosition = { row: number; column: number };

export type TableEdit =
  | { kind: "insertRow"; at: number }
  | { kind: "deleteRow"; at: number }
  | { kind: "insertColumn"; at: number }
  | { kind: "deleteColumn"; at: number }
  | { kind: "align"; column: number; align: TableAlign };

export function applyTableEdit(table: TableModel, edit: TableEdit): TextChange {
  const grid = table.rows.map((row) => row.cells.map((cell) => cell?.text ?? ""));
  const next = nextGrid(grid, table.aligns, edit);
  return { from: table.from, to: table.to, insert: formatTable(next.aligns, next.grid) };
}

export function formatTable(aligns: readonly TableAlign[], grid: readonly string[][]): string {
  const widths = aligns.map((_, column) =>
    Math.max(minimumDashes, ...grid.map((row) => displayWidth(row[column] ?? ""))),
  );
  const [header = [], ...body] = grid;
  const delimiter = `| ${aligns.map((align, column) => delimiterCell(align, widths[column] ?? minimumDashes)).join(" | ")} |`;
  return [formatRow(header, widths), delimiter, ...body.map((row) => formatRow(row, widths))].join(
    "\n",
  );
}

export function emptyTable(columns: number, bodyRows: number): string {
  const aligns = Array.from({ length: columns }, (): TableAlign => null);
  const grid = Array.from({ length: bodyRows + 1 }, () => aligns.map(() => ""));
  return formatTable(aligns, grid);
}

function nextGrid(
  grid: readonly string[][],
  aligns: readonly TableAlign[],
  edit: TableEdit,
): { grid: string[][]; aligns: TableAlign[] } {
  const rows = grid.map((row) => [...row]);
  switch (edit.kind) {
    case "insertRow":
      return {
        grid: rows.toSpliced(
          edit.at,
          0,
          aligns.map(() => ""),
        ),
        aligns: [...aligns],
      };
    case "deleteRow":
      return { grid: rows.toSpliced(edit.at, 1), aligns: [...aligns] };
    case "insertColumn":
      return {
        grid: rows.map((row) => row.toSpliced(edit.at, 0, "")),
        aligns: aligns.toSpliced(edit.at, 0, null),
      };
    case "deleteColumn":
      return {
        grid: rows.map((row) => row.toSpliced(edit.at, 1)),
        aligns: aligns.toSpliced(edit.at, 1),
      };
    default:
      return { grid: rows, aligns: aligns.with(edit.column, edit.align) };
  }
}

function readRow(line: SourceLine, columns: number): TableRow {
  const slots = splitSlots(line.text);
  const cells = Array.from({ length: columns }, (_, index) => {
    const slot = slots[index];
    return slot ? readCell(line, slot) : null;
  });
  return { from: line.from, to: line.from + line.text.length, cells };
}

function readCell(line: SourceLine, slot: Slot): TableCell {
  const raw = line.text.slice(slot.from, slot.to);
  const start = raw.length - raw.trimStart().length;
  const content = raw.trim();
  const from = content === "" ? slot.from + Math.min(1, raw.length) : slot.from + start;
  const isClosed = line.text.charCodeAt(slot.to) === pipe;
  return {
    text: unescapeCell(content),
    from: line.from + from,
    to: line.from + from + content.length,
    pipe: isClosed ? line.from + slot.to : null,
  };
}

function splitSlots(text: string): Slot[] {
  const pipes = unescapedPipes(text);
  const bounds = [-1, ...pipes, text.length];
  const slots = bounds.slice(1).map((end, index) => ({ from: (bounds[index] ?? -1) + 1, to: end }));
  const leading = pipes.at(0) === text.length - text.trimStart().length;
  const trailing = pipes.at(-1) === text.trimEnd().length - 1;
  return slots.slice(leading ? 1 : 0, trailing ? -1 : undefined);
}

function unescapedPipes(text: string): number[] {
  const found: number[] = [];
  let isEscaped = false;
  for (let index = 0; index < text.length; index += 1) {
    const code = text.charCodeAt(index);
    if (code === pipe && !isEscaped) {
      found.push(index);
    }
    isEscaped = !isEscaped && code === backslash;
  }
  return found;
}

function readAlign(raw: string): TableAlign {
  const text = raw.trim();
  const left = text.startsWith(":");
  const right = text.endsWith(":");
  if (left && right) {
    return "center";
  }
  if (right) {
    return "right";
  }
  return left ? "left" : null;
}

function delimiterCell(align: TableAlign, width: number): string {
  switch (align) {
    case "left":
      return `:${"-".repeat(width - 1)}`;
    case "center":
      return `:${"-".repeat(width - 2)}:`;
    case "right":
      return `${"-".repeat(width - 1)}:`;
    default:
      return "-".repeat(width);
  }
}

function formatRow(texts: readonly string[], widths: readonly number[]): string {
  const cells = texts.map((text, column) => {
    const padding = Math.max(0, (widths[column] ?? 0) - displayWidth(text));
    return escapeCell(text) + " ".repeat(padding);
  });
  return `| ${cells.join(" | ")} |`;
}

export function escapeCell(text: string): string {
  return text.replace(/\r?\n/g, " ").replaceAll("|", "\\|");
}

function unescapeCell(text: string): string {
  return text.replace(/\\\|/g, "|");
}

const wideCharacters =
  /[\u1100-\u115F\u2E80-\uA4CF\uAC00-\uD7A3\uF900-\uFAFF\uFE30-\uFE4F\uFF00-\uFF60\uFFE0-\uFFE6]/g;

export function displayWidth(text: string): number {
  return text.length + (text.match(wideCharacters)?.length ?? 0);
}
