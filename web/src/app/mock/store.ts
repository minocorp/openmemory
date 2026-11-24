import { createPatch } from "diff";

import type { components } from "@/shared/api";

export type Schemas = components["schemas"];
type Operation = Schemas["OperationView"];

export type StoredVersion = {
  version: number;
  path: string;
  content: string;
  author: string;
  at: string;
  op: Operation;
  group: number;
  seq: number;
  diff: string;
  revertsVersion: number | null;
};

export type StoredPage = { id: number; versions: StoredVersion[]; isDeleted: boolean };

export type StoredFile = {
  id: number;
  path: string;
  mime: string;
  bytes: Uint8Array<ArrayBuffer>;
  pageCount: number;
  createdAt: string;
};

type StoredGroup = { id: number; author: string; task: string | null };

export type Change = {
  content: string;
  path: string;
  op: Operation;
  author: string;
  task?: string | null;
  at?: string;
  revertsVersion?: number;
};

export const state = {
  meId: 1,
  isSignedIn: true,
  users: new Array<Schemas["UserView"]>(),
  passkeys: new Array<Schemas["PasskeyView"]>(),
  connections: new Array<Schemas["ConnectionView"]>(),
  invites: new Array<Schemas["InviteView"]>(),
  pages: new Array<StoredPage>(),
  files: new Array<StoredFile>(),
  groups: new Array<StoredGroup>(),
  seq: 0,
  lastSeenSeq: 0,
  nextId: 1,
};

export function nextId(): number {
  state.nextId += 1;
  return state.nextId;
}

export function me(): Schemas["UserView"] {
  const user = state.users.find((entry) => entry.id === state.meId);
  if (!user) {
    throw new Error("The mock has no signed-in user");
  }
  return user;
}

export function current(page: StoredPage): StoredVersion {
  const version = page.versions.at(-1);
  if (!version) {
    throw new Error(`Page ${page.id} has no versions`);
  }
  return version;
}

export function livePages(): StoredPage[] {
  return state.pages.filter((page) => !page.isDeleted);
}

export function pageAt(path: string): StoredPage | undefined {
  return livePages().find((page) => current(page).path === path);
}

export function pageById(id: number): StoredPage | undefined {
  return state.pages.find((page) => page.id === id);
}

function groupFor(author: string, task: string | null): number {
  const last = state.groups.at(-1);
  if (task !== null && last?.author === author && last.task === task) {
    return last.id;
  }
  const id = state.groups.length + 1;
  state.groups.push({ id, author, task });
  return id;
}

export function taskOf(group: number): string | null {
  return state.groups.find((entry) => entry.id === group)?.task ?? null;
}

export function record(page: StoredPage, change: Change): StoredVersion {
  const previous = page.versions.at(-1);
  state.seq += 1;
  const version: StoredVersion = {
    version: (previous?.version ?? 0) + 1,
    path: change.path,
    content: change.content,
    author: change.author,
    at: change.at ?? new Date().toISOString(),
    op: change.op,
    group: groupFor(change.author, change.task ?? null),
    seq: state.seq,
    diff: createPatch(change.path, previous?.content ?? "", change.content),
    revertsVersion: change.revertsVersion ?? null,
  };
  page.versions.push(version);
  return version;
}
