import { Link } from "@tanstack/react-router";
import { ChevronRight, File, FileText } from "lucide-react";

import type { TreeNode } from "@/entities/tree";
import { cn } from "@/shared/lib/cn";

import { isFolderOpen, type FolderOverrides } from "../model/folder-state";

type TreeItemProps = {
  node: TreeNode;
  depth: number;
  selectedPath: string | null;
  overrides: FolderOverrides;
  onToggle: (folder: string, isOpen: boolean) => void;
};

const rowStyle =
  "flex w-full items-center gap-1.5 rounded-md py-[5px] pr-2 text-left text-[13.5px] outline-none hover:bg-sidebar-accent focus-visible:ring-2 focus-visible:ring-sidebar-ring";

export function TreeItem({ node, depth, selectedPath, overrides, onToggle }: TreeItemProps) {
  const indent = { paddingLeft: `${8 + depth * 14}px` };
  if (node.kind === "folder") {
    const isOpen = isFolderOpen(node.path, overrides, selectedPath);
    return (
      <li>
        <button
          type="button"
          className={rowStyle}
          style={indent}
          aria-expanded={isOpen}
          onClick={() => onToggle(node.path, !isOpen)}
        >
          <ChevronRight
            className={cn(
              "size-3 shrink-0 text-muted-foreground transition-transform",
              isOpen && "rotate-90",
            )}
          />
          <span className="min-w-0 flex-1 truncate">{node.name}</span>
          <span className="text-xs text-muted-foreground tabular-nums">{node.count}</span>
        </button>
        {isOpen ? (
          <ul>
            {node.children.map((child) => (
              <TreeItem
                key={`${child.kind}:${child.path}`}
                node={child}
                depth={depth + 1}
                selectedPath={selectedPath}
                overrides={overrides}
                onToggle={onToggle}
              />
            ))}
          </ul>
        ) : null}
      </li>
    );
  }
  const isSelected = selectedPath === node.path;
  const Icon = node.kind === "page" ? FileText : File;
  return (
    <li>
      <Link
        to="/p/$"
        params={{ _splat: node.path }}
        className={cn(rowStyle, isSelected && "bg-sidebar-accent font-semibold")}
        style={{ paddingLeft: `${8 + depth * 14 + 16}px` }}
        aria-current={isSelected ? "page" : undefined}
      >
        <Icon className="size-3.5 shrink-0 text-muted-foreground" />
        <span
          className={cn("min-w-0 flex-1 truncate", node.kind === "file" && "text-muted-foreground")}
        >
          {node.name}
        </span>
      </Link>
    </li>
  );
}
