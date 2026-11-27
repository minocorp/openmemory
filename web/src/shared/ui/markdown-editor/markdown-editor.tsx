import { autocompletion } from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { Compartment, EditorState } from "@codemirror/state";
import { EditorView, keymap, placeholder } from "@codemirror/view";
import { useEffect, useEffectEvent, useRef, useState } from "react";

import type { CellPosition, TableEdit } from "@/shared/lib/markdown";

import { makeLink, toggleHeading, toggleInline } from "./formatting";
import type { LinkTarget } from "./link-targets";
import { markdownPreview } from "./markdown-extensions";
import {
  changedLinesFacet,
  handlersFacet,
  linkTargetsFacet,
  type TableFocus,
} from "./preview/context";
import { deleteTable, editTable } from "./preview/table-focus";
import { SelectionToolbar, type ToolbarPosition } from "./selection-toolbar";
import { slashSource, wikiSource } from "./slash-menu";
import { TableToolbar } from "./table-toolbar";
import { textChanges } from "./text-changes";

type MarkdownEditorProps = {
  value: string;
  onChange: (value: string) => void;
  onSave: () => void;
  onOpenLink: (path: string) => void;
  linkTargets: readonly LinkTarget[];
  changedLines: ReadonlySet<string>;
  label: string;
  emptyHint?: string;
};

const toolbarOffset = 44;

type ActiveTable = { focus: TableFocus; position: ToolbarPosition };

export function MarkdownEditor({
  value,
  onChange,
  onSave,
  onOpenLink,
  linkTargets,
  changedLines,
  label,
  emptyHint = "",
}: MarkdownEditorProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const targetsCompartment = useRef(new Compartment());
  const changedCompartment = useRef(new Compartment());
  const [toolbar, setToolbar] = useState<ToolbarPosition | null>(null);
  const [activeTable, setActiveTable] = useState<ActiveTable | null>(null);

  const handleChange = useEffectEvent(onChange);
  const handleSave = useEffectEvent(onSave);
  const handleOpenLink = useEffectEvent(onOpenLink);
  const initialValue = useEffectEvent(() => value);
  const initialTargets = useEffectEvent(() => linkTargets);
  const initialChangedLines = useEffectEvent(() => changedLines);
  const appliedTargets = useRef("");
  const targetsKey = linkTargets.map((target) => `${target.kind}:${target.path}`).join("\n");

  useEffect(() => {
    const host = hostRef.current;
    if (!host) {
      return undefined;
    }
    const view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: initialValue(),
        extensions: [
          history(),
          keymap.of([
            { key: "Mod-s", preventDefault: true, run: () => (handleSave(), true) },
            ...historyKeymap,
            ...defaultKeymap,
          ]),
          markdownPreview(),
          autocompletion({ override: [slashSource, wikiSource], icons: false }),
          EditorView.contentAttributes.of({ "aria-label": label }),
          placeholder(emptyHint),
          targetsCompartment.current.of(linkTargetsFacet.of(initialTargets())),
          changedCompartment.current.of(changedLinesFacet.of(initialChangedLines())),
          handlersFacet.of({
            openLink: handleOpenLink,
            focusTable: (focus) => setActiveTable(focus ? activeTableIn(focus, host) : null),
          }),
          EditorView.updateListener.of((update) => {
            if (update.docChanged) {
              handleChange(update.state.doc.toString());
            }
            if (update.docChanged || update.selectionSet || update.focusChanged) {
              setToolbar(toolbarPosition(update.view, host));
            }
            if (update.focusChanged && update.view.hasFocus) {
              setActiveTable(null);
            }
          }),
        ],
      }),
    });
    viewRef.current = view;
    return () => {
      view.destroy();
      viewRef.current = null;
    };
  }, [label, emptyHint]);

  useEffect(() => {
    const view = viewRef.current;
    if (view && view.state.doc.toString() !== value) {
      view.dispatch({ changes: textChanges(view.state.doc.toString(), value) });
    }
  }, [value]);

  useEffect(() => {
    if (appliedTargets.current === targetsKey) {
      return;
    }
    appliedTargets.current = targetsKey;
    viewRef.current?.dispatch({
      effects: targetsCompartment.current.reconfigure(linkTargetsFacet.of(linkTargets)),
    });
  }, [targetsKey, linkTargets]);

  useEffect(() => {
    viewRef.current?.dispatch({
      effects: changedCompartment.current.reconfigure(changedLinesFacet.of(changedLines)),
    });
  }, [changedLines]);

  const withView = (action: (view: EditorView) => void) => {
    const view = viewRef.current;
    if (view) {
      action(view);
    }
  };

  const handleTableEdit = (edit: TableEdit, next: CellPosition) => {
    const from = activeTable?.focus.from;
    if (from !== undefined) {
      withView((view) => editTable(view, from, edit, next));
    }
  };

  const handleDeleteTable = () => {
    const from = activeTable?.focus.from;
    setActiveTable(null);
    if (from !== undefined) {
      withView((view) => deleteTable(view, from));
    }
  };

  return (
    <div className="relative">
      <div ref={hostRef} />
      {toolbar ? (
        <SelectionToolbar
          position={toolbar}
          onInline={(format) => withView((view) => toggleInline(view, format))}
          onHeading={() => withView(toggleHeading)}
          onLink={() => withView(makeLink)}
        />
      ) : null}
      {activeTable ? (
        <TableToolbar
          focus={activeTable.focus}
          position={activeTable.position}
          onEdit={handleTableEdit}
          onDeleteTable={handleDeleteTable}
        />
      ) : null}
    </div>
  );
}

function toolbarPosition(view: EditorView, host: HTMLElement): ToolbarPosition | null {
  const { from, to, empty } = view.state.selection.main;
  if (empty || !view.hasFocus) {
    return null;
  }
  const start = view.coordsAtPos(from);
  const end = view.coordsAtPos(to);
  if (!start || !end) {
    return null;
  }
  const bounds = host.getBoundingClientRect();
  const left = start.top === end.top ? (start.left + end.left) / 2 : start.left;
  return { top: start.top - bounds.top - toolbarOffset, left: left - bounds.left };
}

function activeTableIn(focus: TableFocus, host: HTMLElement): ActiveTable | null {
  const wrap = focus.element.closest(".cm-table-wrap");
  if (!wrap) {
    return null;
  }
  const bounds = host.getBoundingClientRect();
  const table = wrap.getBoundingClientRect();
  return {
    focus,
    position: { top: table.top - bounds.top - toolbarOffset, left: table.left - bounds.left },
  };
}
