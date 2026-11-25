import { describe, expect, it } from "vitest";

import { sidebarWidth } from "../config/sidebar-width";
import { clampWidth, widthAfterDrag, widthAfterKey } from "./sidebar-width";

describe("sidebar width", () => {
  it("keeps the width between the bounds in whole pixels", () => {
    expect(clampWidth(100)).toBe(sidebarWidth.min);
    expect(clampWidth(900)).toBe(sidebarWidth.max);
    expect(clampWidth(300.6)).toBe(301);
  });

  it("follows the pointer from where the drag started", () => {
    expect(widthAfterDrag({ x: 250, width: 248 }, 350)).toBe(348);
    expect(widthAfterDrag({ x: 250, width: 248 }, 0)).toBe(sidebarWidth.min);
  });

  it("steps with the arrow keys and stops at the bounds", () => {
    expect(widthAfterKey(300, "ArrowRight")).toBe(300 + sidebarWidth.step);
    expect(widthAfterKey(300, "ArrowLeft")).toBe(300 - sidebarWidth.step);
    expect(widthAfterKey(sidebarWidth.max, "ArrowRight")).toBe(sidebarWidth.max);
  });

  it("ignores other keys", () => {
    expect(widthAfterKey(300, "Enter")).toBeNull();
  });
});
