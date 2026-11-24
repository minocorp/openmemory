import { http, HttpResponse } from "msw";

import { revertGroup } from "./page-writes";
import { me, type Schemas, state } from "./store";
import { changesView } from "./views";

export const changeHandlers = [
  http.get("/api/changes", () => HttpResponse.json(changesView())),

  http.post<never, Schemas["SeenRequest"]>("/api/changes/seen", async ({ request }) => {
    const { seq } = await request.json();
    state.lastSeenSeq = Math.max(state.lastSeenSeq, seq);
    return HttpResponse.json({ last_seen_seq: state.lastSeenSeq });
  }),

  http.post<{ group: string }>("/api/changes/:group/revert", ({ params }) => {
    const reverted = revertGroup(Number(params.group), { author: me().username });
    return HttpResponse.json({
      reverted: reverted.map((entry) => ({ path: entry.path, version: entry.version })),
    });
  }),
];
