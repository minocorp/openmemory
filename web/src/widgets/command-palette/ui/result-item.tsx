import { FileText, Folder, FolderSearch } from "lucide-react";

import type { SearchResult } from "@/entities/search";
import { baseName, parentFolder } from "@/shared/lib/page-path";
import { CommandItem } from "@/shared/ui/command";

import { optionId } from "../config/palette";

type ResultItemProps = {
  result: SearchResult;
  index: number;
  isActive: boolean;
  onHover: () => void;
  onOpen: () => void;
};

export function ResultItem({ result, index, isActive, onHover, onOpen }: ResultItemProps) {
  if (result.kind === "folder") {
    return (
      <CommandItem
        id={optionId(index)}
        isActive={isActive}
        onPointerMove={onHover}
        onClick={onOpen}
      >
        <span className="flex items-center gap-2 text-sm text-muted-foreground">
          <FolderSearch className="size-3.5 shrink-0" />
          <span className="truncate">Search in {result.path}</span>
          <span className="ml-auto shrink-0 text-xs">{result.matches}</span>
        </span>
      </CommandItem>
    );
  }
  const folder = parentFolder(result.path);
  const Icon = result.kind === "file" ? Folder : FileText;
  return (
    <CommandItem id={optionId(index)} isActive={isActive} onPointerMove={onHover} onClick={onOpen}>
      <span className="flex items-center gap-2">
        <Icon className="size-3.5 shrink-0 text-muted-foreground" />
        <span className="truncate font-medium">{baseName(result.path)}</span>
        {folder === "" ? null : (
          <span className="truncate text-xs text-muted-foreground">{folder}</span>
        )}
        {result.filePage === null ? null : (
          <span className="ml-auto shrink-0 rounded-sm bg-muted px-1.5 text-xs text-muted-foreground">
            p{result.filePage}
          </span>
        )}
      </span>
      {result.line === null ? null : (
        <span className="truncate pl-[22px] text-sm text-muted-foreground">{result.line}</span>
      )}
    </CommandItem>
  );
}
