import { useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { joinPath } from "@/shared/lib/page-path";

import { filePathUrl, sendUpload, UploadError } from "./upload-request";

export type UploadItem = {
  id: string;
  name: string;
  path: string;
  progress: number;
  state: "uploading" | "done" | "failed";
  message: string | null;
};

export function useUploads(folder: string) {
  const queryClient = useQueryClient();
  const [items, setItems] = useState<UploadItem[]>([]);

  const patch = (id: string, change: Partial<UploadItem>) =>
    setItems((previous) =>
      previous.map((item) => (item.id === id ? { ...item, ...change } : item)),
    );

  const uploadOne = async (file: File) => {
    const id = crypto.randomUUID();
    const path = joinPath(folder, file.name);
    setItems((previous) => [
      ...previous,
      { id, name: file.name, path, progress: 0, state: "uploading", message: null },
    ]);
    try {
      await sendUpload({
        url: filePathUrl(path),
        file,
        onProgress: (progress) => patch(id, { progress }),
      });
      patch(id, { state: "done", progress: 1 });
      await queryClient.invalidateQueries({ queryKey: ["tree"] });
    } catch (error) {
      const message = error instanceof UploadError ? error.message : "The upload failed.";
      patch(id, { state: "failed", message });
    }
  };

  const upload = (files: File[]) => Promise.all(files.map(uploadOne));
  const dismiss = (id: string) => setItems((previous) => previous.filter((item) => item.id !== id));

  return { items, upload, dismiss };
}
