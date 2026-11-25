import { type EditorState, StateEffect, StateField } from "@codemirror/state";
import { EditorView } from "@codemirror/view";

const setFocus = StateEffect.define<boolean>();

export const focusField = StateField.define<boolean>({
  create: () => false,
  update: (isFocused, transaction) =>
    transaction.effects.reduce(
      (current, effect) => (effect.is(setFocus) ? effect.value : current),
      isFocused,
    ),
});

export const trackFocus = [
  focusField,
  EditorView.focusChangeEffect.of((_state, isFocusing) => setFocus.of(isFocusing)),
];

export function touchesSelection(state: EditorState, from: number, to: number): boolean {
  return (
    state.field(focusField, false) === true &&
    state.selection.ranges.some((range) => range.from <= to && range.to >= from)
  );
}
