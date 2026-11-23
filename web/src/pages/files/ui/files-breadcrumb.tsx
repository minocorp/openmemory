import { Link } from "@tanstack/react-router";

import { ancestorFolders, baseName } from "@/shared/lib/page-path";

type FilesBreadcrumbProps = { folder: string };

export function FilesBreadcrumb({ folder }: FilesBreadcrumbProps) {
  const trail = folder === "" ? [] : ancestorFolders(`${folder}/x`);
  return (
    <nav aria-label="Folder" className="flex min-w-0 items-center gap-1.5 text-[13px]">
      {folder === "" ? (
        <span className="font-medium">Files</span>
      ) : (
        <Link to="/files" search={{ folder: "", q: "" }} className="text-muted-foreground">
          Files
        </Link>
      )}
      {trail.map((path) => (
        <span key={path} className="flex min-w-0 items-center gap-1.5">
          <span className="text-muted-foreground">/</span>
          {path === folder ? (
            <span className="truncate font-medium">{baseName(path)}</span>
          ) : (
            <Link
              to="/files"
              search={{ folder: path, q: "" }}
              className="truncate text-muted-foreground"
            >
              {baseName(path)}
            </Link>
          )}
        </span>
      ))}
    </nav>
  );
}
