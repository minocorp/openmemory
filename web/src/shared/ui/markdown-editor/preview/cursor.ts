import { EditorState, type TransactionSpec } from "@codemirror/state";

import { blockPrefix } from "./block-prefix";
import { tableNodes } from "./table-source";

function aroundTable(state: EditorState, head: number, previous: number): TransactionSpec | null {
  const table = tableNodes(state).find((node) => node.from <= head && head <= node.to);
  if (!table) {
    return null;
  }
  const { length } = state.doc;
  if (previous < head || head === table.from) {
    if (table.from > 0) {
      return { selection: { anchor: table.from - 1 } };
    }
    return { changes: { from: 0, insert: "\n" }, selection: { anchor: 0 } };
  }
  if (table.to < length) {
    return { selection: { anchor: table.to + 1 } };
  }
  return { changes: { from: length, insert: "\n" }, selection: { anchor: length + 1 } };
}

function outOfPrefix(state: EditorState, head: number, previous: number): TransactionSpec | null {
  const line = state.doc.lineAt(head);
  const prefix = blockPrefix(state, line);
  if (!prefix || head >= prefix.to) {
    return null;
  }
  const isLeavingLine = previous === prefix.to && line.number > 1;
  return { selection: { anchor: isLeavingLine ? line.from - 1 : prefix.to } };
}

export const cursorFilter = EditorState.transactionFilter.of((transaction) => {
  const { selection } = transaction;
  if (!selection || selection.ranges.length > 1 || !selection.main.empty) {
    return transaction;
  }
  const { state } = transaction;
  const head = selection.main.head;
  const previous = transaction.docChanged ? head : transaction.startState.selection.main.head;
  const adjusted = aroundTable(state, head, previous) ?? outOfPrefix(state, head, previous);
  return adjusted ? [transaction, { ...adjusted, sequential: true }] : transaction;
});
