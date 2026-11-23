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

import { type PurgeTarget, usePurge } from "../api/use-purge";

type PurgeDialogProps = PurgeTarget & {
  path: string;
  onClose: () => void;
  onDone: () => void;
};

export function PurgeDialog({ kind, id, path, onClose, onDone }: PurgeDialogProps) {
  const purge = usePurge();

  const handlePurge = async () => {
    try {
      await purge.mutateAsync({ kind, id });
    } catch {
      return; // the mutation already showed the server message
    }
    onDone();
  };

  return (
    <AlertDialog open onOpenChange={(open) => !open && onClose()}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Purge {path} permanently?</AlertDialogTitle>
          <AlertDialogDescription>
            Every version{kind === "file" ? " and the stored file" : ""} is erased and cannot be
            restored. Backups expire on their own schedule. Confirm with your passkey.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <Button
            variant="destructive"
            disabled={purge.isPending}
            onClick={() => void handlePurge()}
          >
            Purge with passkey
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
