import { EditorState, type Extension } from "@codemirror/state";
import { Decoration, EditorView } from "@codemirror/view";
import { useEffect, useEffectEvent, useRef } from "react";

import { markdownPreview } from "./markdown-extensions";
import { handlersFacet } from "./preview/context";

type MarkdownViewProps = {
  text: string;
  quote: string | null;
  label: string;
  onOpenLink: (path: string) => void;
};

const quoteMark = Decoration.mark({ class: "cm-source-quote" });

export function MarkdownView({ text, quote, label, onOpenLink }: MarkdownViewProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  const handleOpenLink = useEffectEvent(onOpenLink);

  useEffect(() => {
    const host = hostRef.current;
    if (!host) {
      return undefined;
    }
    const start = quote ? text.indexOf(quote) : -1;
    const view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: text,
        extensions: [
          markdownPreview(),
          EditorState.readOnly.of(true),
          EditorView.editable.of(false),
          EditorView.contentAttributes.of({ "aria-label": label }),
          handlersFacet.of({ openLink: handleOpenLink, focusTable: () => {} }),
          quoteHighlight(start, quote),
        ],
      }),
    });
    if (start >= 0) {
      view.dispatch({ effects: EditorView.scrollIntoView(start, { y: "center" }) });
    }
    return () => view.destroy();
  }, [text, quote, label]);

  return <div ref={hostRef} />;
}

function quoteHighlight(start: number, quote: string | null): Extension {
  if (!quote || start < 0) {
    return [];
  }
  return EditorView.decorations.of(Decoration.set([quoteMark.range(start, start + quote.length)]));
}
