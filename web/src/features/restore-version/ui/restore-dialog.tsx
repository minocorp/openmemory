import { useSuspenseQuery } from "@tanstack/react-query";
import { createPatch } from "diff";
import { Suspense } from "react";

import { pageVersionQuery, type Page } from "@/entities/page";
import { Button } from "@/shared/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/shared/ui/dialog";
import { DiffView } from "@/shared/ui/diff-view";
import { Skeleton } from "@/shared/ui/skeleton";

import { useRestoreVersion } from "../api/use-restore-version";

type RestoreDialogProps = {
  page: Page;
  version: number;
  onClose: () => void;
};

export function RestoreDialog({ page, version, onClose }: RestoreDialogProps) {
  const restore = useRestoreVersion();
  const handleRestore = () =>
    restore.mutate({ id: page.id, version, current: page.version }, { onSuccess: onClose });

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>Restore version {version}?</DialogTitle>
          <DialogDescription>
            The page goes back to this version. Below is what changes in the current text. The
            current version stays in history.
          </DialogDescription>
        </DialogHeader>
        <div className="max-h-96 overflow-auto rounded-lg border p-2">
          <Suspense fallback={<Skeleton className="h-24" />}>
            <OverwriteDiff page={page} version={version} />
          </Suspense>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={handleRestore} disabled={restore.isPending}>
            Restore
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function OverwriteDiff({ page, version }: { page: Page; version: number }) {
  const { data: target } = useSuspenseQuery(pageVersionQuery(page.id, version));
  const diff = createPatch(page.path, page.content, target.content, "current", `v${version}`, {
    context: 3,
  });
  return <DiffView diff={diff} />;
}
