import { useBlocker } from "@tanstack/react-router";

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

type LeaveGuardProps = {
  hasUnsavedChanges: boolean;
};

export function LeaveGuard({ hasUnsavedChanges }: LeaveGuardProps) {
  const blocker = useBlocker({
    shouldBlockFn: ({ current, next }) => hasUnsavedChanges && current.pathname !== next.pathname,
    enableBeforeUnload: hasUnsavedChanges,
    disabled: !hasUnsavedChanges,
    withResolver: true,
  });
  const isBlocked = blocker.status === "blocked";

  return (
    <AlertDialog open={isBlocked} onOpenChange={(open) => !open && blocker.reset?.()}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Leave without saving?</AlertDialogTitle>
          <AlertDialogDescription>Your edits to this page will be lost.</AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Keep editing</AlertDialogCancel>
          <Button variant="destructive" onClick={() => blocker.proceed?.()}>
            Leave
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
