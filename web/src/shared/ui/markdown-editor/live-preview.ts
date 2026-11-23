import { syntaxTree } from "@codemirror/language";
import { type EditorState, type Range, StateEffect, StateField } from "@codemirror/state";
import {
  Decoration,
  type DecorationSet,
  EditorView,
  ViewPlugin,
  type ViewUpdate,
  WidgetType,
} from "@codemirror/view";
import type { SyntaxNode } from "@lezer/common";

const hidden = Decoration.replace({});
const wikilinkPattern = /\[\[([^\]\n]+)\]\]/g;
const taskMarkerPattern = /^\[( |x|X)\]$/;

class BulletWidget extends WidgetType {
  override eq(): boolean {
    return true;
  }

  override toDOM(): HTMLElement {
    const bullet = document.createElement("span");
    bullet.className = "cm-bullet";
    bullet.textContent = "•";
    return bullet;
  }
}

class CheckboxWidget extends WidgetType {
  readonly isChecked: boolean;

  constructor(isChecked: boolean) {
    super();
    this.isChecked = isChecked;
  }

  override eq(other: CheckboxWidget): boolean {
    return other.isChecked === this.isChecked;
  }

  override toDOM(): HTMLElement {
    const box = document.createElement("input");
    box.type = "checkbox";
    box.checked = this.isChecked;
    box.className = "cm-task-checkbox";
    box.setAttribute("aria-label", this.isChecked ? "Checked item" : "Unchecked item");
    return box;
  }

  override ignoreEvent(): boolean {
    return false;
  }
}

class RuleWidget extends WidgetType {
  override eq(): boolean {
    return true;
  }

  override toDOM(): HTMLElement {
    const rule = document.createElement("span");
    rule.className = "cm-rule";
    return rule;
  }
}

const bullet = Decoration.replace({ widget: new BulletWidget() });
const rule = Decoration.replace({ widget: new RuleWidget() });

const setFocus = StateEffect.define<boolean>();

const focusField = StateField.define<boolean>({
  create: () => false,
  update: (isFocused, transaction) =>
    transaction.effects.reduce(
      (current, effect) => (effect.is(setFocus) ? effect.value : current),
      isFocused,
    ),
});

function touchesSelection(state: EditorState, from: number, to: number): boolean {
  return (
    state.field(focusField) &&
    state.selection.ranges.some((range) => range.from <= to && range.to >= from)
  );
}

function touchesLine(state: EditorState, position: number): boolean {
  const line = state.doc.lineAt(position);
  return touchesSelection(state, line.from, line.to);
}

function markWithAfterSpace(state: EditorState, from: number, to: number): Range<Decoration> {
  const nextCharacter = state.doc.sliceString(to, to + 1);
  return hidden.range(from, nextCharacter === " " ? to + 1 : to);
}

function decorateHeadingMark(state: EditorState, node: SyntaxNode, found: Range<Decoration>[]) {
  if (!touchesLine(state, node.from)) {
    found.push(markWithAfterSpace(state, node.from, node.to));
  }
}

function decorateInline(
  state: EditorState,
  node: SyntaxNode,
  className: string,
  found: Range<Decoration>[],
) {
  found.push(Decoration.mark({ class: className }).range(node.from, node.to));
  if (touchesSelection(state, node.from, node.to)) {
    return;
  }
  for (const mark of node.getChildren("EmphasisMark")) {
    found.push(hidden.range(mark.from, mark.to));
  }
  for (const mark of node.getChildren("StrikethroughMark")) {
    found.push(hidden.range(mark.from, mark.to));
  }
}

function decorateInlineCode(state: EditorState, node: SyntaxNode, found: Range<Decoration>[]) {
  found.push(Decoration.mark({ class: "cm-inline-code" }).range(node.from, node.to));
  if (touchesSelection(state, node.from, node.to)) {
    return;
  }
  for (const mark of node.getChildren("CodeMark")) {
    found.push(hidden.range(mark.from, mark.to));
  }
}

function decorateLink(state: EditorState, node: SyntaxNode, found: Range<Decoration>[]) {
  const marks = node.getChildren("LinkMark");
  const opening = marks.at(0);
  const closing = marks.at(1);
  if (!opening || !closing) {
    return;
  }
  found.push(Decoration.mark({ class: "cm-link" }).range(opening.to, closing.from));
  if (!touchesSelection(state, node.from, node.to)) {
    found.push(hidden.range(opening.from, opening.to), hidden.range(closing.from, node.to));
  }
}

function decorateQuote(state: EditorState, node: SyntaxNode, found: Range<Decoration>[]) {
  const first = state.doc.lineAt(node.from).number;
  const last = state.doc.lineAt(node.to).number;
  for (let number = first; number <= last; number += 1) {
    found.push(Decoration.line({ class: "cm-quote" }).range(state.doc.line(number).from));
  }
}

function decorateQuoteMark(state: EditorState, node: SyntaxNode, found: Range<Decoration>[]) {
  if (!touchesLine(state, node.from)) {
    found.push(markWithAfterSpace(state, node.from, node.to));
  }
}

