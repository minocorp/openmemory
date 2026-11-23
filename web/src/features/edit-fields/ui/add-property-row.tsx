import { Plus } from "lucide-react";
import { useState, type FormEvent } from "react";

import { focusOnMount } from "@/shared/lib/focus-on-mount";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";

import { isValidFieldKey, parseFieldInput } from "../model/field-values";

type AddPropertyRowProps = {
  existingKeys: readonly string[];
  isSaving: boolean;
  onAdd: (key: string, values: string[]) => void;
};

export function AddPropertyRow({ existingKeys, isSaving, onAdd }: AddPropertyRowProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [key, setKey] = useState("");
  const [value, setValue] = useState("");

  const keyProblem =
    key !== "" && !isValidFieldKey(key)
      ? "Use English letters, digits, - or _"
      : existingKeys.includes(key)
        ? "That field already exists"
        : null;
  const canAdd = key !== "" && keyProblem === null && value.trim() !== "";

  const close = () => {
    setIsOpen(false);
    setKey("");
    setValue("");
  };

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault();
    if (canAdd) {
      onAdd(key, parseFieldInput(value, false));
      close();
    }
  };

  if (!isOpen) {
    return (
      <Button
        variant="ghost"
        size="xs"
        className="mt-1 -ml-2 w-fit text-muted-foreground"
        onClick={() => setIsOpen(true)}
      >
        <Plus /> Add field
      </Button>
    );
  }

  return (
    <form className="mt-1 flex items-start gap-2" onSubmit={handleSubmit}>
      <div className="grid w-[120px] shrink-0 gap-1">
        <Input
          ref={focusOnMount}
          aria-label="Field name"
          placeholder="name"
          className="h-7 font-mono text-[12.5px]"
          value={key}
          aria-invalid={keyProblem !== null}
          onChange={(event) => setKey(event.target.value)}
          onKeyDown={(event) => event.key === "Escape" && close()}
        />
        {keyProblem ? <p className="text-[11px] text-destructive">{keyProblem}</p> : null}
      </div>
      <Input
        aria-label="Field value"
        placeholder="value"
        className="h-7 flex-1 text-[13.5px]"
        value={value}
        onChange={(event) => setValue(event.target.value)}
        onKeyDown={(event) => event.key === "Escape" && close()}
      />
      <Button type="submit" size="sm" disabled={!canAdd || isSaving}>
        Add
      </Button>
      <Button type="button" size="sm" variant="ghost" onClick={close}>
        Cancel
      </Button>
    </form>
  );
}
