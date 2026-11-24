import { useId, useState } from "react";

import { Button } from "@/shared/ui/button";

type RecoveryCodeStepProps = {
  code: string;
  onConfirm: () => void | Promise<void>;
};

export function RecoveryCodeStep({ code, onConfirm }: RecoveryCodeStepProps) {
  const [hasSaved, setHasSaved] = useState(false);
  const checkboxId = useId();
  return (
    <div className="grid gap-5">
      <p className="text-sm text-muted-foreground">
        This code is the only way back in if you lose your passkey. It is shown once.
      </p>
      <p
        className="rounded-lg bg-muted px-4 py-3 text-center font-mono text-sm select-all"
        aria-label="Recovery code"
      >
        {code}
      </p>
      <div className="flex items-start gap-2">
        <input
          id={checkboxId}
          type="checkbox"
          className="mt-0.5 size-4"
          checked={hasSaved}
          onChange={(event) => setHasSaved(event.target.checked)}
        />
        <label htmlFor={checkboxId} className="text-sm">
          I saved it
        </label>
      </div>
      <Button disabled={!hasSaved} onClick={() => void onConfirm()}>
        Continue
      </Button>
    </div>
  );
}
