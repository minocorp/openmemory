import type { SyntaxNode } from "@lezer/common";

import { inlineParser } from "./parser";
import { safeHref } from "./safe-url";

export type InlineNode =
  | { kind: "text"; text: string }
  | { kind: "code"; text: string }
  | { kind: "strong" | "emphasis" | "strike"; children: InlineNode[] }
  | { kind: "link"; href: string; children: InlineNode[] }
  | { kind: "wikilink"; path: string };

type Range = { from: number; to: number };

type Wrapper = { kind: "strong" | "emphasis" | "strike"; mark: string };

const wrappers = new Map<string, Wrapper>([
  ["StrongEmphasis", { kind: "strong", mark: "EmphasisMark" }],
  ["Emphasis", { kind: "emphasis", mark: "EmphasisMark" }],
  ["Strikethrough", { kind: "strike", mark: "StrikethroughMark" }],
]);

export function parseInline(text: string): InlineNode[] {
  const tree = inlineParser.parse(text);
  const blocks = tree.topNode.getChildren("Paragraph");
  return mergeText(
    blocks.flatMap((block, index) => [
      ...(index > 0 ? [{ kind: "text" as const, text: " " }] : []),
      ...convertChildren(text, block, block),
    ]),
  );
}

export function wikiLinkPath(text: string, node: SyntaxNode): string {
  return innerRange(node, "WikiLinkMark", text);
}

function convertChildren(text: string, parent: SyntaxNode, range: Range): InlineNode[] {
  const found: InlineNode[] = [];
  let position = range.from;
  for (let child = parent.firstChild; child; child = child.nextSibling) {
    if (child.to <= range.from || child.from >= range.to) {
      continue;
    }
    if (child.from > position) {
      found.push({ kind: "text", text: text.slice(position, child.from) });
    }
    found.push(...convertNode(text, child));
    position = child.to;
  }
  if (position < range.to) {
    found.push({ kind: "text", text: text.slice(position, range.to) });
  }
  return mergeText(found);
}

function convertNode(text: string, node: SyntaxNode): InlineNode[] {
  const raw = text.slice(node.from, node.to);
  const wrapper = wrappers.get(node.name);
  if (wrapper) {
    return [
      {
        kind: wrapper.kind,
        children: convertChildren(text, node, markedContent(node, wrapper.mark)),
      },
    ];
  }
  switch (node.name) {
    case "InlineCode":
      return [{ kind: "code", text: innerRange(node, "CodeMark", text) }];
    case "WikiLink":
      return [{ kind: "wikilink", path: wikiLinkPath(text, node) }];
    case "Link":
      return convertLink(text, node);
    case "Autolink":
    case "URL":
      return plainLink(raw.replace(/^<|>$/g, ""));
    case "Escape":
      return [{ kind: "text", text: raw.slice(1) }];
    case "HardBreak":
      return [{ kind: "text", text: " " }];
    default:
      return [{ kind: "text", text: raw }];
  }
}

function convertLink(text: string, node: SyntaxNode): InlineNode[] {
  const label = markedContent(node, "LinkMark");
  const children = convertChildren(text, node, label);
  const url = node.getChild("URL");
  const href = url ? safeHref(text.slice(url.from, url.to)) : null;
  return href ? [{ kind: "link", href, children }] : children;
}

function plainLink(raw: string): InlineNode[] {
  const href = safeHref(raw);
  const label: InlineNode = { kind: "text", text: raw };
  return href ? [{ kind: "link", href, children: [label] }] : [label];
}

function markedContent(node: SyntaxNode, markName: string): Range {
  const marks = node.getChildren(markName);
  const opening = marks.at(0);
  const closing = marks.at(1);
  return { from: opening?.to ?? node.from, to: closing?.from ?? node.to };
}

function innerRange(node: SyntaxNode, markName: string, text: string): string {
  const { from, to } = markedContent(node, markName);
  return text.slice(from, to);
}

function mergeText(nodes: InlineNode[]): InlineNode[] {
  return nodes.reduce<InlineNode[]>((merged, node) => {
    const last = merged.at(-1);
    if (last?.kind === "text" && node.kind === "text") {
      return [...merged.slice(0, -1), { kind: "text", text: last.text + node.text }];
    }
    return [...merged, node];
  }, []);
}
