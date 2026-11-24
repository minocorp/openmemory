import { useInfiniteQuery } from "@tanstack/react-query";
import { useState } from "react";

import { versionsQuery, type Page } from "@/entities/page";
import { AuthorBadge, describeOperation, type Version } from "@/entities/version";
import { RestoreDialog } from "@/features/restore-version";
import { formatRelativeTime } from "@/shared/lib/relative-time";
import { Badge } from "@/shared/ui/badge";
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
          <ScrollArea className="min-h-0 flex-1">
            {versions.isPending ? <Skeleton className="m-5 h-24" /> : null}
            <ol className="divide-y border-b">
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
                variant="ghost"
                size="sm"
                className="mx-3 my-3 text-muted-foreground"
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
    <li>
      <div className="flex items-center justify-between gap-3 px-5 py-3">
        <div className="min-w-0">
          <p className="flex flex-wrap items-center gap-x-2">
            <span className="font-medium tabular-nums">v{version.version}</span>
            <span className="text-muted-foreground">{describeOperation(version)}</span>
            {isCurrent ? <Badge>Current</Badge> : null}
          </p>
          <p className="mt-1 flex items-center gap-3 text-xs text-muted-foreground">
            <AuthorBadge author={version.author} />
            <time dateTime={version.at}>{formatRelativeTime(version.at)}</time>
          </p>
        </div>
        <div className="flex shrink-0 gap-1">
          <Button
            variant="ghost"
            size="sm"
            className="text-muted-foreground hover:text-foreground"
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
        <div className="px-5 pb-4">
          <DiffView diff={version.diff} />
        </div>
      ) : null}
    </li>
  );
}
