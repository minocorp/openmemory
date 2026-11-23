import type { UnseenPageChanges } from "@/entities/change";
import { AuthorBadge } from "@/entities/version";
import { formatRelativeTime } from "@/shared/lib/relative-time";
import { Button } from "@/shared/ui/button";

type ChangesIndicatorProps = {
  unseen: UnseenPageChanges;
  onOpen: () => void;
};

export function ChangesIndicator({ unseen, onOpen }: ChangesIndicatorProps) {
  const count = unseen.changes.length;
  return (
    <Button
      variant="ghost"
      size="sm"
      className="gap-2 font-normal text-muted-foreground"
      onClick={onOpen}
    >
      <AuthorBadge author={unseen.author} className="text-foreground max-sm:hidden" />
      {unseen.authorCount > 1 ? <span className="max-sm:hidden">and others</span> : null}
      <time dateTime={unseen.at} className="max-sm:hidden">
        {formatRelativeTime(unseen.at)}
      </time>
      <span className="text-foreground">View changes</span>
      {count > 1 ? <span>({count})</span> : null}
    </Button>
  );
}
