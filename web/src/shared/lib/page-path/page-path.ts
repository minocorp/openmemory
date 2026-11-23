const controlCharacters = /\p{Cc}/u;

export function normalizePath(raw: string): string {
  return raw.normalize("NFC").replace(/^\/+|\/+$/g, "");
}

export function splitPath(path: string): string[] {
  return path === "" ? [] : path.split("/");
}

export function baseName(path: string): string {
  return splitPath(path).at(-1) ?? "";
}

export function parentFolder(path: string): string {
  return splitPath(path).slice(0, -1).join("/");
}

export function joinPath(folder: string, name: string): string {
  return folder === "" ? name : `${folder}/${name}`;
}

export function ancestorFolders(path: string): string[] {
  const segments = splitPath(path).slice(0, -1);
  return segments.map((_, index) => segments.slice(0, index + 1).join("/"));
}

export function isInsideFolder(path: string, folder: string): boolean {
  return path.startsWith(`${folder}/`);
}

export type PathProblem = "empty" | "segment" | "character";

export function findPathProblem(raw: string): PathProblem | null {
  const path = normalizePath(raw);
  if (path === "") {
    return "empty";
  }
  if (raw.endsWith("/")) {
    return "segment";
  }
  const segments = splitPath(path);
  if (segments.some((segment) => segment === "" || segment === "." || segment === "..")) {
    return "segment";
  }
  return controlCharacters.test(path) ? "character" : null;
}

const pagesPrefix = "/p/";

export function pagePathFromPathname(pathname: string): string | null {
  if (!pathname.startsWith(pagesPrefix)) {
    return null;
  }
  try {
    return normalizePath(decodeURIComponent(pathname.slice(pagesPrefix.length)));
  } catch {
    return null;
  }
}

export function pathnameOfPagePath(path: string): string {
  return `${pagesPrefix}${splitPath(path).map(encodeURIComponent).join("/")}`;
}
