import { FileText } from "lucide-react";

import type { Page, TimelineEntry } from "@/entities/page";
import { TimelineAddForm } from "@/features/append-timeline";
import { cn } from "@/shared/lib/cn";
import { baseName } from "@/shared/lib/page-path";

type FileSource = NonNullable<TimelineEntry["file"]>;

type TimelineProps = {
  page: Page;
  changedLines: ReadonlySet<string>;
  openFile: { path: string; page: number | undefined } | null;
  onOpenSource: (source: FileSource) => void;
};

export function Timeline({ page, changedLines, openFile, onOpenSource }: TimelineProps) {
  const entries = page.timeline.toReversed();
  return (
    <section aria-labelledby="timeline-heading" className="mt-8">
      <h2
        id="timeline-heading"
        className="mb-2 flex items-center gap-3 text-xs font-medium tracking-wide text-muted-foreground after:h-px after:flex-1 after:bg-border"
      >
        Timeline
      </h2>
      <TimelineAddForm page={page} />
      <ol className="mt-2">
        {entries.map((entry) => (
          <TimelineRow
            key={`${entry.date}|${entry.source}|${entry.text}`}
            entry={entry}
            isChanged={changedLines.has(`${entry.date} | ${entry.source} — ${entry.text}`)}
            isOpen={isOpenSource(entry, openFile)}
            onOpenSource={onOpenSource}
          />
        ))}
      </ol>
    </section>
  );
}

function isOpenSource(entry: TimelineEntry, openFile: TimelineProps["openFile"]): boolean {
  return openFile !== null && openFile.path === entry.file?.path;
}

type TimelineRowProps = {
  entry: TimelineEntry;
  isChanged: boolean;
  isOpen: boolean;
  onOpenSource: (source: FileSource) => void;
};

function TimelineRow({ entry, isChanged, isOpen, onOpenSource }: TimelineRowProps) {
  return (
    <li className="grid grid-cols-[92px_1fr] gap-3 py-1.5 text-sm">
      <time className="pt-px text-[13px] text-muted-foreground tabular-nums" dateTime={entry.date}>
        {entry.date}
      </time>
      <span className="min-w-0">
        <span className={cn("rounded-sm", isChanged && "bg-highlight px-0.5")}>{entry.text}</span>
        <SourceChip entry={entry} isOpen={isOpen} onOpenSource={onOpenSource} />
      </span>
    </li>
  );
}

function SourceChip({
  entry,
  isOpen,
  onOpenSource,
}: Pick<TimelineRowProps, "entry" | "isOpen" | "onOpenSource">) {
  const chip =
    "ml-1.5 inline-flex max-w-full items-center gap-1 rounded-md border px-1.5 align-baseline text-xs";
  const file = entry.file;
  if (!file) {
    return <span className={cn(chip, "bg-card text-muted-foreground")}>{entry.source}</span>;
  }
  return (
    <button
      type="button"
      className={cn(
        chip,
        "bg-card text-foreground/80 hover:bg-accent",
        isOpen && "border-highlight-border bg-highlight/60",
      )}
      aria-pressed={isOpen}
      onClick={() => onOpenSource(file)}
    >
      <FileText className="size-3 shrink-0" />
      <span className="truncate">
        {baseName(file.path)}
        {file.page ? ` p.${file.page}` : ""}
      </span>
    </button>
  );
}
