import { useQuery } from "@tanstack/react-query";
import { Search, Upload } from "lucide-react";
import { type DragEvent, useRef, useState } from "react";

import { treeQuery } from "@/entities/tree";
import { DeleteFileDialog } from "@/features/delete-file";
import { UploadList, useUploads } from "@/features/upload-file";
import { Button } from "@/shared/ui/button";
import { ErrorState } from "@/shared/ui/error-state";
import { Input } from "@/shared/ui/input";
import { Skeleton } from "@/shared/ui/skeleton";

import type { FilesSearch } from "../model/search";
import { FilesBreadcrumb } from "./files-breadcrumb";
import { FilesListing } from "./files-listing";

type FilesPageProps = {
  search: FilesSearch;
  onSearchChange: (next: FilesSearch) => void;
};

type DeleteTarget = { id: number; path: string } | null;

const hasFiles = (event: DragEvent) => event.dataTransfer.types.includes("Files");

export function FilesPage({ search, onSearchChange }: FilesPageProps) {
  const tree = useQuery(treeQuery());
  const uploads = useUploads(search.folder);
  const input = useRef<HTMLInputElement>(null);
  const [isDragging, setIsDragging] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<DeleteTarget>(null);

  const handleDrop = (event: DragEvent) => {
    event.preventDefault();
    setIsDragging(false);
    void uploads.upload([...event.dataTransfer.files]);
  };

  const handleDragOver = (event: DragEvent) => {
    if (hasFiles(event)) {
      event.preventDefault();
      setIsDragging(true);
    }
  };

  return (
    <div
      className="relative flex h-full flex-col"
      onDragOver={handleDragOver}
      onDragLeave={(event) => {
        const next = event.relatedTarget;
        if (!(next instanceof Node) || !event.currentTarget.contains(next)) {
          setIsDragging(false);
        }
      }}
      onDrop={handleDrop}
    >
      <header className="flex h-12 shrink-0 items-center justify-between gap-3 border-b px-4 sm:px-6">
        <FilesBreadcrumb folder={search.folder} />
        <Button size="sm" onClick={() => input.current?.click()}>
          <Upload /> Upload
        </Button>
        <input
          ref={input}
          type="file"
          multiple
          hidden
          aria-label="Choose files"
          onChange={(event) => {
            void uploads.upload([...(event.target.files ?? [])]);
            event.target.value = "";
          }}
        />
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className="mx-auto grid max-w-3xl gap-4 px-4 py-6 sm:px-6">
          <div className="relative">
            <Search className="pointer-events-none absolute top-2 left-2.5 size-4 text-muted-foreground" />
            <Input
              type="search"
              aria-label="Filter files"
              placeholder="Filter files"
              value={search.q}
              onChange={(event) => onSearchChange({ ...search, q: event.target.value })}
              className="pl-8"
            />
          </div>
          <UploadList items={uploads.items} onDismiss={uploads.dismiss} />
          {tree.isPending ? <Skeleton className="h-32" /> : null}
          {tree.isError ? (
            <ErrorState error={tree.error} onRetry={() => void tree.refetch()} />
          ) : null}
          {tree.data ? (
            <FilesListing
              data={tree.data}
              search={search}
              onDelete={setDeleteTarget}
              onUpload={() => input.current?.click()}
            />
          ) : null}
        </div>
      </div>
      {isDragging ? (
        <div className="pointer-events-none absolute inset-3 grid place-items-center rounded-lg border border-dashed border-ring bg-background/90 text-sm">
          Drop files to upload to {search.folder === "" ? "Files" : search.folder}
        </div>
      ) : null}
      {deleteTarget ? (
        <DeleteFileDialog file={deleteTarget} onClose={() => setDeleteTarget(null)} />
      ) : null}
    </div>
  );
}
