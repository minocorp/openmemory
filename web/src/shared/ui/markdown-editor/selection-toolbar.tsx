import { Bold, Code, Heading2, Italic, Link, Strikethrough } from "lucide-react";
import type { ReactNode } from "react";

import { Button } from "@/shared/ui/button";

import type { InlineFormat } from "./formatting";

export type ToolbarPosition = { top: number; left: number };

type SelectionToolbarProps = {
  position: ToolbarPosition;
  onInline: (format: InlineFormat) => void;
  onHeading: () => void;
  onLink: () => void;
};

type ToolbarButtonProps = { label: string; onPress: () => void; children: ReactNode };

function ToolbarButton({ label, onPress, children }: ToolbarButtonProps) {
  return (
    <Button
      type="button"
      variant="ghost"
      size="icon-sm"
      aria-label={label}
      title={label}
      onMouseDown={(event) => event.preventDefault()}
      onClick={onPress}
    >
      {children}
    </Button>
  );
}

export function SelectionToolbar({ position, onInline, onHeading, onLink }: SelectionToolbarProps) {
  return (
    <div
      role="toolbar"
      aria-label="Text formatting"
      className="absolute z-20 flex -translate-x-1/2 items-center gap-0.5 rounded-lg border bg-popover p-1 shadow-popover"
      style={{ top: position.top, left: position.left }}
    >
      <ToolbarButton label="Bold" onPress={() => onInline("bold")}>
        <Bold />
      </ToolbarButton>
      <ToolbarButton label="Italic" onPress={() => onInline("italic")}>
        <Italic />
      </ToolbarButton>
      <ToolbarButton label="Strikethrough" onPress={() => onInline("strike")}>
        <Strikethrough />
      </ToolbarButton>
      <ToolbarButton label="Code" onPress={() => onInline("code")}>
        <Code />
      </ToolbarButton>
      <ToolbarButton label="Heading" onPress={onHeading}>
        <Heading2 />
      </ToolbarButton>
      <ToolbarButton label="Link" onPress={onLink}>
        <Link />
      </ToolbarButton>
    </div>
  );
}
