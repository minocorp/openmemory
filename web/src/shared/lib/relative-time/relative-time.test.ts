import { describe, expect, it } from "vitest";

import { formatRelativeTime } from "./relative-time";

const now = new Date("2026-11-23T12:00:00Z");

describe("relative time", () => {
  it("says just now under a minute", () => {
    expect(formatRelativeTime("2026-11-23T11:59:30Z", now)).toBe("just now");
  });

  it("uses the largest whole unit", () => {
    expect(formatRelativeTime("2026-11-23T10:00:00Z", now)).toBe("2 hours ago");
    expect(formatRelativeTime("2026-11-20T12:00:00Z", now)).toBe("3 days ago");
  });
});
