import { Switch as SwitchPrimitive } from "@base-ui/react/switch";

import { cn } from "@/shared/lib/cn";

export function Switch({ className, ...props }: SwitchPrimitive.Root.Props) {
  return (
    <SwitchPrimitive.Root
      data-slot="switch"
      className={cn(
        "inline-flex h-[18px] w-8 shrink-0 items-center rounded-full bg-input p-0.5 transition-colors outline-none focus-visible:ring-3 focus-visible:ring-ring/50 disabled:opacity-50 data-checked:bg-primary",
        className,
      )}
      {...props}
    >
      <SwitchPrimitive.Thumb className="size-3.5 rounded-full bg-background shadow-sm transition-transform data-checked:translate-x-3.5" />
    </SwitchPrimitive.Root>
  );
}
