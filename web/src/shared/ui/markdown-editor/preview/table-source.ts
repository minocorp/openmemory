import { syntaxTree } from "@codemirror/language";
import type { EditorState } from "@codemirror/state";
import type { SyntaxNode } from "@lezer/common";

import { readTable, type SourceLine, type TableModel } from "@/shared/lib/markdown";

export function tableNodes(state: EditorState): SyntaxNode[] {
  const found: SyntaxNode[] = [];
  for (let child = syntaxTree(state).topNode.firstChild; child; child = child.nextSibling) {
    if (child.name === "Table") {
      found.push(child);
    }
  }
  return found;
}

export function readTableNode(state: EditorState, node: SyntaxNode): TableModel | null {
  const { doc } = state;
  const first = doc.lineAt(node.from).number;
  const last = doc.lineAt(node.to).number;
  const lines: SourceLine[] = Array.from({ length: last - first + 1 }, (_, index) => {
    const line = doc.line(first + index);
    return { text: line.text, from: line.from };
  });
  return readTable(lines);
}

export function tableAt(state: EditorState, position: number): TableModel | null {
  const node = tableNodes(state).find((table) => table.from <= position && position <= table.to);
  return node ? readTableNode(state, node) : null;
}
