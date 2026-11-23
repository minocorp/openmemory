export type AuthorKind = "person" | "agent";

export type Author = { kind: AuthorKind; name: string; owner: string | null };

const linkUpdateSuffix = " (link update)";

export function parseAuthor(author: string): Author {
  const actor = author.endsWith(linkUpdateSuffix)
    ? author.slice(0, -linkUpdateSuffix.length)
    : author;
  const separator = actor.indexOf("/");
  if (separator === -1) {
    return { kind: "person", name: author, owner: null };
  }
  return {
    kind: "agent",
    name: actor.slice(separator + 1) + (author === actor ? "" : linkUpdateSuffix),
    owner: actor.slice(0, separator),
  };
}
