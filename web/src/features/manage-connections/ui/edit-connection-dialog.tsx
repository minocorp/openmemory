import { useSuspenseQuery } from "@tanstack/react-query";
import { useState, type FormEvent } from "react";

import type { Connection } from "@/entities/connection";
import { treeQuery } from "@/entities/tree";
import { Button } from "@/shared/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/shared/ui/dialog";
import { FolderPicker } from "@/shared/ui/folder-picker";
import { Input } from "@/shared/ui/input";
import { Label } from "@/shared/ui/label";
import { Switch } from "@/shared/ui/switch";

import { useUpdateConnection } from "../api/use-connection-mutations";

type EditConnectionDialogProps = { connection: Connection; onClose: () => void };

export function EditConnectionDialog({ connection, onClose }: EditConnectionDialogProps) {
  const { data: tree } = useSuspenseQuery(treeQuery());
  const update = useUpdateConnection();
  const [name, setName] = useState(connection.name);
  const [scopes, setScopes] = useState(connection.scopes);
  const [canWrite, setCanWrite] = useState(connection.can_write);
  const folders = tree.folders.map((folder) => folder.path).toSorted();
  const isUnscoped = scopes.length === 0;

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault();
    update.mutate(
      { id: connection.id, update: { name, scopes, can_write: canWrite } },
      { onSuccess: onClose },
    );
  };

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-md">
        <form className="grid gap-4" onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle>Edit connection</DialogTitle>
          </DialogHeader>
          <div className="grid gap-2">
            <Label htmlFor="connection-name">Name</Label>
            <Input
              id="connection-name"
              value={name}
              onChange={(event) => setName(event.target.value)}
            />
          </div>
          <FolderPicker folders={folders} value={scopes} onChange={setScopes} />
          <div className="flex items-center justify-between gap-2 text-sm">
            <span>Can write</span>
            <Switch aria-label="Can write" checked={canWrite} onCheckedChange={setCanWrite} />
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={onClose}>
              Cancel
            </Button>
            <Button type="submit" disabled={name.trim() === "" || isUnscoped || update.isPending}>
              Save
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
