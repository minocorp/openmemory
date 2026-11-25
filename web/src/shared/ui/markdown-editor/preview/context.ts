import { Facet } from "@codemirror/state";

import type { TableAlign } from "@/shared/lib/markdown";

import type { LinkTarget } from "../link-targets";

export type TableFocus = {
  from: number;
  row: number;
  column: number;
  rowCount: number;
  columnCount: number;
  align: TableAlign;
  element: HTMLElement;
};

export type PreviewHandlers = {
  openLink: (path: string) => void;
  focusTable: (focus: TableFocus | null) => void;
};

export type LinkIndex = { targets: readonly LinkTarget[]; paths: ReadonlySet<string> };

const noHandlers: PreviewHandlers = { openLink: () => {}, focusTable: () => {} };
const noLines: ReadonlySet<string> = new Set();

export const handlersFacet = Facet.define<PreviewHandlers, PreviewHandlers>({
  combine: (values) => values.at(-1) ?? noHandlers,
});

export const linkTargetsFacet = Facet.define<readonly LinkTarget[], LinkIndex | null>({
  combine: (values) => {
    const targets = values.at(-1);
    if (!targets) {
      return null;
    }
    return { targets, paths: new Set(targets.map((target) => target.path.normalize("NFC"))) };
  },
});

export const changedLinesFacet = Facet.define<ReadonlySet<string>, ReadonlySet<string>>({
  combine: (values) => values.at(-1) ?? noLines,
});

export function isMissingLink(index: LinkIndex | null, path: string): boolean {
  return index !== null && !index.paths.has(path.normalize("NFC"));
}
