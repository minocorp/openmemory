import { parsePage } from "./page-content";
import { folderPaths } from "./page-writes";
import {
  current,
  livePages,
  me,
  type Schemas,
  state,
  type StoredFile,
  type StoredPage,
  type StoredVersion,
  taskOf,
} from "./store";

export function pageView(page: StoredPage, at: StoredVersion = current(page)): Schemas["PageView"] {
  return {
    ...parsePage(at.content),
    id: page.id,
    path: at.path,
    content: at.content,
    version: at.version,
    author: at.author,
    updated_at: at.at,
  };
}

export function fileView(file: StoredFile): Schemas["FileView"] {
  return {
    id: file.id,
    path: file.path,
    mime: file.mime,
    size: file.bytes.byteLength,
    page_count: file.pageCount,
    created_at: file.createdAt,
  };
}

export function treeView(): Schemas["TreeView"] {
  const pages = livePages().map((page) => current(page));
  const isInside = (folder: string) => (path: string) => path.startsWith(`${folder}/`);
  return {
    pages: livePages().map((page) => {
      const { path, version, at } = current(page);
      return { id: page.id, path, version, updated_at: at };
    }),
    files: state.files.map(({ id, path, mime, bytes }) => ({
      id,
      path,
      mime,
      size: bytes.byteLength,
    })),
    folders: folderPaths().map((folder) => ({
      path: folder,
      pages: pages.map((page) => page.path).filter(isInside(folder)).length,
      files: state.files.map((file) => file.path).filter(isInside(folder)).length,
    })),
  };
}

export function versionsView(
  page: StoredPage,
  options: { limit: number; before: number | null },
): Schemas["VersionList"] {
  const older = page.versions
    .filter((entry) => options.before === null || entry.version < options.before)
    .toReversed();
  const shown = older.slice(0, options.limit);
  return {
    versions: shown.map((entry) => ({
      version: entry.version,
      path: entry.path,
      author: entry.author,
      at: entry.at,
      op: entry.op,
      group: entry.group,
      diff: entry.diff,
      reverts_version: entry.revertsVersion,
    })),
    next: older.length > shown.length ? String(shown.at(-1)?.version ?? "") : null,
  };
}

export function changesView(): Schemas["Changes"] {
  const unseen = state.pages.flatMap((page) =>
    page.versions
      .filter((entry) => entry.seq > state.lastSeenSeq && entry.author !== me().username)
      .map((entry) => ({ page, entry })),
  );
  const groupIds = [...new Set(unseen.map(({ entry }) => entry.group))];
  const groups = groupIds.map((group) => {
    const members = unseen.filter(({ entry }) => entry.group === group);
    const latest = members.map(({ entry }) => entry).toSorted((a, b) => b.seq - a.seq)[0];
    const author = latest?.author ?? "";
    return {
      id: group,
      author,
      author_display_name: displayNameOf(author),
      task: taskOf(group),
      at: latest?.at ?? "",
      versions: members.map(({ page, entry }) => ({
        page_id: page.id,
        path: entry.path,
        version: entry.version,
        seq: entry.seq,
        op: entry.op,
        diff: entry.diff,
      })),
    };
  });
  return {
    groups: groups.toSorted((a, b) => b.at.localeCompare(a.at)),
    since: state.lastSeenSeq,
    last_seq: state.seq,
  };
}

function displayNameOf(author: string): string {
  const owner = author.split("/")[0] ?? author;
  return state.users.find((user) => user.username === owner)?.display_name ?? owner;
}
