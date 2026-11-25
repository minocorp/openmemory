import { EditorView } from "@codemirror/view";

import { handlersFacet } from "./context";

const taskMarkerPattern = /^\[( |x|X)\]$/;

export function openLinkTarget(view: EditorView, target: Element): boolean {
  const href = target.closest("[data-href]")?.getAttribute("data-href");
  if (href) {
    window.open(href, "_blank", "noopener,noreferrer");
    return true;
  }
  const path = target.closest("[data-wikilink]")?.getAttribute("data-wikilink");
  if (path) {
    view.state.facet(handlersFacet).openLink(path);
    return true;
  }
  return false;
}

export function toggleTaskAt(view: EditorView, position: number): boolean {
  const text = view.state.doc.sliceString(position, position + 3);
  if (view.state.readOnly || !taskMarkerPattern.test(text)) {
    return false;
  }
  const next = text === "[ ]" ? "[x]" : "[ ]";
  view.dispatch({ changes: { from: position, to: position + 3, insert: next } });
  return true;
}

export const linkClicks = EditorView.domEventHandlers({
  mousedown: (event, view) => {
    const { target } = event;
    if (event.button !== 0 || event.shiftKey || !(target instanceof Element)) {
      return false;
    }
    if (target instanceof HTMLInputElement && target.classList.contains("cm-task-checkbox")) {
      event.preventDefault();
      return toggleTaskAt(view, view.posAtDOM(target));
    }
    if (openLinkTarget(view, target)) {
      event.preventDefault();
      return true;
    }
    return false;
  },
});
