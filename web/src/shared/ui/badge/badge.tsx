import { mergeProps } from "@base-ui/react/merge-props";
import { useRender } from "@base-ui/react/use-render";
import { cva, type VariantProps } from "class-variance-authority";

import { cn } from "@/shared/lib/cn";

const badgeVariants = cva(
  "inline-flex h-5 w-fit shrink-0 items-center gap-1 rounded-sm px-1.5 text-xs font-medium whitespace-nowrap",
  {
    variants: {
      variant: {
        secondary: "bg-muted text-muted-foreground",
        outline: "border text-muted-foreground",
        destructive: "bg-destructive/10 text-destructive",
      },
    },
    defaultVariants: {
      variant: "secondary",
    },
  },
);

function Badge({
  className,
  variant = "secondary",
  render,
  ...props
}: useRender.ComponentProps<"span"> & VariantProps<typeof badgeVariants>) {
  return useRender({
    defaultTagName: "span",
    props: mergeProps<"span">({ className: cn(badgeVariants({ variant }), className) }, props),
    render,
    state: { slot: "badge", variant },
  });
}

export { Badge };
