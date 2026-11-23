import { useState, type FormEvent } from "react";

import type { User } from "@/entities/user";
import { Button } from "@/shared/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/shared/ui/dialog";
import { Input } from "@/shared/ui/input";
import { Label } from "@/shared/ui/label";

import { useUpdateUserProfile } from "../api/use-people-mutations";

type EditUserDialogProps = { user: User; onClose: () => void };

export function EditUserDialog({ user, onClose }: EditUserDialogProps) {
  const update = useUpdateUserProfile();
  const [username, setUsername] = useState(user.username);
  const [displayName, setDisplayName] = useState(user.display_name);

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault();
    update.mutate({ id: user.id, username, displayName }, { onSuccess: onClose });
  };

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-md">
        <form className="grid gap-4" onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle>Edit {user.display_name}</DialogTitle>
          </DialogHeader>
          <div className="grid gap-2">
            <Label htmlFor="user-username">Username</Label>
            <Input
              id="user-username"
              value={username}
              autoComplete="off"
              pattern="[A-Za-z0-9_.\-]{1,32}"
              maxLength={32}
              required
              onChange={(event) => setUsername(event.target.value)}
            />
          </div>
          <div className="grid gap-2">
            <Label htmlFor="user-display-name">Display name</Label>
            <Input
              id="user-display-name"
              value={displayName}
              autoComplete="off"
              maxLength={64}
              required
              onChange={(event) => setDisplayName(event.target.value)}
            />
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={onClose}>
              Cancel
            </Button>
            <Button type="submit" disabled={update.isPending}>
              Save
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
