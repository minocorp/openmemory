import { syntaxTree } from "@codemirror/language";
import { type EditorState, StateField, type Transaction } from "@codemirror/state";
import { Decoration, type DecorationSet, EditorView } from "@codemirror/view";

import { changedLinesFacet, linkTargetsFacet } from "./context";
import { readTableNode, tableNodes } from "./table-source";
import { TableWidget } from "./table-widget";

function buildTables(state: EditorState): DecorationSet {
  const changed = state.facet(changedLinesFacet);
  const targets = state.facet(linkTargetsFacet);
  const ranges = tableNodes(state).flatMap((node) => {
    const table = readTableNode(state, node);
    if (!table) {
      return [];
    }
    const changedRows = table.rows.map((row) => changed.has(state.sliceDoc(row.from, row.to)));
    const widget = new TableWidget({ table, changedRows, isEditable: !state.readOnly, targets });
    return [Decoration.replace({ widget, block: true }).range(table.from, table.to)];
  });
  return Decoration.set(ranges);
}

function needsRebuild(transaction: Transaction): boolean {
  const before = transaction.startState;
  const after = transaction.state;
  return (
    transaction.docChanged ||
    syntaxTree(before) !== syntaxTree(after) ||
    before.readOnly !== after.readOnly ||
    before.facet(changedLinesFacet) !== after.facet(changedLinesFacet) ||
    before.facet(linkTargetsFacet) !== after.facet(linkTargetsFacet)
  );
}

export const tableField = StateField.define<DecorationSet>({
  create: buildTables,
  update: (tables, transaction) =>
    needsRebuild(transaction) ? buildTables(transaction.state) : tables,
  provide: (field) => EditorView.decorations.from(field),
});
