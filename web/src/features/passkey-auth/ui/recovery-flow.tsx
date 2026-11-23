import { useQueryClient } from "@tanstack/react-query";
import { useState, type FormEvent } from "react";

import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { Label } from "@/shared/ui/label";

import { useRecover } from "../api/use-auth-mutations";
import { AuthCard } from "./auth-card";
import { NameField } from "./name-field";
import { RecoveryCodeStep } from "./recovery-code-step";

type RecoveryFlowProps = {
  onDone: () => void;
};

export function RecoveryFlow({ onDone }: RecoveryFlowProps) {
  const [username, setUsername] = useState("");
  const [code, setCode] = useState("");
  const queryClient = useQueryClient();
  const recover = useRecover();

  const handleConfirm = async () => {
    await queryClient.invalidateQueries({ queryKey: ["me"] });
    onDone();
  };

  if (recover.data) {
    return (
      <AuthCard title="Save your new recovery code">
        <RecoveryCodeStep code={recover.data.recovery_code} onConfirm={handleConfirm} />
      </AuthCard>
    );
  }

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault();
    recover.mutate({ username, code });
  };

  return (
    <AuthCard
      title="Recover your account"
      description="Enter your recovery code to create a new passkey. All other sessions and connections are signed out."
    >
      <form className="grid gap-5" onSubmit={handleSubmit}>
        <NameField
          label="Username"
          value={username}
          onChange={setUsername}
          autoComplete="username"
        />
        <div className="grid gap-2">
          <Label htmlFor="recovery-code">Recovery code</Label>
          <Input
            id="recovery-code"
            className="font-mono"
            value={code}
            autoComplete="off"
            onChange={(event) => setCode(event.target.value)}
            required
          />
        </div>
        <Button type="submit" disabled={recover.isPending}>
          Create new passkey
        </Button>
      </form>
    </AuthCard>
  );
}
