import { useCreatePage } from "../api/use-page-mutations";
import { PathDialog } from "./path-dialog";

type CreatePageDialogProps = {
  initialPath: string;
  onClose: () => void;
};

export function CreatePageDialog({ initialPath, onClose }: CreatePageDialogProps) {
  const create = useCreatePage();
  return (
    <PathDialog
      title="New page"
      description="Pages live at a path. Folders appear when a path uses them."
      submitLabel="Create"
      initialPath={initialPath}
      isPending={create.isPending}
      onSubmit={(path) => create.mutate(path, { onSuccess: onClose })}
      onClose={onClose}
    />
  );
}
