import { describe, expect, it } from "vitest";

import { buildTreeNodes } from "./tree-nodes";

const tree = {
  folders: [
    { path: "customers", pages: 2, files: 0 },
    { path: "contracts", pages: 0, files: 1 },
  ],
  pages: [
    { id: 1, path: "start", updated_at: "2026-11-23T00:00:00Z", version: 1 },
    { id: 2, path: "customers/acme", updated_at: "2026-11-23T00:00:00Z", version: 1 },
    { id: 3, path: "customers/zeta/notes", updated_at: "2026-11-23T00:00:00Z", version: 1 },
  ],
  files: [{ id: 9, path: "contracts/acme.pdf", mime: "application/pdf", size: 10 }],
};

describe("tree nodes", () => {
  it("nests folders first, then pages, then files, sorted by name", () => {
    const nodes = buildTreeNodes(tree);
    expect(nodes.map((node) => node.path)).toEqual(["contracts", "customers", "start"]);
    const customers = nodes[1];
    expect(customers?.kind === "folder" && customers.children.map((node) => node.path)).toEqual([
      "customers/zeta",
      "customers/acme",
    ]);
  });

  it("creates folders that hold content but are not listed", () => {
    const nodes = buildTreeNodes({ ...tree, folders: [] });
    expect(nodes.map((node) => node.kind)).toEqual(["folder", "folder", "page"]);
  });

  it("carries the recursive count of listed folders", () => {
    const customers = buildTreeNodes(tree).find((node) => node.path === "customers");
    expect(customers?.kind === "folder" && customers.count).toBe(2);
  });
});
