import { describe, expect, it } from "vitest";

import {
  applyTableEdit,
  editCell,
  emptyTable,
  readTable,
  type SourceLine,
  type TableModel,
  type TextChange,
} from "./table";

function lines(text: string): SourceLine[] {
  return text.split("\n").reduce<SourceLine[]>((found, line) => {
    const last = found.at(-1);
    const from = last ? last.from + last.text.length + 1 : 0;
    return [...found, { text: line, from }];
  }, []);
}

function tableOf(text: string): TableModel {
  const table = readTable(lines(text));
  if (!table) {
    throw new Error("not a table");
  }
  return table;
}

function apply(text: string, change: TextChange | null): string {
  if (!change) {
    return text;
  }
  return text.slice(0, change.from) + change.insert + text.slice(change.to);
}

const source = ["| Name | Fee |", "| :--- | ---: |", "| acme | 450000 |", "| solo |"].join("\n");

describe("readTable", () => {
  it("reads alignment, header and rows", () => {
    const table = tableOf(source);
    expect(table.aligns).toEqual(["left", "right"]);
    expect(table.rows.map((row) => row.cells.map((cell) => cell?.text ?? null))).toEqual([
      ["Name", "Fee"],
      ["acme", "450000"],
      ["solo", null],
    ]);
  });

  it("reads rows without outer pipes and escaped pipes", () => {
    const table = tableOf("a | b\n--- | ---\nx \\| y | z");
    expect(table.rows[1]?.cells.map((cell) => cell?.text)).toEqual(["x | y", "z"]);
  });
});

describe("editCell", () => {
  it("replaces only the cell content, escapes pipes and keeps the column aligned", () => {
    expect(apply(source, editCell(tableOf(source), { row: 1, column: 1 }, "1|2"))).toBe(
      source.replace("| 450000 |", "| 1\\|2   |"),
    );
  });

  it("eats padding when the text grows and keeps one space", () => {
    const text = "| Name | Fee    |\n| ---- | ------ |\n|      | 1      |";
    expect(apply(text, editCell(tableOf(text), { row: 1, column: 0 }, "solo"))).toBe(
      "| Name | Fee    |\n| ---- | ------ |\n| solo | 1      |",
    );
    expect(apply(text, editCell(tableOf(text), { row: 1, column: 0 }, "longer"))).toBe(
      "| Name | Fee    |\n| ---- | ------ |\n| longer | 1      |",
    );
  });

  it("keeps compact tables compact", () => {
    const text = "|a|b|\n|-|-|\n|c|d|";
    expect(apply(text, editCell(tableOf(text), { row: 1, column: 0 }, "x"))).toBe(
      "|a|b|\n|-|-|\n|x|d|",
    );
  });

  it("fills an empty cell in place", () => {
    const text = "| a | b |\n| --- | --- |\n|  | x |";
    expect(apply(text, editCell(tableOf(text), { row: 1, column: 0 }, "new"))).toBe(
      "| a | b |\n| --- | --- |\n| new | x |",
    );
  });

  it("rewrites a short row to fill a missing cell", () => {
    expect(apply(source, editCell(tableOf(source), { row: 2, column: 1 }, "0"))).toBe(
      source.replace("| solo |", "| solo | 0 |"),
    );
  });
});

describe("applyTableEdit", () => {
  it("inserts a row and aligns every column", () => {
    expect(apply(source, applyTableEdit(tableOf(source), { kind: "insertRow", at: 1 }))).toBe(
      [
        "| Name | Fee    |",
        "| :--- | -----: |",
        "|      |        |",
        "| acme | 450000 |",
        "| solo |        |",
      ].join("\n"),
    );
  });

  it("deletes a column with its alignment", () => {
    expect(apply(source, applyTableEdit(tableOf(source), { kind: "deleteColumn", at: 0 }))).toBe(
      ["| Fee    |", "| -----: |", "| 450000 |", "|        |"].join("\n"),
    );
  });

  it("changes the alignment of one column", () => {
    const edit = { kind: "align", column: 0, align: "center" } as const;
    expect(apply(source, applyTableEdit(tableOf(source), edit))).toContain("| :--: | -----: |");
  });

  it("pads wide characters by display width", () => {
    const text = "| 이름 |\n| --- |\n| ab |";
    expect(apply(text, applyTableEdit(tableOf(text), { kind: "insertRow", at: 2 }))).toBe(
      ["| 이름 |", "| ---- |", "| ab   |", "|      |"].join("\n"),
    );
  });
});

describe("emptyTable", () => {
  it("builds a table the parser accepts", () => {
    expect(tableOf(emptyTable(2, 1)).rows).toHaveLength(2);
  });
});
