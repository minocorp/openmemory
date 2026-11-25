// oxlint-disable jsx-a11y/prefer-tag-over-role -- a window splitter is a focusable separator; hr cannot take focus or pointer input
import { useRef, useState, type KeyboardEvent, type PointerEvent, type ReactNode } from "react";

import { readStored, writeStored } from "@/shared/lib/local-storage";

import { sidebarWidth } from "../config/sidebar-width";
import { storageKeys } from "../config/storage-keys";
import { widthAfterDrag, widthAfterKey, widthSchema, type DragStart } from "../model/sidebar-width";

type ResizableSidebarProps = {
  children: ReactNode;
};

export function ResizableSidebar({ children }: ResizableSidebarProps) {
  const [width, setWidth] = useState(
    () => readStored(storageKeys.width, widthSchema) ?? sidebarWidth.initial,
  );
  const dragStart = useRef<DragStart | null>(null);

  const saveWidth = (next: number) => {
    setWidth(next);
    writeStored(storageKeys.width, next);
  };

  const handlePointerDown = (event: PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) {
      return;
    }
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    dragStart.current = { x: event.clientX, width };
  };

  const handlePointerMove = (event: PointerEvent<HTMLDivElement>) => {
    if (dragStart.current) {
      setWidth(widthAfterDrag(dragStart.current, event.clientX));
    }
  };

  const handlePointerUp = (event: PointerEvent<HTMLDivElement>) => {
    if (dragStart.current) {
      saveWidth(widthAfterDrag(dragStart.current, event.clientX));
      dragStart.current = null;
    }
  };

  const handlePointerCancel = () => {
    if (dragStart.current) {
      setWidth(dragStart.current.width);
      dragStart.current = null;
    }
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const next = widthAfterKey(width, event.key);
    if (next !== null) {
      event.preventDefault();
      saveWidth(next);
    }
  };

  return (
    <div className="relative hidden h-svh max-w-1/2 shrink-0 sm:flex" style={{ width }}>
      {children}
      <div
        role="separator"
        aria-label="Resize sidebar"
        aria-orientation="vertical"
        aria-valuemin={sidebarWidth.min}
        aria-valuemax={sidebarWidth.max}
        aria-valuenow={width}
        tabIndex={0}
        className="absolute inset-y-0 -right-1 z-10 w-2 cursor-col-resize touch-none outline-none after:absolute after:inset-y-0 after:left-1/2 after:w-0.5 after:-translate-x-1/2 after:transition-colors hover:after:bg-ring/50 focus-visible:after:bg-ring active:after:bg-ring"
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerCancel={handlePointerCancel}
        onKeyDown={handleKeyDown}
        onDoubleClick={() => saveWidth(sidebarWidth.initial)}
      />
    </div>
  );
}
