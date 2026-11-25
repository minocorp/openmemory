import {
  type Completion,
  type CompletionContext,
  type CompletionResult,
  startCompletion,
} from "@codemirror/autocomplete";
import type { EditorView } from "@codemirror/view";

import { emptyTable } from "@/shared/lib/markdown";

import { linkTargetsFacet } from "./preview/context";
import { focusCell } from "./preview/table-focus";

const slashPattern = /(?:^|\s)\/[\w-]*$/;
const wikiPattern = /\[\[[^\]\n]*$/;
const tableColumns = 3;
const tableBodyRows = 2;

type BlockInsert = (view: EditorView, slashFrom: number, to: number) => void;

type SlashCommand = { label: string; detail: string; insert: BlockInsert };

function prefixLine(prefix: string): BlockInsert {
  return (view, slashFrom, to) => {
    const line = view.state.doc.lineAt(slashFrom);
    view.dispatch({
      changes: [
        { from: line.from, insert: prefix },
        { from: slashFrom, to },
      ],
    });
  };
}

function placeBlock(view: EditorView, slashFrom: number, to: number, block: string): number {
  const { state } = view;
  const line = state.doc.lineAt(slashFrom);
  const rest = state.sliceDoc(line.from, slashFrom) + state.sliceDoc(to, line.to);
  const previous = line.number > 1 ? state.doc.line(line.number - 1).text : "";
  if (rest.trim() === "") {
    const gap = previous.trim() === "" ? "" : "\n";
    view.dispatch({ changes: { from: line.from, to: line.to, insert: gap + block } });
    return line.from + gap.length;
  }
  view.dispatch({
    changes: [
      { from: slashFrom, to },
      { from: line.to, insert: `\n\n${block}` },
    ],
  });
  return line.to - (to - slashFrom) + 2;
}

const insertCode: BlockInsert = (view, slashFrom, to) => {
  const start = placeBlock(view, slashFrom, to, "```\n\n```");
  view.dispatch({ selection: { anchor: start + 4 } });
};

const insertDivider: BlockInsert = (view, slashFrom, to) => {
  const start = placeBlock(view, slashFrom, to, "---\n");
  view.dispatch({ selection: { anchor: start + 4 } });
};

const insertTable: BlockInsert = (view, slashFrom, to) => {
  const start = placeBlock(view, slashFrom, to, emptyTable(tableColumns, tableBodyRows));
  focusCell(view, start, { row: 0, column: 0 }, "end");
};

const blockCommands: SlashCommand[] = [
  { label: "Heading 1", detail: "#", insert: prefixLine("# ") },
  { label: "Heading 2", detail: "##", insert: prefixLine("## ") },
  { label: "Heading 3", detail: "###", insert: prefixLine("### ") },
  { label: "Bulleted list", detail: "-", insert: prefixLine("- ") },
  { label: "Numbered list", detail: "1.", insert: prefixLine("1. ") },
  { label: "Checklist", detail: "- [ ]", insert: prefixLine("- [ ] ") },
  { label: "Quote", detail: ">", insert: prefixLine("> ") },
  { label: "Table", detail: "| |", insert: insertTable },
  { label: "Code block", detail: "```", insert: insertCode },
  { label: "Divider", detail: "---", insert: insertDivider },
];

function blockCompletion(command: SlashCommand, index: number): Completion {
  return {
    label: command.label,
    boost: -index,
    detail: command.detail,
    apply: (view, _completion, from, to) => command.insert(view, from - 1, to),
  };
}

const linkCompletion: Completion = {
  label: "Link to page or file",
  boost: -blockCommands.length,
  detail: "[[ ]]",
  apply: (view: EditorView, _completion, from, to) => {
    view.dispatch({
      changes: { from: from - 1, to, insert: "[[" },
      selection: { anchor: from + 1 },
    });
    startCompletion(view);
  },
};

export function slashSource(context: CompletionContext): CompletionResult | null {
  const match = context.matchBefore(slashPattern);
  if (!match) {
    return null;
  }
  const slash = match.text.lastIndexOf("/");
  return {
    from: match.from + slash + 1,
    options: [...blockCommands.map(blockCompletion), linkCompletion],
    validFor: /^[\w-]*$/,
  };
}

export function wikiSource(context: CompletionContext): CompletionResult | null {
  const match = context.matchBefore(wikiPattern);
  if (!match) {
    return null;
  }
  const targets = context.state.facet(linkTargetsFacet)?.targets ?? [];
  return {
    from: match.from + 2,
    options: targets.map((target) => ({
      label: target.path,
      detail: target.kind,
      apply: (view, _completion, from, to) => {
        const hasClosing = view.state.doc.sliceString(to, to + 2) === "]]";
        view.dispatch({
          changes: { from, to, insert: hasClosing ? target.path : `${target.path}]]` },
          selection: { anchor: from + target.path.length + 2 },
        });
      },
    })),
    validFor: /^[^\]\n]*$/,
  };
}
