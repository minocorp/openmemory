import { useState, type FormEvent } from "react";

import { findPathProblem, normalizePath } from "@/shared/lib/page-path";
import { Button } from "@/shared/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/shared/ui/dialog";
import { Input } from "@/shared/ui/input";
import { Label } from "@/shared/ui/label";

type PathDialogProps = {
  title: string;
  description: string;
  submitLabel: string;
  initialPath: string;
  isPending: boolean;
  onSubmit: (path: string) => void;
  onClose: () => void;
};

const problemText = {
  empty: "Enter a path.",
  segment: "Paths cannot contain empty segments, . or ..",
  character: "Paths cannot contain control characters.",
} as const;

export function PathDialog({
  title,
  description,
  submitLabel,
  initialPath,
  isPending,
  onSubmit,
  onClose,
}: PathDialogProps) {
  const [path, setPath] = useState(initialPath);
  const problem = findPathProblem(path);

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault();
    if (problem === null) {
      onSubmit(normalizePath(path));
    }
  };

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-md">
        <form className="grid gap-4" onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle>{title}</DialogTitle>
            <DialogDescription>{description}</DialogDescription>
          </DialogHeader>
          <div className="grid gap-2">
            <Label htmlFor="page-path">Path</Label>
            <Input
              id="page-path"
              value={path}
              placeholder="customers/acme"
              onChange={(event) => setPath(event.target.value)}
            />
            {problem && path !== "" ? (
              <p className="text-xs text-destructive">{problemText[problem]}</p>
            ) : null}
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={onClose}>
              Cancel
            </Button>
            <Button type="submit" disabled={problem !== null || isPending}>
              {submitLabel}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
