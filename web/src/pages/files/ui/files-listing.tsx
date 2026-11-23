import { Upload } from "lucide-react";

import { listFolder, searchFiles } from "@/entities/file";
import type { TreeData } from "@/entities/tree";
import { Button } from "@/shared/ui/button";

import type { FilesSearch } from "../model/search";
import { FileRow } from "./file-row";
import { FolderRow } from "./folder-row";

type FilesListingProps = {
  data: TreeData;
  search: FilesSearch;
  onDelete: (file: { id: number; path: string }) => void;
  onUpload: () => void;
};

export function FilesListing({ data, search, onDelete, onUpload }: FilesListingProps) {
  const isFiltering = search.q.trim() !== "";
  const listing = isFiltering
    ? { folders: [], files: searchFiles(data, search.q) }
    : listFolder(data, search.folder);

  if (listing.folders.length === 0 && listing.files.length === 0) {
    return (
      <div className="grid justify-items-center gap-3 py-16 text-sm text-muted-foreground">
        <p>{isFiltering ? "No files match." : "No files here yet."}</p>
        {isFiltering ? null : (
          <Button variant="outline" size="sm" onClick={onUpload}>
            <Upload /> Upload
          </Button>
        )}
      </div>
    );
  }
  return (
    <ul aria-label="Files">
      {listing.folders.map((folder) => (
        <FolderRow key={folder.path} {...folder} />
      ))}
      {listing.files.map((file) => (
        <FileRow
          key={file.id}
          file={file}
          isPathShown={isFiltering}
          onDelete={() => onDelete({ id: file.id, path: file.path })}
        />
      ))}
    </ul>
  );
}
