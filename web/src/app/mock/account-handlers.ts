import { http, HttpResponse } from "msw";

import { noContent, problem } from "./respond";
import { me, type Schemas, state } from "./store";

type IdParams = { id: string };

const meView = (): Schemas["Me"] => {
  const { id, username, display_name, is_admin } = me();
  return { id, username, display_name, is_admin };
};

function updateProfile(id: number, update: Schemas["ProfileUpdate"]) {
  const user = state.users.find((entry) => entry.id === id);
  if (!user) {
    return null;
  }
  user.username = update.username ?? user.username;
  user.display_name = update.display_name ?? user.display_name;
  return user;
}

export const accountHandlers = [
  http.get("/api/auth/me", () =>
    state.isSignedIn ? HttpResponse.json(meView()) : problem(401, "unauthenticated"),
  ),

  http.patch<never, Schemas["MeUpdate"]>("/api/auth/me", async ({ request }) => {
    updateProfile(state.meId, await request.json());
    return HttpResponse.json(meView());
  }),

  http.post("/api/auth/logout", () => {
    state.isSignedIn = false;
    return noContent();
  }),

  http.get("/api/auth/passkeys", () => HttpResponse.json(state.passkeys)),

  http.delete<IdParams>("/api/auth/passkeys/:id", ({ params }) => {
    if (state.passkeys.length === 1) {
      return problem(409, "last_passkey");
    }
    state.passkeys = state.passkeys.filter((passkey) => passkey.id !== Number(params.id));
    return noContent();
  }),

  http.get("/api/connections", () => HttpResponse.json(state.connections)),

  http.patch<IdParams, Schemas["ConnectionUpdate"]>(
    "/api/connections/:id",
    async ({ params, request }) => {
      const connection = state.connections.find((entry) => entry.id === Number(params.id));
      if (!connection) {
        return problem(404, "not_found");
      }
      const update = await request.json();
      connection.name = update.name ?? connection.name;
      connection.scopes = update.scopes ?? connection.scopes;
      connection.can_write = update.can_write ?? connection.can_write;
      return HttpResponse.json(connection);
    },
  ),

  http.delete<IdParams>("/api/connections/:id", ({ params }) => {
    state.connections = state.connections.filter((entry) => entry.id !== Number(params.id));
    return noContent();
  }),

  http.get("/api/users", () => HttpResponse.json(state.users)),

  http.patch<IdParams, Schemas["ProfileUpdate"]>(
    "/api/users/:id/profile",
    async ({ params, request }) => {
      const user = updateProfile(Number(params.id), await request.json());
      return user ? HttpResponse.json(user) : problem(404, "not_found");
    },
  ),

  http.get("/api/invites", () => HttpResponse.json(state.invites)),

  http.delete<IdParams>("/api/invites/:id", ({ params }) => {
    state.invites = state.invites.filter((invite) => invite.id !== Number(params.id));
    return noContent();
  }),

  http.get("/api/system", () =>
    HttpResponse.json({
      database: "ok",
      backup: {
        off_host: true,
        problems: [],
        last_backup_age_seconds: 3_600,
        last_full_age_seconds: 86_400 * 3,
        wal_archive_age_seconds: 120,
        blobs_age_seconds: 1_800,
        restore_test: { ok: true, at: new Date(Date.now() - 86_400_000 * 2).toISOString() },
      },
    } satisfies Schemas["SystemStatus"]),
  ),

  http.get("/api/oauth/consents/:reference", () =>
    HttpResponse.json({
      kind: "oauth",
      client: { client_id: "demo", name: "Claude", unverified: false },
      localhost_only: false,
      proposed_name: "claude-ai",
      default_scopes: ["*"],
      default_can_write: true,
      name_matches_redirect: true,
      redirect_host: "claude.ai",
    } satisfies Schemas["ConsentView"]),
  ),
];
