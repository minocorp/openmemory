import type { components } from "@/shared/api";

export type ChangeGroup = components["schemas"]["ChangeGroup"];

export type PageChange = { path: string; pageId: number; op: string; diff: string };

const verbs: Record<string, string> = {
  create: "created",
  edit: "edited",
  move: "moved",
  relink: "relinked",
  delete: "deleted",
  revert: "reverted",
};

export function newestFirst(groups: ChangeGroup[]): ChangeGroup[] {
  return groups.toSorted((a, b) => b.at.localeCompare(a.at));
}

export function pageChanges(group: ChangeGroup): PageChange[] {
  const byPage = new Map<number, PageChange>();
  for (const version of group.versions.toSorted((a, b) => a.seq - b.seq)) {
    const previous = byPage.get(version.page_id);
    byPage.set(version.page_id, {
      pageId: version.page_id,
      path: version.path,
      op: previous?.op === "create" ? "create" : version.op,
      diff: previous ? `${previous.diff}\n${version.diff}` : version.diff,
    });
  }
  return [...byPage.values()];
}

export function describeOperation(op: string): string {
  return verbs[op] ?? op;
}

export function groupTitle(group: ChangeGroup): string {
  return group.task ?? (pageChanges(group).length === 1 ? "Edit" : "Edits");
}

export function pageCountLabel(count: number): string {
  return count === 1 ? "1 page" : `${count} pages`;
}
