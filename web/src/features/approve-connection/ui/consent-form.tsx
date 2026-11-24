import { useSuspenseQuery } from "@tanstack/react-query";
import { type FormEvent, useId, useState } from "react";

import type { Consent } from "@/entities/consent";
import { treeQuery } from "@/entities/tree";
import { Button } from "@/shared/ui/button";
import { FolderPicker } from "@/shared/ui/folder-picker";
import { Input } from "@/shared/ui/input";
import { Label } from "@/shared/ui/label";
import { Switch } from "@/shared/ui/switch";

import { useApprove, useDeny } from "../api/use-consent-decision";
import {
  connectionNameProblem,
  hasScopeSelection,
  type NameProblem,
  type Outcome,
  outcomeOf,
} from "../model/decision";

const nameMessages: Record<NameProblem, string> = {
  empty: "Enter a name.",
  long: "Use at most 64 characters.",
  slash: "A name cannot contain a slash.",
};

type ConsentFormProps = {
  reference: string;
  consent: Consent;
  onDecided: (outcome: Outcome, decision: "approved" | "denied") => void;
};

export function ConsentForm({ reference, consent, onDecided }: ConsentFormProps) {
  const { data: tree } = useSuspenseQuery(treeQuery());
  const [name, setName] = useState(consent.proposed_name);
  const [scopes, setScopes] = useState(consent.default_scopes);
  const [canWrite, setCanWrite] = useState(consent.default_can_write);
  const nameId = useId();
  const writeId = useId();
  const approve = useApprove();
  const deny = useDeny();
  const nameProblem = connectionNameProblem(name);
  const isBusy = approve.isPending || deny.isPending;

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault();
    approve.mutate(
      { reference, name: name.trim(), scopes, canWrite },
      { onSuccess: (decision) => onDecided(outcomeOf(decision), "approved") },
    );
  };

  const handleDeny = () =>
    deny.mutate(reference, { onSuccess: (decision) => onDecided(outcomeOf(decision), "denied") });

  return (
    <form className="grid gap-6" onSubmit={handleSubmit}>
      <div className="grid gap-2">
        <Label htmlFor={nameId}>Connection name</Label>
        <Input
          id={nameId}
          value={name}
          onChange={(event) => setName(event.target.value)}
          autoComplete="off"
          aria-invalid={nameProblem !== null}
        />
        {nameProblem && name !== "" ? (
          <p className="text-sm text-destructive">{nameMessages[nameProblem]}</p>
        ) : null}
      </div>
      <div className="grid gap-2">
        <span className="text-sm font-medium">Folders</span>
        <FolderPicker
          folders={tree.folders.map((folder) => folder.path)}
          value={scopes}
          onChange={setScopes}
        />
      </div>
      <div className="flex items-center justify-between">
        <Label htmlFor={writeId}>Allow writing</Label>
        <Switch id={writeId} checked={canWrite} onCheckedChange={setCanWrite} />
      </div>
      <div className="flex gap-2">
        <Button
          type="submit"
          className="flex-1"
          disabled={isBusy || nameProblem !== null || !hasScopeSelection(scopes)}
        >
          Approve
        </Button>
        <Button type="button" variant="outline" disabled={isBusy} onClick={handleDeny}>
          Deny
        </Button>
      </div>
    </form>
  );
}
