import { syntaxTree } from "@codemirror/language";
import {
  Decoration,
  type DecorationSet,
  EditorView,
  ViewPlugin,
  type ViewUpdate,
} from "@codemirror/view";

import { isInsideCode } from "./block-prefix";
import { decorateBlock } from "./blocks";
import { createBuild, flushListLines, type PreviewBuild } from "./build";
import { linkTargetsFacet } from "./context";
import { decorateInline } from "./inline";
import { decorateLinePrefix } from "./lines";
import { focusField } from "./reveal";
import { HintWidget } from "./widgets";

const commandHint = Decoration.widget({ widget: new HintWidget("Type / for commands"), side: 1 });

type Preview = { decorations: DecorationSet; atomic: DecorationSet };

function buildPreview(view: EditorView): Preview {
  const build = createBuild(view.state);
  const tree = syntaxTree(view.state);
  for (const { from, to } of view.visibleRanges) {
    tree.iterate({
      from,
      to,
      enter: (reference) => {
        if (reference.name === "Table") {
          return false;
        }
        decorateBlock(build, reference.node);
        decorateInline(build, reference.node);
        return undefined;
      },
    });
    for (let position = from; position <= to;) {
      const line = view.state.doc.lineAt(position);
      decorateLinePrefix(build, line);
      position = line.to + 1;
    }
  }
  addCommandHint(build);
  flushListLines(build);
  return {
    decorations: Decoration.set(build.decorations, true),
    atomic: Decoration.set(build.atomic, true),
  };
}

function addCommandHint(build: PreviewBuild) {
  const { state } = build;
  const { main } = state.selection;
  if (!build.isEditable || !state.field(focusField) || !main.empty || state.doc.length === 0) {
    return;
  }
  const line = state.doc.lineAt(main.head);
  if (line.length === 0 && !isInsideCode(state, line.from)) {
    build.decorations.push(commandHint.range(line.from));
  }
}

function needsRebuild(update: ViewUpdate): boolean {
  return (
    update.docChanged ||
    update.selectionSet ||
    update.viewportChanged ||
    update.focusChanged ||
    syntaxTree(update.startState) !== syntaxTree(update.state) ||
    update.startState.field(focusField) !== update.state.field(focusField) ||
    update.startState.facet(linkTargetsFacet) !== update.state.facet(linkTargetsFacet)
  );
}

export const previewPlugin = ViewPlugin.fromClass(
  class {
    preview: Preview;

    constructor(view: EditorView) {
      this.preview = buildPreview(view);
    }

    update(update: ViewUpdate) {
      if (needsRebuild(update)) {
        this.preview = buildPreview(update.view);
      }
    }
  },
  { decorations: (plugin) => plugin.preview.decorations },
);

export const previewAtomicRanges = EditorView.atomicRanges.of(
  (view) => view.plugin(previewPlugin)?.preview.atomic ?? Decoration.none,
);
