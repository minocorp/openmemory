import { syntaxTree } from "@codemirror/language";
import type { EditorState, Line } from "@codemirror/state";
import type { SyntaxNode, Tree } from "@lezer/common";

export type PrefixMark = { name: string; from: number; to: number };

export type BlockPrefix = { from: number; to: number; marks: readonly PrefixMark[] };

const prefixMarks = new Set(["HeaderMark", "QuoteMark", "TaskMarker"]);
const codeBlocks = new Set(["FencedCode", "CodeBlock"]);
const listItems = new Set(["ListItem"]);

export function blockPrefix(state: EditorState, line: Line): BlockPrefix | null {
  const tree = syntaxTree(state);
  const indent = leadingSpace(line.text, 0);
  const marks: PrefixMark[] = [];
  let position = line.from + indent;
  for (
    let mark = prefixMarkAt(tree, line, position);
    mark;
    mark = prefixMarkAt(tree, line, position)
  ) {
    position = mark.to + leadingSpace(line.text, mark.to - line.from);
    marks.push({ name: mark.name, from: mark.from, to: position });
  }
  if (marks.length > 0) {
    return { from: line.from, to: position, marks };
  }
  const isIndentedListLine = indent > 0 && isInsideList(tree, line.from + indent);
  return isIndentedListLine ? { from: line.from, to: line.from + indent, marks } : null;
}

export function isInsideCode(state: EditorState, position: number): boolean {
  return hasAncestor(syntaxTree(state).resolveInner(position, 1), codeBlocks);
}

export function listDepth(node: SyntaxNode): number {
  let depth = 0;
  for (let parent = node.parent; parent; parent = parent.parent) {
    if (parent.name === "ListItem") {
      depth += 1;
    }
  }
  return depth;
}

function prefixMarkAt(tree: Tree, line: Line, position: number): SyntaxNode | null {
  const node = tree.resolveInner(position, 1);
  const isFollowedBySpace = /^[ \t]/.test(line.text.slice(node.to - line.from));
  if (node.from !== position || node.to === position || !isFollowedBySpace) {
    return null;
  }
  if (prefixMarks.has(node.name)) {
    return node;
  }
  const isBullet = node.name === "ListMark" && node.parent?.parent?.name === "BulletList";
  return isBullet ? node : null;
}

function isInsideList(tree: Tree, position: number): boolean {
  const node = tree.resolveInner(position, 1);
  return !hasAncestor(node, codeBlocks) && hasAncestor(node, listItems);
}

function hasAncestor(node: SyntaxNode, names: ReadonlySet<string>): boolean {
  for (let current: SyntaxNode | null = node; current; current = current.parent) {
    if (names.has(current.name)) {
      return true;
    }
  }
  return false;
}

function leadingSpace(text: string, from: number): number {
  return /^[ \t]*/.exec(text.slice(from))?.[0].length ?? 0;
}
