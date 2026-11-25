import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { tags } from "@lezer/highlight";

const codeStyle = HighlightStyle.define([
  {
    tag: [
      tags.keyword,
      tags.controlKeyword,
      tags.operatorKeyword,
      tags.definitionKeyword,
      tags.moduleKeyword,
      tags.modifier,
    ],
    color: "var(--code-keyword)",
  },
  { tag: [tags.string, tags.special(tags.string), tags.regexp], color: "var(--code-string)" },
  {
    tag: [tags.comment, tags.lineComment, tags.blockComment, tags.docComment],
    color: "var(--code-comment)",
  },
  { tag: [tags.number, tags.bool, tags.null, tags.atom], color: "var(--code-number)" },
  {
    tag: [tags.function(tags.variableName), tags.function(tags.propertyName)],
    color: "var(--code-function)",
  },
  { tag: [tags.typeName, tags.className, tags.namespace, tags.tagName], color: "var(--code-type)" },
  { tag: [tags.propertyName, tags.attributeName], color: "var(--code-property)" },
]);

export const codeHighlighting = syntaxHighlighting(codeStyle);
