import { Decoration } from "@codemirror/view";
import type { SyntaxNode } from "@lezer/common";

import { safeHref } from "@/shared/lib/markdown";

import { hidden, type PreviewBuild } from "./build";
import { isMissingLink, linkTargetsFacet } from "./context";
import { touchesSelection } from "./reveal";

const wrapped = new Map([
  ["StrongEmphasis", { className: "cm-strong", mark: "EmphasisMark" }],
  ["Emphasis", { className: "cm-em", mark: "EmphasisMark" }],
  ["Strikethrough", { className: "cm-strike", mark: "StrikethroughMark" }],
  ["InlineCode", { className: "cm-inline-code", mark: "CodeMark" }],
]);

const linkParents = new Set(["Link", "Autolink", "Image"]);

export function decorateInline(build: PreviewBuild, node: SyntaxNode) {
  const wrapper = wrapped.get(node.name);
  if (wrapper) {
    decorateWrapped(build, node, wrapper);
    return;
  }
  switch (node.name) {
    case "Link":
      decorateLink(build, node);
      break;
    case "Autolink":
      decorateAutolink(build, node);
      break;
    case "URL":
      decorateBareUrl(build, node);
      break;
    case "WikiLink":
      decorateWikiLink(build, node);
      break;
    case "Escape":
      if (!isRevealed(build, node)) {
        build.decorations.push(hidden.range(node.from, node.from + 1));
      }
      break;
    default:
      break;
  }
}

function isRevealed(build: PreviewBuild, node: SyntaxNode): boolean {
  return touchesSelection(build.state, node.from, node.to);
}

function decorateWrapped(
  build: PreviewBuild,
  node: SyntaxNode,
  wrapper: { className: string; mark: string },
) {
  build.decorations.push(Decoration.mark({ class: wrapper.className }).range(node.from, node.to));
  if (isRevealed(build, node)) {
    return;
  }
  for (const mark of node.getChildren(wrapper.mark)) {
    build.decorations.push(hidden.range(mark.from, mark.to));
  }
}

function linkMark(href: string | null, revealed: boolean): Decoration {
  const attributes: Record<string, string> = href && !revealed ? { "data-href": href } : {};
  return Decoration.mark({ class: "cm-link", attributes });
}

function decorateLink(build: PreviewBuild, node: SyntaxNode) {
  const [opening, closing] = node.getChildren("LinkMark");
  if (!opening || !closing || closing.from <= opening.to) {
    return;
  }
  const url = node.getChild("URL");
  const href = url ? safeHref(build.state.sliceDoc(url.from, url.to)) : null;
  const revealed = isRevealed(build, node);
  build.decorations.push(linkMark(href, revealed).range(opening.to, closing.from));
  if (!revealed) {
    build.decorations.push(
      hidden.range(opening.from, opening.to),
      hidden.range(closing.from, node.to),
    );
  }
}

function decorateAutolink(build: PreviewBuild, node: SyntaxNode) {
  const url = node.getChild("URL");
  if (!url) {
    return;
  }
  const revealed = isRevealed(build, node);
  const href = safeHref(build.state.sliceDoc(url.from, url.to));
  build.decorations.push(linkMark(href, revealed).range(url.from, url.to));
  if (!revealed) {
    build.decorations.push(hidden.range(node.from, url.from), hidden.range(url.to, node.to));
  }
}

function decorateBareUrl(build: PreviewBuild, node: SyntaxNode) {
  if (node.parent && linkParents.has(node.parent.name)) {
    return;
  }
  const href = safeHref(build.state.sliceDoc(node.from, node.to));
  build.decorations.push(linkMark(href, isRevealed(build, node)).range(node.from, node.to));
}

function decorateWikiLink(build: PreviewBuild, node: SyntaxNode) {
  const [opening, closing] = node.getChildren("WikiLinkMark");
  if (!opening || !closing) {
    return;
  }
  const path = build.state.sliceDoc(opening.to, closing.from);
  const isMissing = isMissingLink(build.state.facet(linkTargetsFacet), path);
  const revealed = isRevealed(build, node);
  build.decorations.push(
    Decoration.mark({
      class: isMissing ? "cm-wikilink cm-wikilink-missing" : "cm-wikilink",
      attributes: revealed ? {} : { "data-wikilink": path },
    }).range(opening.to, closing.from),
  );
  if (!revealed) {
    build.decorations.push(
      hidden.range(opening.from, opening.to),
      hidden.range(closing.from, closing.to),
    );
  }
}
