import { Plus } from "lucide-react";
import { useState, type FormEvent } from "react";

import type { Page } from "@/entities/page";
import { focusOnMount } from "@/shared/lib/focus-on-mount";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";

import { useAppendTimeline } from "../api/use-append-timeline";

type TimelineAddFormProps = { page: Page };

function localIsoDate(): string {
  const now = new Date();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  return `${now.getFullYear()}-${month}-${day}`;
}

export function TimelineAddForm({ page }: TimelineAddFormProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [text, setText] = useState("");
  const [source, setSource] = useState("");
  const [today, setToday] = useState(localIsoDate);
  const [date, setDate] = useState(today);
  const append = useAppendTimeline();

  const close = () => {
    setIsOpen(false);
    setText("");
    setSource("");
    const reset = localIsoDate();
    setToday(reset);
    setDate(reset);
  };

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault();
    const trimmed = source.trim();
    append.mutate(
      {
        page,
        text: text.trim(),
        source: trimmed === "" ? null : trimmed,
        date: date === "" || date === today ? null : date,
      },
      { onSuccess: close },
    );
  };

  if (!isOpen) {
    return (
      <Button
        variant="ghost"
        size="xs"
        className="mt-1 -ml-2 w-fit text-muted-foreground"
        onClick={() => setIsOpen(true)}
      >
        <Plus /> Add to timeline
      </Button>
    );
  }

  return (
    <form className="mt-2 grid gap-2" onSubmit={handleSubmit} aria-label="Add timeline line">
      <Input
        ref={focusOnMount}
        aria-label="Timeline text"
        placeholder="What happened"
        value={text}
        onChange={(event) => setText(event.target.value)}
        onKeyDown={(event) => event.key === "Escape" && close()}
      />
      <Input
        aria-label="Timeline source"
        placeholder="Source: a note like call, or a file like contracts/acme.pdf#p2"
        value={source}
        onChange={(event) => setSource(event.target.value)}
        onKeyDown={(event) => event.key === "Escape" && close()}
      />
      <Input
        type="date"
        aria-label="Timeline date"
        value={date}
        onChange={(event) => setDate(event.target.value)}
        onKeyDown={(event) => event.key === "Escape" && close()}
      />
      <div className="flex gap-2">
        <Button type="submit" size="sm" disabled={text.trim() === "" || append.isPending}>
          Add line
        </Button>
        <Button type="button" size="sm" variant="ghost" onClick={close}>
          Cancel
        </Button>
      </div>
    </form>
  );
}
