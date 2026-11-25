import { describe, expect, it } from "vitest";

import { safeHref } from "./safe-url";

describe("safeHref", () => {
  it.each([
    ["https://example.com/a?b=1", "https://example.com/a?b=1"],
    ["http://example.com", "http://example.com/"],
    ["www.example.com", "https://www.example.com/"],
    ["mailto:me@example.com", "mailto:me@example.com"],
    ["me@example.com", "mailto:me@example.com"],
  ])("allows %s", (raw, expected) => {
    expect(safeHref(raw)).toBe(expected);
  });

  it.each(["javascript:alert(1)", "data:text/html,x", "file:///etc/passwd", "relative/path", ""])(
    "rejects %j",
    (raw) => {
      expect(safeHref(raw)).toBeNull();
    },
  );
});
