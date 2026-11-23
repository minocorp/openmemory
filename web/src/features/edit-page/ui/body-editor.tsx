import { useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { applySavedPage, getPage, type Page } from "@/entities/page";
import { describeError, isApiError } from "@/shared/api";
import { Button } from "@/shared/ui/button";
import { MarkdownEditor, type LinkTarget } from "@/shared/ui/markdown-editor";
import { toast } from "@/shared/ui/toaster";

import { useSaveBody } from "../api/use-save-body";
import { ConflictDialog } from "./conflict-dialog";
import { LeaveGuard } from "./leave-guard";

type BodyEditorProps = {
  page: Page;
  changedLines: ReadonlySet<string>;
  linkTargets: readonly LinkTarget[];
  onOpenLink: (path: string) => void;
};

export function BodyEditor({ page, changedLines, linkTargets, onOpenLink }: BodyEditorProps) {
  const queryClient = useQueryClient();
  const save = useSaveBody();
  const [draft, setDraft] = useState<string | null>(null);
  const [theirs, setTheirs] = useState<Page | null>(null);

  const handleChange = (text: string) => setDraft(text === page.body ? null : text);

  const submit = async (base: Page) => {
    if (draft === null) {
      return;
    }
    try {
      await save.mutateAsync({ page: base, body: draft });
      setDraft(null);
      setTheirs(null);
    } catch (error) {
      if (isApiError(error, 412)) {
        setTheirs(await getPage(page.id));
        return;
      }
      toast.error(describeError(error));
    }
  };

  const handleUseTheirs = () => {
    if (theirs) {
      applySavedPage(queryClient, theirs);
    }
    setDraft(null);
    setTheirs(null);
  };

  return (
    <div>
      <LeaveGuard hasUnsavedChanges={draft !== null} />
      <MarkdownEditor
        value={draft ?? page.body}
        onChange={handleChange}
        onSave={() => void submit(page)}
        onOpenLink={onOpenLink}
        linkTargets={linkTargets}
        changedLines={changedLines}
        label="Page body"
        emptyHint="Write the current facts. Type / for commands."
      />
      {draft !== null ? (
        <div className="mt-3 flex items-center gap-3">
          <Button size="sm" disabled={save.isPending} onClick={() => void submit(page)}>
            Save
          </Button>
          <Button size="sm" variant="ghost" onClick={() => setDraft(null)}>
            Discard
          </Button>
          <span className="text-xs text-muted-foreground">Unsaved changes</span>
        </div>
      ) : null}
      {theirs && draft !== null ? (
        <ConflictDialog
          theirs={theirs}
          mine={draft}
          isSaving={save.isPending}
          onKeepMine={() => void submit(theirs)}
          onUseTheirs={handleUseTheirs}
        />
      ) : null}
    </div>
  );
}
