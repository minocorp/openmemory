import { describe, expect, it } from "vitest";

import { removeField, setField, splitContent, withBody, withHead } from "./content";

const head = "---\nstatus: active\nrenewal: 2027-03-01\n---\n";
const body = "Design partner.\n\n";
const tail = "## Timeline\n2026-11-20 | call — agreed\n";
const page = { content: `${head}${body}${tail}`, body };

describe("page content", () => {
  it("splits the stored text around the body", () => {
    expect(splitContent(page)).toEqual({ head, tail });
  });

  it("keeps the head and timeline byte for byte when the body changes", () => {
    expect(withBody(page, "New text.\n\n")).toBe(`${head}New text.\n\n${tail}`);
  });

  it("keeps the timeline heading on its own line", () => {
    expect(withBody(page, "No trailing newline")).toBe(`${head}No trailing newline\n${tail}`);
  });

  it("handles a page without frontmatter or timeline", () => {
    const plain = { content: "Just text\n", body: "Just text\n" };
    expect(splitContent(plain)).toEqual({ head: "", tail: "" });
    expect(withBody(plain, "Other\n")).toBe("Other\n");
  });

  it("replaces the head and keeps the rest", () => {
    expect(withHead(page, "---\nstatus: done\n---\n")).toBe(
      `---\nstatus: done\n---\n${body}${tail}`,
    );
  });
});

describe("frontmatter fields", () => {
  it("replaces an existing field in place", () => {
    expect(setField(head, "status", ["done"])).toBe(
      "---\nstatus: done\nrenewal: 2027-03-01\n---\n",
    );
  });

  it("appends a new field before the closing fence", () => {
    expect(setField(head, "contact", ["Kim"])).toBe(
      `${head.replace(/---\n$/, "")}contact: Kim\n---\n`,
    );
  });

  it("creates frontmatter when there is none", () => {
    expect(setField("", "status", ["active"])).toBe("---\nstatus: active\n---\n");
  });

  it("writes several values as a flow list and quotes risky text", () => {
    expect(setField(head, "tags", ["a", "b"])).toContain("tags: [a, b]\n");
    expect(setField(head, "note", ["a: b"])).toContain('note: "a: b"\n');
  });

  it("replaces a whole block list", () => {
    const block = "---\ntags:\n  - a\n  - b\nstatus: x\n---\n";
    expect(setField(block, "tags", ["c"])).toBe("---\ntags: c\nstatus: x\n---\n");
  });

  it("removes a field and leaves the others", () => {
    expect(removeField(head, "status")).toBe("---\nrenewal: 2027-03-01\n---\n");
    expect(removeField(head, "missing")).toBe(head);
  });
});
