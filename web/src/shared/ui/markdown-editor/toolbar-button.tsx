import type { ReactNode } from "react";

import { cn } from "@/shared/lib/cn";
import { Button } from "@/shared/ui/button";

type ToolbarButtonProps = {
  label: string;
  onPress: () => void;
  isPressed?: boolean;
  isDisabled?: boolean;
  children: ReactNode;
};

export function ToolbarButton({
  label,
  onPress,
  isPressed,
  isDisabled = false,
  children,
}: ToolbarButtonProps) {
  return (
    <Button
      type="button"
      variant="ghost"
      size="icon-sm"
      className={cn(isPressed && "bg-accent text-accent-foreground")}
      aria-label={label}
      aria-pressed={isPressed}
      title={label}
      disabled={isDisabled}
      onMouseDown={(event) => event.preventDefault()}
      onClick={onPress}
    >
      {children}
    </Button>
  );
}
