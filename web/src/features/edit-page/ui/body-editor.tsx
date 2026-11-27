import { useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { applySavedPage, getPage, type Page } from "@/entities/page";
import { describeError, isApiError } from "@/shared/api";
import { Button } from "@/shared/ui/button";
import { MarkdownEditor, type LinkTarget } from "@/shared/ui/markdown-editor";
import { toast } from "@/shared/ui/toaster";

import { useSaveBody } from "../api/use-save-body";
import { rebaseDraft, type Draft } from "../model/rebase-draft";
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
  const [savedDraft, setSavedDraft] = useState<Draft<Page> | null>(null);
  const [theirs, setTheirs] = useState<Page | null>(null);
  const draft = rebaseDraft(savedDraft, page);
  const isBehind = draft !== null && draft.base.version !== page.version;

  const handleChange = (text: string) => {
    const base = draft?.base ?? page;
    setSavedDraft(text === base.body ? null : { base, text });
  };

  const submit = async (base: Page, text: string) => {
    try {
      await save.mutateAsync({ page: base, body: text });
      setSavedDraft(null);
      setTheirs(null);
    } catch (error) {
      if (isApiError(error, 412)) {
        setTheirs(await getPage(page.id));
        return;
      }
      toast.error(describeError(error));
    }
  };

  const handleSave = () => {
    if (draft !== null) {
      void submit(draft.base, draft.text);
    }
  };

  const handleUseTheirs = () => {
    if (theirs) {
      applySavedPage(queryClient, theirs);
    }
    setSavedDraft(null);
    setTheirs(null);
  };

  return (
    <div>
      <LeaveGuard hasUnsavedChanges={draft !== null} />
      <MarkdownEditor
        value={draft?.text ?? page.body}
        onChange={handleChange}
        onSave={handleSave}
        onOpenLink={onOpenLink}
        linkTargets={linkTargets}
        changedLines={changedLines}
        label="Page body"
        emptyHint="Write the current facts. Type / for commands."
      />
      {draft !== null ? (
        <div className="mt-3 flex items-center gap-3">
          <Button size="sm" disabled={save.isPending} onClick={handleSave}>
            Save
          </Button>
          <Button size="sm" variant="ghost" onClick={() => setSavedDraft(null)}>
            Discard
          </Button>
          <span className="text-xs text-muted-foreground">
            {isBehind
              ? "Unsaved changes. Someone else edited the same lines; saving asks which text to keep."
              : "Unsaved changes"}
          </span>
        </div>
      ) : null}
      {theirs && draft !== null ? (
        <ConflictDialog
          theirs={theirs}
          mine={draft.text}
          isSaving={save.isPending}
          onKeepMine={() => void submit(theirs, draft.text)}
          onUseTheirs={handleUseTheirs}
        />
      ) : null}
    </div>
  );
}
