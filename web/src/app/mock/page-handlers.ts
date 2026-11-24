import { http, HttpResponse } from "msw";

import {
  appendTimeline,
  createPage,
  deletePage,
  editPage,
  movePage,
  restoreVersion,
} from "./page-writes";
import { noContent, problem, queryOf, staleVersion, today } from "./respond";
import { current, me, pageAt, pageById, type Schemas, type StoredPage } from "./store";
import { pageView, treeView, versionsView } from "./views";

type IdParams = { id: string };

const livePage = (params: IdParams): StoredPage | null => {
  const page = pageById(Number(params.id));
  return page && !page.isDeleted ? page : null;
};

const author = () => ({ author: me().username });

export const pageHandlers = [
  http.get("/api/tree", () => HttpResponse.json(treeView())),

  http.get("/api/pages", ({ request }) => {
    const page = pageAt(queryOf(request, "path") ?? "");
    return page ? HttpResponse.json(pageView(page)) : problem(404, "not_found");
  }),

  http.put<never, Schemas["PageCreate"]>("/api/pages", async ({ request }) => {
    const path = queryOf(request, "path") ?? "";
    if (pageAt(path)) {
      return problem(409, "path_taken");
    }
    const { content } = await request.json();
    return HttpResponse.json(pageView(createPage(path, content, author())), { status: 201 });
  }),

  http.get<IdParams>("/api/pages/:id", ({ params, request }) => {
    const page = livePage(params);
    const asked = queryOf(request, "version");
    const version = page?.versions.find((entry) => entry.version === Number(asked));
    if (!page) {
      return problem(404, "not_found");
    }
    return HttpResponse.json(pageView(page, version));
  }),

  http.put<IdParams, Schemas["PageOverwrite"]>("/api/pages/:id", async ({ params, request }) => {
    const page = livePage(params);
    if (!page) {
      return problem(404, "not_found");
    }
    const stale = staleVersion(request, page);
    if (stale) {
      return stale;
    }
    const { content } = await request.json();
    editPage(page, content, author());
    return ok(page);
  }),

  http.patch<IdParams, Schemas["PageEdit"]>("/api/pages/:id", async ({ params, request }) => {
    const page = livePage(params);
    if (!page) {
      return problem(404, "not_found");
    }
    return staleVersion(request, page) ?? applyEdit(page, await request.json());
  }),

  http.post<IdParams, Schemas["PageMove"]>("/api/pages/:id/move", async ({ params, request }) => {
    const page = livePage(params);
    const { to } = await request.json();
    if (!page) {
      return problem(404, "not_found");
    }
    if (pageAt(to)) {
      return problem(409, "path_taken");
    }
    const stale = staleVersion(request, page);
    if (stale) {
      return stale;
    }
    const moved = movePage(page, to, author());
    return HttpResponse.json({
      id: page.id,
      path: moved.path,
      version: moved.version,
      rewritten: [],
      rewritten_out_of_scope: 0,
    });
  }),

  http.delete<IdParams>("/api/pages/:id", ({ params, request }) => {
    const page = livePage(params);
    if (!page) {
      return problem(404, "not_found");
    }
    const stale = staleVersion(request, page);
    if (stale) {
      return stale;
    }
    deletePage(page, author());
    return noContent();
  }),

  http.get<IdParams>("/api/pages/:id/versions", ({ params, request }) => {
    const page = pageById(Number(params.id));
    if (!page) {
      return problem(404, "not_found");
    }
    const before = queryOf(request, "before");
    const limit = Number(queryOf(request, "limit") ?? "20");
    return HttpResponse.json(versionsView(page, { limit, before: before ? Number(before) : null }));
  }),

  http.post<IdParams & { n: string }>(
    "/api/pages/:id/versions/:n/restore",
    ({ params, request }) => {
      const page = pageById(Number(params.id));
      if (!page) {
        return problem(404, "not_found");
      }
      if (current(page).version === Number(params.n)) {
        return problem(409, "already_current");
      }
      const stale = staleVersion(request, page);
      if (stale) {
        return stale;
      }
      const restored = restoreVersion(page, Number(params.n), author());
      return HttpResponse.json({ reverted: [{ path: restored.path, version: restored.version }] });
    },
  ),
];

function ok(page: StoredPage) {
  return HttpResponse.json(pageView(page));
}

function applyEdit(page: StoredPage, edit: Schemas["PageEdit"]) {
  const { replace, append } = edit;
  if (append?.date && append.date > today()) {
    return problem(400, "future_date");
  }
  if (replace && !current(page).content.includes(replace.old)) {
    return problem(422, "no_change");
  }
  if (replace) {
    editPage(page, current(page).content.replace(replace.old, replace.new), author());
  }
  if (append) {
    appendTimeline(
      page,
      { text: append.text, source: append.source ?? me().username, date: append.date ?? today() },
      author(),
    );
  }
  return ok(page);
}
