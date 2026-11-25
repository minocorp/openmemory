import { ancestorFolders } from "@/shared/lib/page-path";
import { z } from "@/shared/lib/zod";

export type FolderOverrides = Readonly<Record<string, boolean>>;

export const folderOverridesSchema = z.record(z.string(), z.boolean());

export function isFolderOpen(
  folder: string,
  overrides: FolderOverrides,
  selectedPath: string | null,
): boolean {
  const explicit = overrides[folder];
  if (explicit !== undefined) {
    return explicit;
  }
  return selectedPath !== null && ancestorFolders(selectedPath).includes(folder);
}

export function revealSelection(overrides: FolderOverrides, selectedPath: string): FolderOverrides {
  const hidden = new Set(ancestorFolders(selectedPath));
  return Object.fromEntries(Object.entries(overrides).filter(([folder]) => !hidden.has(folder)));
}
