import { type InlineNode, parseInline } from "@/shared/lib/markdown";

import { isMissingLink, type LinkIndex } from "./context";

const containers = {
  strong: { tag: "strong", className: "cm-strong" },
  emphasis: { tag: "em", className: "cm-em" },
  strike: { tag: "s", className: "cm-strike" },
} as const;

export function inlineDom(nodes: readonly InlineNode[], targets: LinkIndex | null): Node[] {
  return nodes.map((node) => nodeDom(node, targets));
}

export function renderCell(element: HTMLElement, text: string, targets: LinkIndex | null) {
  element.dataset["text"] = text;
  element.replaceChildren(...inlineDom(parseInline(text), targets));
}

function nodeDom(node: InlineNode, targets: LinkIndex | null): Node {
  switch (node.kind) {
    case "text":
      return document.createTextNode(node.text);
    case "code":
      return createElement("code", "cm-inline-code", [document.createTextNode(node.text)]);
    case "strong":
    case "emphasis":
    case "strike": {
      const { tag, className } = containers[node.kind];
      return createElement(tag, className, inlineDom(node.children, targets));
    }
    case "link": {
      const link = createElement("span", "cm-link", inlineDom(node.children, targets));
      link.dataset["href"] = node.href;
      return link;
    }
    default: {
      const className = isMissingLink(targets, node.path)
        ? "cm-wikilink cm-wikilink-missing"
        : "cm-wikilink";
      const link = createElement("span", className, [document.createTextNode(node.path)]);
      link.dataset["wikilink"] = node.path;
      return link;
    }
  }
}

function createElement(tag: string, className: string, children: readonly Node[]): HTMLElement {
  const created = document.createElement(tag);
  created.className = className;
  created.append(...children);
  return created;
}
