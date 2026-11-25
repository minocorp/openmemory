import { describe, expect, it } from "vitest";

import { inviteLink } from "./invite-link";

describe("inviteLink", () => {
  it("builds the registration link from the origin and token", () => {
    expect(inviteLink("https://openmemory.example", "a b")).toBe(
      "https://openmemory.example/register?token=a%20b",
    );
  });
});
