import { useState } from "react";

import type { UnseenPageChanges } from "@/entities/change";
import type { Page } from "@/entities/page";
import { AuthorBadge } from "@/entities/version";
import { RestoreDialog } from "@/features/restore-version";
import { formatRelativeTime } from "@/shared/lib/relative-time";
import { Button } from "@/shared/ui/button";
import { DiffView } from "@/shared/ui/diff-view";

type ChangesNoteProps = {
  page: Page;
  unseen: UnseenPageChanges;
  onClose: () => void;
};

export function ChangesNote({ page, unseen, onClose }: ChangesNoteProps) {
  const [isReverting, setIsReverting] = useState(false);
  const revertTo = unseen.firstVersion - 1;

  return (
    <section
      aria-label="Changed since you last looked"
      className="mb-6 rounded-lg border bg-card px-3 py-2.5 text-[13px]"
    >
      <div className="flex items-center gap-3">
        <span className="font-medium">Changed since you last looked</span>
        <span className="ml-auto flex gap-1">
          {revertTo >= 1 ? (
            <Button variant="ghost" size="xs" onClick={() => setIsReverting(true)}>
              Revert
            </Button>
          ) : null}
          <Button variant="ghost" size="xs" onClick={onClose}>
            Close
          </Button>
        </span>
      </div>
      <ul className="mt-2 flex flex-col gap-2">
        {unseen.changes.map((change) => (
          <li key={change.version}>
            <div className="mb-1 flex items-center gap-2 text-muted-foreground">
              <AuthorBadge author={change.author} className="text-foreground" />
              <time dateTime={change.at}>{formatRelativeTime(change.at)}</time>
            </div>
            <div className="rounded-md border bg-background p-1.5">
              <DiffView diff={change.diff} />
            </div>
          </li>
        ))}
      </ul>
      {isReverting ? (
        <RestoreDialog page={page} version={revertTo} onClose={() => setIsReverting(false)} />
      ) : null}
    </section>
  );
}
