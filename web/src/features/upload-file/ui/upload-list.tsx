import { AlertCircle, Check, X } from "lucide-react";

import { Button } from "@/shared/ui/button";

import type { UploadItem } from "../api/use-uploads";

type UploadListProps = {
  items: UploadItem[];
  onDismiss: (id: string) => void;
};

export function UploadList({ items, onDismiss }: UploadListProps) {
  if (items.length === 0) {
    return null;
  }
  return (
    <ul aria-label="Uploads" className="grid gap-2.5 rounded-lg border px-3.5 py-3">
      {items.map((item) => (
        <li key={item.id} className="grid gap-1 text-sm">
          <div className="flex items-center gap-2">
            <StateIcon state={item.state} />
            <span className="min-w-0 flex-1 truncate">{item.path}</span>
            {item.state === "uploading" ? (
              <span className="text-muted-foreground tabular-nums">
                {Math.round(item.progress * 100)}%
              </span>
            ) : (
              <Button
                variant="ghost"
                size="icon-xs"
                aria-label={`Dismiss ${item.name}`}
                onClick={() => onDismiss(item.id)}
              >
                <X />
              </Button>
            )}
          </div>
          {item.state === "uploading" ? (
            <progress
              value={item.progress}
              max={1}
              aria-label={`Uploading ${item.name}`}
              className="h-1 w-full [&::-moz-progress-bar]:bg-primary [&::-webkit-progress-bar]:rounded-full [&::-webkit-progress-bar]:bg-muted [&::-webkit-progress-value]:rounded-full [&::-webkit-progress-value]:bg-primary"
            />
          ) : null}
          {item.message ? <p className="pl-6 text-destructive">{item.message}</p> : null}
        </li>
      ))}
    </ul>
  );
}

function StateIcon({ state }: { state: UploadItem["state"] }) {
  if (state === "done") {
    return <Check className="size-4 text-added-foreground" aria-label="Uploaded" />;
  }
  if (state === "failed") {
    return <AlertCircle className="size-4 text-destructive" aria-label="Failed" />;
  }
  return <span className="size-4" />;
}
