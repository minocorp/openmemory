import type { components } from "@/shared/api";

type SearchView = components["schemas"]["SearchResults"];
type HitView = components["schemas"]["SearchHit"];

export type SearchResult = {
  key: string;
  kind: "page" | "file" | "folder";
  path: string;
  line: string | null;
  filePage: number | null;
  matches: number | null;
};

export type FolderResults = {
  path: string;
  results: SearchResult[];
};

export type SearchResults = {
  hits: SearchResult[];
  folders: FolderResults[];
};

function toResult(group: string, hit: HitView, index: number): SearchResult {
  return {
    key: `${group}:${hit.kind}:${hit.id}:${hit.line_number ?? "path"}:${index}`,
    kind: hit.kind,
    path: hit.path,
    line: hit.line ?? null,
    filePage: hit.kind === "file" ? (hit.page ?? null) : null,
    matches: null,
  };
}

export function toSearchResults(view: SearchView): SearchResults {
  return {
    hits: view.hits.map((hit, index) => toResult("hits", hit, index)),
    folders: view.folders.map((folder) => ({
      path: folder.path,
      results: [
        ...folder.hits.map((hit, index) => toResult(folder.path, hit, index)),
        {
          key: `folder:${folder.path}`,
          kind: "folder",
          path: folder.path,
          line: null,
          filePage: null,
          matches: folder.matches,
        },
      ],
    })),
  };
}

export function flattenResults({ hits, folders }: SearchResults): SearchResult[] {
  return [...hits, ...folders.flatMap((folder) => folder.results)];
}
