import { useId } from "react";

import { Input } from "@/shared/ui/input";
import { Label } from "@/shared/ui/label";

type NameFieldProps = {
  label: string;
  value: string;
  onChange: (value: string) => void;
  autoComplete?: string;
  placeholder?: string;
  pattern?: string;
  maxLength?: number;
  hint?: string;
};

export function NameField({
  label,
  value,
  onChange,
  autoComplete,
  placeholder,
  pattern,
  maxLength,
  hint,
}: NameFieldProps) {
  const id = useId();
  return (
    <div className="grid gap-2">
      <Label htmlFor={id}>{label}</Label>
      <Input
        id={id}
        value={value}
        autoComplete={autoComplete}
        placeholder={placeholder}
        pattern={pattern}
        maxLength={maxLength}
        aria-describedby={hint ? `${id}-hint` : undefined}
        onChange={(event) => onChange(event.target.value)}
        required
      />
      {hint ? (
        <p id={`${id}-hint`} className="text-sm text-muted-foreground">
          {hint}
        </p>
      ) : null}
    </div>
  );
}
