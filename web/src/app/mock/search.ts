import { parsePage } from "./page-content";
import { current, livePages, type Schemas, state } from "./store";

const textDecoder = new TextDecoder();

export function searchView(query: string | null): Schemas["SearchView"] {
  const pages = query === null ? [] : pageHits(query);
  const files = query === null ? [] : fileHits(query);
  const isEmpty = pages.length === 0 && files.length === 0;
  return {
    pages,
    files,
    next_cursor: null,
    no_hits: isEmpty
      ? { query: query ?? "", fields_in_scope: fieldKeys(), folders_out_of_scope: 0 }
      : null,
  };
}

function pageHits(query: string): Schemas["PageHitView"][] {
  return livePages().flatMap((page) => {
    const { path, content, author, at } = current(page);
    const lines = content.split("\n");
    const lineIndex = lines.findIndex((line) => line.includes(query));
    if (lineIndex === -1 && !path.includes(query)) {
      return [];
    }
    return [
      {
        page_id: page.id,
        path,
        author,
        at,
        line: lineIndex === -1 ? null : (lines[lineIndex] ?? null),
        line_number: lineIndex === -1 ? null : lineIndex + 1,
      },
    ];
  });
}

function fileHits(query: string): Schemas["FileHitView"][] {
  return state.files.flatMap((file) => {
    const text = file.mime.startsWith("text/") ? textDecoder.decode(file.bytes) : "";
    const line = text.split("\n").find((entry) => entry.includes(query)) ?? null;
    if (line === null && !file.path.includes(query)) {
      return [];
    }
    return [
      {
        file_id: file.id,
        path: file.path,
        author: "mino",
        at: file.createdAt,
        line,
        line_number: null,
        page: null,
      },
    ];
  });
}

function fieldKeys(): string[] {
  const keys = livePages().flatMap((page) =>
    parsePage(current(page).content).fields.map((field) => field.key),
  );
  return [...new Set(keys)].toSorted();
}
