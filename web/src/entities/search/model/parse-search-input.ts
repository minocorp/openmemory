const conditionPattern = /^[\p{L}_][\p{L}\p{N}_.-]*(?:!=|<|>|=|~|:exists$)/u;
const folderPrefix = "in:";

export type SearchInput = {
  text: string;
  conditions: string[];
  folder: string | null;
};

export function parseSearchInput(raw: string): SearchInput {
  const words: string[] = [];
  const conditions: string[] = [];
  let folder: string | null = null;
  for (const token of raw.trim().split(/\s+/).filter(Boolean)) {
    if (conditionPattern.test(token)) {
      conditions.push(token);
    } else if (token.startsWith(folderPrefix) && token.length > folderPrefix.length) {
      folder = token.slice(folderPrefix.length);
    } else {
      words.push(token);
    }
  }
  return { text: words.join(" "), conditions, folder };
}

export function isSearchable({ text, conditions, folder }: SearchInput): boolean {
  return text !== "" || conditions.length > 0 || folder !== null;
}

export function withFolder(raw: string, folder: string): string {
  const kept = raw
    .trim()
    .split(/\s+/)
    .filter((token) => token !== "" && !token.startsWith(folderPrefix));
  return [...kept, `${folderPrefix}${folder}`].join(" ");
}
