import { useSuspenseQuery } from "@tanstack/react-query";
import { Search } from "lucide-react";
import { Suspense } from "react";

import { changesQuery } from "@/entities/change";
import { openSearch } from "@/shared/lib/search-palette";
import { Button } from "@/shared/ui/button";
import { Skeleton } from "@/shared/ui/skeleton";

import { startPageQuery } from "../api/start-page-query";
import { ChangeGroupCard } from "./change-group-card";
import { FirstRunPanel } from "./first-run-panel";
import { MarkAllSeen } from "./mark-all-seen";

export function HomePage() {
  return (
    <div className="flex h-full flex-col">
      <header className="flex h-12 shrink-0 items-center justify-between border-b px-4 sm:px-6">
        <h1 className="text-sm font-medium">Home</h1>
        <Button variant="ghost" size="sm" className="-mr-2 sm:hidden" onClick={openSearch}>
          <Search /> Search
        </Button>
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className="mx-auto grid max-w-2xl gap-10 px-4 py-8 sm:px-6 sm:py-12">
          <Suspense fallback={<FeedSkeleton />}>
            <HomeContent />
          </Suspense>
        </div>
      </div>
    </div>
  );
}

function HomeContent() {
  const { data: start } = useSuspenseQuery(startPageQuery());
  const { data: changes } = useSuspenseQuery(changesQuery());
  return (
    <>
      {start.exists ? null : <FirstRunPanel />}
      <section aria-label="Changes since you last looked">
        <div className="mb-3 flex min-h-7 items-center justify-between">
          <h2 className="text-sm font-medium">Changes since you last looked</h2>
          {changes.groups.length > 0 ? <MarkAllSeen seq={changes.last_seq} /> : null}
        </div>
        {changes.groups.length === 0 ? (
          <p className="text-sm text-muted-foreground">Nothing new.</p>
        ) : (
          <ul className="divide-y rounded-lg border">
            {changes.groups.map((group) => (
              <ChangeGroupCard key={group.id} group={group} />
            ))}
          </ul>
        )}
      </section>
    </>
  );
}

function FeedSkeleton() {
  return (
    <div className="grid gap-3">
      <Skeleton className="h-4 w-48" />
      <Skeleton className="h-28" />
    </div>
  );
}
