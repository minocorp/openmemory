import { mergeText } from "./merge-text";

type Revision = { version: number; body: string };

export type Draft<P extends Revision> = { base: P; text: string };

export function rebaseDraft<P extends Revision>(draft: Draft<P> | null, page: P): Draft<P> | null {
  if (draft === null || draft.base.version === page.version) {
    return draft;
  }
  const merged = mergeText(draft.base.body, draft.text, page.body);
  if (merged === null) {
    return draft;
  }
  return merged === page.body ? null : { base: page, text: merged };
}
