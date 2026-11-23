import { Button } from "@/shared/ui/button";
import { CopySnippet } from "@/shared/ui/copy-snippet";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/shared/ui/dialog";

type TokenCreatedDialogProps = { token: string; onClose: () => void };

export function TokenCreatedDialog({ token, onClose }: TokenCreatedDialogProps) {
  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Copy your token</DialogTitle>
          <DialogDescription>
            It is shown only once. Set it as MEMORY_TOKEN in your scripts.
          </DialogDescription>
        </DialogHeader>
        <CopySnippet label="Personal access token" text={token} />
        <DialogFooter>
          <Button onClick={onClose}>Done</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
