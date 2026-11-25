import { useSuspenseQuery } from "@tanstack/react-query";
import { Link, useRouterState } from "@tanstack/react-router";
import { Folder, Home, Plus, Search } from "lucide-react";
import { Suspense, useState } from "react";

import { buildTreeNodes, treeQuery } from "@/entities/tree";
import { CreatePageDialog } from "@/features/manage-page";
import { cn } from "@/shared/lib/cn";
import { pagePathFromPathname, parentFolder } from "@/shared/lib/page-path";
import { openSearch } from "@/shared/lib/search-palette";
import { Brand } from "@/shared/ui/brand";
import { Button } from "@/shared/ui/button";
import { Skeleton } from "@/shared/ui/skeleton";

import { rowStyle } from "../config/row-style";
import { revealSelection, type FolderOverrides } from "../model/folder-state";
import { TreeItem } from "./tree-item";
import { UserMenu } from "./user-menu";
type TreePanelProps = {
  className?: string;
  isSidebar?: boolean;
};

export function SidebarTree() {
  return <TreePanel className="hidden h-svh w-62 shrink-0 border-r bg-sidebar sm:flex" isSidebar />;
}

export function PagesPanel() {
  return <TreePanel className="h-full bg-background" />;
}

function TreePanel({ className, isSidebar = false }: TreePanelProps) {
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
      <div className="flex h-12 shrink-0 items-center justify-between pr-2 pl-4">
        <Brand />
        <Button
          variant="ghost"
          size="icon-sm"
          className="text-muted-foreground hover:text-foreground"
          aria-label="New page"
          onClick={() => setIsCreating(true)}
        >
          <Plus />
        </Button>
      </div>
      <div className="px-2 pb-2">
        <SearchButton />
      </div>
      <nav aria-label="Pages and files" className="flex min-h-0 flex-1 flex-col">
        {isSidebar ? <SectionLinks /> : null}
        <div className="min-h-0 flex-1 overflow-y-auto px-2 pt-1 pb-4">
          <Suspense fallback={<TreeSkeleton />}>
            <TreeNodes selectedPath={selectedPath} overrides={overrides} onToggle={toggleFolder} />
          </Suspense>
        </div>
      </nav>
      {isSidebar ? (
        <div className="p-2">
          <Suspense fallback={<Skeleton className="h-8" />}>
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

function SectionLinks() {
  const linkStyle = cn(rowStyle, "px-2 [&.active]:bg-sidebar-accent [&.active]:font-medium");
  return (
    <div className="grid gap-px px-2 pb-2">
      <Link to="/" activeOptions={{ exact: true }} className={linkStyle}>
        <Home className="size-4 text-muted-foreground" /> Home
      </Link>
      <Link to="/files" activeOptions={{ includeSearch: false }} className={linkStyle}>
        <Folder className="size-4 text-muted-foreground" /> Files
      </Link>
    </div>
  );
}

function SearchButton() {
  return (
    <button
      type="button"
      onClick={openSearch}
      className="flex h-8 w-full items-center gap-2 rounded-md border bg-background px-2 text-sm text-muted-foreground transition-colors hover:border-input hover:text-foreground"
    >
      <Search className="size-4" />
      <span className="flex-1 text-left">Search</span>
      <kbd className="font-sans text-xs text-muted-foreground max-sm:hidden">⌘K</kbd>
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
    return <p className="px-2 py-1.5 text-sm text-muted-foreground">No pages yet.</p>;
  }
  return (
    <ul className="grid grid-cols-1 gap-px">
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
    <div className="grid gap-2.5 px-2 py-1.5">
      <Skeleton className="h-3.5 w-24" />
      <Skeleton className="h-3.5 w-32" />
      <Skeleton className="h-3.5 w-20" />
    </div>
  );
}
