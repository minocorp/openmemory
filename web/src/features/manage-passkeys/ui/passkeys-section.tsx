import { useSuspenseQuery } from "@tanstack/react-query";
import { Plus } from "lucide-react";
import { useState, type FormEvent } from "react";

import { type Passkey, passkeysQuery } from "@/entities/passkey";
import { formatRelativeTime } from "@/shared/lib/relative-time";
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
import { Input } from "@/shared/ui/input";
import { Section, SectionList, SectionRow } from "@/shared/ui/section";

import { useAddPasskey, useRemovePasskey } from "../api/use-passkey-mutations";

export function PasskeysSection() {
  const { data: passkeys } = useSuspenseQuery(passkeysQuery());
  const [removing, setRemoving] = useState<Passkey | null>(null);

  return (
    <Section title="Passkeys">
      <SectionList aria-label="Passkeys">
        {passkeys.map((passkey) => (
          <SectionRow key={passkey.id}>
            <div className="min-w-0">
              <p className="truncate font-medium">{passkey.name}</p>
              <p className="text-xs text-muted-foreground">
                Added {formatRelativeTime(passkey.created_at)}
                {passkey.last_used_at ? `, used ${formatRelativeTime(passkey.last_used_at)}` : ""}
              </p>
            </div>
            <Button
              variant="ghost"
              size="sm"
              aria-label={`Remove passkey ${passkey.name}`}
              onClick={() => setRemoving(passkey)}
            >
              Remove
            </Button>
          </SectionRow>
        ))}
      </SectionList>
      <AddPasskeyForm />
      {removing ? (
        <RemovePasskeyDialog passkey={removing} onClose={() => setRemoving(null)} />
      ) : null}
    </Section>
  );
}

function AddPasskeyForm() {
  const add = useAddPasskey();
  const [name, setName] = useState("");

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault();
    add.mutate(name.trim(), { onSuccess: () => setName("") });
  };

  return (
    <form className="flex gap-2" onSubmit={handleSubmit}>
      <Input
        aria-label="Passkey name"
        placeholder="Passkey name, e.g. Work laptop"
        value={name}
        onChange={(event) => setName(event.target.value)}
      />
      <Button type="submit" variant="outline" disabled={name.trim() === "" || add.isPending}>
        <Plus /> Add passkey
      </Button>
    </form>
  );
}

type RemovePasskeyDialogProps = { passkey: Passkey; onClose: () => void };

function RemovePasskeyDialog({ passkey, onClose }: RemovePasskeyDialogProps) {
  const remove = useRemovePasskey();
  return (
    <AlertDialog open onOpenChange={(open) => !open && onClose()}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Remove {passkey.name}?</AlertDialogTitle>
          <AlertDialogDescription>
            This device or key can no longer sign in. Your last passkey cannot be removed.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <Button
            variant="destructive"
            disabled={remove.isPending}
            onClick={() => remove.mutate(passkey.id, { onSettled: onClose })}
          >
            Remove
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
