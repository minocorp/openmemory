const conditionPattern = /^[\p{L}_][\p{L}\p{N}_.-]*(?:!=|<|>|=|~|:exists$)/u;

export type SearchInput = {
  text: string;
  conditions: string[];
};

export function parseSearchInput(raw: string): SearchInput {
  const tokens = raw.trim().split(/\s+/).filter(Boolean);
  const conditions = tokens.filter((token) => conditionPattern.test(token));
  const text = tokens.filter((token) => !conditionPattern.test(token)).join(" ");
  return { text, conditions };
}

export function isSearchable({ text, conditions }: SearchInput): boolean {
  return text !== "" || conditions.length > 0;
}
