import type { Page } from "@/entities/page";
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

import { useDeletePage } from "../api/use-page-mutations";

type DeletePageDialogProps = {
  page: Page;
  onClose: () => void;
};

export function DeletePageDialog({ page, onClose }: DeletePageDialogProps) {
  const remove = useDeletePage();
  return (
    <AlertDialog open onOpenChange={(open) => !open && onClose()}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete {page.path}?</AlertDialogTitle>
          <AlertDialogDescription>
            The page stays in history and can be restored from there.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <Button
            variant="destructive"
            disabled={remove.isPending}
            onClick={() => remove.mutate(page, { onSuccess: onClose })}
          >
            Delete
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
