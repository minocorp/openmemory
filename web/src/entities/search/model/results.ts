import type { components } from "@/shared/api";

type SearchView = components["schemas"]["SearchResults"];

export type SearchResult = {
  key: string;
  kind: "page" | "file";
  path: string;
  line: string | null;
  filePage: number | null;
};

export type SearchResults = {
  pages: SearchResult[];
  files: SearchResult[];
};

export function toSearchResults(view: SearchView): SearchResults {
  return {
    pages: view.pages.map((hit, index) => ({
      key: `page:${hit.page_id}:${hit.line_number ?? "path"}:${index}`,
      kind: "page",
      path: hit.path,
      line: hit.line ?? null,
      filePage: null,
    })),
    files: view.files.map((hit, index) => ({
      key: `file:${hit.file_id}:${hit.page ?? 0}:${hit.line_number ?? "path"}:${index}`,
      kind: "file",
      path: hit.path,
      line: hit.line ?? null,
      filePage: hit.page ?? null,
    })),
  };
}

export function flattenResults({ pages, files }: SearchResults): SearchResult[] {
  return [...pages, ...files];
}
