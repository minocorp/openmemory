import { X } from "lucide-react";
import { useState, type KeyboardEvent } from "react";

import type { PageField } from "@/entities/page";
import { cn } from "@/shared/lib/cn";
import { focusOnMount } from "@/shared/lib/focus-on-mount";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";

import { parseFieldInput } from "../model/field-values";

type PropertyRowProps = {
  field: PageField;
  isChanged: boolean;
  isSaving: boolean;
  onSave: (key: string, values: string[]) => void;
};

export function PropertyRow({ field, isChanged, isSaving, onSave }: PropertyRowProps) {
  const [draft, setDraft] = useState<string | null>(null);
  const text = field.values.join(", ");

  const commit = () => {
    if (draft !== null && draft !== text) {
      onSave(field.key, parseFieldInput(draft, field.values.length > 1));
    }
    setDraft(null);
  };

  const handleKeyDown = (event: KeyboardEvent) => {
    if (event.key === "Enter") {
      commit();
    }
    if (event.key === "Escape") {
      setDraft(null);
    }
  };

  return (
    <div className="group/row col-span-2 grid grid-cols-subgrid items-center">
      <dt className="max-w-56 truncate font-mono text-xs text-muted-foreground" title={field.key}>
        {field.key}
      </dt>
      <dd className="flex min-w-0 items-center gap-1">
        {draft === null ? (
          <button
            type="button"
            aria-label={`Edit ${field.key}`}
            className={cn(
              "min-h-7 min-w-0 flex-1 rounded-md px-2 py-0.5 text-left text-sm break-words hover:bg-accent",
              isChanged && "bg-highlight hover:bg-highlight",
            )}
            disabled={isSaving}
            onClick={() => setDraft(text)}
          >
            {text}
          </button>
        ) : (
          <Input
            ref={focusOnMount}
            aria-label={field.key}
            className="flex-1 md:h-7"
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            onBlur={commit}
            onKeyDown={handleKeyDown}
          />
        )}
        <Button
          variant="ghost"
          size="icon-xs"
          className="opacity-0 group-hover/row:opacity-100 focus-visible:opacity-100"
          aria-label={`Remove ${field.key}`}
          disabled={isSaving}
          onClick={() => onSave(field.key, [])}
        >
          <X />
        </Button>
      </dd>
    </div>
  );
}
