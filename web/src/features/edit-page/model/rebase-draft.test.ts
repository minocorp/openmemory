import { describe, expect, it } from "vitest";

import { rebaseDraft } from "./rebase-draft";

const v1 = { version: 1, body: "intro\n\nmiddle\n\nend\n" };
const v2 = { version: 2, body: "intro\n\nmiddle\n\nthe end\n" };

describe("rebase draft", () => {
  it("moves an unsaved edit onto the newer version when the edits do not overlap", () => {
    const draft = { base: v1, text: "Intro\n\nmiddle\n\nend\n" };
    expect(rebaseDraft(draft, v2)).toEqual({ base: v2, text: "Intro\n\nmiddle\n\nthe end\n" });
  });

  it("keeps the draft on its old version when the edits overlap", () => {
    const draft = { base: v1, text: "intro\n\nmiddle\n\nfin\n" };
    expect(rebaseDraft(draft, v2)).toBe(draft);
  });

  it("drops the draft when the newer version already says the same", () => {
    expect(rebaseDraft({ base: v1, text: v2.body }, v2)).toBeNull();
  });

  it("leaves a draft on the current version alone", () => {
    const draft = { base: v2, text: "x\n" };
    expect(rebaseDraft(draft, v2)).toBe(draft);
    expect(rebaseDraft(null, v2)).toBeNull();
  });
});
