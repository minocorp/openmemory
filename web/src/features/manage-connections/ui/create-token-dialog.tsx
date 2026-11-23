import { useSuspenseQuery } from "@tanstack/react-query";
import { type FormEvent, useId, useState } from "react";

import { treeQuery } from "@/entities/tree";
import { Button } from "@/shared/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/shared/ui/dialog";
import { FolderPicker } from "@/shared/ui/folder-picker";
import { Input } from "@/shared/ui/input";
import { Label } from "@/shared/ui/label";
import { Switch } from "@/shared/ui/switch";

import { useCreateToken } from "../api/use-create-token";
import { defaultTokenExpiryDays, tokenExpiryOptions } from "../config/token-expiry";

type CreateTokenDialogProps = {
  onClose: () => void;
  onCreated: (token: string) => void;
};

export function CreateTokenDialog({ onClose, onCreated }: CreateTokenDialogProps) {
  const { data: tree } = useSuspenseQuery(treeQuery());
  const create = useCreateToken();
  const [name, setName] = useState("");
  const [scopes, setScopes] = useState<string[]>(["*"]);
  const [canWrite, setCanWrite] = useState(true);
  const [expiresInDays, setExpiresInDays] = useState<number>(defaultTokenExpiryDays);
  const nameId = useId();
  const expiryId = useId();
  const hasScopes = scopes.length > 0;

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault();
    create.mutate(
      { name: name.trim(), scopes, canWrite, expiresInDays },
      { onSuccess: (created) => onCreated(created.token) },
    );
  };

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-md">
        <form className="grid gap-4" onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle>New personal access token</DialogTitle>
          </DialogHeader>
          <div className="grid gap-2">
            <Label htmlFor={nameId}>Name</Label>
            <Input
              id={nameId}
              value={name}
              onChange={(event) => setName(event.target.value)}
              autoComplete="off"
            />
          </div>
          <FolderPicker
            folders={tree.folders.map((folder) => folder.path).toSorted()}
            value={scopes}
            onChange={setScopes}
          />
          <div className="flex items-center justify-between gap-2 text-[13.5px]">
            <span>Can write</span>
            <Switch aria-label="Can write" checked={canWrite} onCheckedChange={setCanWrite} />
          </div>
          <div className="grid gap-2">
            <Label htmlFor={expiryId}>Expires after</Label>
            <select
              id={expiryId}
              value={expiresInDays}
              onChange={(event) => setExpiresInDays(Number(event.target.value))}
              className="h-9 rounded-md border bg-background px-2 text-sm"
            >
              {tokenExpiryOptions.map((option) => (
                <option key={option.days} value={option.days}>
                  {option.label}
                </option>
              ))}
            </select>
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={onClose}>
              Cancel
            </Button>
            <Button type="submit" disabled={name.trim() === "" || !hasScopes || create.isPending}>
              Create with passkey
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
