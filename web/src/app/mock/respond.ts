import { HttpResponse } from "msw";

import { versionOfEtag } from "@/shared/lib/etag";

import { current, type StoredPage } from "./store";
import { pageView } from "./views";

export const problem = (status: number, error: string) => HttpResponse.json({ error }, { status });

export const noContent = () => new HttpResponse(null, { status: 204 });

export const today = () => new Date().toISOString().slice(0, 10);

export function staleVersion(request: Request, page: StoredPage) {
  if (versionOfEtag(request.headers.get("If-Match")) === current(page).version) {
    return null;
  }
  return HttpResponse.json(
    { error: "precondition_failed", current: pageView(page) },
    { status: 412 },
  );
}

export function queryOf(request: Request, name: string): string | null {
  return new URL(request.url).searchParams.get(name);
}
