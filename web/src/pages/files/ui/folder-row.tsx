import { Link } from "@tanstack/react-router";
import { Folder } from "lucide-react";

type FolderRowProps = { path: string; name: string; fileCount: number };

export function FolderRow({ path, name, fileCount }: FolderRowProps) {
  return (
    <li>
      <Link
        to="/files"
        search={{ folder: path, q: "" }}
        className="flex items-center gap-3 rounded-md px-2 py-2 pr-9 text-sm hover:bg-accent/60"
      >
        <Folder className="size-4 shrink-0 text-muted-foreground" />
        <span className="min-w-0 flex-1 truncate">{name}</span>
        <span className="text-muted-foreground tabular-nums">{fileCount}</span>
      </Link>
    </li>
  );
}
