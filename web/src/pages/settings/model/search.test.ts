import { describe, expect, it } from "vitest";

import { resolveTab, settingsSearchSchema, visibleTabs } from "./search";

describe("settings tabs", () => {
  it("defaults to the account tab", () => {
    expect(settingsSearchSchema.parse({}).tab).toBe("account");
  });

  it("hides the admin tabs from members", () => {
    expect(visibleTabs(false)).toEqual(["account", "connections"]);
    expect(visibleTabs(true)).toEqual(["account", "people", "connections", "system"]);
  });

  it("falls back to account when a member asks for an admin tab", () => {
    expect(resolveTab("system", false)).toBe("account");
    expect(resolveTab("system", true)).toBe("system");
  });
});
