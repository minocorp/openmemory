import { Decoration } from "@codemirror/view";
import type { SyntaxNode } from "@lezer/common";

import { isInsideCode, listDepth } from "./block-prefix";
import { addLineClass, hideAtomic, type PreviewBuild } from "./build";
import { touchesSelection } from "./reveal";
import { CodeHeaderWidget, RuleWidget } from "./widgets";

const rule = Decoration.replace({ widget: new RuleWidget() });
const listNumber = Decoration.mark({ class: "cm-list-number" });
const taskDone = Decoration.mark({ class: "cm-task-done" });
const headingPattern = /^ATXHeading(\d)$/;

export function decorateBlock(build: PreviewBuild, node: SyntaxNode) {
  const level = headingPattern.exec(node.name)?.[1];
  if (level) {
    addLineClass(build, node.from, `cm-heading cm-heading-${level}`);
    return;
  }
  switch (node.name) {
    case "Blockquote":
      eachLine(build, node, (from) => addLineClass(build, from, "cm-quote"));
      break;
    case "ListItem":
      decorateListItem(build, node);
      break;
    case "ListMark":
      decorateOrderedMark(build, node);
      break;
    case "Task":
      decorateTask(build, node);
      break;
    case "HorizontalRule":
      build.decorations.push(rule.range(node.from, node.to));
      build.atomic.push(rule.range(node.from, node.to));
      break;
    case "FencedCode":
      decorateFencedCode(build, node);
      break;
    case "CodeBlock":
      decorateCodeLines(build, node);
      break;
    default:
      break;
  }
}

function eachLine(
  build: PreviewBuild,
  node: SyntaxNode,
  visit: (from: number, to: number) => void,
) {
  const { doc } = build.state;
  const last = doc.lineAt(node.to).number;
  for (let number = doc.lineAt(node.from).number; number <= last; number += 1) {
    const line = doc.line(number);
    visit(line.from, line.to);
  }
}

function decorateListItem(build: PreviewBuild, node: SyntaxNode) {
  const depth = listDepth(node);
  const firstLine = build.state.doc.lineAt(node.from).from;
  eachLine(build, node, (from, to) => {
    if (from === to || isInsideCode(build.state, to)) {
      return;
    }
    const className = from === firstLine ? "cm-list-item" : "cm-list-continue";
    build.listLines.set(from, { className, depth });
  });
}

function decorateOrderedMark(build: PreviewBuild, node: SyntaxNode) {
  if (node.parent?.parent?.name !== "OrderedList") {
    return;
  }
  const end = build.state.sliceDoc(node.to, node.to + 1) === " " ? node.to + 1 : node.to;
  build.decorations.push(listNumber.range(node.from, end));
}

function decorateTask(build: PreviewBuild, node: SyntaxNode) {
  const marker = node.getChild("TaskMarker");
  if (!marker) {
    return;
  }
  const isChecked = build.state.sliceDoc(marker.from, marker.to).toLowerCase() === "[x]";
  const lineEnd = build.state.doc.lineAt(marker.to).to;
  if (isChecked && lineEnd > marker.to) {
    build.decorations.push(taskDone.range(marker.to, lineEnd));
  }
}

function decorateCodeLines(build: PreviewBuild, node: SyntaxNode) {
  const { doc } = build.state;
  const first = doc.lineAt(node.from).number;
  const last = doc.lineAt(node.to).number;
  eachLine(build, node, (from) => {
    const number = doc.lineAt(from).number;
    const edges = [number === first ? "cm-code-first" : "", number === last ? "cm-code-last" : ""];
    addLineClass(build, from, ["cm-code-line", ...edges].filter(Boolean).join(" "));
  });
}

function decorateFencedCode(build: PreviewBuild, node: SyntaxNode) {
  decorateCodeLines(build, node);
  const { state } = build;
  if (touchesSelection(state, node.from, node.to)) {
    return;
  }
  const first = state.doc.lineAt(node.from);
  const last = state.doc.lineAt(node.to);
  const info = node.getChild("CodeInfo");
  const text = node.getChild("CodeText");
  const widget = new CodeHeaderWidget(
    info ? state.sliceDoc(info.from, info.to) : "",
    text ? state.sliceDoc(text.from, text.to) : "",
  );
  const header = Decoration.replace({ widget });
  build.decorations.push(header.range(first.from, first.to));
  build.atomic.push(header.range(first.from, first.to));
  addLineClass(build, first.from, "cm-code-header-line");
  const isClosed = node.getChildren("CodeMark").length === 2;
  if (isClosed && last.number > first.number) {
    hideAtomic(build, last.from, last.to);
    addLineClass(build, last.from, "cm-code-end");
  }
}
