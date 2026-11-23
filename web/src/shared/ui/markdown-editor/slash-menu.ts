import {
  type Completion,
  type CompletionContext,
  type CompletionResult,
  startCompletion,
} from "@codemirror/autocomplete";
import type { EditorView } from "@codemirror/view";

import type { LinkTarget } from "./link-targets";

const slashPattern = /(?:^|\s)\/[\w-]*$/;
const wikiPattern = /\[\[[^\]\n]*$/;

type SlashCommand = { label: string; detail: string; prefix: string };

const blockCommands: SlashCommand[] = [
  { label: "Heading 1", detail: "#", prefix: "# " },
  { label: "Heading 2", detail: "##", prefix: "## " },
  { label: "Heading 3", detail: "###", prefix: "### " },
  { label: "Bulleted list", detail: "-", prefix: "- " },
  { label: "Checklist", detail: "- [ ]", prefix: "- [ ] " },
];

function blockCompletion(command: SlashCommand, index: number): Completion {
  return {
    label: command.label,
    boost: -index,
    detail: command.detail,
    apply: (view, _completion, from, to) => {
      const line = view.state.doc.lineAt(from);
      view.dispatch({
        changes: [
          { from: line.from, insert: command.prefix },
          { from: from - 1, to },
        ],
      });
    },
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

export function wikiSource(getTargets: () => readonly LinkTarget[]) {
  return (context: CompletionContext): CompletionResult | null => {
    const match = context.matchBefore(wikiPattern);
    if (!match) {
      return null;
    }
    return {
      from: match.from + 2,
      options: getTargets().map((target) => ({
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
  };
}
