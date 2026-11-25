import { z } from "@/shared/lib/zod";

import { sidebarWidth } from "../config/sidebar-width";

export type DragStart = {
  x: number;
  width: number;
};

export const widthSchema = z.number().int().min(sidebarWidth.min).max(sidebarWidth.max);

export function clampWidth(width: number): number {
  return Math.min(sidebarWidth.max, Math.max(sidebarWidth.min, Math.round(width)));
}

export function widthAfterDrag(start: DragStart, x: number): number {
  return clampWidth(start.width + x - start.x);
}

export function widthAfterKey(width: number, key: string): number | null {
  if (key === "ArrowLeft") {
    return clampWidth(width - sidebarWidth.step);
  }
  if (key === "ArrowRight") {
    return clampWidth(width + sidebarWidth.step);
  }
  return null;
}
