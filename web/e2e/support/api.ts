import type { Page } from "@playwright/test";

import { webOrigin } from "./config";

type Json = Record<string, unknown>;

export async function apiGet(page: Page, path: string): Promise<{ data: Json; etag: string }> {
  const response = await page.request.get(path);
  return { data: await response.json(), etag: response.headers()["etag"] ?? "" };
}

export async function apiSend(
  page: Page,
  options: { method: "PUT" | "PATCH" | "POST"; path: string; headers?: Record<string, string> } & {
    json?: Json;
    body?: Buffer;
    contentType?: string;
  },
) {
  const response = await page.request.fetch(options.path, {
    method: options.method,
    headers: {
      Origin: webOrigin,
      "Content-Type": options.contentType ?? "application/json",
      ...options.headers,
    },
    data: options.body ?? options.json,
  });
  if (!response.ok()) {
    throw new Error(`${options.method} ${options.path} failed: ${response.status()}`);
  }
  return response;
}
