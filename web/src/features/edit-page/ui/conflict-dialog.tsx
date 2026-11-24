import type { Page } from "@/entities/page";
import { AuthorBadge } from "@/entities/version";
import { cn } from "@/shared/lib/cn";
import { formatRelativeTime } from "@/shared/lib/relative-time";
import { Button } from "@/shared/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/shared/ui/dialog";

import { sideBySide, type SideLine } from "../model/side-by-side";

type ConflictDialogProps = {
  theirs: Page;
  mine: string;
  isSaving: boolean;
  onKeepMine: () => void;
  onUseTheirs: () => void;
};

export function ConflictDialog({
  theirs,
  mine,
  isSaving,
  onKeepMine,
  onUseTheirs,
}: ConflictDialogProps) {
  const sides = sideBySide(theirs.body, mine);
  return (
    <Dialog open>
      <DialogContent showCloseButton={false} className="sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>This page changed while you were editing</DialogTitle>
          <DialogDescription>
            Choose which text to keep. Their fields and timeline stay either way.
          </DialogDescription>
        </DialogHeader>
        <div className="grid grid-cols-2 gap-3">
          <Side
            title="Their version"
            meta={
              <span className="flex items-center gap-1.5">
                <AuthorBadge author={theirs.author} />
                <span>{formatRelativeTime(theirs.updated_at)}</span>
              </span>
            }
            lines={sides.theirs}
          />
          <Side title="Your version" meta={<span>Unsaved</span>} lines={sides.mine} />
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={onUseTheirs} disabled={isSaving}>
            Use their version
          </Button>
          <Button onClick={onKeepMine} disabled={isSaving}>
            Keep my version
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

type SideProps = { title: string; meta: React.ReactNode; lines: SideLine[] };

function Side({ title, meta, lines }: SideProps) {
  return (
    <section aria-label={title} className="min-w-0 rounded-lg border">
      <header className="flex h-9 items-center justify-between border-b px-3 text-xs">
        <h3 className="font-medium">{title}</h3>
        <span className="text-muted-foreground">{meta}</span>
      </header>
      <div className="max-h-80 overflow-auto p-2 font-mono text-xs leading-6">
        {lines.map((line, index) => (
          <div
            key={`${index}-${line.text}`}
            className={cn(
              "rounded-sm px-2 break-words whitespace-pre-wrap",
              line.isChanged && "bg-highlight",
            )}
          >
            {line.text || " "}
          </div>
        ))}
      </div>
    </section>
  );
}
