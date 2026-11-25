import { useQuery, useSuspenseQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { lazy, Suspense } from "react";

import { changesQuery, unseenChangesOfPage } from "@/entities/change";
import { fileByPathQuery } from "@/entities/file";
import { pageQuery } from "@/entities/page";
import { treeQuery } from "@/entities/tree";
import { PropertyRows } from "@/features/edit-fields";
import { PageTitle } from "@/features/manage-page";
import { Skeleton } from "@/shared/ui/skeleton";
import { HistoryDrawer } from "@/widgets/history-drawer";
import { Timeline } from "@/widgets/timeline";

import type { PageSearch } from "../model/search";
import { ChangesNote } from "./changes-note";
import { LazyFileViewer } from "./lazy-file-viewer";
import { PageHeader } from "./page-header";

const BodyEditor = lazy(async () => ({
  default: (await import("@/features/edit-page")).BodyEditor,
}));

const noChangedLines: ReadonlySet<string> = new Set();

type PageScreenProps = {
  id: number;
  search: PageSearch;
  onSearchChange: (next: PageSearch) => void;
};

export function PageScreen({ id, search, onSearchChange }: PageScreenProps) {
  const navigate = useNavigate();
  const { data: page } = useSuspenseQuery(pageQuery(id));
  const { data: tree } = useQuery(treeQuery());
  const { data: changes } = useQuery(changesQuery());
  const unseen = changes ? unseenChangesOfPage(changes, id) : null;
  const isViewingChanges = Boolean(unseen && search.changes);
  const changedLines = unseen && isViewingChanges ? unseen.addedLines : noChangedLines;
  const openPage = (path: string) => void navigate({ to: "/p/$", params: { _splat: path } });
  const linkTargets = [
    ...(tree?.pages.map((entry) => ({ path: entry.path, kind: "page" as const })) ?? []),
    ...(tree?.files.map((entry) => ({ path: entry.path, kind: "file" as const })) ?? []),
  ];

  return (
    <div className="flex h-full min-h-0">
      <div className="flex min-w-0 flex-1 flex-col">
        <PageHeader
          page={page}
          unseen={unseen}
          onOpenHistory={() => onSearchChange({ ...search, history: true })}
          onOpenChanges={() => onSearchChange({ ...search, changes: true })}
        />
        <div className="min-h-0 flex-1 overflow-y-auto">
          <article className="mx-auto w-full max-w-2xl px-4 pt-8 pb-32 sm:px-6 sm:pt-14">
            <PageTitle key={page.path} page={page} />
            {unseen && isViewingChanges ? (
              <ChangesNote
                page={page}
                unseen={unseen}
                onClose={() => onSearchChange({ ...search, changes: undefined })}
              />
            ) : null}
            <PropertyRows page={page} changedLines={changedLines} />
            <Suspense fallback={<Skeleton className="h-24" />}>
              <BodyEditor
                page={page}
                changedLines={changedLines}
                linkTargets={linkTargets}
                onOpenLink={openPage}
              />
            </Suspense>
            <Timeline
              page={page}
              changedLines={changedLines}
              openFile={search.file ? { path: search.file, page: search.fpage } : null}
              onOpenLink={openPage}
              onOpenSource={(source) =>
                onSearchChange({
                  ...search,
                  file: source.path,
                  fpage: source.page ?? undefined,
                  quote: source.quote ?? undefined,
                })
              }
            />
          </article>
        </div>
      </div>
      {search.file ? (
        <aside
          className="w-105 shrink-0 border-l bg-background max-sm:fixed max-sm:inset-0 max-sm:z-40 max-sm:w-auto max-sm:border-l-0"
          aria-label="Source file"
        >
          <SourcePanel search={search} onSearchChange={onSearchChange} path={search.file} />
        </aside>
      ) : null}
      {search.history ? (
        <HistoryDrawer
          page={page}
          onClose={() => onSearchChange({ ...search, history: undefined })}
        />
      ) : null}
    </div>
  );
}

type SourcePanelProps = {
  path: string;
  search: PageSearch;
  onSearchChange: (next: PageSearch) => void;
};

function SourcePanel({ path, search, onSearchChange }: SourcePanelProps) {
  const { data: file, isError } = useQuery(fileByPathQuery(path));
  if (isError) {
    return <p className="p-6 text-sm text-muted-foreground">This file is not available.</p>;
  }
  if (!file) {
    return <Skeleton className="m-4 h-64" />;
  }
  return (
    <LazyFileViewer
      file={file}
      page={search.fpage ?? 1}
      quote={search.quote ?? null}
      onPageChange={(fpage) => onSearchChange({ ...search, fpage })}
      onClose={() =>
        onSearchChange({ ...search, file: undefined, fpage: undefined, quote: undefined })
      }
    />
  );
}
