import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/shared/ui/alert-dialog";
import { Button } from "@/shared/ui/button";

import { useDeleteFile } from "../api/use-delete-file";

type DeleteFileDialogProps = {
  file: { id: number; path: string };
  onClose: () => void;
};

export function DeleteFileDialog({ file, onClose }: DeleteFileDialogProps) {
  const remove = useDeleteFile();
  return (
    <AlertDialog open onOpenChange={(open) => !open && onClose()}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete {file.path}?</AlertDialogTitle>
          <AlertDialogDescription>
            Pages that cite this file keep their source text.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <Button
            variant="destructive"
            disabled={remove.isPending}
            onClick={() => remove.mutate(file.id, { onSuccess: onClose })}
          >
            Delete
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
