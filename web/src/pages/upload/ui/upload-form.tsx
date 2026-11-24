import { Upload } from "lucide-react";
import { type DragEvent, useRef, useState } from "react";

import { sendUpload, uploadLinkUrl, UploadError } from "@/features/upload-file";
import { cn } from "@/shared/lib/cn";
import { Button } from "@/shared/ui/button";

import { UploadCard } from "./upload-card";

type UploadFormProps = { token: string; path: string };

type Status =
  | { kind: "idle" }
  | { kind: "uploading"; name: string; progress: number }
  | { kind: "done"; path: string }
  | { kind: "failed"; message: string };

export function UploadForm({ token, path }: UploadFormProps) {
  const input = useRef<HTMLInputElement>(null);
  const [status, setStatus] = useState<Status>({ kind: "idle" });
  const [isDragging, setIsDragging] = useState(false);

  const send = async (file: File | undefined) => {
    if (!file) {
      return;
    }
    setStatus({ kind: "uploading", name: file.name, progress: 0 });
    try {
      const stored = await sendUpload({
        url: uploadLinkUrl(token),
        file,
        onProgress: (progress) => setStatus({ kind: "uploading", name: file.name, progress }),
      });
      setStatus({ kind: "done", path: stored.path });
    } catch (error) {
      const message = error instanceof UploadError ? error.message : "The upload failed.";
      setStatus({ kind: "failed", message });
    }
  };

  const handleDrop = (event: DragEvent) => {
    event.preventDefault();
    setIsDragging(false);
    void send(event.dataTransfer.files[0]);
  };

  if (status.kind === "done") {
    return (
      <UploadCard title="Uploaded">
        <p className="font-mono text-sm break-all">{status.path}</p>
      </UploadCard>
    );
  }

  return (
    <UploadCard title="Upload a file">
      <p className="mb-4 font-mono text-sm break-all text-muted-foreground">{path}</p>
      <div
        onDragOver={(event) => {
          event.preventDefault();
          setIsDragging(true);
        }}
        onDragLeave={() => setIsDragging(false)}
        onDrop={handleDrop}
        className={cn(
          "grid justify-items-center gap-3 rounded-lg border border-dashed border-input p-8 text-sm text-muted-foreground",
          isDragging && "border-ring bg-accent/50",
        )}
      >
        {status.kind === "uploading" ? (
          <>
            <span className="max-w-full truncate">{status.name}</span>
            <progress
              value={status.progress}
              max={1}
              aria-label="Upload progress"
              className="h-1 w-full"
            />
          </>
        ) : (
          <>
            <span>Drop a file here</span>
            <Button onClick={() => input.current?.click()}>
              <Upload /> Choose file
            </Button>
          </>
        )}
        <input
          ref={input}
          type="file"
          hidden
          aria-label="Choose file"
          onChange={(event) => void send(event.target.files?.[0])}
        />
      </div>
      {status.kind === "failed" ? (
        <p role="alert" className="mt-3 text-sm text-destructive">
          {status.message}
        </p>
      ) : null}
    </UploadCard>
  );
}
