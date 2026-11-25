import { Link } from "@tanstack/react-router";
import { ChevronRight, File, FileText } from "lucide-react";

import type { TreeNode } from "@/entities/tree";
import { cn } from "@/shared/lib/cn";

import { rowStyle } from "../config/row-style";
import { isFolderOpen, type FolderOverrides } from "../model/folder-state";

type TreeItemProps = {
  node: TreeNode;
  depth: number;
  selectedPath: string | null;
  overrides: FolderOverrides;
  onToggle: (folder: string, isOpen: boolean) => void;
};

const indentRem = (depth: number) => `${0.5 + depth * 0.875}rem`;

export function TreeItem({ node, depth, selectedPath, overrides, onToggle }: TreeItemProps) {
  if (node.kind === "folder") {
    const isOpen = isFolderOpen(node.path, overrides, selectedPath);
    return (
      <li>
        <button
          type="button"
          className={rowStyle}
          style={{ paddingLeft: indentRem(depth) }}
          aria-expanded={isOpen}
          onClick={() => onToggle(node.path, !isOpen)}
        >
          <ChevronRight
            className={cn(
              "size-3.5 shrink-0 text-muted-foreground transition-transform",
              isOpen && "rotate-90",
            )}
          />
          <span className="min-w-0 flex-1 truncate">{node.name}</span>
          <span className="text-xs text-muted-foreground tabular-nums">{node.count}</span>
        </button>
        {isOpen ? (
          <ul className="mt-px grid grid-cols-1 gap-px">
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
        className={cn(rowStyle, isSelected && "bg-sidebar-accent font-medium")}
        style={{ paddingLeft: indentRem(depth) }}
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
