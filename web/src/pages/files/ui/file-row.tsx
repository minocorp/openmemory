import { Link } from "@tanstack/react-router";
import { FileText, MoreHorizontal } from "lucide-react";

import { fileTypeLabel, formatFileSize } from "@/entities/file";
import { baseName, parentFolder } from "@/shared/lib/page-path";
import { Button } from "@/shared/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/shared/ui/dropdown-menu";

type FileRowProps = {
  file: { id: number; path: string; mime: string; size: number };
  isPathShown: boolean;
  onDelete: () => void;
};

export function FileRow({ file, isPathShown, onDelete }: FileRowProps) {
  const folder = parentFolder(file.path);
  return (
    <li className="group flex items-center gap-1 rounded-md hover:bg-accent/60">
      <Link
        to="/p/$"
        params={{ _splat: file.path }}
        className="flex min-w-0 flex-1 items-center gap-3 rounded-md px-2 py-2 text-sm"
      >
        <FileText className="size-4 shrink-0 text-muted-foreground" />
        <span className="min-w-0 flex-1 truncate">
          {baseName(file.path)}
          {isPathShown && folder !== "" ? (
            <span className="ml-2 text-muted-foreground">{folder}</span>
          ) : null}
        </span>
        <span className="w-24 shrink-0 text-muted-foreground max-sm:hidden">
          {fileTypeLabel(file.mime)}
        </span>
        <span className="w-16 shrink-0 text-right text-muted-foreground tabular-nums">
          {formatFileSize(file.size)}
        </span>
      </Link>
      <DropdownMenu>
        <DropdownMenuTrigger
          render={<Button variant="ghost" size="icon-sm" className="shrink-0" />}
          aria-label={`Actions for ${baseName(file.path)}`}
        >
          <MoreHorizontal className="size-4" />
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem variant="destructive" onClick={onDelete}>
            Delete
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </li>
  );
}
