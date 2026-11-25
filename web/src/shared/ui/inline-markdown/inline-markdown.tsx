import type { ReactNode } from "react";

import { cn } from "@/shared/lib/cn";
import { type InlineNode, parseInline } from "@/shared/lib/markdown";

type InlineMarkdownProps = {
  text: string;
  onOpenLink: (path: string) => void;
};

const linkStyle =
  "underline decoration-foreground/35 underline-offset-3 transition-colors hover:decoration-foreground";

export function InlineMarkdown({ text, onOpenLink }: InlineMarkdownProps) {
  return <>{renderNodes(parseInline(text), onOpenLink)}</>;
}

function renderNodes(
  nodes: readonly InlineNode[],
  onOpenLink: (path: string) => void,
): ReactNode[] {
  return nodes.map((node, index) => {
    const key = `${index}-${node.kind}`;
    switch (node.kind) {
      case "text":
        return node.text;
      case "code":
        return (
          <code key={key} className="rounded-sm border bg-muted px-1 font-mono text-sm">
            {node.text}
          </code>
        );
      case "strong":
        return (
          <strong key={key} className="font-semibold">
            {renderNodes(node.children, onOpenLink)}
          </strong>
        );
      case "emphasis":
        return <em key={key}>{renderNodes(node.children, onOpenLink)}</em>;
      case "strike":
        return (
          <s key={key} className="text-muted-foreground">
            {renderNodes(node.children, onOpenLink)}
          </s>
        );
      case "link":
        return (
          <a
            key={key}
            href={node.href}
            target="_blank"
            rel="noopener noreferrer"
            className={linkStyle}
          >
            {renderNodes(node.children, onOpenLink)}
          </a>
        );
      default:
        return (
          <button
            key={key}
            type="button"
            className={cn(linkStyle, "cursor-pointer")}
            onClick={() => onOpenLink(node.path)}
          >
            {node.path}
          </button>
        );
    }
  });
}
