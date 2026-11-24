import { cn } from "@/shared/lib/cn";
import { parseUnifiedDiff, type DiffLine } from "@/shared/lib/unified-diff";

type DiffViewProps = {
  diff: string;
  className?: string;
};

const marker = { add: "+", remove: "−", context: " " } as const;

const lineStyle = {
  add: "bg-added text-added-foreground",
  remove: "bg-removed text-removed-foreground",
  context: "text-muted-foreground",
} as const;

export function DiffView({ diff, className }: DiffViewProps) {
  const lines = parseUnifiedDiff(diff);
  if (lines.length === 0) {
    return <p className="text-xs text-muted-foreground">No textual change.</p>;
  }
  return (
    <div className={cn("font-mono text-xs leading-6", className)}>
      {lines.map((line, index) => (
        <DiffRow key={`${index}-${line.kind}`} line={line} isFirst={index === 0} />
      ))}
    </div>
  );
}

function DiffRow({ line, isFirst }: { line: DiffLine; isFirst: boolean }) {
  if (line.kind === "hunk") {
    return isFirst ? null : (
      <div aria-hidden className="px-2 text-muted-foreground/60 select-none">
        …
      </div>
    );
  }
  return (
    <div className={cn("flex gap-2 rounded-sm px-2", lineStyle[line.kind])}>
      <span aria-hidden className="w-3 shrink-0 select-none">
        {marker[line.kind]}
      </span>
      <span className="min-w-0 flex-1 break-words whitespace-pre-wrap">{line.text || " "}</span>
      {line.kind === "context" ? null : (
        <span className="sr-only">{line.kind === "add" ? "added line" : "removed line"}</span>
      )}
    </div>
  );
}
