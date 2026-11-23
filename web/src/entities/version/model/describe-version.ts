import type { Version } from "./types";

type Described = Pick<Version, "op" | "reverts_version">;

const labels = {
  create: "created",
  edit: "edited",
  move: "moved",
  relink: "updated links",
  delete: "deleted",
  revert: "reverted",
} as const;

export function describeOperation(version: Described): string {
  if (version.op === "revert" && version.reverts_version !== null) {
    return `restored v${version.reverts_version}`;
  }
  return labels[version.op];
}
