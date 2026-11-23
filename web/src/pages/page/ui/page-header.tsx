import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { History, MoreHorizontal } from "lucide-react";
import { useState } from "react";

import type { UnseenPageChanges } from "@/entities/change";
import type { Page } from "@/entities/page";
import { meQuery } from "@/entities/user";
import { AuthorBadge } from "@/entities/version";
import { DeletePageDialog, MovePageDialog } from "@/features/manage-page";
import { PurgeDialog } from "@/features/purge";
import { parentFolder, splitPath } from "@/shared/lib/page-path";
import { formatRelativeTime } from "@/shared/lib/relative-time";
import { Button } from "@/shared/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/shared/ui/dropdown-menu";

import { ChangesIndicator } from "./changes-indicator";

type PageHeaderProps = {
  page: Page;
  unseen: UnseenPageChanges | null;
  onOpenHistory: () => void;
  onOpenChanges: () => void;
};

type Dialog = "move" | "delete" | "purge" | null;

export function PageHeader({ page, unseen, onOpenHistory, onOpenChanges }: PageHeaderProps) {
  const [dialog, setDialog] = useState<Dialog>(null);
  const navigate = useNavigate();
  const { data: me } = useQuery(meQuery());
  const folders = splitPath(parentFolder(page.path));
  return (
    <header className="flex h-12 shrink-0 items-center justify-between gap-3 border-b px-4 text-[13px] sm:px-6">
      <p className="min-w-0 truncate text-muted-foreground">
        {folders.map((folder) => `${folder} / `)}
        <span className="font-medium text-foreground">{splitPath(page.path).at(-1)}</span>
      </p>
      <div className="flex items-center gap-3">
        {unseen ? (
          <ChangesIndicator unseen={unseen} onOpen={onOpenChanges} />
        ) : (
          <span className="flex items-center gap-2 text-muted-foreground max-sm:hidden">
            <AuthorBadge author={page.author} className="text-foreground" />
            <time dateTime={page.updated_at}>{formatRelativeTime(page.updated_at)}</time>
          </span>
        )}
        <Button variant="outline" size="sm" aria-label="History" onClick={onOpenHistory}>
          <History /> <span className="max-sm:hidden">History</span>
        </Button>
        <DropdownMenu>
          <DropdownMenuTrigger
            className="grid size-7 place-items-center rounded-md outline-none hover:bg-accent focus-visible:ring-3 focus-visible:ring-ring/50"
            aria-label="Page actions"
          >
            <MoreHorizontal className="size-4" />
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onClick={() => setDialog("move")}>Move or rename</DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem variant="destructive" onClick={() => setDialog("delete")}>
              Delete
            </DropdownMenuItem>
            {me?.is_admin ? (
              <DropdownMenuItem variant="destructive" onClick={() => setDialog("purge")}>
                Purge permanently
              </DropdownMenuItem>
            ) : null}
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
      {dialog === "move" ? <MovePageDialog page={page} onClose={() => setDialog(null)} /> : null}
      {dialog === "purge" ? (
        <PurgeDialog
          kind="page"
          id={page.id}
          path={page.path}
          onClose={() => setDialog(null)}
          onDone={() => void navigate({ to: "/" })}
        />
      ) : null}
      {dialog === "delete" ? (
        <DeletePageDialog page={page} onClose={() => setDialog(null)} />
      ) : null}
    </header>
  );
}
