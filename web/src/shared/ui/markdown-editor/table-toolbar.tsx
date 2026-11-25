import {
  AlignCenter,
  AlignLeft,
  AlignRight,
  BetweenHorizontalEnd,
  BetweenHorizontalStart,
  BetweenVerticalEnd,
  BetweenVerticalStart,
  Ellipsis,
} from "lucide-react";

import type { CellPosition, TableAlign, TableEdit } from "@/shared/lib/markdown";
import { Button } from "@/shared/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/shared/ui/dropdown-menu";

import type { TableFocus } from "./preview/context";
import type { ToolbarPosition } from "./selection-toolbar";
import { ToolbarButton } from "./toolbar-button";

type TableToolbarProps = {
  focus: TableFocus;
  position: ToolbarPosition;
  onEdit: (edit: TableEdit, next: CellPosition) => void;
  onDeleteTable: () => void;
};

const alignments = [
  { align: "left", label: "Align left", icon: AlignLeft },
  { align: "center", label: "Align center", icon: AlignCenter },
  { align: "right", label: "Align right", icon: AlignRight },
] as const;

export function TableToolbar({ focus, position, onEdit, onDeleteTable }: TableToolbarProps) {
  const { row, column, rowCount, columnCount } = focus;
  const here = { row, column };
  const setAlign = (align: TableAlign) =>
    onEdit({ kind: "align", column, align: focus.align === align ? null : align }, here);
  return (
    <div
      role="toolbar"
      aria-label="Table"
      data-table-tools=""
      className="absolute z-20 flex items-center gap-0.5 rounded-lg border bg-popover p-1 shadow-popover"
      style={{ top: position.top, left: position.left }}
    >
      <ToolbarButton
        label="Insert row above"
        isDisabled={row === 0}
        onPress={() => onEdit({ kind: "insertRow", at: row }, here)}
      >
        <BetweenHorizontalStart />
      </ToolbarButton>
      <ToolbarButton
        label="Insert row below"
        onPress={() => onEdit({ kind: "insertRow", at: row + 1 }, { row: row + 1, column })}
      >
        <BetweenHorizontalEnd />
      </ToolbarButton>
      <ToolbarButton
        label="Insert column left"
        onPress={() => onEdit({ kind: "insertColumn", at: column }, here)}
      >
        <BetweenVerticalStart />
      </ToolbarButton>
      <ToolbarButton
        label="Insert column right"
        onPress={() =>
          onEdit({ kind: "insertColumn", at: column + 1 }, { row, column: column + 1 })
        }
      >
        <BetweenVerticalEnd />
      </ToolbarButton>
      <span className="mx-0.5 h-4 w-px bg-border" aria-hidden="true" />
      {alignments.map(({ align, label, icon: Icon }) => (
        <ToolbarButton
          key={align}
          label={label}
          isPressed={focus.align === align}
          onPress={() => setAlign(align)}
        >
          <Icon />
        </ToolbarButton>
      ))}
      <span className="mx-0.5 h-4 w-px bg-border" aria-hidden="true" />
      <DropdownMenu>
        <DropdownMenuTrigger
          render={<Button variant="ghost" size="icon-sm" />}
          aria-label="More table actions"
          onMouseDown={(event) => event.preventDefault()}
        >
          <Ellipsis />
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" data-table-tools="" finalFocus={() => focus.element}>
          <DropdownMenuItem
            disabled={row === 0}
            onClick={() =>
              onEdit({ kind: "deleteRow", at: row }, { row: Math.min(row, rowCount - 2), column })
            }
          >
            Delete row
          </DropdownMenuItem>
          <DropdownMenuItem
            disabled={columnCount === 1}
            onClick={() =>
              onEdit(
                { kind: "deleteColumn", at: column },
                { row, column: Math.min(column, columnCount - 2) },
              )
            }
          >
            Delete column
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem variant="destructive" onClick={onDeleteTable}>
            Delete table
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}
