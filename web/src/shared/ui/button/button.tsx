import { Button as ButtonPrimitive } from "@base-ui/react/button";
import { cva, type VariantProps } from "class-variance-authority";

import { cn } from "@/shared/lib/cn";

const buttonVariants = cva(
  "inline-flex shrink-0 items-center justify-center gap-1.5 rounded-md border border-transparent text-sm font-medium whitespace-nowrap transition-colors select-none disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground hover:bg-primary/85",
        outline: "border-input bg-background hover:bg-accent aria-expanded:bg-accent",
        ghost: "hover:bg-accent aria-expanded:bg-accent",
        destructive: "bg-destructive text-white hover:bg-destructive/90",
      },
      size: {
        default: "h-9 px-3 md:h-8",
        xs: "h-6 gap-1 rounded-sm px-1.5 text-xs [&_svg:not([class*='size-'])]:size-3.5",
        sm: "h-7 px-2.5",
        "icon-xs": "size-6 rounded-sm [&_svg:not([class*='size-'])]:size-3.5",
        "icon-sm": "size-7",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  },
);

function Button({
  className,
  variant = "default",
  size = "default",
  ...props
}: ButtonPrimitive.Props & VariantProps<typeof buttonVariants>) {
  return (
    <ButtonPrimitive
      data-slot="button"
      className={cn(buttonVariants({ variant, size, className }))}
      {...props}
    />
  );
}

export { Button, buttonVariants };
