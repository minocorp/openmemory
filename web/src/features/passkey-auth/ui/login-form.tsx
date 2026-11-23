import { browserSupportsWebAuthnAutofill, WebAuthnAbortService } from "@simplewebauthn/browser";
import { useEffect, useEffectEvent, useState, type FormEvent, type ReactNode } from "react";

import { Button } from "@/shared/ui/button";

import { useAutofillSignIn, useSignIn } from "../api/use-auth-mutations";
import { AuthCard } from "./auth-card";
import { NameField } from "./name-field";

type LoginFormProps = {
  onDone: () => void;
  recoveryLink: ReactNode;
};

function PasskeyAutofill({ onDone }: { onDone: () => void }) {
  const { mutate } = useAutofillSignIn();
  const done = useEffectEvent(onDone);

  useEffect(() => {
    let active = true;
    void browserSupportsWebAuthnAutofill().then((supported) => {
      if (active && supported) {
        mutate(undefined, { onSuccess: () => active && done() });
      }
    });
    return () => {
      active = false;
      WebAuthnAbortService.cancelCeremony();
    };
  }, [mutate]);

  return null;
}

export function LoginForm({ onDone, recoveryLink }: LoginFormProps) {
  const [username, setUsername] = useState("");
  const [attempt, setAttempt] = useState(0);
  const signIn = useSignIn();

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault();
    signIn.mutate(
      { username },
      { onSuccess: onDone, onError: () => setAttempt((count) => count + 1) },
    );
  };

  return (
    <AuthCard title="Sign in">
      <PasskeyAutofill key={attempt} onDone={onDone} />
      <form className="grid gap-5" onSubmit={handleSubmit}>
        <NameField
          label="Username"
          value={username}
          onChange={setUsername}
          autoComplete="username webauthn"
        />
        <Button type="submit" disabled={signIn.isPending}>
          Continue
        </Button>
        <div className="text-center text-sm text-muted-foreground">{recoveryLink}</div>
      </form>
    </AuthCard>
  );
}
