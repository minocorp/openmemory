import { describe, expect, it } from "vitest";

import type { components } from "@/shared/api";

import { fileTypeLabel, formatFileSize, listFolder, searchFiles } from "./file-list";

type TreeData = components["schemas"]["TreeView"];

const file = (id: number, path: string): TreeData["files"][number] => ({
  id,
  path,
  mime: "application/pdf",
  size: 10,
});

const tree: TreeData = {
  folders: [],
  pages: [],
  files: [
    file(1, "contracts/acme.pdf"),
    file(2, "contracts/2026/renewal.pdf"),
    file(3, "contracts/2026/Annex.pdf"),
    file(4, "readme.txt"),
  ],
};

describe("listFolder", () => {
  it("lists top-level folders with recursive file counts and root files", () => {
    const listing = listFolder(tree, "");
    expect(listing.folders).toEqual([{ path: "contracts", name: "contracts", fileCount: 3 }]);
    expect(listing.files.map((entry) => entry.path)).toEqual(["readme.txt"]);
  });

  it("lists the direct children of a folder", () => {
    const listing = listFolder(tree, "contracts");
    expect(listing.folders).toEqual([{ path: "contracts/2026", name: "2026", fileCount: 2 }]);
    expect(listing.files.map((entry) => entry.path)).toEqual(["contracts/acme.pdf"]);
  });
});

describe("searchFiles", () => {
  it("matches any part of the path ignoring case", () => {
    expect(searchFiles(tree, "ANNEX").map((entry) => entry.id)).toEqual([3]);
    expect(searchFiles(tree, "2026").map((entry) => entry.id)).toEqual([3, 2]);
  });
});

describe("formatFileSize", () => {
  it("scales to the nearest unit", () => {
    expect(formatFileSize(0)).toBe("0 B");
    expect(formatFileSize(512)).toBe("512 B");
    expect(formatFileSize(1536)).toBe("1.5 KB");
    expect(formatFileSize(5 * 1024 * 1024)).toBe("5 MB");
  });
});

describe("fileTypeLabel", () => {
  it("names common types", () => {
    expect(fileTypeLabel("application/pdf")).toBe("PDF");
    expect(fileTypeLabel("image/png")).toBe("Image PNG");
    expect(fileTypeLabel("application/vnd.ms-excel")).toBe("VND.MS-EXCEL");
  });
});
