import { http, HttpResponse } from "msw";

import { noContent, problem, queryOf } from "./respond";
import { searchView } from "./search";
import { nextId, state, type StoredFile } from "./store";
import { fileView } from "./views";

const uploadToken = "demo";

const fileById = (id: string) => state.files.find((file) => file.id === Number(id));

async function storeUpload(request: Request, path: string) {
  if (state.files.some((file) => file.path === path)) {
    return problem(409, "path_taken");
  }
  const file: StoredFile = {
    id: nextId(),
    path,
    mime: request.headers.get("Content-Type") ?? "application/octet-stream",
    bytes: new Uint8Array(await request.arrayBuffer()),
    pageCount: 1,
    createdAt: new Date().toISOString(),
  };
  state.files.push(file);
  return HttpResponse.json(fileView(file), { status: 201 });
}

export const fileHandlers = [
  http.get("/api/files", ({ request }) => {
    const file = state.files.find((entry) => entry.path === queryOf(request, "path"));
    return file ? HttpResponse.json(fileView(file)) : problem(404, "not_found");
  }),

  http.post("/api/files", ({ request }) => storeUpload(request, queryOf(request, "path") ?? "")),

  http.get<{ id: string }>("/api/files/:id", ({ params }) => {
    const file = fileById(params.id);
    return file ? HttpResponse.json(fileView(file)) : problem(404, "not_found");
  }),

  http.get<{ id: string }>("/api/files/:id/content", ({ params }) => {
    const file = fileById(params.id);
    if (!file) {
      return problem(404, "not_found");
    }
    return new HttpResponse(file.bytes, { headers: { "Content-Type": file.mime } });
  }),

  http.delete<{ id: string }>("/api/files/:id", ({ params }) => {
    state.files = state.files.filter((file) => file.id !== Number(params.id));
    return noContent();
  }),

  http.get("/api/search", ({ request }) => HttpResponse.json(searchView(queryOf(request, "q")))),

  http.get<{ token: string }>("/api/upload-links/:token", ({ params }) =>
    params.token === uploadToken
      ? HttpResponse.json({
          path: "contracts/signed-quote.pdf",
          expires_at: new Date(Date.now() + 3_600_000).toISOString(),
        })
      : problem(404, "not_found"),
  ),

  http.post("/api/upload-links/:token", ({ request }) =>
    storeUpload(request, "contracts/signed-quote.pdf"),
  ),
];
