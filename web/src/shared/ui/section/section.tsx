import type { ReactNode } from "react";

import { cn } from "@/shared/lib/cn";

type SectionProps = {
  title: string;
  action?: ReactNode;
  children: ReactNode;
};

export function Section({ title, action, children }: SectionProps) {
  return (
    <section aria-label={title} className="grid gap-3">
      <div className="flex min-h-8 items-center justify-between gap-3">
        <h2 className="text-sm font-semibold">{title}</h2>
        {action}
      </div>
      {children}
    </section>
  );
}

export function SectionList({ className, ...props }: React.ComponentProps<"ul">) {
  return (
    <ul
      className={cn("divide-y overflow-hidden rounded-lg border bg-card text-sm", className)}
      {...props}
    />
  );
}

export function SectionRow({ className, ...props }: React.ComponentProps<"li">) {
  return (
    <li
      className={cn(
        "flex flex-wrap items-center justify-between gap-x-4 gap-y-2 px-3 py-2.5",
        className,
      )}
      {...props}
    />
  );
}

export function SectionEmpty({ children }: { children: ReactNode }) {
  return (
    <p className="rounded-lg border border-dashed px-3 py-4 text-sm text-muted-foreground">
      {children}
    </p>
  );
}
