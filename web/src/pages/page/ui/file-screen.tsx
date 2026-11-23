import { useSuspenseQuery } from "@tanstack/react-query";

import { fileQuery } from "@/entities/file";

import type { PageSearch } from "../model/search";
import { LazyFileViewer } from "./lazy-file-viewer";

type FileScreenProps = {
  id: number;
  search: PageSearch;
  onSearchChange: (next: PageSearch) => void;
};

export function FileScreen({ id, search, onSearchChange }: FileScreenProps) {
  const { data: file } = useSuspenseQuery(fileQuery(id));
  return (
    <LazyFileViewer
      file={file}
      page={search.fpage ?? 1}
      quote={search.quote ?? null}
      onPageChange={(fpage) => onSearchChange({ ...search, fpage })}
    />
  );
}
