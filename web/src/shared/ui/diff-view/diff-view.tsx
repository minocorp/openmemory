import { cn } from "@/shared/lib/cn";
import { parseUnifiedDiff, type DiffLine } from "@/shared/lib/unified-diff";

type DiffViewProps = {
  diff: string;
  className?: string;
};

const marker = { add: "+", remove: "−", context: " ", hunk: "" } as const;

const lineStyle = {
  add: "bg-added text-added-foreground",
  remove: "bg-removed text-removed-foreground",
  context: "text-muted-foreground",
  hunk: "text-muted-foreground/70",
} as const;

export function DiffView({ diff, className }: DiffViewProps) {
  const lines = parseUnifiedDiff(diff);
  if (lines.length === 0) {
    return <p className="px-2 py-1 text-xs text-muted-foreground">No textual change.</p>;
  }
  return (
    <div className={cn("font-mono text-xs leading-6", className)}>
      {lines.map((line, index) => (
        <DiffRow key={`${index}-${line.kind}`} line={line} />
      ))}
    </div>
  );
}

function DiffRow({ line }: { line: DiffLine }) {
  return (
    <div className={cn("flex gap-2 rounded-sm px-2", lineStyle[line.kind])}>
      <span aria-hidden className="w-3 shrink-0 select-none">
        {marker[line.kind]}
      </span>
      <span className="min-w-0 flex-1 break-words whitespace-pre-wrap">
        {line.kind === "hunk" ? line.text : line.text || " "}
      </span>
      {line.kind === "add" || line.kind === "remove" ? (
        <span className="sr-only">{line.kind === "add" ? "added line" : "removed line"}</span>
      ) : null}
    </div>
  );
}
