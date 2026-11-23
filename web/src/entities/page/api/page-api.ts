import { client, unwrap, unwrapEmpty } from "@/shared/api";
import { etagOf } from "@/shared/lib/etag";

export function getPage(id: number) {
  return unwrap(client.GET("/api/pages/{id}", { params: { path: { id } } }));
}

export function getPageVersion(id: number, version: number) {
  return unwrap(client.GET("/api/pages/{id}", { params: { path: { id }, query: { version } } }));
}

export function getPageByPath(path: string) {
  return unwrap(client.GET("/api/pages", { params: { query: { path } } }));
}

export function listVersions(options: { id: number; before?: number | undefined }) {
  return unwrap(
    client.GET("/api/pages/{id}/versions", {
      params: {
        path: { id: options.id },
        query: options.before === undefined ? { limit: 20 } : { limit: 20, before: options.before },
      },
    }),
  );
}

export function createPage(options: { path: string; content: string }) {
  return unwrap(
    client.PUT("/api/pages", {
      params: { query: { path: options.path }, header: { "If-None-Match": "*" } },
      body: { content: options.content },
    }),
  );
}

export function overwritePage(options: { id: number; version: number; content: string }) {
  return unwrap(
    client.PUT("/api/pages/{id}", {
      params: { path: { id: options.id }, header: { "If-Match": etagOf(options.version) } },
      body: { content: options.content },
    }),
  );
}

export function appendTimeline(options: {
  id: number;
  version: number;
  text: string;
  source: string | null;
  date: string | null;
}) {
  return unwrap(
    client.PATCH("/api/pages/{id}", {
      params: { path: { id: options.id }, header: { "If-Match": etagOf(options.version) } },
      body: { append: { text: options.text, source: options.source, date: options.date } },
    }),
  );
}

export function movePage(options: { id: number; version: number; to: string }) {
  return unwrap(
    client.POST("/api/pages/{id}/move", {
      params: { path: { id: options.id }, header: { "If-Match": etagOf(options.version) } },
      body: { to: options.to },
    }),
  );
}

export function deletePage(options: { id: number; version: number }) {
  return unwrapEmpty(
    client.DELETE("/api/pages/{id}", {
      params: { path: { id: options.id }, header: { "If-Match": etagOf(options.version) } },
    }),
  );
}

export function restorePageVersion(options: { id: number; version: number; current: number }) {
  return unwrap(
    client.POST("/api/pages/{id}/versions/{n}/restore", {
      params: {
        path: { id: options.id, n: options.version },
        header: { "If-Match": etagOf(options.current) },
      },
    }),
  );
}
