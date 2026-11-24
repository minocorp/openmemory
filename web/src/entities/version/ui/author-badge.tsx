import { parseAuthor } from "@/shared/lib/author";
import { cn } from "@/shared/lib/cn";

type AuthorBadgeProps = {
  author: string;
  displayName?: string;
  className?: string;
};

export function AuthorBadge({ author, displayName, className }: AuthorBadgeProps) {
  const parsed = parseAuthor(author);
  const label = parsed.owner ? `${parsed.owner}/${parsed.name}` : parsed.name;
  const title = displayName ? `${displayName} (${label})` : label;
  const isAgent = parsed.kind === "agent";
  return (
    <span className={cn("inline-flex min-w-0 items-center gap-1.5", className)} title={title}>
      <span
        aria-hidden
        className={cn(
          "grid size-4 shrink-0 place-items-center text-2xs leading-none font-semibold",
          isAgent
            ? "rounded-sm border border-foreground/30 text-foreground"
            : "rounded-full bg-foreground text-background",
        )}
      >
        {parsed.name.slice(0, 1).toUpperCase()}
      </span>
      <span className="truncate">{parsed.name}</span>
      {isAgent ? <span className="sr-only">(agent)</span> : null}
    </span>
  );
}
