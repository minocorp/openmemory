import type { MarkdownConfig } from "@lezer/markdown";

const openBracket = 91;
const backslash = 92;
const closing = "]]";

export const WikiLink: MarkdownConfig = {
  defineNodes: ["WikiLink", "WikiLinkMark"],
  parseInline: [
    {
      name: "WikiLink",
      before: "Link",
      parse(cx, next, pos) {
        if (next !== openBracket || cx.char(pos + 1) !== openBracket) {
          return -1;
        }
        const previous = cx.char(pos - 1);
        if (previous === openBracket || previous === backslash) {
          return -1;
        }
        const innerFrom = pos + 2;
        const close = cx.slice(innerFrom, cx.end).indexOf(closing);
        if (close <= 0) {
          return -1;
        }
        const inner = cx.slice(innerFrom, innerFrom + close);
        if (/[\n[]/.test(inner)) {
          return -1;
        }
        const innerTo = innerFrom + close;
        return cx.addElement(
          cx.elt("WikiLink", pos, innerTo + 2, [
            cx.elt("WikiLinkMark", pos, innerFrom),
            cx.elt("WikiLinkMark", innerTo, innerTo + 2),
          ]),
        );
      },
    },
  ],
};
