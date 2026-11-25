import { GFM, parser } from "@lezer/markdown";

import { WikiLink } from "./wiki-link";

export const markdownExtensions = [GFM, WikiLink];

const blockParsers = [
  "LinkReference",
  "IndentedCode",
  "FencedCode",
  "Blockquote",
  "HorizontalRule",
  "BulletList",
  "OrderedList",
  "ATXHeading",
  "HTMLBlock",
  "SetextHeading",
  "Table",
];

export const inlineParser = parser.configure([...markdownExtensions, { remove: blockParsers }]);
