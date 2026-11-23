import { useQueryClient } from "@tanstack/react-query";
import { useState, type FormEvent } from "react";

import { Button } from "@/shared/ui/button";

import type { SignupKind } from "../api/ceremonies";
import { useSignUp } from "../api/use-auth-mutations";
import { AuthCard } from "./auth-card";
import { NameField } from "./name-field";
import { RecoveryCodeStep } from "./recovery-code-step";

type SignUpFlowProps = {
  kind: SignupKind;
  token: string;
  onDone: () => void;
};

const copy = {
  bootstrap: {
    title: "Create the first account",
    description: "This account becomes the administrator.",
  },
  invite: { title: "Join Memory", description: "Choose a username and create a passkey." },
} as const;

export function SignUpFlow({ kind, token, onDone }: SignUpFlowProps) {
  const [username, setUsername] = useState("");
  const [displayName, setDisplayName] = useState("");
  const queryClient = useQueryClient();
  const signUp = useSignUp();

  const handleConfirm = async () => {
    await queryClient.invalidateQueries({ queryKey: ["me"] });
    onDone();
  };

  if (signUp.data) {
    return (
      <AuthCard title="Save your recovery code">
        <RecoveryCodeStep code={signUp.data.recovery_code} onConfirm={handleConfirm} />
      </AuthCard>
    );
  }

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault();
    signUp.mutate({ kind, token, username, displayName });
  };

  return (
    <AuthCard title={copy[kind].title} description={copy[kind].description}>
      <form className="grid gap-5" onSubmit={handleSubmit}>
        <NameField
          label="Username"
          value={username}
          onChange={setUsername}
          autoComplete="username"
          pattern="[A-Za-z0-9_.\-]{1,32}"
          maxLength={32}
          hint="Letters, digits, dash, underscore and dot. Used to sign in and in connection names."
        />
        <NameField
          label="Display name"
          value={displayName}
          onChange={setDisplayName}
          autoComplete="name"
          maxLength={64}
        />
        <Button type="submit" disabled={signUp.isPending}>
          Create passkey
        </Button>
      </form>
    </AuthCard>
  );
}
