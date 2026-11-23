import { ApiError } from "@/shared/api";
import { z } from "@/shared/lib/zod";

import { describeUploadError, fileMime } from "../model/upload-problem";

const uploaded = z.object({ id: z.number(), path: z.string() });

export type UploadedFile = z.infer<typeof uploaded>;

export class UploadError extends Error {
  readonly status: number;

  constructor(status: number, message: string) {
    super(message);
    this.name = "UploadError";
    this.status = status;
  }
}

type UploadRequest = {
  url: string;
  file: File;
  onProgress: (fraction: number) => void;
};

function parseBody(text: string): unknown {
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
}

// fetch cannot report upload progress, XMLHttpRequest can
export function sendUpload({ url, file, onProgress }: UploadRequest): Promise<UploadedFile> {
  return new Promise((resolve, reject) => {
    const request = new XMLHttpRequest();
    request.open("POST", url);
    request.setRequestHeader("Content-Type", fileMime(file));
    request.upload.addEventListener("progress", (event) => {
      if (event.lengthComputable) {
        onProgress(event.loaded / event.total);
      }
    });
    request.addEventListener("load", () => {
      const body = parseBody(request.responseText);
      if (request.status >= 200 && request.status < 300) {
        const parsed = uploaded.safeParse(body);
        if (parsed.success) {
          resolve(parsed.data);
          return;
        }
      }
      const error = new ApiError(request.status, body);
      reject(new UploadError(request.status, describeUploadError(error, body)));
    });
    request.addEventListener("error", () =>
      reject(new UploadError(0, "The upload failed. Check your connection.")),
    );
    request.send(file);
  });
}

export const filePathUrl = (path: string) => `/api/files?path=${encodeURIComponent(path)}`;

export const uploadLinkUrl = (token: string) => `/api/upload-links/${encodeURIComponent(token)}`;
