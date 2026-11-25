import { describe, expect, it } from "vitest";

import { parseInline } from "./inline";

describe("parseInline", () => {
  it("keeps plain text as one node", () => {
    expect(parseInline("paid 450000 on 2026-11-24")).toEqual([
      { kind: "text", text: "paid 450000 on 2026-11-24" },
    ]);
  });

  it("nests emphasis inside strong text", () => {
    expect(parseInline("**a *b***")).toEqual([
      {
        kind: "strong",
        children: [
          { kind: "text", text: "a " },
          { kind: "emphasis", children: [{ kind: "text", text: "b" }] },
        ],
      },
    ]);
  });

  it("reads code, strike and escapes", () => {
    expect(parseInline("`x` ~~y~~ \\*z")).toEqual([
      { kind: "code", text: "x" },
      { kind: "text", text: " " },
      { kind: "strike", children: [{ kind: "text", text: "y" }] },
      { kind: "text", text: " *z" },
    ]);
  });

  it("turns markdown links and bare urls into links", () => {
    expect(parseInline("[site](https://example.com) or www.example.org")).toEqual([
      { kind: "link", href: "https://example.com/", children: [{ kind: "text", text: "site" }] },
      { kind: "text", text: " or " },
      {
        kind: "link",
        href: "https://www.example.org/",
        children: [{ kind: "text", text: "www.example.org" }],
      },
    ]);
  });

  it("drops links with unsafe schemes and keeps their label", () => {
    expect(parseInline("[x](javascript:alert(1))")).toEqual([{ kind: "text", text: "x" }]);
  });

  it("does not read block syntax", () => {
    expect(parseInline("# not a heading")).toEqual([{ kind: "text", text: "# not a heading" }]);
    expect(parseInline("1. not a list")).toEqual([{ kind: "text", text: "1. not a list" }]);
  });
});
