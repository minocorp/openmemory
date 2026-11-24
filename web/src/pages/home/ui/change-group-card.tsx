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
    <li>
      <button
        type="button"
        aria-expanded={isOpen}
        aria-label={`${title} by ${group.author}`}
        onClick={() => setIsOpen(!isOpen)}
        className="flex w-full items-center gap-3 px-3.5 py-3 text-left -outline-offset-2 transition-colors first:rounded-t-lg hover:bg-accent/60"
      >
        <span className="min-w-0 flex-1">
          <span className="block truncate font-medium">{title}</span>
          <span className="mt-1 flex flex-wrap items-center gap-x-3 gap-y-0.5 text-xs text-muted-foreground">
            <AuthorBadge
              author={group.author}
              displayName={group.author_display_name}
              className="text-foreground"
            />
            <span>{pageCountLabel(changes.length)}</span>
            <time dateTime={group.at}>{formatRelativeTime(group.at)}</time>
          </span>
        </span>
        <ChevronRight
          className={cn(
            "size-4 shrink-0 text-muted-foreground transition-transform",
            isOpen && "rotate-90",
          )}
        />
      </button>
      {isOpen ? (
        <div className="grid gap-4 px-3.5 pt-1 pb-3.5">
          {changes.map((change) => (
            <div key={change.pageId} className="grid gap-1.5">
              <p className="flex items-baseline gap-2 text-xs">
                <span className="truncate font-mono">{change.path}</span>
                <span className="text-muted-foreground">{describeOperation(change.op)}</span>
              </p>
              <DiffView diff={change.diff} />
            </div>
          ))}
          <RevertGroupButton groupId={group.id} />
        </div>
      ) : null}
    </li>
  );
}
