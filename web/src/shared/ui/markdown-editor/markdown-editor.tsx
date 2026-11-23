import { autocompletion } from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
import { Compartment, EditorState } from "@codemirror/state";
import { EditorView, keymap, placeholder } from "@codemirror/view";
import { useEffect, useEffectEvent, useRef, useState } from "react";

import { editorTheme } from "./editor-theme";
import { makeLink, toggleHeading, toggleInline } from "./formatting";
import type { LinkTarget } from "./link-targets";
import { livePreview, toggleTaskAt } from "./live-preview";
import { markedLines } from "./marked-lines";
import { SelectionToolbar, type ToolbarPosition } from "./selection-toolbar";
import { slashSource, wikiSource } from "./slash-menu";

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
  const changedLinesCompartment = useRef(new Compartment());
  const [toolbar, setToolbar] = useState<ToolbarPosition | null>(null);

  const handleChange = useEffectEvent(onChange);
  const handleSave = useEffectEvent(onSave);
  const handleOpenLink = useEffectEvent(onOpenLink);
  const currentTargets = useEffectEvent(() => linkTargets);
  const initialValue = useEffectEvent(() => value);
  const initialChangedLines = useEffectEvent(() => changedLines);

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
          EditorState.lineSeparator.of("\n"),
          history(),
          keymap.of([
            { key: "Mod-s", preventDefault: true, run: () => (handleSave(), true) },
            ...historyKeymap,
            ...defaultKeymap,
          ]),
          markdown({ base: markdownLanguage }),
          livePreview,
          autocompletion({
            override: [slashSource, wikiSource(currentTargets)],
            icons: false,
          }),
          EditorView.lineWrapping,
          EditorView.contentAttributes.of({ "aria-label": label }),
          placeholder(emptyHint),
          editorTheme,
          changedLinesCompartment.current.of(markedLines(initialChangedLines())),
          EditorView.domEventHandlers({
            mousedown: (event, editor) => {
              const target = event.target;
              if (
                target instanceof HTMLInputElement &&
                target.classList.contains("cm-task-checkbox")
              ) {
                event.preventDefault();
                return toggleTaskAt(editor, editor.posAtDOM(target));
              }
              const link = target instanceof Element ? target.closest("[data-wikilink]") : null;
              const path = link?.getAttribute("data-wikilink");
              if (path && (event.metaKey || event.ctrlKey)) {
                event.preventDefault();
                handleOpenLink(path);
                return true;
              }
              return false;
            },
          }),
          EditorView.updateListener.of((update) => {
            if (update.docChanged) {
              handleChange(update.state.doc.toString());
            }
            if (update.docChanged || update.selectionSet || update.focusChanged) {
              setToolbar(toolbarPosition(update.view, host));
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
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: value } });
    }
  }, [value]);

  useEffect(() => {
    viewRef.current?.dispatch({
      effects: changedLinesCompartment.current.reconfigure(markedLines(changedLines)),
    });
  }, [changedLines]);

  const withView = (action: (view: EditorView) => void) => {
    const view = viewRef.current;
    if (view) {
      action(view);
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
    </div>
  );
}

const toolbarOffset = 44;

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
