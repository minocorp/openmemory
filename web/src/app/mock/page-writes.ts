import { appendTimelineLine } from "./page-content";
import {
  current,
  livePages,
  nextId,
  record,
  state,
  type StoredPage,
  type StoredVersion,
} from "./store";

type Write = { author: string; task?: string | null; at?: string };

export function createPage(path: string, content: string, write: Write): StoredPage {
  const page: StoredPage = { id: nextId(), versions: [], isDeleted: false };
  state.pages.push(page);
  record(page, { ...write, path, content, op: "create" });
  return page;
}

export function editPage(page: StoredPage, content: string, write: Write): StoredVersion {
  return record(page, { ...write, path: current(page).path, content, op: "edit" });
}

export function appendTimeline(
  page: StoredPage,
  entry: { text: string; source: string; date: string },
  write: Write,
): StoredVersion {
  const line = `${entry.date} | ${entry.source} — ${entry.text}`;
  return editPage(page, appendTimelineLine(current(page).content, line), write);
}

export function movePage(page: StoredPage, to: string, write: Write): StoredVersion {
  return record(page, { ...write, path: to, content: current(page).content, op: "move" });
}

export function deletePage(page: StoredPage, write: Write) {
  record(page, { ...write, path: current(page).path, content: "", op: "delete" });
  page.isDeleted = true;
}

export function restoreVersion(page: StoredPage, version: number, write: Write): StoredVersion {
  const target = page.versions.find((entry) => entry.version === version);
  if (!target) {
    throw new Error(`Page ${page.id} has no version ${version}`);
  }
  page.isDeleted = false;
  return record(page, {
    ...write,
    path: current(page).path,
    content: target.content,
    op: "revert",
    revertsVersion: version,
  });
}

export function revertGroup(group: number, write: Write): StoredVersion[] {
  const touched = state.pages.filter((page) =>
    page.versions.some((version) => version.group === group),
  );
  return touched.flatMap((page) => {
    const first = page.versions.find((version) => version.group === group);
    if (!first) {
      return [];
    }
    if (first.version === 1) {
      deletePage(page, write);
      return [current(page)];
    }
    return [restoreVersion(page, first.version - 1, write)];
  });
}

export function folderPaths(): string[] {
  const paths = [
    ...livePages().map((page) => current(page).path),
    ...state.files.map((file) => file.path),
  ];
  return [...new Set(paths.flatMap(ancestors))].toSorted();
}

function ancestors(path: string): string[] {
  const parts = path.split("/").slice(0, -1);
  return parts.map((_, index) => parts.slice(0, index + 1).join("/"));
}
