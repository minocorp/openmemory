import { FileText } from "lucide-react";

import type { Page, TimelineEntry } from "@/entities/page";
import { TimelineAddForm } from "@/features/append-timeline";
import { cn } from "@/shared/lib/cn";
import { baseName } from "@/shared/lib/page-path";
import { InlineMarkdown } from "@/shared/ui/inline-markdown";

type FileSource = NonNullable<TimelineEntry["file"]>;

type TimelineProps = {
  page: Page;
  changedLines: ReadonlySet<string>;
  openFile: { path: string; page: number | undefined } | null;
  onOpenSource: (source: FileSource) => void;
  onOpenLink: (path: string) => void;
};

export function Timeline({
  page,
  changedLines,
  openFile,
  onOpenSource,
  onOpenLink,
}: TimelineProps) {
  const entries = page.timeline.toReversed();
  return (
    <section aria-labelledby="timeline-heading" className="mt-12 border-t pt-6">
      <h2 id="timeline-heading" className="mb-1 text-sm font-medium">
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
            onOpenLink={onOpenLink}
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
  onOpenLink: (path: string) => void;
};

function TimelineRow({ entry, isChanged, isOpen, onOpenSource, onOpenLink }: TimelineRowProps) {
  return (
    <li className="flex gap-3 py-1.5 text-md max-sm:text-sm">
      <time
        className="w-24 shrink-0 pt-0.5 text-sm text-muted-foreground tabular-nums"
        dateTime={entry.date}
      >
        {entry.date}
      </time>
      <span className="min-w-0 flex-1">
        <span className={cn("rounded-sm", isChanged && "bg-highlight px-0.5")}>
          <InlineMarkdown text={entry.text} onOpenLink={onOpenLink} />
        </span>
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
    "ml-2 inline-flex h-5 max-w-full items-center gap-1 rounded-sm px-1.5 align-text-bottom text-xs transition-colors";
  const file = entry.file;
  if (!file) {
    return <span className={cn(chip, "bg-muted text-muted-foreground")}>{entry.source}</span>;
  }
  return (
    <button
      type="button"
      className={cn(
        chip,
        "bg-muted text-foreground hover:bg-accent",
        isOpen && "bg-highlight hover:bg-highlight",
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
