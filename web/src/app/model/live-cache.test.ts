import { QueryClient } from "@tanstack/react-query";
import { describe, expect, it } from "vitest";

import { pageQuery } from "@/entities/page";

import { createLiveCache, filtersFor } from "./live-cache";

const keysOf = (filters: ReturnType<typeof filtersFor>) => filters.map((filter) => filter.queryKey);

describe("live cache", () => {
  it("refreshes a page only when the change is newer than what is cached", () => {
    const queryClient = new QueryClient();
    const change = {
      kind: "page",
      id: 7,
      version: 3,
      op: "edit",
      path: "notes/a",
      previous_path: null,
    } as const;
    expect(keysOf(filtersFor(queryClient, change))).toContainEqual(["page", 7]);

    queryClient.setQueryData(pageQuery(7).queryKey.slice(), { version: 3 });
    expect(keysOf(filtersFor(queryClient, change))).not.toContainEqual(["page", 7]);
  });

  it("refreshes the path a page moved from and the path it moved to", () => {
    const keys = keysOf(
      filtersFor(new QueryClient(), {
        kind: "page",
        id: 1,
        version: 2,
        op: "move",
        path: "b",
        previous_path: "a",
      }),
    );
    expect(keys).toContainEqual(["target", "a"]);
    expect(keys).toContainEqual(["target", "b"]);
    expect(keys).toContainEqual(["tree"]);
  });

  it("forgets everything about a purged page", () => {
    const keys = keysOf(
      filtersFor(new QueryClient(), { kind: "purge", subject: "page", id: 4, path: "gone" }),
    );
    expect(keys).toContainEqual(["page", 4]);
    expect(keys).toContainEqual(["target", "gone"]);
  });
});

describe("a stream that could not resume", () => {
  it("refreshes memory data but leaves account data alone", async () => {
    const queryClient = new QueryClient();
    queryClient.setQueryData(["tree"], {});
    queryClient.setQueryData(["me"], {});
    createLiveCache(queryClient).onReady({ resumed: false });
    await Promise.resolve();
    expect(queryClient.getQueryState(["tree"])?.isInvalidated).toBe(true);
    expect(queryClient.getQueryState(["me"])?.isInvalidated).toBe(false);
  });
});
