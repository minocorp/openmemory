import { useSuspenseQuery } from "@tanstack/react-query";
import { Link, useRouterState } from "@tanstack/react-router";
import { Home, Plus, Search } from "lucide-react";
import { Suspense, useState } from "react";

import { buildTreeNodes, treeQuery } from "@/entities/tree";
import { CreatePageDialog } from "@/features/manage-page";
import { cn } from "@/shared/lib/cn";
import { pagePathFromPathname, parentFolder } from "@/shared/lib/page-path";
import { openSearch } from "@/shared/lib/search-palette";
import { Button } from "@/shared/ui/button";
import { Skeleton } from "@/shared/ui/skeleton";

import { revealSelection, type FolderOverrides } from "../model/folder-state";
import { TreeItem } from "./tree-item";
import { UserMenu } from "./user-menu";

type TreePanelProps = {
  className?: string;
  hasUserMenu?: boolean;
};

export function SidebarTree() {
  return (
    <TreePanel
      className="hidden h-svh w-[248px] shrink-0 border-r bg-sidebar sm:flex"
      hasUserMenu
    />
  );
}

export function PagesPanel() {
  return <TreePanel className="h-full bg-background" />;
}

function TreePanel({ className, hasUserMenu = false }: TreePanelProps) {
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const selectedPath = pagePathFromPathname(pathname);
  const [isCreating, setIsCreating] = useState(false);
  const [overrides, setOverrides] = useState<FolderOverrides>({});
  const [revealedFor, setRevealedFor] = useState(selectedPath);

  if (revealedFor !== selectedPath) {
    setRevealedFor(selectedPath);
    if (selectedPath) {
      setOverrides(revealSelection(overrides, selectedPath));
    }
  }

  const toggleFolder = (folder: string, isOpen: boolean) =>
    setOverrides((previous) => ({ ...previous, [folder]: isOpen }));

  return (
    <aside className={cn("flex flex-col", className)}>
      <div className="flex items-center justify-between px-3 pt-3.5 pb-2">
        <span className="px-2 text-sm font-semibold">Memory</span>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label="New page"
          onClick={() => setIsCreating(true)}
        >
          <Plus />
        </Button>
      </div>
      <div className="px-3 pb-2">
        <SearchButton />
      </div>
      <nav aria-label="Pages and files" className="flex min-h-0 flex-1 flex-col px-2">
        {hasUserMenu ? (
          <Link
            to="/"
            activeOptions={{ exact: true }}
            className="mb-1 flex items-center gap-2 rounded-md px-2 py-[5px] text-[13.5px] hover:bg-sidebar-accent [&.active]:bg-sidebar-accent [&.active]:font-semibold"
          >
            <Home className="size-3.5 text-muted-foreground" /> Home
          </Link>
        ) : null}
        <div className="min-h-0 flex-1 overflow-y-auto pb-4">
          <Suspense fallback={<TreeSkeleton />}>
            <TreeNodes selectedPath={selectedPath} overrides={overrides} onToggle={toggleFolder} />
          </Suspense>
        </div>
      </nav>
      {hasUserMenu ? (
        <div className="border-t p-2">
          <Suspense fallback={<Skeleton className="h-7" />}>
            <UserMenu />
          </Suspense>
        </div>
      ) : null}
      {isCreating ? (
        <CreatePageDialog
          initialPath={newPageFolder(selectedPath)}
          onClose={() => setIsCreating(false)}
        />
      ) : null}
    </aside>
  );
}

function SearchButton() {
  return (
    <button
      type="button"
      onClick={openSearch}
      className="flex h-8 w-full items-center gap-2 rounded-lg border bg-background px-2.5 text-[13px] text-muted-foreground outline-none hover:bg-muted focus-visible:ring-3 focus-visible:ring-ring/50"
    >
      <Search className="size-3.5" />
      <span className="flex-1 text-left">Search</span>
      <kbd className="rounded border px-1 text-[11px] max-sm:hidden">⌘K</kbd>
    </button>
  );
}

function newPageFolder(selectedPath: string | null): string {
  const folder = selectedPath ? parentFolder(selectedPath) : "";
  return folder === "" ? "" : `${folder}/`;
}

type TreeNodesProps = {
  selectedPath: string | null;
  overrides: FolderOverrides;
  onToggle: (folder: string, isOpen: boolean) => void;
};

function TreeNodes({ selectedPath, overrides, onToggle }: TreeNodesProps) {
  const { data: tree } = useSuspenseQuery(treeQuery());
  const nodes = buildTreeNodes(tree);

  if (nodes.length === 0) {
    return <p className="px-2 py-3 text-sm text-muted-foreground">No pages yet.</p>;
  }
  return (
    <ul>
      {nodes.map((node) => (
        <TreeItem
          key={`${node.kind}:${node.path}`}
          node={node}
          depth={0}
          selectedPath={selectedPath}
          overrides={overrides}
          onToggle={onToggle}
        />
      ))}
    </ul>
  );
}

function TreeSkeleton() {
  return (
    <div className="grid gap-2 px-2 py-2">
      <Skeleton className="h-4 w-24" />
      <Skeleton className="h-4 w-32" />
      <Skeleton className="h-4 w-20" />
    </div>
  );
}
