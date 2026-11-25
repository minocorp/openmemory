import { ApiError } from "@/shared/api";
import { confirmWithPasskey } from "@/shared/lib/webauthn";
import { z } from "@/shared/lib/zod";

const importReport = z.object({
  pages: z.number(),
  files: z.number(),
  skipped: z.array(z.object({ name: z.string(), reason: z.string() })),
});

const zipType = "application/zip";

export async function downloadExport() {
  const response = await fetch("/api/system/export", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ confirmation: await confirmWithPasskey("export") }),
  });
  if (!response.ok) {
    throw new ApiError(response.status, await response.json().catch(() => null));
  }
  const url = URL.createObjectURL(await response.blob());
  const link = document.createElement("a");
  link.href = url;
  link.download = "openmemory-export.zip";
  link.click();
  URL.revokeObjectURL(url);
}

export async function importArchive(file: File) {
  const response = await fetch("/api/system/import", {
    method: "POST",
    headers: { "Content-Type": zipType },
    body: file,
  });
  const body: unknown = await response.json().catch(() => null);
  if (!response.ok) {
    throw new ApiError(response.status, body);
  }
  return importReport.parse(body);
}
