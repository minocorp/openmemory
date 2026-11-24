import { useState, type KeyboardEvent } from "react";

import type { Page } from "@/entities/page";
import { baseName, joinPath, normalizePath, parentFolder } from "@/shared/lib/page-path";

import { useMovePage } from "../api/use-page-mutations";

type PageTitleProps = { page: Page };

export function PageTitle({ page }: PageTitleProps) {
  const title = baseName(page.path);
  const [draft, setDraft] = useState<string | null>(null);
  const move = useMovePage();

  const commit = () => {
    const next = normalizePath(draft ?? title);
    setDraft(null);
    if (next === "" || next === title || next.includes("/")) {
      return;
    }
    move.mutate({ page, to: joinPath(parentFolder(page.path), next) });
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      event.currentTarget.blur();
    }
    if (event.key === "Escape") {
      setDraft(null);
      event.currentTarget.blur();
    }
  };

  return (
    <h1 className="mb-6">
      <input
        aria-label="Page title"
        className="w-full bg-transparent text-3xl font-semibold tracking-tight outline-none placeholder:text-muted-foreground"
        value={draft ?? title}
        disabled={move.isPending}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={commit}
        onKeyDown={handleKeyDown}
      />
    </h1>
  );
}
