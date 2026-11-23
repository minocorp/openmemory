import { describe, expect, it } from "vitest";

import { inviteLink } from "./invite-link";

describe("inviteLink", () => {
  it("builds the registration link from the origin and token", () => {
    expect(inviteLink("https://memory.example", "a b")).toBe(
      "https://memory.example/register?token=a%20b",
    );
  });
});
