import { useInfiniteQuery } from "@tanstack/react-query";
import { useState } from "react";

import { versionsQuery, type Page } from "@/entities/page";
import { AuthorBadge, describeOperation, type Version } from "@/entities/version";
import { RestoreDialog } from "@/features/restore-version";
import { formatRelativeTime } from "@/shared/lib/relative-time";
import { Button } from "@/shared/ui/button";
import { DiffView } from "@/shared/ui/diff-view";
import { ScrollArea } from "@/shared/ui/scroll-area";
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from "@/shared/ui/sheet";
import { Skeleton } from "@/shared/ui/skeleton";

type HistoryDrawerProps = {
  page: Page;
  onClose: () => void;
};

export function HistoryDrawer({ page, onClose }: HistoryDrawerProps) {
  const versions = useInfiniteQuery(versionsQuery(page.id));
  const [restoring, setRestoring] = useState<number | null>(null);
  const [expanded, setExpanded] = useState<ReadonlySet<number>>(new Set());

  const toggle = (version: number) =>
    setExpanded((previous) => {
      const next = new Set(previous);
      if (!next.delete(version)) {
        next.add(version);
      }
      return next;
    });

  return (
    <>
      <Sheet open onOpenChange={(open) => !open && onClose()}>
        <SheetContent className="sm:max-w-lg" aria-describedby="history-description">
          <SheetHeader>
            <SheetTitle>History</SheetTitle>
            <SheetDescription id="history-description">
              Every save is a version. Restoring adds a new version; nothing is lost.
            </SheetDescription>
          </SheetHeader>
          <ScrollArea className="min-h-0 flex-1 px-4">
            {versions.isPending ? <Skeleton className="h-24" /> : null}
            <ol className="grid gap-3 pb-6">
              {versions.data?.pages
                .flatMap((chunk) => chunk.versions)
                .map((version) => (
                  <VersionRow
                    key={version.version}
                    version={version}
                    isCurrent={version.version === page.version}
                    isExpanded={expanded.has(version.version)}
                    onToggle={() => toggle(version.version)}
                    onRestore={() => setRestoring(version.version)}
                  />
                ))}
            </ol>
            {versions.hasNextPage ? (
              <Button
                variant="outline"
                size="sm"
                className="mb-6"
                disabled={versions.isFetchingNextPage}
                onClick={() => void versions.fetchNextPage()}
              >
                Load older versions
              </Button>
            ) : null}
          </ScrollArea>
        </SheetContent>
      </Sheet>
      {restoring === null ? null : (
        <RestoreDialog page={page} version={restoring} onClose={() => setRestoring(null)} />
      )}
    </>
  );
}

type VersionRowProps = {
  version: Version;
  isCurrent: boolean;
  isExpanded: boolean;
  onToggle: () => void;
  onRestore: () => void;
};

function VersionRow({ version, isCurrent, isExpanded, onToggle, onRestore }: VersionRowProps) {
  return (
    <li className="rounded-lg border">
      <div className="flex items-center justify-between gap-3 px-3 py-2.5">
        <div className="min-w-0">
          <p className="flex flex-wrap items-center gap-x-2 text-sm">
            <span className="font-medium">v{version.version}</span>
            <span className="text-muted-foreground">{describeOperation(version)}</span>
            {isCurrent ? (
              <span className="rounded bg-secondary px-1.5 text-[11px] font-medium">Current</span>
            ) : null}
          </p>
          <p className="mt-0.5 flex items-center gap-2 text-xs text-muted-foreground">
            <AuthorBadge author={version.author} />
            <time dateTime={version.at}>{formatRelativeTime(version.at)}</time>
          </p>
        </div>
        <div className="flex shrink-0 gap-1">
          <Button
            variant="ghost"
            size="sm"
            aria-expanded={isExpanded}
            aria-label={`${isExpanded ? "Hide" : "Show"} changes in version ${version.version}`}
            onClick={onToggle}
          >
            {isExpanded ? "Hide changes" : "Changes"}
          </Button>
          {isCurrent ? null : (
            <Button
              variant="outline"
              size="sm"
              aria-label={`Restore version ${version.version}`}
              onClick={onRestore}
            >
              Restore
            </Button>
          )}
        </div>
      </div>
      {isExpanded ? (
        <div className="border-t p-2">
          <DiffView diff={version.diff} />
        </div>
      ) : null}
    </li>
  );
}
