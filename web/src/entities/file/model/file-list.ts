import type { components } from "@/shared/api";
import {
  baseName,
  isInsideFolder,
  joinPath,
  parentFolder,
  splitPath,
} from "@/shared/lib/page-path";

type TreeData = components["schemas"]["TreeView"];
type TreeFile = TreeData["files"][number];

export type FolderEntry = { path: string; name: string; fileCount: number };

export type FolderListing = { folders: FolderEntry[]; files: TreeFile[] };

const byName = (a: { path: string }, b: { path: string }) =>
  baseName(a.path).localeCompare(baseName(b.path));

export function listFolder(tree: TreeData, folder: string): FolderListing {
  const inside = tree.files.filter((file) =>
    folder === "" ? true : isInsideFolder(file.path, folder),
  );
  const folders = new Map<string, number>();
  const files: TreeFile[] = [];
  for (const file of inside) {
    if (parentFolder(file.path) === folder) {
      files.push(file);
      continue;
    }
    const next = splitPath(file.path)[splitPath(folder).length] ?? "";
    const path = joinPath(folder, next);
    folders.set(path, (folders.get(path) ?? 0) + 1);
  }
  return {
    folders: [...folders]
      .map(([path, fileCount]) => ({ path, name: baseName(path), fileCount }))
      .toSorted(byName),
    files: files.toSorted(byName),
  };
}

export function searchFiles(tree: TreeData, query: string): TreeFile[] {
  const needle = query.trim().toLowerCase();
  return tree.files.filter((file) => file.path.toLowerCase().includes(needle)).toSorted(byName);
}

const sizeUnits = ["B", "KB", "MB", "GB"] as const;

export function formatFileSize(bytes: number): string {
  const unit = Math.min(Math.floor(Math.log(Math.max(bytes, 1)) / Math.log(1024)), 3);
  const value = bytes / 1024 ** unit;
  const rounded = unit === 0 || value >= 10 ? Math.round(value) : Math.round(value * 10) / 10;
  return `${rounded} ${sizeUnits[unit] ?? "B"}`;
}

const typeLabels: Record<string, string> = {
  "application/pdf": "PDF",
  "text/plain": "Text",
  "text/markdown": "Markdown",
  "text/csv": "CSV",
  "text/html": "HTML",
  "application/xml": "XML",
  "text/xml": "XML",
  "image/svg+xml": "Image SVG",
  "application/vnd.openxmlformats-officedocument.wordprocessingml.document": "Word",
};

export function fileTypeLabel(mime: string): string {
  const known = typeLabels[mime];
  if (known) {
    return known;
  }
  const [family, subtype = ""] = mime.split("/");
  if (family === "image" || family === "audio") {
    return `${family === "image" ? "Image" : "Audio"} ${subtype.toUpperCase()}`;
  }
  return subtype.toUpperCase() || mime;
}
