import type { components } from "@/shared/api";
import { baseName, parentFolder } from "@/shared/lib/page-path";

export type TreeData = components["schemas"]["TreeView"];

export type TreeNode =
  | { kind: "folder"; path: string; name: string; count: number; children: TreeNode[] }
  | { kind: "page" | "file"; path: string; name: string; id: number };

type FolderNode = Extract<TreeNode, { kind: "folder" }>;

const kindOrder = { folder: 0, page: 1, file: 2 } as const;

export function buildTreeNodes(tree: TreeData): TreeNode[] {
  const folders = new Map<string, FolderNode>();
  const roots: TreeNode[] = [];
  const counts = new Map(tree.folders.map((folder) => [folder.path, folder.pages + folder.files]));

  const childrenOf = (folder: string): TreeNode[] => {
    if (folder === "") {
      return roots;
    }
    const existing = folders.get(folder);
    if (existing) {
      return existing.children;
    }
    const created: FolderNode = {
      kind: "folder",
      path: folder,
      name: baseName(folder),
      count: counts.get(folder) ?? 0,
      children: [],
    };
    folders.set(folder, created);
    childrenOf(parentFolder(folder)).push(created);
    return created.children;
  };

  for (const folder of tree.folders) {
    childrenOf(folder.path);
  }
  for (const page of tree.pages) {
    childrenOf(parentFolder(page.path)).push({
      kind: "page",
      id: page.id,
      path: page.path,
      name: baseName(page.path),
    });
  }
  for (const file of tree.files) {
    childrenOf(parentFolder(file.path)).push({
      kind: "file",
      id: file.id,
      path: file.path,
      name: baseName(file.path),
    });
  }
  return sortNodes(roots);
}

function sortNodes(nodes: TreeNode[]): TreeNode[] {
  return nodes
    .toSorted(
      (a, b) =>
        kindOrder[a.kind] - kindOrder[b.kind] ||
        a.name.localeCompare(b.name, undefined, { numeric: true }),
    )
    .map((node) =>
      node.kind === "folder" ? { ...node, children: sortNodes(node.children) } : node,
    );
}
