// oxlint-disable jsx-a11y/prefer-tag-over-role -- a combobox input with aria-activedescendant needs a custom listbox; native select and datalist cannot show rich results
import { Dialog as DialogPrimitive } from "@base-ui/react/dialog";
import { SearchIcon } from "lucide-react";
import type * as React from "react";

import { cn } from "@/shared/lib/cn";
import { overlayStyle } from "@/shared/ui/dialog";

type CommandDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  label: string;
  children: React.ReactNode;
};

function CommandDialog({ open, onOpenChange, label, children }: CommandDialogProps) {
  return (
    <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Backdrop className={overlayStyle} />
        <DialogPrimitive.Popup
          aria-label={label}
          className="fixed top-0 left-1/2 z-50 flex max-h-svh w-full -translate-x-1/2 flex-col overflow-hidden bg-popover text-sm text-popover-foreground transition-opacity duration-100 outline-none data-ending-style:opacity-0 data-starting-style:opacity-0 max-sm:h-svh sm:top-[12svh] sm:max-h-[min(34rem,76svh)] sm:max-w-xl sm:rounded-xl sm:border sm:shadow-modal"
        >
          <DialogPrimitive.Title className="sr-only">{label}</DialogPrimitive.Title>
          {children}
        </DialogPrimitive.Popup>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}

function CommandInput({
  className,
  "aria-controls": controls,
  action,
  ...props
}: React.ComponentProps<"input"> & { "aria-controls": string; action?: React.ReactNode }) {
  return (
    <div className="flex items-center gap-2.5 border-b px-4">
      <SearchIcon className="size-4 shrink-0 text-muted-foreground" />
      <input
        type="text"
        role="combobox"
        aria-expanded="true"
        aria-controls={controls}
        autoComplete="off"
        spellCheck={false}
        className={cn(
          "h-12 w-full bg-transparent text-md outline-none placeholder:text-muted-foreground",
          className,
        )}
        {...props}
      />
      {action}
    </div>
  );
}

function CommandList({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      role="listbox"
      className={cn("min-h-0 flex-1 overflow-y-auto overscroll-contain p-1.5", className)}
      {...props}
    />
  );
}

type CommandGroupProps = {
  heading: string;
  children: React.ReactNode;
};

function CommandGroup({ heading, children }: CommandGroupProps) {
  return (
    <div role="group" aria-label={heading} className="pb-1">
      <p aria-hidden className="px-2 pt-2 pb-1 text-xs text-muted-foreground">
        {heading}
      </p>
      {children}
    </div>
  );
}

type CommandItemProps = Omit<React.ComponentProps<"div">, "role"> & { isActive: boolean };

function CommandItem({ className, isActive, ...props }: CommandItemProps) {
  return (
    <div
      role="option"
      aria-selected={isActive}
      className={cn(
        "flex cursor-default flex-col gap-0.5 rounded-md px-2 py-2 select-none aria-selected:bg-accent max-sm:py-3",
        className,
      )}
      {...props}
    />
  );
}

function CommandMessage({ className, ...props }: React.ComponentProps<"output">) {
  return (
    <output
      className={cn("block px-4 py-8 text-center text-sm text-muted-foreground", className)}
      {...props}
    />
  );
}

function CommandFooter({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div className={cn("border-t px-4 py-2 text-xs text-muted-foreground", className)} {...props} />
  );
}

export {
  CommandDialog,
  CommandFooter,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandMessage,
};
