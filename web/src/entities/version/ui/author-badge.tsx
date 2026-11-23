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
  return (
    <span className={cn("inline-flex items-center gap-1.5", className)} title={title}>
      <span
        aria-hidden
        className={cn(
          "grid size-[18px] shrink-0 place-items-center rounded-[5px] text-[10px] leading-none font-bold",
          parsed.kind === "agent" ? "bg-agent text-white" : "bg-foreground text-background",
        )}
      >
        {parsed.name.slice(0, 1).toUpperCase()}
      </span>
      <span className="truncate">{parsed.name}</span>
      {parsed.kind === "agent" ? <span className="sr-only">(agent)</span> : null}
    </span>
  );
}
