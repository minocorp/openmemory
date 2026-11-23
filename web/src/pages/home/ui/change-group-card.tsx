import { ChevronRight } from "lucide-react";
import { useState } from "react";

import {
  groupTitle,
  pageChanges,
  pageCountLabel,
  describeOperation,
  type ChangeGroup,
} from "@/entities/change";
import { AuthorBadge } from "@/entities/version";
import { RevertGroupButton } from "@/features/revert-group";
import { cn } from "@/shared/lib/cn";
import { formatRelativeTime } from "@/shared/lib/relative-time";
import { DiffView } from "@/shared/ui/diff-view";

export function ChangeGroupCard({ group }: { group: ChangeGroup }) {
  const [isOpen, setIsOpen] = useState(false);
  const changes = pageChanges(group);
  const title = groupTitle(group);
  return (
    <li className="rounded-lg border bg-background">
      <button
        type="button"
        aria-expanded={isOpen}
        aria-label={`${title} by ${group.author}`}
        onClick={() => setIsOpen(!isOpen)}
        className="flex w-full items-start gap-2 rounded-lg px-3 py-2.5 text-left outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
      >
        <ChevronRight
          className={cn(
            "mt-0.5 size-4 shrink-0 text-muted-foreground transition-transform",
            isOpen && "rotate-90",
          )}
        />
        <span className="min-w-0 flex-1">
          <span className="block truncate text-[13.5px] font-medium">{title}</span>
          <span className="mt-1 flex flex-wrap items-center gap-x-2 gap-y-0.5 text-xs text-muted-foreground">
            <AuthorBadge
              author={group.author}
              displayName={group.author_display_name}
              className="text-foreground"
            />
            <span>{pageCountLabel(changes.length)}</span>
            <time dateTime={group.at}>{formatRelativeTime(group.at)}</time>
          </span>
        </span>
      </button>
      {isOpen ? (
        <div className="space-y-3 border-t px-3 py-3">
          {changes.map((change) => (
            <div key={change.pageId}>
              <p className="mb-1 flex items-baseline gap-2 text-xs">
                <span className="truncate font-mono">{change.path}</span>
                <span className="text-muted-foreground">{describeOperation(change.op)}</span>
              </p>
              <div className="rounded-md border bg-card p-1.5">
                <DiffView diff={change.diff} />
              </div>
            </div>
          ))}
          <RevertGroupButton groupId={group.id} />
        </div>
      ) : null}
    </li>
  );
}
