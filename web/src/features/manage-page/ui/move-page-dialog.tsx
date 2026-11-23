import type { Page } from "@/entities/page";

import { useMovePage } from "../api/use-page-mutations";
import { PathDialog } from "./path-dialog";

type MovePageDialogProps = {
  page: Page;
  onClose: () => void;
};

export function MovePageDialog({ page, onClose }: MovePageDialogProps) {
  const move = useMovePage();
  return (
    <PathDialog
      title="Move page"
      description="Links to this page keep working. Pages that link here are updated."
      submitLabel="Move"
      initialPath={page.path}
      isPending={move.isPending}
      onSubmit={(to) => move.mutate({ page, to }, { onSuccess: onClose })}
      onClose={onClose}
    />
  );
}
