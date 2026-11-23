import { allFolders } from "@/shared/ui/folder-picker";

export function scopeSummary(scopes: string[]): string {
  if (scopes.includes(allFolders)) {
    return "All folders";
  }
  return scopes.length === 0 ? "No folders" : scopes.join(", ");
}
