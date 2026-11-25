import { commonmarkLanguage, markdown } from "@codemirror/lang-markdown";
import { languages } from "@codemirror/language-data";
import { EditorState, type Extension, Prec } from "@codemirror/state";
import { EditorView, keymap } from "@codemirror/view";

import { markdownExtensions } from "@/shared/lib/markdown";

import { codeHighlighting } from "./code-highlight";
import { editorTheme } from "./editor-theme";
import { markedLines } from "./marked-lines";
import { cursorFilter } from "./preview/cursor";
import { previewKeymap } from "./preview/keymap";
import { linkClicks } from "./preview/link-click";
import { previewAtomicRanges, previewPlugin } from "./preview/plugin";
import { trackFocus } from "./preview/reveal";
import { tableField } from "./preview/tables";

export function markdownPreview(): Extension[] {
  return [
    EditorState.lineSeparator.of("\n"),
    markdown({
      base: commonmarkLanguage,
      extensions: markdownExtensions,
      codeLanguages: languages,
      completeHTMLTags: false,
    }),
    codeHighlighting,
    trackFocus,
    previewPlugin,
    previewAtomicRanges,
    tableField,
    cursorFilter,
    markedLines,
    linkClicks,
    Prec.high(keymap.of(previewKeymap)),
    EditorView.lineWrapping,
    editorTheme,
  ];
}
