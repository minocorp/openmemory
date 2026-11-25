import type { ApiError } from "@/shared/api";
import { z } from "@/shared/lib/zod";

const limitBody = z.object({ limit: z.number() });

const formatMegabytes = (bytes: number) => `${Math.round(bytes / 1024 / 1024)} MB`;

export function describeUploadError(error: ApiError, body: unknown): string {
  switch (error.status) {
    case 409:
      return "A file or page already exists at that path.";
    case 413: {
      const limit = limitBody.safeParse(body);
      return limit.success
        ? `The file is larger than ${formatMegabytes(limit.data.limit)}.`
        : "The file is too large.";
    }
    case 415:
      return "This file type is not accepted. Use a pdf, text, markdown, csv, html, xml, image, svg, spreadsheet, docx or audio file.";
    case 422:
      return error.code === "credential"
        ? `The text looks like a secret (${error.pattern ?? "credential"}) and was rejected.`
        : "The file could not be read.";
    case 404:
      return "This link has expired or was already used.";
    case 400:
      return error.code === "empty" ? "The file is empty." : "That path is not valid.";
    default:
      return "The upload failed. Try again.";
  }
}

export function fileMime(file: { type: string }): string {
  return file.type === "" ? "application/octet-stream" : file.type;
}