function decorateListMark(state: EditorState, node: SyntaxNode, found: Range<Decoration>[]) {
  const item = node.parent;
  const isBullet = item?.parent?.name === "BulletList";
  if (!item || !isBullet || touchesLine(state, node.from)) {
    return;
  }
  const isTask = item.getChild("Task") !== null;
  const range = markWithAfterSpace(state, node.from, node.to);
  found.push(isTask ? range : bullet.range(node.from, node.to));
}

function decorateTask(state: EditorState, node: SyntaxNode, found: Range<Decoration>[]) {
  const marker = node.getChild("TaskMarker");
  if (!marker) {
    return;
  }
  const text = state.doc.sliceString(marker.from, marker.to);
  const isChecked = text.toLowerCase() === "[x]";
  if (isChecked) {
    const line = state.doc.lineAt(marker.to);
    found.push(Decoration.mark({ class: "cm-task-done" }).range(marker.to, line.to));
  }
  if (taskMarkerPattern.test(text) && !touchesSelection(state, marker.from, marker.to)) {
    found.push(
      Decoration.replace({ widget: new CheckboxWidget(isChecked) }).range(marker.from, marker.to),
    );
  }
}

function decorateFencedCode(state: EditorState, node: SyntaxNode, found: Range<Decoration>[]) {
  const first = state.doc.lineAt(node.from).number;
  const last = state.doc.lineAt(node.to).number;
  for (let number = first; number <= last; number += 1) {
    found.push(Decoration.line({ class: "cm-code-line" }).range(state.doc.line(number).from));
  }
  if (touchesSelection(state, node.from, node.to)) {
    return;
  }
  for (const part of [...node.getChildren("CodeMark"), ...node.getChildren("CodeInfo")]) {
    found.push(hidden.range(part.from, part.to));
  }
}

function decorateWikilinks(
  state: EditorState,
  range: { from: number; to: number },
  found: Range<Decoration>[],
) {
  const text = state.doc.sliceString(range.from, range.to);
  for (const match of text.matchAll(wikilinkPattern)) {
    const path = match[1];
    if (path === undefined) {
      continue;
    }
    const from = range.from + match.index;
    const to = from + match[0].length;
    found.push(
      Decoration.mark({ class: "cm-wikilink", attributes: { "data-wikilink": path } }).range(
        from + 2,
        to - 2,
      ),
    );
    if (!touchesSelection(state, from, to)) {
      found.push(hidden.range(from, from + 2), hidden.range(to - 2, to));
    }
  }
}

function buildDecorations(view: EditorView): DecorationSet {
  const { state } = view;
  const found: Range<Decoration>[] = [];
  for (const visible of view.visibleRanges) {
    decorateWikilinks(state, visible, found);
    syntaxTree(state).iterate({
      from: visible.from,
      to: visible.to,
      enter: (reference) => {
        const node = reference.node;
        const heading = /^(?:ATX|Setext)Heading(\d)$/.exec(node.name);
        if (heading?.[1]) {
          found.push(
            Decoration.line({ class: `cm-heading cm-heading-${heading[1]}` }).range(
              state.doc.lineAt(node.from).from,
            ),
          );
        }
        switch (node.name) {
          case "HeaderMark":
            if (node.parent?.name.startsWith("ATX")) {
              decorateHeadingMark(state, node, found);
            }
            break;
          case "StrongEmphasis":
            decorateInline(state, node, "cm-strong", found);
            break;
          case "Emphasis":
            decorateInline(state, node, "cm-em", found);
            break;
          case "Strikethrough":
            decorateInline(state, node, "cm-strike", found);
            break;
          case "InlineCode":
            decorateInlineCode(state, node, found);
            break;
          case "Link":
            decorateLink(state, node, found);
            break;
          case "Blockquote":
            decorateQuote(state, node, found);
            break;
          case "QuoteMark":
            decorateQuoteMark(state, node, found);
            break;
          case "ListMark":
            decorateListMark(state, node, found);
            break;
          case "Task":
            decorateTask(state, node, found);
            break;
          case "FencedCode":
            decorateFencedCode(state, node, found);
            break;
          case "HorizontalRule":
            if (!touchesLine(state, node.from)) {
              found.push(rule.range(node.from, node.to));
            }
            break;
          default:
            break;
        }
      },
    });
  }
  return Decoration.set(found, true);
}

const previewPlugin = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;

    constructor(view: EditorView) {
      this.decorations = buildDecorations(view);
    }

    update(update: ViewUpdate) {
      if (
        update.docChanged ||
        update.selectionSet ||
        update.viewportChanged ||
        update.focusChanged
      ) {
        this.decorations = buildDecorations(update.view);
      }
    }
  },
  { decorations: (plugin) => plugin.decorations },
);

export const livePreview = [
  focusField,
  EditorView.focusChangeEffect.of((_state, isFocusing) => setFocus.of(isFocusing)),
  previewPlugin,
];

export function toggleTaskAt(view: EditorView, position: number): boolean {
  const text = view.state.doc.sliceString(position, position + 3);
  if (!taskMarkerPattern.test(text)) {
    return false;
  }
  const next = text === "[ ]" ? "[x]" : "[ ]";
  view.dispatch({ changes: { from: position, to: position + 3, insert: next } });
  return true;
}
